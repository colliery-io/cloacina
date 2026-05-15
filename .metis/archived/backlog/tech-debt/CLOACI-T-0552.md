---
id: relocate-triggerentry
level: task
title: "Relocate TriggerEntry + TriggerlessGraphEntry to cloacina-workflow-plugin"
short_code: "CLOACI-T-0552"
created_at: 2026-05-03T13:25:59.116563+00:00
updated_at: 2026-05-03T14:12:14.151606+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Relocate TriggerEntry + TriggerlessGraphEntry to cloacina-workflow-plugin

## Objective

I-0102 follow-up. The unified `cloacina::package!()` shell macro currently stubs `get_trigger_metadata` returning `Ok(vec![])` because `TriggerEntry` and `TriggerlessGraphEntry` still live in `crates/cloacina/src/inventory_entries.rs` (engine-only) and aren't reachable from packaged cdylibs. Relocate both to `cloacina-workflow-plugin/src/inventory_entries.rs` (alongside `ReactorEntry`, `TaskEntry`, `ComputationGraphEntry` from I-0102) so the shell can walk inventory at FFI call time and produce real trigger metadata.

This unblocks T-0553 (daemon auto-trigger restoration + trigger-only / mixed-rust fixtures), and any future packaged crate that wants to expose triggers through the unified shell.

## Backlog Item Details

### Type
- [x] Tech Debt — completes deferred I-0102 work

### Priority
- [x] P2 — Medium (nice to have, unblocks follow-up)

### Technical Debt Impact
- **Current Problems**: Shell macro's `get_trigger_metadata` is a stub returning empty Vec. Daemon's automatic trigger registration from packages is gone (loop deleted in T-E). T-D's trigger-only and mixed-rust fixtures can't be built meaningfully — they'd just assert empty trigger metadata.
- **Benefits of Fixing**: Restores trigger metadata flow end-to-end, unblocks daemon auto-registration, enables full T-D fixture coverage.
- **Risk Assessment**: Low — same shape as the I-0102 ReactorEntry / TaskEntry / ComputationGraphEntry relocations that landed cleanly. The hard part is the `Trigger` trait location.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### TriggerEntry relocation

- [ ] `Trigger` trait + `TriggerError` + `TriggerResult` reachable from `cloacina-workflow-plugin`. Two options:
  - **(a)** Move the `Trigger` trait from `cloacina/src/trigger/mod.rs` to `cloacina-workflow` (which already houses `Task`, `Context`, `TaskNamespace` and exports `TriggerError`/`TriggerResult`). Engine wraps if it needs Diesel/Tokio-heavy methods elsewhere.
  - **(b)** Define a parallel "leaf-crate Trigger" trait in `cloacina-workflow-plugin` and have the engine impl wrap it. Heavier coupling but doesn't disturb the engine's existing `Trigger` consumers.
  - Probably (a) — matches how `Task` was handled.
- [ ] `TriggerEntry` struct + `inventory::collect!` moved to `cloacina-workflow-plugin/src/inventory_entries.rs`. Re-exported from `cloacina/src/inventory_entries.rs` (mirrors `TaskEntry`/`ReactorEntry` pattern).
- [ ] `#[trigger]` macro emission paths updated to use `cloacina_workflow_plugin::TriggerEntry` and the relocated `Trigger` trait. Inventory submission un-gated for both packaged and embedded modes (currently gated by `cfg(not(feature = "packaged"))`).

### TriggerlessGraphEntry relocation

- [ ] `TriggerlessGraphRegistration` lives in `cloacina/src/computation_graph/triggerless.rs`. Move to `cloacina-computation-graph` so `cloacina-workflow-plugin` (which already depends on cg) can reference it.
- [ ] `TriggerlessGraphEntry` struct + `inventory::collect!` moved to `cloacina-workflow-plugin/src/inventory_entries.rs`. Re-exported from `cloacina`.
- [ ] `#[computation_graph]` macro's triggerless `ctor` emission updated paths and un-gated.
- [ ] `#[task(invokes = computation_graph("name"))]` runtime walk in `tasks.rs` retargeted at `cloacina_workflow_plugin::TriggerlessGraphEntry`.

### Shell macro completion

- [ ] `cloacina::package!()`'s `get_trigger_metadata` body walks `inventory::iter::<TriggerEntry>`, calls each entry's constructor, queries `Trigger::poll_interval()` / `Trigger::cron_expression()` / `Trigger::allow_concurrent()`, and projects to `Vec<TriggerPackageMetadata>`.

### Test gates

- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green.

## Implementation Notes

### Technical Approach

1. Decide on (a) vs (b) for the `Trigger` trait. Probably (a). Move trait + dependent types to `cloacina-workflow`. Update `cloacina/src/trigger/mod.rs` to re-export.
2. Same pattern for `TriggerlessGraphRegistration`: move to `cloacina-computation-graph`, re-export from `cloacina`.
3. Move both entry struct definitions into `cloacina-workflow-plugin/src/inventory_entries.rs`.
4. Update macro emissions and un-gate inventory submission for packaged builds.
5. Replace shell stub bodies with real inventory walks.
6. `cargo check --workspace --all-features`, run unit + integration tests.

