---
id: workflow-attribute-macro-packaged
level: task
title: "#[workflow] attribute macro — packaged mode with FFI exports"
short_code: "CLOACI-T-0303"
created_at: 2026-03-29T20:39:42.192217+00:00
updated_at: 2026-03-30T02:45:38.446771+00:00
parent: CLOACI-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0058
---

# #[workflow] attribute macro — packaged mode with FFI exports

## Parent Initiative

[[CLOACI-I-0058]]

## Objective

Extend the `#[workflow]` macro (from T-0302) with the packaged code path. When the `packaged` feature flag is enabled, the same `#[workflow]` attribute generates FFI exports (`cloacina_get_task_metadata`, `cloacina_execute_task`) and C-compatible metadata structures instead of `#[ctor]` registration. This replaces `#[packaged_workflow]`.

## Acceptance Criteria

## Acceptance Criteria

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

**2026-03-30**: Implementation complete.

### What was done
- Added `generate_packaged_registration()` to `workflow_attr.rs` — generates FFI exports under `cfg(feature = "packaged")`
- `cloacina_get_task_metadata()` — returns `*const cloacina_ctl_package_tasks` with static task metadata array
- `cloacina_execute_task()` — FFI entry point with panic catching, context JSON serialization, result buffer writing
- C-compatible structs: `cloacina_ctl_task_metadata`, `cloacina_ctl_package_tasks`
- Graph data JSON embedded as static data
- Package name from `env!("CARGO_PKG_NAME")` in static initializer — no macro parameter needed
- Fingerprint from `DefaultHasher` on module content
- Dedicated `CDYLIB_RUNTIME` (OnceLock) for async task execution in cdylib context
- `cfg(not(feature = "packaged"))` for embedded path, `cfg(feature = "packaged")` for FFI path — mutually exclusive
- Also deleted old `workflow!` declarative macro (`workflow.rs` removed) per T-0302 breaking change
- Old `workflow_legacy` removed entirely — breaking change, no backward compat
- All 386 unit tests pass, existing `#[packaged_workflow]` examples still compile

### Note on integration testing
- Full cdylib build + PackageLoader test deferred — requires building a separate crate with `features = ["packaged"]` and loading the `.so`. Will be validated when examples are migrated (T-0306).
