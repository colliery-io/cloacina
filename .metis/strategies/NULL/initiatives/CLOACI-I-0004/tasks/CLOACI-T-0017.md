---
id: replace-silent-defaults-with
level: task
title: "Replace silent defaults with proper error returns"
short_code: "CLOACI-T-0017"
created_at: 2025-12-05T22:35:47.281702+00:00
updated_at: 2025-12-05T22:50:20.492005+00:00
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

# Replace silent defaults with proper error returns

## Parent Initiative

[[CLOACI-I-0004]]

## Objective

Replace all `.unwrap_or()` and silent default patterns in FFI string/JSON handling with proper error returns using the helper functions from CLOACI-T-0015. This ensures failures are reported rather than silently masked.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All `CStr::from_ptr().to_str().unwrap_or()` patterns replaced with `safe_cstr_to_string()`
- [ ] Graph data JSON parsing (line 190) returns error instead of defaulting to "{}"
- [ ] Dependencies JSON parsing (line 243) returns `ManifestError::InvalidDependencies`
- [ ] Optional fields (description, author, fingerprint) use `safe_cstr_to_option_string()`
- [ ] Errors include context about which field/task failed
- [ ] Code compiles without warnings
- [ ] Existing tests still pass

## Implementation Notes

### Location
`crates/cloacina/src/packaging/manifest.rs` - function `extract_task_info_and_graph_from_library()`

### Patterns to Replace

1. **Lines 187-191** - Graph data JSON:
   ```rust
   // Before
   .to_str().unwrap_or("{}")
   // After
   .to_str().map_err(|e| ManifestError::InvalidUtf8 { ... })?
   ```

2. **Lines 214-239** - Task metadata strings (local_id, description, source_location, dependencies_json):
   ```rust
   // Before
   CStr::from_ptr(task_metadata.local_id).to_str().unwrap_or("unknown")
   // After
   safe_cstr_to_string(task_metadata.local_id, "local_id")?
   ```

3. **Line 243** - Dependencies parsing:
   ```rust
   // Before
   serde_json::from_str(dependencies_json).unwrap_or_else(|_| vec![])
   // After
   serde_json::from_str(dependencies_json).map_err(|e| ManifestError::InvalidDependencies { ... })?
   ```

4. **Lines 256-287** - Optional package metadata fields use `safe_cstr_to_option_string()`

### Dependencies
- CLOACI-T-0015 (helper functions)
- CLOACI-T-0016 (pointer validation patterns established)

## Status Updates

*To be added during implementation*
