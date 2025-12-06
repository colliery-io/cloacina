---
id: add-test-coverage-for-malformed
level: task
title: "Add test coverage for malformed FFI inputs"
short_code: "CLOACI-T-0018"
created_at: 2025-12-05T22:35:47.387100+00:00
updated_at: 2025-12-05T22:51:40.391324+00:00
parent: CLOACI-I-0004
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0004
---

# Add test coverage for malformed FFI inputs

## Parent Initiative

[[CLOACI-I-0004]]

## Objective

Add unit tests for the FFI validation helpers and error handling to ensure proper behavior with malformed inputs. Tests should verify that invalid pointers, corrupt data, and edge cases produce appropriate errors rather than undefined behavior.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Tests for `safe_cstr_to_string()` with null pointer
- [ ] Tests for `safe_cstr_to_option_string()` with null and valid pointers
- [ ] Tests for `validate_package_tasks_ptr()` with null pointer
- [ ] Tests for `validate_task_slice()` with null pointer and non-zero count
- [ ] Tests for `validate_task_slice()` with count exceeding MAX_TASKS
- [ ] Tests verify correct `ManifestError` variants are returned
- [ ] All new tests pass
- [ ] Existing tests still pass

## Test Cases

### Test Case 1: Null C String Pointer
- **Test ID**: TC-001
- **Function**: `safe_cstr_to_string()`
- **Input**: null pointer, field name "test_field"
- **Expected**: `Err(ManifestError::NullString { field: "test_field" })`

### Test Case 2: Valid C String
- **Test ID**: TC-002
- **Function**: `safe_cstr_to_string()`
- **Input**: pointer to "hello\0", field name "test"
- **Expected**: `Ok("hello".to_string())`

### Test Case 3: Null Optional String
- **Test ID**: TC-003
- **Function**: `safe_cstr_to_option_string()`
- **Input**: null pointer
- **Expected**: `Ok(None)`

### Test Case 4: Null Package Tasks Pointer
- **Test ID**: TC-004
- **Function**: `validate_package_tasks_ptr()`
- **Input**: null pointer
- **Expected**: `Err(ManifestError::NullPointer { field: "package_tasks" })`

### Test Case 5: Null Task Slice with Non-Zero Count
- **Test ID**: TC-005
- **Function**: `validate_task_slice()`
- **Input**: null pointer, count = 5
- **Expected**: `Err(ManifestError::NullTaskSlice { count: 5 })`

### Test Case 6: Task Count Exceeds Maximum
- **Test ID**: TC-006
- **Function**: `validate_task_slice()`
- **Input**: valid pointer, count = MAX_TASKS + 1
- **Expected**: `Err(ManifestError::TooManyTasks { ... })`

### Test Case 7: Empty Task Slice
- **Test ID**: TC-007
- **Function**: `validate_task_slice()`
- **Input**: null pointer, count = 0
- **Expected**: `Ok(&[])` (empty slice is valid)

## Implementation Notes

### Location
`crates/cloacina/src/packaging/tests.rs` or inline in `manifest.rs` under `#[cfg(test)]`

### Approach
- Use `std::ffi::CString` to create test C strings
- Use `std::ptr::null()` for null pointer tests
- Test both success and error paths for each helper

### Dependencies
- CLOACI-T-0015, CLOACI-T-0016, CLOACI-T-0017 (implementation must be complete)

## Status Updates

*To be added during implementation*
