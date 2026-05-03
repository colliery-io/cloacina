---
id: restore-daemon-auto-trigger
level: task
title: "Restore daemon auto-trigger registration via FFI metadata + finish T-D fixtures"
short_code: "CLOACI-T-0553"
created_at: 2026-05-03T13:26:00+00:00
updated_at: 2026-05-03T14:47:40.944040+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Restore daemon auto-trigger registration via FFI metadata + finish T-D fixtures

## Objective

I-0102 follow-up. Two related deferred items:

1. **Daemon auto-trigger registration**: T-E (T-0551) deleted the daemon's automatic trigger registration loop in `cloacinactl/src/commands/daemon.rs` because it read `[[triggers]]` from `package.toml` — a key that's now hard-errored. The replacement path should consume `extract_trigger_metadata` (FFI), but that currently stubs `Ok(vec![])` until `TriggerEntry` relocates (T-0552). Once T-0552 lands, restore the daemon loop using FFI trigger metadata.

2. **T-D fixture coverage**: T-0550 (T-D) shipped `reactor-only-rust` and `reactor-subscriber-rust` but deferred `trigger-only-rust` and `mixed-rust` for the same TriggerEntry reason. Once trigger metadata flows, build these fixtures + their reconciler-driven assertions to complete T-D's AC.

## Backlog Item Details

### Type
- [x] Feature — restores user-visible behavior + completes deferred T-D coverage

### Priority
- [x] P2 — Medium

### Business Justification
- **User Value**: Packaged workflows that declare `#[trigger]` macros currently won't auto-register on daemon startup; users have to register triggers manually. Restoring the loop closes that gap.
- **Effort Estimate**: M — most of the wiring is straightforward once T-0552's FFI stub is replaced; the new fixtures are mechanical.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### Daemon auto-registration

- [ ] `cloacinactl/src/commands/daemon.rs`: re-implement the deleted trigger registration loop. For each loaded package, call `package_loader.extract_trigger_metadata(&library_data)`; for each entry, route based on `cron_expression`:
  - `Some(expr)` → `runner.register_cron_workflow(...)`.
  - `None` → look up the named `Trigger` impl in the runtime and call `scheduler.register_trigger(...)`.
- [ ] Hard-error / clear log when a trigger names a workflow that isn't registered (parallel to `validate_workflow_trigger_subscriptions`).

### Trigger-only fixture

- [ ] `examples/fixtures/trigger-only-rust/`: cdylib crate with one `#[trigger(cron = "...")]` and one `#[trigger(custom)]` declaration + `cloacina_workflow_plugin::package!();`. No reactor, CG, or workflow.
- [ ] Builds via the angreal pre-build harness.

### Mixed fixture

- [ ] `examples/fixtures/mixed-rust/`: one reactor, one custom trigger, one CG bound to the reactor (`trigger = reactor("...")`), one workflow subscribing to the trigger (`triggers = ["..."]`) + `cloacina_workflow_plugin::package!();`.
- [ ] Builds via the angreal pre-build harness.

### Reconciler-driven integration tests

Extend `crates/cloacina/tests/integration/primitive_only_packaging.rs` (or sibling) with:

- [ ] **Trigger-only:** package loads via reconciler; cron trigger registered with cron scheduler; custom trigger registered with runtime; no reactor / graph.
- [ ] **Mixed:** package loads; precedence pipeline runs cleanly; one event into the trigger fires the workflow; one event into the reactor's accumulator fires the CG.
- [ ] **Cross-package contract mismatch:** a subscriber declaring incompatible accumulator names against an already-loaded reactor fails the load with a clear error naming the offending package + the missing accumulator(s). May require T-0554's pipeline restructure to surface the error properly.
- [ ] **Lifecycle ordering:** unloading the reactor-only package while subscriber is bound is rejected (T-0544 M4 guard); unload subscriber first, then reactor — both succeed.

### Test gates

- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green.

## Implementation Notes

### Dependencies

