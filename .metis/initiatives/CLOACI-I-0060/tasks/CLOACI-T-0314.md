---
id: update-workflow-packaged-mode-to
level: task
title: "Update #[workflow] packaged mode to generate #[plugin_impl] instead of raw FFI"
short_code: "CLOACI-T-0314"
created_at: 2026-03-31T23:39:09.820521+00:00
updated_at: 2026-04-01T01:18:06.244771+00:00
parent: CLOACI-I-0060
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0060
---

# Update #[workflow] packaged mode to generate #[plugin_impl] instead of raw FFI

## Parent Initiative

[[CLOACI-I-0060]]

## Objective

Rewrite the `generate_packaged_registration()` function in `workflow_attr.rs` to emit `#[plugin_impl(CloacinaPlugin)]` code instead of hand-written `extern "C"` FFI functions and `#[repr(C)]` structs. The `#[workflow]` macro's packaged mode should produce a fidius-compatible plugin that the host can load via `fidius-host`.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `workflow_attr.rs` `generate_packaged_registration()` generates `#[plugin_impl(CloacinaPlugin)]` impl block instead of raw FFI
- [ ] Generated code creates a struct implementing `CloacinaPlugin` with `get_task_metadata()` and `execute_task()`
- [ ] Generated code includes `fidius::fidius_plugin_registry!()` macro call for plugin discovery
- [ ] No more `#[no_mangle] pub extern "C" fn cloacina_get_task_metadata()` generation
- [ ] No more `#[no_mangle] pub extern "C" fn cloacina_execute_task()` generation
- [ ] No more manual `#[repr(C)]` struct definitions (`cloacina_ctl_package_tasks`, etc.)
- [ ] No more manual `CDYLIB_RUNTIME` tokio runtime management (fidius handles async block_on)
- [ ] No more manual `_write_error_result` / panic-catching shims (fidius handles this)
- [ ] `cloacina-macros` Cargo.toml depends on `cloacina-plugin-api` for the interface types
- [ ] Packaged examples compile with new macro output (`cargo build --release` on packaged-workflows example)
- [ ] `fidius inspect` on the resulting dylib shows the CloacinaPlugin interface with correct hash

## Implementation Notes

### Key file
- `crates/cloacina-macros/src/workflow_attr.rs` — `generate_packaged_registration()` function (lines ~618-860)

### What the macro currently generates (to be replaced)
- `cloacina_ctl_task_metadata` / `cloacina_ctl_package_tasks` repr(C) structs
- Static `TASK_METADATA_ARRAY` and `PACKAGE_TASKS_METADATA`
- `cloacina_get_task_metadata()` extern "C" function
- `cloacina_execute_task()` extern "C" function with manual serde, panic-catching, buffer management
- `_cloacina_execute_task_inner()` helper
- `_write_error_result()` / `_write_success_result()` helpers
- `CDYLIB_RUNTIME` OnceLock for dedicated tokio runtime

### What the macro should generate instead
```rust
struct WorkflowPlugin;

#[plugin_impl(CloacinaPlugin)]
impl CloacinaPlugin for WorkflowPlugin {
    fn get_task_metadata(&self) -> Result<PackageTasksMetadata, PluginError> {
        // Build PackageTasksMetadata from compile-time task info
    }
    fn execute_task(&self, request: TaskExecutionRequest) -> Result<TaskExecutionResult, PluginError> {
        // Match task name, deserialize context, run task, serialize result
    }
}

fidius::fidius_plugin_registry!();
```

### Depends on
- T-0313 (interface crate must exist)

## Status Updates

*To be added during implementation*
