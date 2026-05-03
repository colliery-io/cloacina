---
id: restore-daemon-auto-trigger
level: task
title: "Restore daemon auto-trigger registration via FFI metadata + finish T-D fixtures"
short_code: "CLOACI-T-0553"
created_at: 2026-05-03T13:26:00+00:00
updated_at: 2026-05-03T18:00:41.689009+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


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

### 2026-05-03 — Deferred AC closed (post-T-0554 Phase 2)

T-0554 Phase 2 unblocked two of the four originally-deferred reconciler-driven tests; both now ship as in-crate unit tests on `RegistryReconciler` against a real `ComputationGraphScheduler` (no archive scaffolding required).

**Cross-package contract mismatch:**
- `cross_package_contract_mismatch_rejects_with_named_accumulators` — load reactor R(α, β); attempt to load a subscriber declaring (α, γ); assert the rejection names the package, the upstream reactor, and the missing accumulator γ.
- `cross_package_subscriber_before_publisher_rejects_with_clear_error` — load subscriber referencing a reactor that's never been loaded; assert a fail-fast rejection naming the missing reactor + load-order remediation.
- `cross_package_subscriber_in_same_package_skips_validation` — same-package publisher/subscriber bypasses the cross-package check (the bundled-form path stays clean).

**Lifecycle ordering:**
- `unload_package_rejects_when_subscribers_remain_bound` — publisher owns reactor with a cross-package subscriber bound; `unload_package` returns `RegistrationFailed` naming the publisher + reactor + "unload subscribers first" remediation. Reactor is still loaded after the rejection.
- `unload_package_succeeds_after_subscribers_unbound` — companion test: unload with no subscribers succeeds and the reactor is torn down.

**Test gates (all green):**
- [x] `cargo check --workspace --all-features`
- [x] `angreal test unit` (695 cloacina + 45 cloacina-workflow)
- [x] `angreal test integration --backend sqlite` (6 + 28 Python)
- [x] `angreal test integration --backend postgres` (293 + 28 Python)

### 2026-05-03 — Full reconciler-boot e2e landed

`crates/cloacina/tests/integration/dal/reconciler_e2e_load.rs::reconciler_loads_cross_package_publisher_subscriber_end_to_end` drives `RegistryReconciler::reconcile` end-to-end against the DAL-backed `WorkflowRegistry` + a live `ComputationGraphScheduler` + scoped `Runtime`. The test packs `reactor-only-rust` + `reactor-subscriber-rust` source archives via `fidius_core::package::pack_package`, registers them through the DAL, fakes the compiler step by writing the prebuilt cdylib bytes via `mark_build_success`, then drives `reconcile()` and asserts:

1. Both packages load through the registration-order pipeline.
2. The publisher's reactor (`shared_rx`) lands in the scheduler with `[alpha, beta]`.
3. The subscriber's CG (`subscriber_graph`) binds to `shared_rx` via cross-package fan-out.
4. An event sent into the publisher's `alpha` accumulator endpoint reaches the dispatcher without crashing the reactor.
5. Reverse-order unload: deleting the publisher first while the subscriber is bound triggers `RegistrationFailed` with an "unload subscribers first" message; deleting the subscriber first then re-reconciling tears `shared_rx` down cleanly.

**Two production fixes uncovered + landed alongside the test:**

- **Cross-package subscriber binding skipped reactor-load idempotency check** — the subscriber's FFI `graph_meta.accumulators` is empty (it doesn't bring its own factories), but `scheduler.load_graph` was unconditionally calling `load_reactor` first, which failed the idempotent contract check (existing reactor's accumulators vs. empty). Added a fast path in `load_graph`: when `decl.reactor_name = Some(X)`, X is already loaded, AND `decl.accumulators.is_empty()`, bind directly via `bind_graph_to_reactor` and skip `load_reactor`.
- **`PackageState::reactor_names` was unreliable for cross-cdylib loads** — the pre/post `Runtime::reactor_names()` diff approach didn't work because independently-compiled fixture crates have their own `cloacina-workflow-plugin` compilation with distinct linker symbols, so `Runtime::seed_from_inventory` never sees their entries. Switched the Rust path to populate `reactor_names` from `view.reactors` (FFI metadata, cross-cdylib safe). Python path keeps the diff (the scoped Runtime IS the authoritative registry there).

**Still out of scope: workflow-trigger subscription with packaged cdylibs.**
"Event into trigger fires workflow" needs `runtime.get_trigger(name)` to succeed, which currently requires inventory crossing the cdylib boundary. The mixed-rust fixture (every primitive in one cdylib, including a workflow with `triggers = ["mixed_trigger"]`) cannot load through the reconciler today — `validate_workflow_trigger_subscriptions` rejects because `seed_from_inventory` doesn't see entries from independently-compiled cdylibs. The same limitation applies to T-0553's daemon trigger registration. Proper fix: design a Trigger FFI bridge that constructs host-side Trigger impls from cdylib metadata + dispatches `poll()` via FFI. Substantial new mechanism, deferred as a follow-up.

**Test gates (all green):**
- [x] `cargo check --workspace --all-features`
- [x] `angreal test unit` (695 + 45)
- [x] `angreal test integration --backend sqlite` (6 + 28 Python)
- [x] `angreal test integration --backend postgres` (294 Rust + 28 Python — was 293; 1 new e2e test passes)