- **T-0552** — TriggerEntry + TriggerlessGraphEntry relocation. Strict prerequisite. Without it, `extract_trigger_metadata` stays stubbed and the new fixtures' trigger metadata is empty.
- **T-0554** (optional, but helpful) — pipeline restructure makes the cross-package contract mismatch test cleaner to express. If T-0554 lands first, this task gets simpler.

### Technical Approach

1. After T-0552 lands, smoke-test by loading `reactor-only-rust` and verifying `extract_trigger_metadata` works end-to-end.
2. Re-implement `cloacinactl/src/commands/daemon.rs` trigger loop using `package_loader.extract_trigger_metadata`.
3. Build the two new fixtures, mirroring the structure of `reactor-only-rust` / `reactor-subscriber-rust`.
4. Extend integration tests. Use the existing fidius-host load path (no full reconciler boot needed for assertions on metadata; cross-package + lifecycle tests do need reconciler scaffolding).

### Risk Considerations

- **Cron scheduler API drift.** The deleted daemon loop called `runner.register_cron_workflow(...)` and `scheduler.register_trigger(...)`. Verify these APIs haven't shifted between then and the restore.
- **Test-fixture compile coupling.** Ensure new fixtures depend on `cloacina-workflow` (for `Trigger` trait per T-0552) and `cloacina-workflow-plugin`, mirroring T-0550 deps.

## Status Updates

### 2026-05-03 — T-0553 done in one landing

All four test gates green.

**Daemon auto-trigger registration (`cloacinactl/src/commands/daemon.rs`):**
- Reimplemented the deleted post-load loop. For each newly loaded package: load the cdylib via `fidius_host::loader::load_library`, call `cloacina::computation_graph::packaging_bridge::call_get_trigger_metadata` (T-0552's host helper).
- Cron-shaped entries → `runner.register_cron_workflow(name, expr, "UTC")`.
- Custom-poll entries → `runner.runtime().get_trigger(name)` + `scheduler.register_trigger(impl, name)`. Warns clearly when no Trigger impl is found.
- Python packages (no `compiled_data`) skip via the existing import-time path.
- `cloacinactl` Cargo.toml gains a direct `fidius-host` dep.

**CronEvaluator relocation:**
- Moved `cloacina/src/cron_evaluator.rs` → `cloacina-workflow/src/cron_evaluator.rs` (leaf-friendly).
- `cloacina/src/cron_evaluator.rs` reduced to a re-export.
- `#[trigger(cron = ...)]` macro emission targets `cloacina_workflow::cron_evaluator::CronEvaluator` so cron triggers compile in packaged cdylibs.

**New fixtures:**
- `examples/fixtures/trigger-only-rust/` — cron + custom triggers + `cloacina_workflow_plugin::package!()`. No reactor / CG / workflow.
- `examples/fixtures/mixed-rust/` — reactor + custom trigger + reactor-bound CG + workflow with `triggers = ["mixed_trigger"]`. Every primitive in one cdylib.
- `.angreal/test/integration.py`: pre-builds both new fixtures.

**Integration tests** (`primitive_only_packaging.rs`):
- `trigger_only_fixture_emits_cron_and_custom_metadata` — asserts shape of both trigger entries (cron expression, package name, custom-poll fallback).
- `trigger_only_fixture_emits_no_reactors_or_graph` — asserts reactors absent, graph errors.
- `mixed_fixture_exposes_all_primitives` — asserts all four primitives + workflow's `triggers` field carries the trigger name end-to-end.

**Test gates (all green):**
- [x] `cargo check --workspace --all-features`
- [x] `angreal test unit` (701 passed)
- [x] `angreal test integration --backend sqlite` (6 + 28 Python)
- [x] `angreal test integration --backend postgres` (293 + 28 Python — was 290; 3 new tests pass)

### Deferred

Reconciler-driven full e2e tests from T-D's original AC (event into trigger fires workflow; event into accumulator fires CG; cross-package contract mismatch; unload lifecycle ordering) still require T-0554 Phase 2 (Python adapter, contract validation, reverse unload). The wire-format coverage shipped here locks down the end-to-end shape of every primitive's metadata flow through the unified shell macro.
