---
id: adopt-fidius-for-packaged-workflow
level: initiative
title: "Adopt fidius for packaged workflow FFI — eliminate manual ABI structs"
short_code: "CLOACI-I-0060"
created_at: 2026-03-31T13:51:45.446782+00:00
updated_at: 2026-04-01T04:12:31.537334+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: adopt-fidius-for-packaged-workflow
---

# Adopt fidius for packaged workflow FFI — eliminate manual ABI structs

## Context

Cloacina's packaged workflow system uses hand-written `#[repr(C)]` FFI structs to exchange metadata and execute tasks across the cdylib boundary. These structs are defined independently in **four** locations:

1. **`workflow_attr.rs`** (macro) — `cloacina_ctl_package_tasks`, `cloacina_ctl_task_metadata`
2. **`types.rs`** (extraction) — `TaskMetadataCollection`, `TaskMetadata`
3. **`manifest.rs`** (manifest generation) — inline `CPackageTasks`, `CTaskMetadata`
4. **`package_loader.rs`** (package loading) — inline `CPackageTasks`, `CTaskMetadata`

Plus hand-written `extern "C"` shim functions (`cloacina_execute_task`, `cloacina_get_task_metadata`), manual JSON serialization of `Context<Value>` across the FFI boundary, and manual panic-catching at every call site.

This caused the SIGSEGV bug fixed in `d07b878` — the `#[workflow]` macro changed the FFI struct layout but three consumer sites were not updated, causing a NULL pointer dereference when a `u32` index field was misinterpreted as a pointer.

**colliery-io/fidius** (`../fides`) is a Rust plugin framework that solves this problem. It transforms a Rust trait into a dynamically loaded plugin with a stable C ABI — no handwritten FFI:

- **`#[plugin_interface]`** — Generates `#[repr(C)]` vtable, FNV-1a interface hash, capability bits from a trait definition
- **`#[plugin_impl]`** — Generates `extern "C"` shims with serde serialization, panic-catching, and buffer management
- **`fidius-host`** — Host-side loading with validation (magic bytes, ABI version, interface hash, wire format) and type-safe `PluginHandle::call_method<I,O>()`
- **Automatic ABI drift detection** — Interface hash checked at load time; mismatches produce a clear error, not a SIGSEGV
- **Wire format safety** — Debug=JSON, release=bincode, mismatches rejected at load
- **Optional method evolution** — Capability bitfield tracks which optional methods a plugin implements

## Goals & Non-Goals

**Goals:**
- Replace all hand-written FFI structs with a single `#[plugin_interface]` trait definition
- Replace all hand-written `extern "C"` shims with `#[plugin_impl]` macro generation
- Replace manual dlopen/dlsym/validation with `fidius-host` loading pipeline
- Get automatic ABI drift detection so struct mismatches are caught at load time, not as SIGSEGVs
- Eliminate the "4 copies of the same struct" maintenance burden
- Adopt fidius source package model — `.cloacina` archives contain source + `package.toml`, host builds cdylib locally for cross-architecture support
- Replace cloacina's binary packaging pipeline (`create_package_archive`, `CompileResult`, manifest generation) with fidius `pack_package`/`unpack_package`/`build_package`
- Host-defined metadata schema via `PackageManifest<CloacinaMetadata>` for workflow-specific fields

**Non-Goals:**
- Changing the workflow/task execution model — fidius replaces the FFI plumbing and packaging, not the workflow engine
- Replacing the `#[workflow]`/`#[trigger]` macros — these stay; only the FFI export layer and packaging format change
- Using fidius-cli for scaffolding — cloacina has its own tooling

## Detailed Design

### Interface Trait

Define a `CloacinaPlugin` trait in a dedicated interface crate:

```rust
#[plugin_interface(version = 1, buffer = PluginAllocated)]
pub trait CloacinaPlugin: Send + Sync {
    fn get_task_metadata(&self) -> Result<PackageTasksMetadata, PluginError>;
    fn execute_task(&self, request: TaskExecutionRequest) -> Result<TaskExecutionResult, PluginError>;
}
```

The `#[workflow]` macro's packaged mode generates `#[plugin_impl(CloacinaPlugin)]` instead of raw `extern "C"` functions. The host uses `fidius-host` to load, validate, and call through a `PluginHandle`.

### Crate Structure Changes

- **New: `cloacina-plugin-api`** — Interface crate with `#[plugin_interface]` trait + shared serde types. Re-exports `fidius::plugin_impl` and `fidius::PluginError` so plugin authors have a single dependency.
- **`cloacina-macros`** — `#[workflow]` packaged mode generates `#[plugin_impl(CloacinaPlugin)]` + `fidius_plugin_registry!()`
- **`cloacina-workflow`** — Depends on `cloacina-plugin-api` for shared types (cdylib-safe, no host deps)
- **`cloacina`** — Depends on `fidius-host` for loading; removes `types.rs`, `extraction.rs` FFI structs, `package_loader.rs` inline structs, `manifest.rs` inline structs

### What Gets Deleted

- `crates/cloacina/src/registry/loader/task_registrar/types.rs` — `TaskMetadata`, `TaskMetadataCollection`
- `crates/cloacina/src/registry/loader/task_registrar/extraction.rs` — manual FFI extraction
- Inline `CPackageTasks`/`CTaskMetadata` in `manifest.rs` and `package_loader.rs`
- Manual `cloacina_execute_task` / `cloacina_get_task_metadata` generation in `workflow_attr.rs`
- Manual `_write_error_result` / `_cloacina_execute_task_inner` shim functions
- Manual `CDYLIB_RUNTIME` tokio runtime management (fidius shims handle async via block_on)

## Alternatives Considered

**Keep manual FFI with better discipline** — Add compile-time layout assertions across all 4 struct sites. Doesn't solve the fundamental problem: every new field requires coordinated changes in 4 files, and doesn't address the shim/serialization/panic-catching boilerplate.

**Single shared FFI types crate** — Move the `#[repr(C)]` structs into one crate used by both macro and host. Reduces 4 sites to 2 but doesn't eliminate hand-written shims, serialization, panic-catching, or provide ABI drift detection.

**cbindgen/cxx** — Generate bindings from Rust types but don't provide the full plugin lifecycle (loading, validation, ABI versioning, wire format). Less boilerplate but still manual coordination.

## Implementation Plan

1. **Interface crate** — Create `cloacina-plugin-api` with `#[plugin_interface]` trait and shared serde types
2. **Macro update** — `#[workflow]` packaged mode generates `#[plugin_impl(CloacinaPlugin)]` + `fidius_plugin_registry!()`
3. **Host-side loading** — Replace `package_loader.rs` and `extraction.rs` with `fidius-host` loading pipeline
4. **Remove legacy FFI** — Delete all hand-written FFI structs, shims, and inline struct definitions
5. **Example/test migration** — Rebuild all packaged examples with new macro output, update integration tests
6. **Validation** — Verify ABI drift detection works (intentionally mismatch interface version, confirm rejection)
