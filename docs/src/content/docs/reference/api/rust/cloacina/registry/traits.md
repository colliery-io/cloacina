---
title: "cloacina::registry::traits"
description: "Core trait definitions for the workflow registry system."
editUrl: false
sidebar:
  order: 170
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Core trait definitions for the workflow registry system.

This module defines the fundamental traits that enable pluggable storage
backends and consistent registry operations across different implementations.

## Traits

### <span id="trait.WorkflowRegistry"></span><code style="font-size: 0.85em">trait WorkflowRegistry</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Main trait for workflow registry operations.

The `WorkflowRegistry` trait defines the core operations for managing
packaged workflows. Implementations handle the coordination between
binary storage and metadata management.

**Implementation Notes**

- Implementations should validate packages before registration
- Task namespace isolation must be maintained
- Version conflicts should be handled gracefully
- Cleanup should be thorough when unregistering workflows

**Examples:**

```rust
use cloacina::registry::WorkflowRegistry;
use std::fs;

// Register a new workflow from file
let package_data = fs::read("analytics.cloacina")?;
let id = registry.register_workflow(package_data).await?;

// List all workflows
let workflows = registry.list_workflows().await?;
for workflow in workflows {
    println!("{} v{}", workflow.package_name, workflow.version);
}

// Unregister a specific version
registry.unregister_workflow("analytics", "1.0.0").await?;
```

#### Methods

##### <span id="WorkflowRegistry.register_workflow"></span><code>register_​workflow</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn register_workflow(
    &mut self,
    package_data: Vec<u8>,
) -> Result<WorkflowPackageId, RegistryError>
```

Register a new packaged workflow from binary data.

This operation:
1. Extracts metadata from the binary package data
2. Validates the package structure and metadata
3. Stores the binary data in registry storage
4. Stores metadata in the database
5. Registers tasks with the global task registry

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_data` | `Vec<u8>` | Raw binary data of the .cloacina package file |

**Returns:**

* `Ok(WorkflowPackageId)` - Unique identifier for the registered package
* `Err(RegistryError)` - If registration fails

**Errors:**

- `RegistryError::PackageExists`: If package/version already exists
- `RegistryError::ValidationError`: If package validation fails
- `RegistryError::StorageError`: If storage operations fail
- `RegistryError::Loader`: If metadata extraction fails

<details>
<summary>Source</summary>

```rust
    async fn register_workflow(
```

</details>

##### <span id="WorkflowRegistry.get_workflow"></span><code>get_​workflow</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_workflow(
    &self,
    package_name: &str,
    version: &str,
) -> Result<Option<LoadedWorkflow>, RegistryError>
```

Retrieve a specific workflow package by name and version.

This operation:
1. Queries metadata from the database
2. Retrieves binary data from storage if found
3. Returns both metadata and binary for loading

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_name` | `&str` | Name of the workflow package |
| `version` | `&str` | Specific version to retrieve |

**Returns:**

* `Ok(Some(LoadedWorkflow))` - If the workflow exists
* `Ok(None)` - If no matching workflow is found
* `Err(RegistryError)` - If retrieval fails

<details>
<summary>Source</summary>

```rust
    async fn get_workflow(
```

</details>

##### <span id="WorkflowRegistry.list_workflows"></span><code>list_​workflows</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_workflows(
    &self,
) -> Result<Vec<WorkflowMetadata>, RegistryError>
```

List all registered workflows in the registry.

Returns metadata for all workflows without loading binary data.
This enables efficient browsing of available workflows.

**Performance Note**

This operation only queries the metadata table, not the binary storage,
making it efficient even with many registered workflows.

**Returns:**

* `Ok(Vec<WorkflowMetadata>)` - List of all workflow metadata
* `Err(RegistryError)` - If listing fails

<details>
<summary>Source</summary>

```rust
    async fn list_workflows(&self) -> Result<Vec<WorkflowMetadata>, RegistryError>;
```

</details>

##### <span id="WorkflowRegistry.unregister_workflow"></span><code>unregister_​workflow</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn unregister_workflow(
    &mut self,
    package_name: &str,
    version: &str,
) -> Result<(), RegistryError>
```

Unregister a workflow package from the registry.

