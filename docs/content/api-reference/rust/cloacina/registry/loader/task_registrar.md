# cloacina::registry::loader::task_registrar <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Task registrar for integrating packaged workflow tasks with the global registry.

This module provides functionality to register tasks from dynamically loaded
library packages with cloacina's global task registry, ensuring proper namespace
isolation and task lifecycle management.

## Structs

### `cloacina::registry::loader::task_registrar::TaskRegistrar`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Task registrar for managing dynamically loaded package tasks.

This registrar integrates packaged workflow tasks with cloacina's global
task registry while maintaining proper namespace isolation and lifecycle
management for dynamic libraries.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `temp_dir` | `TempDir` | Temporary directory for library file operations |
| `registered_tasks` | `Arc < RwLock < HashMap < String , Vec < TaskNamespace > > > >` | Map of package IDs to registered task namespaces for cleanup tracking |
| `loaded_packages` | `Arc < RwLock < HashMap < String , () > > >` | Tracks which package IDs have been registered (for cleanup bookkeeping) |
| `handle_cache` | `crate :: registry :: loader :: package_loader :: PluginHandleCache` | Shared cache — prevents dlclose of loaded libraries.
See `PackageLoader` docs for full explanation. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Result < Self , LoaderError >
```

Create a new task registrar with a temporary directory for operations.

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Result<Self, LoaderError> {
        let temp_dir = TempDir::new().map_err(|e| LoaderError::TempDirectory {
            error: e.to_string(),
        })?;

        Ok(Self {
            temp_dir,
            registered_tasks: Arc::new(RwLock::new(HashMap::new())),
            loaded_packages: Arc::new(RwLock::new(HashMap::new())),
            handle_cache: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        })
    }
```

</details>



##### `with_handle_cache` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_handle_cache (cache : crate :: registry :: loader :: package_loader :: PluginHandleCache ,) -> Result < Self , LoaderError >
```

Create a task registrar with a shared handle cache.

<details>
<summary>Source</summary>

```rust
    pub fn with_handle_cache(
        cache: crate::registry::loader::package_loader::PluginHandleCache,
    ) -> Result<Self, LoaderError> {
        let temp_dir = TempDir::new().map_err(|e| LoaderError::TempDirectory {
            error: e.to_string(),
        })?;

        Ok(Self {
            temp_dir,
            registered_tasks: Arc::new(RwLock::new(HashMap::new())),
            loaded_packages: Arc::new(RwLock::new(HashMap::new())),
            handle_cache: cache,
        })
    }
```

</details>



##### `register_package_tasks` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn register_package_tasks (& self , package_id : & str , package_data : & [u8] , _metadata : & PackageMetadata , tenant_id : Option < & str > ,) -> Result < Vec < TaskNamespace > , LoaderError >
```

