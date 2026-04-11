# cloacina::registry::loader::task_registrar::dynamic_task <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Dynamic library task implementation using fidius-host for task execution.

The plugin library is loaded once during package registration and the handle
is shared across all task instances from that package. No per-execution
temp files or dlopen/dlclose cycles.

## Structs

### `cloacina::registry::loader::task_registrar::dynamic_task::LoadedWorkflowPlugin`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


A persistent handle to a loaded workflow plugin library.

Loaded once from compiled library bytes, kept alive for the lifetime of the
package. All task instances from the same package share this handle.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `handle` | `std :: sync :: Mutex < fidius_host :: PluginHandle >` |  |
| `_temp_dir` | `tempfile :: TempDir` |  |

#### Methods

##### `load` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


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



##### `execute_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn execute_task (& self , request : TaskExecutionRequest) -> Result < TaskExecutionResult , String >
```

Call execute_task (method index 1) on the loaded plugin.

<details>
<summary>Source</summary>

```rust
    fn execute_task(&self, request: TaskExecutionRequest) -> Result<TaskExecutionResult, String> {
        let handle = self
            .handle
            .lock()
            .map_err(|e| format!("Plugin mutex poisoned: {}", e))?;
        handle
            .call_method(1, &(request,))
            .map_err(|e| format!("execute_task FFI call failed: {}", e))
    }
```

</details>





### `cloacina::registry::loader::task_registrar::dynamic_task::DynamicLibraryTask`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


**Derives:** `Debug`

A task implementation that executes via the fidius plugin API.

The plugin handle is loaded once and shared across all task instances
from the same package. No per-execution temp files or dlopen cycles.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `plugin` | `Arc < LoadedWorkflowPlugin >` | Shared handle to the loaded plugin library |
| `task_name` | `String` | Name of the task within the package |
| `package_name` | `String` | Name of the package containing this task |
| `dependencies` | `Vec < TaskNamespace >` | Task dependencies as fully qualified namespaces |

#### Methods

##### `load_plugin` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


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



##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


```rust
fn new (plugin : Arc < LoadedWorkflowPlugin > , task_name : String , package_name : String , dependencies : Vec < TaskNamespace > ,) -> Self
```

Create a new dynamic library task with a shared plugin handle.

<details>
<summary>Source</summary>

```rust
    pub(super) fn new(
        plugin: Arc<LoadedWorkflowPlugin>,
        task_name: String,
        package_name: String,
        dependencies: Vec<TaskNamespace>,
    ) -> Self {
        Self {
            plugin,
            task_name,
            package_name,
            dependencies,
        }
    }
```

</details>
