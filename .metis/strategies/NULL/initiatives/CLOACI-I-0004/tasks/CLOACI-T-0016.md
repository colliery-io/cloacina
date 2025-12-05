---
id: replace-unsafe-dereferences-with
level: task
title: "Replace unsafe dereferences with validated versions"
short_code: "CLOACI-T-0016"
created_at: 2025-12-05T22:35:47.174554+00:00
updated_at: 2025-12-05T22:47:14.804132+00:00
parent: CLOACI-I-0004
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0004
---

# Replace unsafe dereferences with validated versions

## Parent Initiative

[[CLOACI-I-0004]]

## Objective

Replace all raw pointer dereferences in `extract_task_info_and_graph_from_library()` with calls to the validation helper functions created in CLOACI-T-0015. This eliminates undefined behavior from null or misaligned pointer access.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `package_tasks_ptr` dereference (line 183) uses `validate_package_tasks_ptr()`
- [ ] Task slice creation (lines 209-210) uses `validate_task_slice()`
- [ ] All pointer validation happens before any dereference
- [ ] Function signature updated to return `Result<..., ManifestError>` internally
- [ ] Errors are properly propagated up to callers via anyhow
- [ ] Code compiles without warnings
- [ ] Existing tests still pass

## Implementation Notes

### Location
`crates/cloacina/src/packaging/manifest.rs` - function `extract_task_info_and_graph_from_library()`

### Changes Required

1. **Line 183** - Replace:
   ```rust
   let package_tasks = unsafe { &*package_tasks_ptr };
   ```
   With validated version using `validate_package_tasks_ptr()`

2. **Lines 209-210** - Replace:
   ```rust
   let tasks_slice = unsafe {
       std::slice::from_raw_parts(package_tasks.tasks, package_tasks.task_count as usize)
   };
   ```
   With `validate_task_slice()`

3. Update error handling to convert `ManifestError` to `anyhow::Error` at the function boundary

### Dependencies
- CLOACI-T-0015 (validation helpers must exist first)

## Status Updates

*To be added during implementation*