Register package tasks with the global task registry using new host-managed approach.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_id` | `-` | Unique identifier for the package (for cleanup tracking) |
| `package_data` | `-` | Binary data of the library package |
| `metadata` | `-` | Package metadata containing task information (legacy, for compatibility) |
| `tenant_id` | `-` | Tenant ID for namespace isolation (default: "public") |


**Returns:**

* `Ok(Vec<TaskNamespace>)` - List of registered task namespaces * `Err(LoaderError)` - If registration fails

<details>
<summary>Source</summary>

```rust
    pub async fn register_package_tasks(
        &self,
        package_id: &str,
        package_data: &[u8],
        _metadata: &PackageMetadata,
        tenant_id: Option<&str>,
    ) -> Result<Vec<TaskNamespace>, LoaderError> {
        let tenant_id = tenant_id.unwrap_or("public");

        // Extract task metadata from library using new FFI approach.
        // This returns owned data - all strings are copied before the library is unloaded.
        let task_metadata = self
            .extract_task_metadata_from_library(package_data)
            .await?;

        // Load the plugin library once — all tasks from this package share the handle.
        let plugin = Arc::new(
            DynamicLibraryTask::load_plugin(package_data, &task_metadata.package_name).map_err(
                |e| LoaderError::MetadataExtraction {
                    reason: format!("Failed to load plugin for task execution: {}", e),
                },
            )?,
        );

        // Register tasks in HOST global registry using metadata.
        let mut registered_namespaces = Vec::new();

        let workflow_name = &task_metadata.workflow_name;
        let package_name = &task_metadata.package_name;

        for task in &task_metadata.tasks {
            let task_id = &task.local_id;
            let dependencies_json = &task.dependencies_json;

            // Parse dependencies JSON to get dependency namespaces
            let dependency_namespaces: Vec<TaskNamespace> = if dependencies_json.trim() == "[]" {
                Vec::new()
            } else {
                let dep_names: Vec<String> =
                    serde_json::from_str(dependencies_json).map_err(|e| {
                        LoaderError::MetadataExtraction {
                            reason: format!(
                                "Failed to parse dependencies JSON '{}': {}",
                                dependencies_json, e
                            ),
                        }
                    })?;

                dep_names
                    .into_iter()
                    .map(|dep_name| {
                        if dep_name.contains("::") {
                            let full_name = dep_name.replace("{tenant}", tenant_id);
                            crate::parse_namespace(&full_name).map_err(|e| {
                                LoaderError::MetadataExtraction {
                                    reason: format!(
                                        "Invalid dependency namespace '{}': {}",
                                        full_name, e
                                    ),
                                }
                            })
                        } else {
                            Ok(TaskNamespace::new(
                                tenant_id,
                                package_name,
                                workflow_name,
                                &dep_name,
                            ))
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?
            };

            let namespace = TaskNamespace::new(tenant_id, package_name, workflow_name, task_id);

            let plugin = plugin.clone();
            let task_name = task_id.to_string();
            let pkg_name = package_name.to_string();
            let deps = dependency_namespaces.clone();

            let constructor = Box::new(move || {
                Arc::new(DynamicLibraryTask::new(
                    plugin.clone(),
                    task_name.clone(),
                    pkg_name.clone(),
                    deps.clone(),
                )) as Arc<dyn Task>
            });

            register_task_constructor(namespace.clone(), constructor);

            registered_namespaces.push(namespace);
        }

        // Track registered tasks for cleanup
        {
            let mut registered = self.registered_tasks.write();
            registered.insert(package_id.to_string(), registered_namespaces.clone());
        }

        tracing::info!(
            "Successfully registered {} tasks for package {} using host-managed approach",
            registered_namespaces.len(),
            package_name
        );

        Ok(registered_namespaces)
    }
```

</details>



##### `unregister_package_tasks` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn unregister_package_tasks (& self , package_id : & str) -> Result < () , LoaderError >
```

Unregister package tasks from the global registry.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_id` | `-` | Package identifier used during registration |


**Returns:**

* `Ok(())` - Tasks successfully unregistered * `Err(LoaderError)` - If unregistration fails

<details>
<summary>Source</summary>

```rust
    pub fn unregister_package_tasks(&self, package_id: &str) -> Result<(), LoaderError> {
        // Remove from tracked registrations
        let namespaces = {
            let mut registered = self.registered_tasks.write();
            registered.remove(package_id)
        };

        if let Some(namespaces) = namespaces {
            // Unregister tasks from global registry
            // Note: The global registry doesn't currently support removal,
            // so we'll track this for future implementation
            tracing::warn!(
                "Task unregistration requested for package '{}' with {} tasks, but global registry doesn't support removal yet",
                package_id,
                namespaces.len()
            );
        }

        // Remove package tracking entry
        {
            let mut packages = self.loaded_packages.write();
            packages.remove(package_id);
        }

        Ok(())
    }
```

</details>



##### `get_registered_namespaces` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_registered_namespaces (& self , package_id : & str) -> Vec < TaskNamespace >
```

Get the list of task namespaces registered for a package.

<details>
<summary>Source</summary>

```rust
    pub fn get_registered_namespaces(&self, package_id: &str) -> Vec<TaskNamespace> {
        let registered = self.registered_tasks.read();
        registered.get(package_id).cloned().unwrap_or_default()
    }
```

</details>



##### `loaded_package_count` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn loaded_package_count (& self) -> usize
```

Get the number of currently loaded packages.

<details>
<summary>Source</summary>

```rust
    pub fn loaded_package_count(&self) -> usize {
        let packages = self.loaded_packages.read();
        packages.len()
    }
```

</details>



##### `total_registered_tasks` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn total_registered_tasks (& self) -> usize
```

Get the total number of registered tasks across all packages.

<details>
<summary>Source</summary>

```rust
    pub fn total_registered_tasks(&self) -> usize {
        let registered = self.registered_tasks.read();
        registered.values().map(|tasks| tasks.len()).sum()
    }
```

</details>



##### `temp_dir` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn temp_dir (& self) -> & Path
```

Get the temporary directory path for manual operations.

<details>
<summary>Source</summary>

```rust
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }
```

</details>
