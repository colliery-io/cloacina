---
title: "cloacina::registry::loader::package_loader"
description: "Package loader for extracting metadata from workflow library files."
editUrl: false
sidebar:
  order: 162
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Package loader for extracting metadata from workflow library files.

This module provides functionality to safely load dynamic library files (.so/.dylib/.dll)
via the fidius-host plugin API and extract package metadata.

## Structs

### <span id="struct.PackageMetadata"></span><code style="font-size: 0.85em">struct PackageMetadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Metadata extracted from a workflow package.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package_name` | `String` | Package name |
| `workflow_name` | `String` | Workflow name the package registers (the `#[workflow(name = "...")]` value) — the identifier the runner/cron scheduler executes by. Distinct from `package_name` (e.g. package `demo-slow-rust` → workflow `demo_slow_workflow`). Persisted so the API can expose it and callers can execute by it (CLOACI-T-0671 / T-0663). `#[serde(default)]` keeps older stored metadata (without this field) deserializable. |
| `version` | `String` | Package version (extracted from library or defaults to "1.0.0") |
| `description` | `Option<String>` | Package description |
| `author` | `Option<String>` | Package author |
| `tasks` | `Vec<TaskMetadata>` | List of tasks provided by this package |
| `graph_data` | `Option<serde_json::Value>` | Workflow graph data (if available) |
| `architecture` | `String` | Library architecture info |
| `symbols` | `Vec<String>` | Required symbols present in the library |
| `workflow_triggers` | `Vec<String>` | I-0102 / T-A: Trigger names this package's workflow subscribes to, sourced from `#[workflow(triggers = […])]`. The reconciler binds each named trigger to the workflow at load time. |
| `declared_params` | `Vec<cloacina_api_types::InputSlot>` | CLOACI-I-0128 / T-0756: declared workflow params (named, JSON-Schema-typed input slots) from `#[workflow(params(...))]`, read via the input-interface FFI entrypoint at extraction time. Empty for packages that declare none or predate the entrypoint. |
| `declared_surfaces` | `Vec<cloacina_api_types::DeclaredSurface>` | CLOACI-I-0128 / T-0758: declared input interfaces of the package's non-workflow injectable surfaces (computation graphs, reactors, accumulators), read from the same `get_input_interface` entrypoint. Lets the server validate operator injections (reactor fire / accumulator inject) against the surface's boundary types. Empty when none are declared or the package predates the entrypoint. |
| `task_docs` | `std::collections::HashMap<String, TaskDocs>` | CLOACI-T-0754: the compiler's raw build-time doc parse (`what`/`why` per local task id), preserved verbatim so load paths that rebuild the task list AFTER build — the Python path has no cdylib, so its `tasks` are written by the reconciler at load — can re-merge docs instead of losing them. Rust packages get their docs overlaid onto `tasks` at build; this map is the durable source either way. Empty for undocumented packages and metadata predating the field. |

### <span id="struct.TaskMetadata"></span><code style="font-size: 0.85em">struct TaskMetadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Individual task metadata.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `index` | `u32` | Task index in the package |
| `local_id` | `String` | Local task identifier |
| `namespaced_id_template` | `String` | Namespaced ID template |
| `dependencies` | `Vec<String>` | Task dependencies as a list of local task IDs |
| `description` | `String` | Human-readable description |
| `source_location` | `String` | Source location information |
| `doc_what` | `Option<String>` | CLOACI-T-0752 "what" — a short summary of what the task does, parsed from the author's doc-comment / docstring at build time. `None` when the task is undocumented. `#[serde(default)]` keeps older stored metadata deserializable. |
| `doc_why` | `Option<String>` | CLOACI-T-0752 "why" — the rationale for the task (why it exists / when it matters), parsed from the doc-comment / docstring. `None` when absent. |

### <span id="struct.TaskDocs"></span><code style="font-size: 0.85em">struct TaskDocs</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Default`, `Serialize`, `Deserialize`

Structured "what & why" documentation for a single task (CLOACI-T-0752), parsed compiler-side from the author's source (Rust doc-comments / Python docstrings) and overlaid onto the persisted [`TaskMetadata`](../../../../../rust/cloacina/registry/loader/package_loader/#struct.TaskMetadata). Keyed by the task's local id.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `what` | `Option<String>` | Short summary of what the task does. |
| `why` | `Option<String>` | Rationale — why the task exists / when it matters. |

### <span id="struct.PackageLoader"></span><code style="font-size: 0.85em">struct PackageLoader</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

#### Methods

##### <span id="PackageLoader.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Result<Self, LoaderError>
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

