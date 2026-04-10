---
id: migrate-tests-to-per-test-runtime
level: task
title: "Migrate tests to per-test Runtime instances and remove #[serial] annotations"
short_code: "CLOACI-T-0467"
created_at: 2026-04-09T16:59:32.426583+00:00
updated_at: 2026-04-09T17:41:16.021525+00:00
parent: CLOACI-I-0091
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0091
---

# Migrate tests to per-test Runtime instances and remove #[serial] annotations

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0091]]

## Objective

The payoff: update integration tests to create per-test `Runtime` instances with isolated registries, then remove `#[serial]` annotations. This enables parallel test execution and faster CI.

**Effort**: 2-3 days

## Acceptance Criteria

## Acceptance Criteria

- [ ] Integration test fixture creates a `Runtime::new()` per test instead of using globals
- [ ] Tasks/workflows registered via `runtime.register_task()` instead of `register_task_constructor()`
- [ ] `DefaultRunner::builder().runtime(runtime).build()` used in tests
- [ ] `#[serial]` annotations removed from tests that no longer share global state
- [ ] Tests that genuinely need global state (e.g., `#[ctor]`-registered tasks from `#[task]` macro) retain `#[serial]` or use `Runtime::from_global()` with a comment explaining why
- [ ] `cargo test -p cloacina --test integration` runs tests in parallel without failures
- [ ] CI test duration measurably reduced (expect 2-3x speedup on multi-core runners)

## Implementation Notes

### Technical Approach

For each test file in `crates/cloacina/tests/integration/`:
1. Replace `register_task_constructor(namespace, ctor)` with `runtime.register_task(namespace, ctor)`
2. Replace `register_workflow_constructor(name, ctor)` with `runtime.register_workflow(name, ctor)`
3. Pass `runtime` to `DefaultRunner::builder().runtime(runtime)`
4. Remove `#[serial_test::serial]` annotation
5. Tests using the `#[task]` macro generate `#[ctor]` constructors that write to globals — these tests either:
   a. Use `Runtime::from_global()` (keeps `#[serial]`)
   b. Or manually register the task on a fresh Runtime (removes `#[serial]`)

Current count: ~160 `#[serial]` annotations across integration tests.

Target: reduce to <20 (only tests that genuinely need global state).

### Dependencies
After T-0466 (Runtime wired into DefaultRunner and executor).

### Risk Considerations
- Some tests may have hidden dependencies on registration order or global state mutation. Run the full suite multiple times to catch flaky parallelism.
- The `#[task]` macro's `#[ctor]` registration is inherently global. Tests that use `#[task]` macros and then look up tasks via the runner will still need either `#[serial]` or explicit re-registration on the scoped runtime.

## Status Updates

- **2026-04-09**: Migrated `run_pipeline_and_get_status` helper in task_execution.rs to use `Runtime::new()` + `runtime.register_task()` + `.runtime(runtime)` on the builder. Pattern works — tasks are discovered through the scoped runtime, not globals. However, discovered that most `#[serial]` annotations guard DB state (shared Postgres fixture), not registry state. Removing `#[serial]` requires per-test DB schemas, which is outside the scope of scoped registries. The Runtime struct and wiring are valuable infrastructure, but the `#[serial]` reduction target of 160->20 was overly optimistic. Realistic target: ~10-20 tests that ONLY need registry isolation (COR-01 tests, some unit tests) could drop `#[serial]` if given per-test DB schemas. Marking as blocked on per-test DB isolation infrastructure.