### Dependencies

- I-0102 closure (T-0546–T-0551) — done.

### Risk Considerations

- **Trigger trait surface.** If the trait carries methods that depend on engine-only types, split them between leaf and engine carefully. Today's `Trigger` is `async_trait` with `poll()` and a few config methods — should fit `cloacina-workflow` cleanly.
- **Packaged mode tokio.** Trigger constructors may allocate tokio handles. Must work inside the cdylib's package-shell tokio runtime (same constraint hit for `Task` in I-0102; resolved via `cloacina_workflow::__private::tokio`).

## Status Updates

### 2026-05-03 — T-0552 done in one landing

Picked option (a) — Trigger trait moved wholesale to cloacina-workflow with leaf-crate types. All four test gates green; packaged fixtures (reactor-only-rust, reactor-subscriber-rust) build clean.

**Changes shipped:**

- `cloacina-workflow/src/trigger.rs`:
  - `Trigger` trait moved here. Uses `cloacina_workflow::TriggerResult` and `cloacina_workflow::TriggerError`.
  - `TriggerResult` gained `should_fire()` / `into_context()` / `context_hash()` helpers (previously engine-only).
  - `TriggerError::PollError` switched from tuple variant `(String)` to struct variant `{ message: String }` to match Python bindings' existing call sites — fewer churn at consumer sites.
- `cloacina-workflow/src/lib.rs`: re-exports `Trigger`.
- `cloacina/src/trigger/mod.rs`: deleted local trait + TriggerResult duplicate; re-exports `cloacina_workflow::Trigger` and `cloacina_workflow::TriggerResult`. Engine `TriggerError` (with Diesel/connection-pool variants) stays separate; existing `From<cloacina_workflow::TriggerError>` impl bridges the two at consumer sites.
- `cloacina-workflow-plugin/src/inventory_entries.rs`: added `TriggerEntry`, `TriggerlessGraphEntry`, `TriggerlessGraphRegistration`, `TriggerlessGraphFn`, `TriggerlessGraph` trait. `TriggerlessGraph*` types relocated from `cloacina/src/computation_graph/triggerless.rs`.
- `cloacina-workflow-plugin/src/lib.rs`: re-exports the new types.
- `cloacina/src/inventory_entries.rs`: deleted local `TriggerEntry` and `TriggerlessGraphEntry`; re-exports from cloacina-workflow-plugin. `WorkflowEntry` stays (engine-only `Workflow` type).
- `cloacina/src/computation_graph/triggerless.rs`: reduced to re-exports from cloacina-workflow-plugin.
- `cloacina-macros/src/trigger_attr.rs`:
  - Custom-poll trigger codegen targets `cloacina_workflow::Trigger` and submits to `cloacina_workflow_plugin::TriggerEntry`. Inventory submission un-gated for both packaged and embedded modes.
  - Cron trigger codegen does the same. Cron impls now also override `cron_expression()` to return `Some(expr)` so the reconciler can route them appropriately (per T-B's design).
- `cloacina-macros/src/computation_graph/codegen.rs`:
  - `TriggerlessGraphEntry` submission targets `cloacina_workflow_plugin` paths and is un-gated.
  - `TriggerlessGraph` trait impl on `__CGHandle_<mod>` targets `cloacina_workflow_plugin::TriggerlessGraph`.
- `cloacina-macros/src/tasks.rs`: `#[task(invokes = computation_graph("name"))]` runtime walk targets `cloacina_workflow_plugin::TriggerlessGraphEntry`.
- `cloacina-python/src/trigger.rs` and `bindings/trigger.rs`: switched `TriggerError`/`TriggerResult` imports from engine to `cloacina_workflow`. Trait impl is now `cloacina::Trigger` (re-export).
- `cloacina-workflow-plugin/src/lib.rs` shell macro: `get_trigger_metadata` now walks `inventory::iter::<TriggerEntry>`, calls each constructor, and projects to `TriggerPackageMetadata` (name, package_name, poll_interval, cron_expression, allow_concurrent).
- Engine test fixtures (`DummyTrigger` in `loading.rs`, `TestTrigger` in `trigger/mod.rs`) updated to return `cloacina_workflow::TriggerError`.

**Test gates (all green):**

- [x] `cargo check --workspace --all-features`
- [x] `angreal test unit` (701 passed)
- [x] `angreal test integration --backend sqlite` (6 + 28 Python)
- [x] `angreal test integration --backend postgres` (290 + 28 Python)
- [x] Packaged fixtures (`reactor-only-rust`, `reactor-subscriber-rust`) build clean as cdylibs.

T-0552 complete. T-0553 (daemon auto-trigger restoration + trigger-only/mixed-rust fixtures) is now unblocked.
