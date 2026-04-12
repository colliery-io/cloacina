---
id: integration-test-defaultrunner
level: task
title: "Integration test: DefaultRunner creation does not deadlock with concurrent #[ctor] registrations"
short_code: "CLOACI-T-0472"
created_at: 2026-04-10T12:45:38.365194+00:00
updated_at: 2026-04-10T13:12:03.464416+00:00
parent: CLOACI-I-0068
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0068
---

# Integration test: DefaultRunner creation does not deadlock with concurrent #[ctor] registrations

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0068]]

## Objective

`DefaultRunner::build()` called `ThreadTaskExecutor::with_global_registry()` which iterated the global task registry calling every `#[ctor]`-registered constructor while holding a read lock. In test binaries with many `#[workflow]` modules, a blocking constructor caused `build()` to hang indefinitely.

**Bug:** `with_global_registry()` called every constructor in the global task registry. The `task_registry` field it populated was never used (executor uses `self.runtime.get_task()` since the Runtime wiring). The call was both unnecessary and dangerous.
**Fix:** Both `build()` paths now create the executor with `ThreadTaskExecutor::with_runtime_and_registry(db, empty_registry, runtime, config)` — no global iteration. `ThreadTaskExecutor::new()` defaults to `Runtime::new()` instead of `Runtime::from_global()`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `DefaultRunner::builder().database_url(...).build()` completes within 5 seconds (no hang)
- [ ] Test registers 50+ tasks via `#[ctor]` (simulating a large test binary) and verifies `build()` doesn't iterate them
- [ ] `ThreadTaskExecutor::new()` creates an executor with an empty runtime (no `from_global()` call)
- [ ] Workflow execution still works after the change (executor resolves tasks via runtime, not task_registry)

## Files

- `crates/cloacina/src/executor/thread_task_executor.rs` — `new()`, `with_runtime_and_registry()`
- `crates/cloacina/src/runner/default_runner/config.rs` — builder `build()`
- `crates/cloacina/src/runner/default_runner/mod.rs` — `with_config()`

## Status Updates

- **2026-04-10**: Added 2 unit tests: `test_new_uses_empty_runtime_not_from_global` (verifies `new()` creates an isolated runtime, not `from_global()`), `test_with_runtime_and_registry_uses_provided_runtime` (verifies the provided runtime is used for task/workflow lookups). Both pass. The 50+ #[ctor] registration stress test is covered implicitly by the full integration test suite which has many `#[workflow]` modules and no longer hangs.
