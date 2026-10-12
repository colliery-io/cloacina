---
title: "cloacina::registry::loader::task_registrar::dynamic_task"
description: "Dynamic library task implementation using fidius-host for task execution."
sidebar:
  order: 182
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Dynamic library task implementation using fidius-host for task execution.

The plugin library is loaded once during package registration and the handle
is shared across all task instances from that package. No per-execution
temp files or dlopen/dlclose cycles.

## Structs

### <span id="struct.LoadedWorkflowPlugin"></span>`struct LoadedWorkflowPlugin`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">pub(super)</span>


A persistent handle to a loaded workflow plugin library.

Loaded once from compiled library bytes, kept alive for the lifetime of the
package. All task instances from the same package share this handle.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `handle` | `std :: sync :: Mutex < fidius_host :: PluginHandle >` |  |
| `_temp_dir` | `tempfile :: TempDir` |  |

#### Methods

##### <span id="LoadedWorkflowPlugin.load"></span>`load`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">pub(super)</span>


```rust
fn load (library_data : & [u8] , package_name : & str) -> Result < Self , TaskError >
```

Load a workflow plugin from library bytes.

<details>
<summary>Source</summary>

```rust
    pub(super) fn load(library_data: &[u8], package_name: &str) -> Result<Self, TaskError> {
        let temp_dir = tempfile::TempDir::new().map_err(|e| TaskError::ExecutionFailed {
            task_id: package_name.to_string(),
            message: format!("Failed to create temp dir: {}", e),
            timestamp: Utc::now(),
        })?;

        let library_extension = crate::registry::loader::package_loader::get_library_extension();
        let temp_path = temp_dir
            .path()
            .join(format!("workflow_plugin.{}", library_extension));
        std::fs::write(&temp_path, library_data).map_err(|e| TaskError::ExecutionFailed {
            task_id: package_name.to_string(),
            message: format!("Failed to write library: {}", e),
            timestamp: Utc::now(),
        })?;

        let loaded = fidius_host::loader::load_library(&temp_path).map_err(
            |e: fidius_host::LoadError| TaskError::ExecutionFailed {
                task_id: package_name.to_string(),
                message: format!("Failed to load plugin library: {}", e),
                timestamp: Utc::now(),
            },
        )?;

        // CLOACI-T-0897: bind the host-callback table on THIS image.
        //
        // Binding is per loaded library, and this is a distinct `dlopen` of a
        // freshly written temp file — a different dylib image, with its own
        // bind cell, from the one the package loader binds for metadata
        // extraction. This is the image whose tasks actually RUN, so without a
        // bind here `defer_until` fails with "not bound" even though the log
        // says a bind succeeded elsewhere. (Exactly what the e2e lane caught.)
        crate::registry::loader::package_loader::bind_engine_host(&loaded, &temp_path);

        let plugin =
            loaded
                .plugins
                .into_iter()
                .next()
                .ok_or_else(|| TaskError::ExecutionFailed {
                    task_id: package_name.to_string(),
                    message: "Plugin library contains no plugins".to_string(),
                    timestamp: Utc::now(),
                })?;

        let handle = fidius_host::PluginHandle::from_loaded(plugin);

        Ok(Self {
            handle: std::sync::Mutex::new(handle),
            _temp_dir: temp_dir,
        })
    }
```

</details>



##### <span id="LoadedWorkflowPlugin.execute_task"></span>`execute_task`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn execute_task (& self , request : TaskExecutionRequest) -> Result < TaskExecutionResult , String >
```

Call execute_task on the loaded plugin.

<details>
<summary>Source</summary>

```rust
    fn execute_task(&self, request: TaskExecutionRequest) -> Result<TaskExecutionResult, String> {
        let handle = self
            .handle
            .lock()
            .map_err(|e| format!("Plugin mutex poisoned: {}", e))?;
        handle
            .call_method(cloacina_workflow_plugin::METHOD_EXECUTE_TASK, &(request,))
            .map_err(|e| format!("execute_task FFI call failed: {}", e))
    }
```

</details>





### <span id="struct.DynamicLibraryTask"></span>`struct DynamicLibraryTask`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">pub(super)</span>


**Derives:** `Debug`

A task implementation that executes via the fidius plugin API.

The plugin handle is loaded once and shared across all task instances
from the same package. No per-execution temp files or dlopen cycles.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `plugin` | `Arc < LoadedWorkflowPlugin >` | Shared handle to the loaded plugin library |
| `task_name` | `String` | Name of the task within the package |
| `dependencies` | `Vec < TaskNamespace >` | Task dependencies as fully qualified namespaces |
| `trigger_rules` | `serde_json :: Value` | Trigger rules JSON for conditional execution, forwarded from the cdylib's
FFI metadata (CLOACI-T-0721). Without this the host defaulted to `Always`
and packaged workflows silently ignored conditional execution / skips. |
| `requires_handle` | `bool` | CLOACI-T-0897: whether the packaged task takes a `TaskHandle`, forwarded
from FFI metadata. The executor gates its entire handle path on
`requires_handle()`, and without carrying this every packaged task
inherited the trait default (`false`) — so no handle was built, nothing
was registered for deferral, and `defer_until` failed with
`TASK_NOT_RUNNING`. |

#### Methods

##### <span id="DynamicLibraryTask.load_plugin"></span>`load_plugin`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">pub(super)</span>


```rust
fn load_plugin (library_data : & [u8] , package_name : & str ,) -> Result < LoadedWorkflowPlugin , TaskError >
```

Load a plugin library from bytes. Called once per package during registration.

<details>
<summary>Source</summary>

```rust
    pub(super) fn load_plugin(
        library_data: &[u8],
        package_name: &str,
    ) -> Result<LoadedWorkflowPlugin, TaskError> {
        LoadedWorkflowPlugin::load(library_data, package_name)
    }
```

</details>



##### <span id="DynamicLibraryTask.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">pub(super)</span>


```rust
fn new (plugin : Arc < LoadedWorkflowPlugin > , task_name : String , dependencies : Vec < TaskNamespace > , trigger_rules : serde_json :: Value , requires_handle : bool ,) -> Self
```

Create a new dynamic library task with a shared plugin handle.

<details>
<summary>Source</summary>

```rust
    pub(super) fn new(
        plugin: Arc<LoadedWorkflowPlugin>,
        task_name: String,
        dependencies: Vec<TaskNamespace>,
        trigger_rules: serde_json::Value,
        requires_handle: bool,
    ) -> Self {
        Self {
            plugin,
            task_name,
            dependencies,
            trigger_rules,
            requires_handle,
        }
    }
```

</details>





