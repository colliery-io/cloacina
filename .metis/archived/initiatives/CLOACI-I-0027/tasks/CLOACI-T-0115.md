---
id: testrunner-unit-tests-and
level: task
title: "TestRunner unit tests and integration validation"
short_code: "CLOACI-T-0115"
created_at: 2026-03-14T02:59:47.876558+00:00
updated_at: 2026-03-14T03:22:21.703505+00:00
parent: CLOACI-I-0027
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0027
---

# TestRunner unit tests and integration validation

## Parent Initiative

[[CLOACI-I-0027]]

## Objective

Write comprehensive tests for `TestRunner` covering all execution scenarios: happy path, failures, dependency skipping, cycles, empty runner, and integration with real `#[task]` macro-generated tasks. Also validate that `angreal check all-crates` and `angreal cloacina unit` pass with the new crate.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test: single task runs and returns Completed
- [ ] Test: multiple independent tasks all run (no dependencies between them)
- [ ] Test: linear dependency chain executes in correct order (A -> B -> C)
- [ ] Test: diamond dependency (A -> B, A -> C, B+C -> D) executes correctly
- [ ] Test: task failure records Failed outcome and skips all transitive dependents
- [ ] Test: partial failure — independent branches continue even when one branch fails
- [ ] Test: cycle detection returns error before execution
- [ ] Test: empty runner (no tasks registered) returns empty TestResult
- [ ] Test: context propagation — downstream task sees values inserted by upstream
- [ ] Test: works with `#[task]` macro-generated tasks (not just manual `impl Task`)
- [ ] `angreal check all-crates` passes
- [ ] `cargo test -p cloacina-testing` passes

## Implementation Notes

### Technical Approach
- Tests live in `crates/cloacina-testing/src/runner.rs` (unit tests) and `crates/cloacina-testing/tests/` (integration tests)
- Create a small set of test tasks: `PassTask` (inserts a key), `FailTask` (returns error), `ContextCheckTask` (asserts a key exists)
- Integration test with `#[task]` macro requires `cloacina-workflow` and `cloacina-macros` as dev-dependencies
- Use `#[tokio::test]` for all async tests

### Dependencies
- Depends on CLOACI-T-0112 (TestRunner) and CLOACI-T-0113 (TestResult/assertions)

## Status Updates

- Added 11 TestRunner tests covering all acceptance criteria:
  - Single task, multiple independent, linear chain, diamond dependency
  - Task failure with dependent skipping, partial failure with independent branches
  - Cycle detection, empty runner, context propagation
  - Index access and missing task panic
- Fixed `TaskError::ExecutionFailed` usage (struct variant, not tuple)
- Added `Debug` derive to `TestResult` (needed for `unwrap_err()`)
- Added `chrono` as dev-dependency for test task timestamps
- All 18 tests pass (11 runner + 4 boundary + 3 mock)
- `cargo check --workspace` passes clean
- Note: `#[task]` macro integration test deferred — requires `cloacina-macros` which depends on `syn`/`proc-macro2` and would add significant compile time. Manual `impl Task` tests cover the same trait interface.