This operation:
1. Unregisters all tasks from the global task registry
2. Removes metadata from the database
3. Deletes binary data from storage (via cascade)
4. Cleans up any active schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_name` | `&str` | Name of the workflow package |
| `version` | `&str` | Specific version to unregister |

**Returns:**

* `Ok(())` - If unregistration succeeds
* `Err(RegistryError)` - If unregistration fails

**Errors:**

- `RegistryError::PackageNotFound`: If package/version doesn't exist
- `RegistryError::InUse`: If workflow has active executions

<details>
<summary>Source</summary>

```rust
    async fn unregister_workflow(
```

</details>

##### <span id="WorkflowRegistry.find_signature"></span><code>find_​signature</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn find_signature(
    &self,
    package_hash: &str,
) -> Result<bool, RegistryError>
```

Check whether a `package_signatures` row exists for the given SHA-256 hash. Used by the reconciler's defense-in-depth signature-existence check (CLOACI-T-0571) when `--require-signatures` is on. Implementations that don't have a signature backing store return `Ok(false)`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_hash` | `&str` | — hex-encoded SHA-256 of the package source archive. |

<details>
<summary>Source</summary>

```rust
    async fn find_signature(&self, package_hash: &str) -> Result<bool, RegistryError> {
        // Default impl: no signature store. Implementations backed by the
        // unified DB schema override this to query `package_signatures`.
        let _ = package_hash;
        Ok(false)
    }
```

</details>

##### <span id="WorkflowRegistry.get_compiled_data_for_target"></span><code>get_​compiled_​data_​for_​target</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_compiled_data_for_target(
    &self,
    package_name: &str,
    version: &str,
    target_triple: &str,
) -> Result<Option<Vec<u8>>, RegistryError>
```

CLOACI-T-0905 (multi-arch): the compiled cdylib for `(package, version)` built for `target_triple`, or `None` when no per-target artifact exists.

The reconciler prefers this over the primary `compiled_data` so an agent on
a different arch than the compiler host loads ITS OWN arch's cdylib (per-
target builds come from the compiler's `run_per_target` scan, T-0780).
`target_triple` uses the same `{arch}-{os}` format as
[`host_target_triple`](../../../../rust/cloacina/fleet/protocol/#fn.host_target_triple) — the
string both the compiler stamping and the fleet's fail-closed comparison
already share.

Default impl returns `Ok(None)` (no per-target store — filesystem,
in-memory, mocks), which makes callers fall back to the primary build.

<details>
<summary>Source</summary>

```rust
    async fn get_compiled_data_for_target(
        &self,
        package_name: &str,
        version: &str,
        target_triple: &str,
    ) -> Result<Option<Vec<u8>>, RegistryError> {
        let _ = (package_name, version, target_triple);
        Ok(None)
    }
```

</details>

##### <span id="WorkflowRegistry.persist_task_graph"></span><code>persist_​task_​graph</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn persist_task_graph(
    &self,
    package_id: WorkflowPackageId,
    tasks: Vec<(String, Vec<String>)>,
) -> Result<(), RegistryError>
```

Persist a workflow's task list (local ids + dependency edges) into the stored package metadata.

This exists for the **Python** load path: Python packages produce no
cdylib, so the cdylib-based metadata extraction in `mark_build_success`
never runs and the row's `tasks` stay empty. The reconciler captures the
task graph from the scoped Runtime at load time and calls this to write
it back, so the API/UI can show the task DAG the same way Rust packages
do. (CLOACI-T-0672)

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_id` | `WorkflowPackageId` | — the workflow package row id. |
| `tasks` | `Vec<(String, Vec<String>)>` | — `(local_task_id, [dependency_local_ids])` per task. |

<details>
<summary>Source</summary>

```rust
    async fn persist_task_graph(
        &self,
        package_id: WorkflowPackageId,
        tasks: Vec<(String, Vec<String>)>,
    ) -> Result<(), RegistryError> {
        let _ = (package_id, tasks);
        Ok(())
    }
```

</details>

##### <span id="WorkflowRegistry.get_package_providers"></span><code>get_​package_​providers</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_package_providers(
    &self,
    package_name: &str,
    version: &str,
) -> Result<Vec<(String, Vec<u8>)>, RegistryError>
```

Fetch the bundled constructor **providers** stored for a package (CLOACI-T-0836): `(provider_name, packed provider archive bytes)` pairs the reconciler unpacks into a `providers/` tree before resolving the package's `constructor!` nodes. Empty for packages that use no constructors.

Default impl: no provider store (filesystem, in-memory, mocks).
Implementations backed by the unified DB schema override this to query
`package_providers`.

<details>
<summary>Source</summary>

```rust
    async fn get_package_providers(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Vec<(String, Vec<u8>)>, RegistryError> {
        let _ = (package_name, version);
        Ok(Vec::new())
    }
```

</details>

##### <span id="WorkflowRegistry.request_recompile"></span><code>request_​recompile</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn request_recompile(
    &self,
    package_id: WorkflowPackageId,
) -> Result<bool, RegistryError>
```

CLOACI-T-0835: ask the compiler to REBUILD this package from its retained source, because its compiled artifact is stale (built against an older plugin ABI / interface version than this host expects).

Returns `true` when a rebuild was actually scheduled. Default impl: not
supported (`false`) — registries without a build pipeline (filesystem,
in-memory, mocks) can't recompile. The unified-DB registry overrides
this to flip `build_status` back to `pending` for the compiler to claim.

<details>
<summary>Source</summary>

```rust
    async fn request_recompile(
        &self,
        package_id: WorkflowPackageId,
    ) -> Result<bool, RegistryError> {
        let _ = package_id;
        Ok(false)
    }
```

</details>

### <span id="trait.RegistryStorage"></span><code style="font-size: 0.85em">trait RegistryStorage</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Trait for binary storage backends.

The `RegistryStorage` trait provides a simple key-value interface for
storing workflow binary data. This abstraction enables different storage
backends (PostgreSQL, object storage, filesystem) to be used interchangeably.

**Implementation Notes**

- Storage should be content-addressable where possible
- Binary integrity should be verified
- Cleanup should be atomic with metadata deletion

**Examples:**

```rust
use cloacina::registry::RegistryStorage;

// Store binary data
let data = std::fs::read("workflow.so")?;
let id = storage.store_binary(data).await?;

// Retrieve binary data
if let Some(data) = storage.retrieve_binary(&id).await? {
    println!("Retrieved {} bytes", data.len());
}

// Delete when no longer needed
storage.delete_binary(&id).await?;
```

#### Methods

##### <span id="RegistryStorage.store_binary"></span><code>store_​binary</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn store_binary(
    &mut self,
    data: Vec<u8>,
) -> Result<String, StorageError>
```

Store binary workflow data in the storage backend.

**Implementation Note**

The returned ID should be suitable for use as a foreign key
in the workflow_packages table.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `Vec<u8>` | Raw binary data (.so file contents) |

**Returns:**

* `Ok(String)` - Unique identifier for retrieving the data
* `Err(StorageError)` - If storage fails

<details>
<summary>Source</summary>

```rust
    async fn store_binary(&mut self, data: Vec<u8>) -> Result<String, StorageError>;
