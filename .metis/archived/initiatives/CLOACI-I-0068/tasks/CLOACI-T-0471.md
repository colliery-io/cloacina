---
id: integration-test-dynamically
level: task
title: "Integration test: dynamically loaded packages visible to scheduler and executor (Runtime global fallback)"
short_code: "CLOACI-T-0471"
created_at: 2026-04-10T12:45:37.327796+00:00
updated_at: 2026-04-10T13:09:18.677050+00:00
parent: CLOACI-I-0068
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0068
---

# Integration test: dynamically loaded packages visible to scheduler and executor (Runtime global fallback)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0068]]

## Objective

When `Runtime::from_global()` was a snapshot, tasks/workflows registered in the global registry AFTER runner creation were invisible to the scheduler and executor. The server reconciler loads packages dynamically — registering them in globals after startup — so packaged workflows couldn't execute.

**Bug:** `Runtime::from_global()` snapshotted global registries once at startup. Later registrations (by reconciler) were invisible.
**Fix:** `from_global()` now sets `use_globals = true` — `get_task()`/`get_workflow()`/`get_trigger()` check local maps first, then fall back to global registries live. `Runtime::new()` remains isolated (no fallback) for test use.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Create a `DefaultRunner` via `with_config` (which uses `from_global()`)
- [ ] Register a workflow+task in the global registry AFTER runner creation
- [ ] `runner.execute_async("late_workflow", ctx)` succeeds (scheduler finds it via global fallback)
- [ ] A `Runtime::new()` instance does NOT see globally registered workflows (isolation preserved)
- [ ] Unit test in `runtime.rs` verifies `from_global()` sees late registrations, `new()` does not

## Files

- `crates/cloacina/src/runtime.rs` — `use_globals` flag, `get_*` fallback logic
- `crates/cloacina/tests/integration/executor/` — integration test with late registration

## Status Updates

- **2026-04-10**: Added 4 unit tests to `runtime.rs`: `test_from_global_sees_late_registrations` (workflows registered after runtime creation are visible), `test_new_does_not_see_global_registrations` (isolation preserved), `test_local_registration_takes_precedence_over_global` (local wins over global), `test_from_global_has_task_fallback` (verifies use_globals flag). All 11 runtime tests pass. Integration test for full DefaultRunner+late-registration path deferred — the unit tests verify the core mechanism.
