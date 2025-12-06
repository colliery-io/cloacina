---
id: add-tests-for-refactored-dal
level: task
title: "Add tests for refactored DAL methods"
short_code: "CLOACI-T-0024"
created_at: 2025-12-06T02:46:35.687012+00:00
updated_at: 2025-12-06T02:46:35.687012+00:00
parent: CLOACI-I-0006
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0006
---

# Add tests for refactored DAL methods

## Parent Initiative

[[CLOACI-I-0006]]

## Objective

Verify that the refactored DAL methods maintain correct behavior after eliminating N+1 query patterns. Run existing tests and add any missing coverage for the affected methods.

## Acceptance Criteria

- [ ] All existing task_execution DAL tests pass
- [ ] `cargo test --features sqlite` passes
- [ ] `cargo test --features postgres` passes (if postgres available)
- [ ] Verify test coverage exists for: create, mark_ready, mark_skipped, reset_task_for_recovery
- [ ] Add tests if any gaps in coverage are found

## Test Cases

### Test Case 1: create method returns correct TaskExecution
- **Test ID**: TC-001
- **Preconditions**: Database initialized with unified schema
- **Steps**:
  1. Create a pipeline execution
  2. Call `TaskExecutionDAL::create()` with valid task data
  3. Verify returned TaskExecution has correct fields
- **Expected Results**: Returned task has all fields populated correctly including generated id, timestamps
- **Status**: Existing tests should cover this

### Test Case 2: mark_ready transitions task state correctly
- **Test ID**: TC-002
- **Preconditions**: Task in "Pending" status exists
- **Steps**:
  1. Call `mark_ready()` on the task
  2. Query task from database
- **Expected Results**: Task status is "Ready", updated_at is updated
- **Status**: Existing tests should cover this

### Test Case 3: mark_skipped transitions task state correctly
- **Test ID**: TC-003
- **Preconditions**: Task exists in database
- **Steps**:
  1. Call `mark_skipped()` on the task with a reason
  2. Query task from database
- **Expected Results**: Task status is "Skipped", skip_reason is set, updated_at is updated
- **Status**: Existing tests should cover this

### Test Case 4: reset_task_for_recovery increments recovery_attempts
- **Test ID**: TC-004
- **Preconditions**: Task in "Running" status exists
- **Steps**:
  1. Note current recovery_attempts value
  2. Call `reset_task_for_recovery()` on the task
  3. Query task from database
- **Expected Results**: recovery_attempts is incremented by 1, status is "Ready", last_recovery_at is set
- **Status**: Existing tests should cover this

## Implementation Notes

### Technical Approach

1. Run existing test suite to establish baseline
2. Review test coverage for affected methods in `crates/cloacina/src/dal/unified/task_execution.rs`
3. If gaps exist, add focused tests for the refactored methods
4. Tests should verify behavior, not implementation details (don't test query count directly)

### Dependencies

- Depends on T-0022 and T-0023 being complete (the refactoring)

### Risk Considerations

- Low risk: This is validation work
- If tests fail after refactoring, it indicates a bug in the refactored code

## Status Updates

*To be added during implementation*