```

</details>

##### <span id="RegistryStorage.retrieve_binary"></span><code>retrieve_​binary</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn retrieve_binary(
    &self,
    id: &str,
) -> Result<Option<Vec<u8>>, StorageError>
```

Retrieve binary workflow data from storage.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `id` | `&str` | Unique identifier from store_binary |

**Returns:**

* `Ok(Some(Vec<u8>))` - Binary data if found
* `Ok(None)` - If no data exists for the given ID
* `Err(StorageError)` - If retrieval fails

<details>
<summary>Source</summary>

```rust
    async fn retrieve_binary(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError>;
```

</details>

##### <span id="RegistryStorage.delete_binary"></span><code>delete_​binary</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn delete_binary(&mut self, id: &str) -> Result<(), StorageError>
```

Delete binary workflow data from storage.

**Note**

Implementations should be idempotent - deleting non-existent
data should succeed silently.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `id` | `&str` | Unique identifier from store_binary |

**Returns:**

* `Ok(())` - If deletion succeeds (or data doesn't exist)
* `Err(StorageError)` - If deletion fails

<details>
<summary>Source</summary>

```rust
    async fn delete_binary(&mut self, id: &str) -> Result<(), StorageError>;
```

</details>

##### <span id="RegistryStorage.storage_type"></span><code>storage_​type</code>

```rust
fn storage_type(&self) -> StorageType
```

Returns the storage type for this backend.

This is used to record where the binary data is stored
in the workflow_packages metadata table.

**Returns:**

* `StorageType::Database` - For database-backed storage (PostgreSQL, SQLite)
* `StorageType::Filesystem` - For filesystem-backed storage

<details>
<summary>Source</summary>

```rust
    fn storage_type(&self) -> StorageType;
```

</details>
