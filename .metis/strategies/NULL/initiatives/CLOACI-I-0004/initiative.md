---
id: fix-ffi-safety-unchecked-null
level: initiative
title: "Fix FFI Safety - Unchecked Null Pointers in Package Loading"
short_code: "CLOACI-I-0004"
created_at: 2025-11-29T02:40:07.034926+00:00
updated_at: 2025-11-29T02:40:07.034926+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
strategy_id: NULL
initiative_id: fix-ffi-safety-unchecked-null
---

# Fix FFI Safety - Unchecked Null Pointers in Package Loading Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

The FFI metadata extraction in `cloacina/src/packaging/manifest.rs` (lines 169-296) performs extensive unsafe operations with inadequate null pointer validation when loading packaged workflow libraries.

**Current problematic code:**
```rust
let package_tasks_ptr = unsafe { get_metadata() };  // Line 169 - No null check before deref
if package_tasks_ptr.is_null() { ... }
let package_tasks = unsafe { &*package_tasks_ptr };  // Line 183 - Dereference after null check
```

**Issues identified:**
- Multiple `unsafe { CStr::from_ptr(...).to_str().unwrap_or(...) }` calls (lines 214-239) that silently default if C string conversion fails
- No validation that the pointer slice is valid before reading (line 209-210)
- If `package_tasks.tasks` is invalid/dangling, this creates memory safety vulnerabilities
- Line 190: `.unwrap_or("{}")` silently loses graph data on parse failure

**Risk:** Memory unsafety - potential segfaults or buffer overflows when loading malformed packages.

## Goals & Non-Goals

**Goals:**
- Eliminate all undefined behavior in FFI boundary code
- Return proper errors instead of silently defaulting on failures
- Validate all pointer data before dereferencing
- Add comprehensive logging for FFI failures

**Non-Goals:**
- Sandboxing dynamic library execution (separate initiative CLOACI-I-0008)
- Package signing/verification (separate initiative CLOACI-I-0008)
- Changing the overall package loading architecture

## Detailed Design

### 1. Add Pointer Validation Layer

Create a validation module that checks pointer validity before any dereference:

```rust
fn validate_package_tasks_ptr(ptr: *const PackageTasks) -> Result<&PackageTasks, ManifestError> {
    if ptr.is_null() {
        return Err(ManifestError::NullPointer("package_tasks"));
    }
    // Validate alignment
    if (ptr as usize) % std::mem::align_of::<PackageTasks>() != 0 {
        return Err(ManifestError::MisalignedPointer("package_tasks"));
    }
    Ok(unsafe { &*ptr })
}
```

### 2. Replace Silent Defaults with Errors

Change all `.unwrap_or()` patterns to return `Result`:

```rust
// Before
let dependencies: Vec<String> = serde_json::from_str(dependencies_json).unwrap_or_else(|_| vec![]);

// After
let dependencies: Vec<String> = serde_json::from_str(dependencies_json)
    .map_err(|e| ManifestError::InvalidDependencies { 
        task_id: task_id.to_string(), 
        source: e 
    })?;
```

### 3. Add CStr Validation Helper

```rust
fn safe_cstr_to_string(ptr: *const c_char, field_name: &str) -> Result<String, ManifestError> {
    if ptr.is_null() {
        return Err(ManifestError::NullString(field_name.to_string()));
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| s.to_string())
        .map_err(|e| ManifestError::InvalidUtf8 { field: field_name.to_string(), source: e })
}
```

### 4. Add Bounds Checking for Slice Access

```rust
fn validate_task_slice(tasks: *const TaskInfo, count: usize) -> Result<&[TaskInfo], ManifestError> {
    if tasks.is_null() && count > 0 {
        return Err(ManifestError::NullTaskSlice);
    }
    if count > MAX_TASKS {
        return Err(ManifestError::TooManyTasks { count, max: MAX_TASKS });
    }
    Ok(unsafe { std::slice::from_raw_parts(tasks, count) })
}
```

## Testing Strategy

### Unit Testing
- Test with null pointers at every nullable position
- Test with misaligned pointers
- Test with invalid UTF-8 in strings
- Test with corrupted JSON in metadata fields
- Test with task counts exceeding bounds

### Integration Testing
- Create intentionally malformed .so files
- Test loading packages with missing symbols
- Test concurrent package loading

## Alternatives Considered

1. **Use `safer-ffi` crate** - Rejected because it would require significant changes to the FFI interface and packaged workflow compilation
2. **Wrap all FFI in `catch_unwind`** - Rejected because it doesn't prevent UB, just catches panics after the fact
3. **Move to pure Rust package format** - Larger scope change, could be future initiative

## Implementation Plan

1. **Phase 1:** Create error types and validation helpers
2. **Phase 2:** Replace all unsafe dereferences with validated versions
3. **Phase 3:** Replace silent defaults with proper error returns
4. **Phase 4:** Add comprehensive test coverage for malformed inputs
5. **Phase 5:** Update documentation on error handling