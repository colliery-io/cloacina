---
id: delete-legacy-ffi-code-manual
level: task
title: "Delete legacy FFI code — manual structs, shims, and inline definitions"
short_code: "CLOACI-T-0316"
created_at: 2026-03-31T23:39:34.837408+00:00
updated_at: 2026-03-31T23:39:34.837408+00:00
parent: CLOACI-I-0060
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0060
---

# Delete legacy FFI code — manual structs, shims, and inline definitions

## Parent Initiative

[[CLOACI-I-0060]]

## Objective

Clean sweep of all legacy manual FFI code now that fidius handles everything. Delete files, remove dead imports, grep-verify no remaining references.

## Acceptance Criteria

- [ ] Delete `crates/cloacina/src/registry/loader/task_registrar/types.rs` (old `TaskMetadataCollection`/`TaskMetadata`)
- [ ] Delete `crates/cloacina/src/registry/loader/task_registrar/extraction.rs` (old manual FFI extraction)
- [ ] Remove inline `CPackageTasks`/`CTaskMetadata` structs from `manifest.rs` (replaced by fidius calls)
- [ ] Remove inline `CPackageTasks`/`CTaskMetadata` structs from `package_loader.rs` (replaced by fidius calls)
- [ ] Remove `crates/cloacina-macros/src/packaged_workflow.rs` (old macro, no longer used)
- [ ] Remove `safe_cstr_to_string`, `safe_cstr_to_option_string`, `validate_ptr`, `validate_slice` from `manifest.rs` (no longer needed)
- [ ] Remove `ManifestError` enum variants for null pointers/misaligned pointers (fidius handles this)
- [ ] `grep -r "cloacina_ctl_" crates/` returns zero results
- [ ] `grep -r "TaskMetadataCollection" crates/` returns zero results
- [ ] `grep -r "CPackageTasks" crates/` returns zero results
- [ ] All tests pass
- [ ] `libloading` can be removed from `cloacina` Cargo.toml (fidius-host handles loading)

## Implementation Notes

### Depends on
- T-0315 (host-side loading fully migrated to fidius-host — no callers of old code remain)

## Status Updates

*To be added during implementation*
