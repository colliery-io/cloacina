# cloacina::registry::loader::package_loader <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Package loader for extracting metadata from workflow library files.

This module provides functionality to safely load dynamic library files (.so/.dylib/.dll)
via the fidius-host plugin API and extract package metadata.

## Structs

### `cloacina::registry::loader::package_loader::PackageMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Metadata extracted from a workflow package.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package_name` | `String` | Package name |
| `version` | `String` | Package version (extracted from library or defaults to "1.0.0") |
| `description` | `Option < String >` | Package description |
| `author` | `Option < String >` | Package author |
| `tasks` | `Vec < TaskMetadata >` | List of tasks provided by this package |
| `graph_data` | `Option < serde_json :: Value >` | Workflow graph data (if available) |
| `architecture` | `String` | Library architecture info |
| `symbols` | `Vec < String >` | Required symbols present in the library |



### `cloacina::registry::loader::package_loader::TaskMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Individual task metadata.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `index` | `u32` | Task index in the package |
| `local_id` | `String` | Local task identifier |
| `namespaced_id_template` | `String` | Namespaced ID template |
| `dependencies` | `Vec < String >` | Task dependencies as a list of local task IDs |
| `description` | `String` | Human-readable description |
| `source_location` | `String` | Source location information |



### `cloacina::registry::loader::package_loader::PackageLoader`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


#### Fields

| Name | Type | Description |
|------|------|-------------|
| `temp_dir` | `TempDir` |  |
| `handle_cache` | `PluginHandleCache` | Shared cache — prevents dlclose of loaded libraries. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Result < Self , LoaderError >
```

Create a new package loader with a temporary directory for safe operations.

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Result<Self, LoaderError> {
        let temp_dir = TempDir::new().map_err(|e| LoaderError::TempDirectory {
            error: e.to_string(),
        })?;

        Ok(Self {
            temp_dir,
            handle_cache: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        })
    }
```

</details>



##### `with_handle_cache` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_handle_cache (cache : PluginHandleCache) -> Result < Self , LoaderError >
```

Create a package loader with a shared handle cache.

<details>
<summary>Source</summary>

```rust
    pub fn with_handle_cache(cache: PluginHandleCache) -> Result<Self, LoaderError> {
        let temp_dir = TempDir::new().map_err(|e| LoaderError::TempDirectory {
            error: e.to_string(),
        })?;

        Ok(Self {
            temp_dir,
            handle_cache: cache,
        })
    }
```

</details>



##### `handle_cache` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn handle_cache (& self) -> PluginHandleCache
```

Get the shared handle cache (for passing to TaskRegistrar).

<details>
<summary>Source</summary>

```rust
    pub fn handle_cache(&self) -> PluginHandleCache {
        self.handle_cache.clone()
    }
```

</details>



##### `generate_graph_data_from_tasks` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn generate_graph_data_from_tasks (& self , tasks : & [TaskMetadata] ,) -> Result < serde_json :: Value , LoaderError >
```

Generate graph data from task dependencies.

<details>
<summary>Source</summary>

```rust
    fn generate_graph_data_from_tasks(
        &self,
        tasks: &[TaskMetadata],
    ) -> Result<serde_json::Value, LoaderError> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for task in tasks {
            nodes.push(serde_json::json!({
                "id": task.local_id,
                "label": task.local_id,
                "description": task.description,
                "node_type": "task"
            }));
        }

        for task in tasks {
            for dependency in &task.dependencies {
                edges.push(serde_json::json!({
                    "source": dependency,
                    "target": task.local_id,
                    "edge_type": "dependency"
                }));
            }
        }

        Ok(serde_json::json!({
            "nodes": nodes,
            "edges": edges,
            "metadata": {
                "task_count": tasks.len(),
                "generated_from": "task_dependencies"
            }
        }))
    }
```

</details>



##### `extract_metadata` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn extract_metadata (& self , package_data : & [u8] ,) -> Result < PackageMetadata , LoaderError >
```

