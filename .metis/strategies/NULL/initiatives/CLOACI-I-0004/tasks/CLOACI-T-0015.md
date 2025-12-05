---
id: create-manifesterror-type-and-ffi
level: task
title: "Create ManifestError type and FFI validation helpers"
short_code: "CLOACI-T-0015"
created_at: 2025-12-05T22:35:47.099961+00:00
updated_at: 2025-12-05T22:47:01.971682+00:00
parent: CLOACI-I-0004
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0004
---

# Create ManifestError type and FFI validation helpers

## Parent Initiative

[[CLOACI-I-0004]]

## Objective

Create a comprehensive `ManifestError` enum type using `thiserror` and implement helper functions for safe FFI pointer validation. This provides the foundation for replacing unsafe operations with proper error-returning alternatives.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ManifestError` enum created with variants for all FFI failure modes
- [ ] `safe_cstr_to_string()` helper function implemented
- [ ] `safe_cstr_to_option_string()` helper function implemented
- [ ] `validate_package_tasks_ptr()` helper function implemented
- [ ] `validate_task_slice()` helper function implemented
- [ ] `MAX_TASKS` constant defined to prevent resource exhaustion
- [ ] All helpers are `pub(crate)` and documented
- [ ] Code compiles without warnings

## Implementation Notes

### Location
`crates/cloacina/src/packaging/manifest.rs`

### ManifestError Variants
```rust
pub enum ManifestError {
    NullPointer { field: &'static str },
    MisalignedPointer { field: &'static str },
    NullString { field: String },
    InvalidUtf8 { field: String, source: std::str::Utf8Error },
    InvalidDependencies { task_id: String, source: serde_json::Error },
    NullTaskSlice { count: usize },
    TooManyTasks { count: usize, max: usize },
    InvalidGraphData { source: serde_json::Error },
    LibraryError { message: String },
}
```

### Helper Functions

1. **safe_cstr_to_string(ptr, field_name)** - Convert C string to Rust String, error on null/invalid UTF-8
2. **safe_cstr_to_option_string(ptr, field_name)** - Convert C string to Option<String>, None on null, error on invalid UTF-8
3. **validate_package_tasks_ptr(ptr, field_name)** - Validate pointer is non-null and properly aligned
4. **validate_task_slice(tasks, count, field_name)** - Validate task array pointer with bounds checking

### Dependencies
None - this is the foundation task

## Status Updates

*To be added during implementation*
