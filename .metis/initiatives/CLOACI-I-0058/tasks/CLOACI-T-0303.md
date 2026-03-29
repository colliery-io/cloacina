---
id: workflow-attribute-macro-packaged
level: task
title: "#[workflow] attribute macro — packaged mode with FFI exports"
short_code: "CLOACI-T-0303"
created_at: 2026-03-29T20:39:42.192217+00:00
updated_at: 2026-03-29T20:39:42.192217+00:00
parent: CLOACI-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0058
---

# #[workflow] attribute macro — packaged mode with FFI exports

## Parent Initiative

[[CLOACI-I-0058]]

## Objective

Extend the `#[workflow]` macro (from T-0302) with the packaged code path. When the `packaged` feature flag is enabled, the same `#[workflow]` attribute generates FFI exports (`cloacina_get_task_metadata`, `cloacina_execute_task`) and C-compatible metadata structures instead of `#[ctor]` registration. This replaces `#[packaged_workflow]`.

## Acceptance Criteria

- [ ] Same `#[workflow]` macro from T-0302 generates FFI exports when `features = ["packaged"]` is enabled
- [ ] Generates `#[no_mangle] pub extern "C" fn cloacina_get_task_metadata()` — C-compatible metadata
- [ ] Generates `#[no_mangle] pub extern "C" fn cloacina_execute_task()` — task execution entry point
- [ ] C-compatible static metadata structures (`cloacina_ctl_task_metadata`, `cloacina_ctl_package_tasks`)
- [ ] JSON graph data (nodes, edges, dependencies) embedded as static data
- [ ] No `#[ctor]` registration generated in packaged mode
- [ ] Package fingerprinting for versioning
- [ ] Builds as cdylib, loadable by the existing `PackageLoader`
- [ ] Existing package loader + reconciler can load the output without changes
- [ ] Integration test: build a cdylib with `#[workflow]`, load via PackageLoader, verify metadata extraction

## Implementation Notes

### Files to modify
- `crates/cloacina-macros/src/workflow.rs` — add `cfg(feature = "packaged")` branch for FFI code generation
- Reuse FFI generation logic from `packaged_workflow.rs`

### Key design points
- Single macro impl with `cfg` branch — not two separate macros
- FFI structures must match what `PackageLoader::extract_metadata()` expects
- The `package` name comes from `CARGO_PKG_NAME`, not a macro parameter

### Depends on
- T-0302 (embedded mode must exist first)

## Status Updates

*To be added during implementation*