##### <span id="PackageLoader.with_handle_cache"></span><code>with_​handle_​cache</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_handle_cache(
    cache: PluginHandleCache,
) -> Result<Self, LoaderError>
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

##### <span id="PackageLoader.handle_cache"></span><code>handle_​cache</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn handle_cache(&self) -> PluginHandleCache
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

##### <span id="PackageLoader.extract_metadata"></span><code>extract_​metadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn extract_metadata(
    &self,
    package_data: &[u8],
) -> Result<PackageMetadata, LoaderError>
```

Extract metadata from compiled library bytes.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_data` | `&[u8]` | Raw bytes of the compiled cdylib (.so / .dylib). The reconciler is responsible for unpacking and compiling any source archives before calling this method. |

**Returns:**

* `Ok(PackageMetadata)` - Successfully extracted metadata
* `Err(LoaderError)` - If extraction fails

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

##### <span id="PackageLoader.extract_graph_metadata"></span><code>extract_​graph_​metadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn extract_graph_metadata(
    &self,
    package_data: &[u8],
) -> Result<Option<cloacina_workflow_plugin::GraphPackageMetadata>, LoaderError>
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

        let result = match handle.call_method::<(), cloacina_workflow_plugin::GraphPackageMetadata>(
            cloacina_workflow_plugin::METHOD_GET_GRAPH_METADATA,
            &(),
        ) {
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

##### <span id="PackageLoader.extract_reactor_metadata"></span><code>extract_​reactor_​metadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn extract_reactor_metadata(
    &self,
    package_data: &[u8],
) -> Result<Vec<cloacina_workflow_plugin::ReactorPackageMetadata>, LoaderError>
```

Extract reactor metadata from compiled library bytes (T-B / I-0102).

Calls `get_reactor_metadata()` (method index 4) on the fidius plugin.
Plugins that predate trait v2 — and per-macro `_ffi` blocks emitting
the empty stub — both return `Ok(vec![])` here. Real reactor entries
come from packages built against the unified `cloacina::package!()`
shell.

<details>
<summary>Source</summary>

```rust
    pub async fn extract_reactor_metadata(
        &self,
        package_data: &[u8],
    ) -> Result<Vec<cloacina_workflow_plugin::ReactorPackageMetadata>, LoaderError> {
        let library_extension = get_library_extension();
        let temp_path = self.temp_dir.path().join(format!(
            "reactor_{}.{}",
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

        let result = crate::computation_graph::packaging_bridge::call_get_reactor_metadata(&handle)
            .map_err(|e| LoaderError::MetadataExtraction { reason: e });

        if let Ok(mut cache) = self.handle_cache.lock() {
            cache.push(handle);
        }

        result
    }
```

</details>

##### <span id="PackageLoader.extract_constructor_metadata"></span><code>extract_​constructor_​metadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn extract_constructor_metadata(
    &self,
    package_data: &[u8],
) -> Result<Vec<cloacina_workflow_plugin::ConstructorPackageMetadata>, LoaderError>
```

Extract packaged `constructor!(...)` node declarations from compiled library bytes (CLOACI-T-0832).

Calls `get_constructor_metadata()` (method index 10) on the fidius plugin.
Plugins that predate trait v4 return `Ok(vec![])` here. Each returned
`cloacina_workflow_plugin::ConstructorPackageMetadata` is resolved by the
host (which links the WASM constructor loader) via `load_constructor_node`
— applying the tenant capability grants — and injected into the rebuilt
workflow DAG.

<details>
<summary>Source</summary>

```rust
    pub async fn extract_constructor_metadata(
        &self,
        package_data: &[u8],
    ) -> Result<Vec<cloacina_workflow_plugin::ConstructorPackageMetadata>, LoaderError> {
        let library_extension = get_library_extension();
        let temp_path = self.temp_dir.path().join(format!(
            "constructor_{}.{}",
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

        let result =
            crate::computation_graph::packaging_bridge::call_get_constructor_metadata(&handle)
                .map_err(|e| LoaderError::MetadataExtraction { reason: e });

        if let Ok(mut cache) = self.handle_cache.lock() {
            cache.push(handle);
        }

        result
    }
```

</details>

##### <span id="PackageLoader.extract_trigger_metadata"></span><code>extract_​trigger_​metadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn extract_trigger_metadata(
    &self,
    package_data: &[u8],
) -> Result<Vec<cloacina_workflow_plugin::TriggerPackageMetadata>, LoaderError>
```

Extract trigger metadata from compiled library bytes (T-B / I-0102).

Calls `get_trigger_metadata()` (method index 5) on the fidius plugin.
Cron-vs-custom routing happens at the reconciler based on the
returned `cron_expression` field.

<details>
<summary>Source</summary>

```rust
    pub async fn extract_trigger_metadata(
        &self,
        package_data: &[u8],
    ) -> Result<Vec<cloacina_workflow_plugin::TriggerPackageMetadata>, LoaderError> {
        let library_extension = get_library_extension();
        let temp_path = self.temp_dir.path().join(format!(
            "trigger_{}.{}",
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

        let result = crate::computation_graph::packaging_bridge::call_get_trigger_metadata(&handle)
            .map_err(|e| LoaderError::MetadataExtraction { reason: e });

        if let Ok(mut cache) = self.handle_cache.lock() {
            cache.push(handle);
        }

        result
    }
```

</details>

##### <span id="PackageLoader.extract_triggerless_graph_metadata"></span><code>extract_​triggerless_​graph_​metadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn extract_triggerless_graph_metadata(
    &self,
    package_data: &[u8],
) -> Result<Vec<cloacina_workflow_plugin::TriggerlessGraphMetadataEntry>, LoaderError>
```

Extract trigger-less computation graph metadata from compiled library bytes (T-0553 follow-up — Trigger-less CG FFI bridge).

Calls `get_triggerless_graph_metadata()` (method index 7) on
the fidius plugin. Returns one entry per `#[computation_graph]`
without a `trigger = reactor(...)` clause; the reconciler
installs each into the runtime via a host-side adapter that
dispatches `graph_fn` through `invoke_triggerless_graph` (FFI
method index 8).

<details>
<summary>Source</summary>

```rust
    pub async fn extract_triggerless_graph_metadata(
        &self,
        package_data: &[u8],
    ) -> Result<Vec<cloacina_workflow_plugin::TriggerlessGraphMetadataEntry>, LoaderError> {
        let library_extension = get_library_extension();
        let temp_path = self.temp_dir.path().join(format!(
            "triggerless_{}.{}",
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

        // Method index 7 = get_triggerless_graph_metadata. NotImplemented
        // (older plugins) returns an Ok(Vec::new()) at the call_method
        // layer — the reconciler treats that as "package declares no
        // trigger-less graphs".
        let result: Result<
            Vec<cloacina_workflow_plugin::TriggerlessGraphMetadataEntry>,
            fidius_host::CallError,
        > = handle.call_method(
            cloacina_workflow_plugin::METHOD_GET_TRIGGERLESS_GRAPH_METADATA,
            &(),
        );

        let out = match result {
            Ok(v) => Ok(v),
            Err(fidius_host::CallError::NotImplemented { .. }) => Ok(Vec::new()),
            Err(e) => Err(LoaderError::MetadataExtraction {
                reason: format!("get_triggerless_graph_metadata failed: {:?}", e),
            }),
        };

        if let Ok(mut cache) = self.handle_cache.lock() {
            cache.push(handle);
        }

        out
    }
```

</details>

##### <span id="PackageLoader.temp_dir"></span><code>temp_​dir</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn temp_dir(&self) -> &Path
```

Get the temporary directory path. Test-only: production code does not inspect the loader's temp dir directly.

<details>
<summary>Source</summary>

```rust
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }
```

</details>

##### <span id="PackageLoader.validate_package_symbols"></span><code>validate_​package_​symbols</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn validate_package_symbols(
    &self,
    package_data: &[u8],
) -> Result<Vec<String>, LoaderError>
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

### <span id="fn.get_library_extension"></span><code style="font-size: 0.85em">fn get_​library_​extension</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_library_extension() -> &'static str
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

## Type aliases

| Name | Type | Description |
|------|------|-------------|
| <span id="type.PluginHandleCache"></span>`PluginHandleCache` | `std::sync::Arc<std::sync::Mutex<Vec<fidius_host::PluginHandle>>>` | Package loader for extracting metadata from workflow library files.<br>Loaded libraries are cached to prevent dlclose. The `inventory` crate (used by fidius) builds a global linked list via `__attribute__((constructor))`. If a cdylib is dlclosed, the linked-list nodes are unmapped but the head pointer still references them. Loading a second cdylib then crashes when its constructors traverse the corrupted list (macOS sends SIGKILL). Keeping libraries alive avoids this entirely. Shared cache of plugin handles kept alive to prevent dlclose.<br>The `inventory` crate (used by fidius) builds a global intrusive linked list via `__attribute__((constructor))`. If a cdylib is dlclosed, the list nodes are unmapped but the head pointer still references them. Loading a second cdylib then crashes when its constructors traverse the corrupted list (macOS sends SIGKILL). Keeping handles alive avoids dlclose entirely. |