Extract metadata from compiled library bytes.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_data` | `-` | Raw bytes of the compiled cdylib (.so / .dylib). The reconciler is responsible for unpacking and compiling any source archives before calling this method. |


**Returns:**

* `Ok(PackageMetadata)` - Successfully extracted metadata * `Err(LoaderError)` - If extraction fails

<details>
<summary>Source</summary>

```rust
    pub async fn extract_metadata(
        &self,
        package_data: &[u8],
    ) -> Result<PackageMetadata, LoaderError> {
        let library_extension = get_library_extension();
        let unique_id = uuid::Uuid::new_v4();
        let temp_path = self
            .temp_dir
            .path()
            .join(format!("pkg_{}.{}", unique_id, library_extension));
        fs::write(&temp_path, package_data)
            .await
            .map_err(|e| LoaderError::FileSystem {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?;

        self.extract_metadata_from_so(&temp_path).await
    }
```

</details>



##### `extract_metadata_from_so` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn extract_metadata_from_so (& self , library_path : & Path ,) -> Result < PackageMetadata , LoaderError >
```

Extract metadata from a library file using the fidius-host plugin API.

The loaded library is cached to prevent dlclose — see struct-level docs.

<details>
<summary>Source</summary>

```rust
    async fn extract_metadata_from_so(
        &self,
        library_path: &Path,
    ) -> Result<PackageMetadata, LoaderError> {
        // Load via fidius-host — validates magic, ABI version, wire format, etc.
        let loaded = fidius_host::loader::load_library(library_path).map_err(
            |e: fidius_host::LoadError| LoaderError::LibraryLoad {
                path: library_path.to_string_lossy().to_string(),
                error: e.to_string(),
            },
        )?;

        let plugin =
            loaded
                .plugins
                .into_iter()
                .next()
                .ok_or_else(|| LoaderError::MetadataExtraction {
                    reason: "Plugin library contains no plugins".to_string(),
                })?;

        let handle = fidius_host::PluginHandle::from_loaded(plugin);

        // Method index 0 = get_task_metadata (zero-arg, encoded as empty tuple)
        let ffi_metadata: cloacina_workflow_plugin::PackageTasksMetadata = handle
            .call_method(0, &())
            .map_err(|e| LoaderError::MetadataExtraction {
                reason: format!("Failed to call get_task_metadata: {}", e),
            })?;

        // Keep the handle alive — dropping it triggers dlclose which corrupts
        // the inventory linked list (see struct-level docs).
        // PluginHandle holds an Arc<Library> that keeps the dylib mapped.
        // Keep the handle alive — dropping it triggers dlclose which corrupts
        // the inventory linked list (see struct-level docs).
        // PluginHandle holds an Arc<Library> that keeps the dylib mapped.
        if let Ok(mut cache) = self.handle_cache.lock() {
            cache.push(handle);
        }

        self.convert_plugin_metadata_to_rust(ffi_metadata)
    }
```

</details>



##### `convert_plugin_metadata_to_rust` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn convert_plugin_metadata_to_rust (& self , meta : cloacina_workflow_plugin :: PackageTasksMetadata ,) -> Result < PackageMetadata , LoaderError >
```

Convert `PackageTasksMetadata` from the fidius plugin into the `PackageMetadata` struct used by the rest of the registry.

<details>
<summary>Source</summary>

```rust
    fn convert_plugin_metadata_to_rust(
        &self,
        meta: cloacina_workflow_plugin::PackageTasksMetadata,
    ) -> Result<PackageMetadata, LoaderError> {
        let tasks: Vec<TaskMetadata> = meta
            .tasks
            .into_iter()
            .map(|t| TaskMetadata {
                index: t.index,
                local_id: t.id,
                namespaced_id_template: t.namespaced_id_template,
                dependencies: t.dependencies,
                description: t.description,
                source_location: t.source_location,
            })
            .collect();

        // Build graph data from tasks if no serialized graph is present
        let graph_data = match meta.graph_data_json.as_deref() {
            Some(json) if !json.trim().is_empty() => {
                match serde_json::from_str::<serde_json::Value>(json) {
                    Ok(v) => Some(v),
                    Err(_) => {
                        tracing::debug!(
                            "graph_data_json is not valid JSON, generating from {} tasks",
                            tasks.len()
                        );
                        self.generate_graph_data_from_tasks(&tasks).ok()
                    }
                }
            }
            _ => {
                if !tasks.is_empty() {
                    self.generate_graph_data_from_tasks(&tasks).ok()
                } else {
                    None
                }
            }
        };

        let architecture = if cfg!(target_arch = "x86_64") {
            "x86_64".to_string()
        } else if cfg!(target_arch = "aarch64") {
            "aarch64".to_string()
        } else {
            std::env::consts::ARCH.to_string()
        };

        Ok(PackageMetadata {
            package_name: meta.package_name,
            version: "1.0.0".to_string(),
            description: meta.package_description,
            author: meta.package_author,
            tasks,
            graph_data,
            architecture,
            symbols: vec!["fidius_get_registry".to_string()],
        })
    }
```

</details>



##### `extract_graph_metadata` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn extract_graph_metadata (& self , package_data : & [u8] ,) -> Result < Option < cloacina_workflow_plugin :: GraphPackageMetadata > , LoaderError >
```

Extract computation graph metadata from compiled library bytes.

Calls `get_graph_metadata()` (method index 2) on the fidius plugin.
Returns `None` if the plugin doesn't support graph metadata (workflow-only packages).

<details>
<summary>Source</summary>

```rust
    pub async fn extract_graph_metadata(
        &self,
        package_data: &[u8],
    ) -> Result<Option<cloacina_workflow_plugin::GraphPackageMetadata>, LoaderError> {
        let library_extension = get_library_extension();
        let temp_path = self.temp_dir.path().join(format!(
            "graph_{}.{}",
            uuid::Uuid::new_v4(),
            library_extension
        ));
        fs::write(&temp_path, package_data)
            .await
            .map_err(|e| LoaderError::FileSystem {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?;

        let loaded = fidius_host::loader::load_library(&temp_path).map_err(
            |e: fidius_host::LoadError| LoaderError::LibraryLoad {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            },
        )?;

        let plugin =
            loaded
                .plugins
                .into_iter()
                .next()
                .ok_or_else(|| LoaderError::MetadataExtraction {
                    reason: "Plugin library contains no plugins".to_string(),
                })?;

        let handle = fidius_host::PluginHandle::from_loaded(plugin);

        // Method index 2 = get_graph_metadata (zero-arg)
        let result = match handle
            .call_method::<(), cloacina_workflow_plugin::GraphPackageMetadata>(2, &())
        {
            Ok(meta) => Ok(Some(meta)),
            Err(e) => {
                // Plugin doesn't support graph metadata — that's OK for workflow-only packages
                tracing::debug!("get_graph_metadata not supported by plugin: {}", e);
                Ok(None)
            }
        };

        // Keep handle alive to prevent dlclose
        if let Ok(mut cache) = self.handle_cache.lock() {
            cache.push(handle);
        }

        result
    }
```

</details>



##### `temp_dir` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn temp_dir (& self) -> & Path
```

Get the temporary directory path for manual file operations.

<details>
<summary>Source</summary>

```rust
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }
```

</details>



##### `validate_package_symbols` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn validate_package_symbols (& self , package_data : & [u8] ,) -> Result < Vec < String > , LoaderError >
```

Validate that a package has the required symbols by loading it via fidius-host.

Returns an empty `Vec` on success (fidius validated the plugin registry),
or the known symbol names if the library loads without error.

<details>
<summary>Source</summary>

```rust
    pub async fn validate_package_symbols(
        &self,
        package_data: &[u8],
    ) -> Result<Vec<String>, LoaderError> {
        let library_extension = get_library_extension();
        let temp_path = self
            .temp_dir
            .path()
            .join(format!("validation_package.{}", library_extension));
        fs::write(&temp_path, package_data)
            .await
            .map_err(|e| LoaderError::FileSystem {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?;

        // Load via fidius-host — if this succeeds the plugin is valid.
        // Keep the loaded library alive to prevent dlclose.
        let loaded = fidius_host::loader::load_library(&temp_path).map_err(
            |e: fidius_host::LoadError| LoaderError::LibraryLoad {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            },
        )?;
        // Prevent dlclose by keeping the library handle alive
        std::mem::forget(loaded);

        // Return the fidius registry symbol
        Ok(vec!["fidius_get_registry".to_string()])
    }
```

</details>





## Functions

### `cloacina::registry::loader::package_loader::get_library_extension`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_library_extension () -> & 'static str
```

Get the platform-specific dynamic library extension.

<details>
<summary>Source</summary>

```rust
pub fn get_library_extension() -> &'static str {
    if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}
```

</details>
