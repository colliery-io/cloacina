---
title: "cloacina::registry::workflow_registry"
description: "Complete implementation of the workflow registry."
editUrl: false
sidebar:
  order: 172
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Complete implementation of the workflow registry.

This module provides the `WorkflowRegistryImpl` that combines all registry
components - storage, loading, validation, and task registration - into a
cohesive system for managing packaged workflows.

## Re-exports

- [`BuildQueueStats`](../../../../rust/cloacina/registry/workflow_registry/database/#struct.BuildQueueStats)
- [`ReconcilerStats`](../../../../rust/cloacina/registry/workflow_registry/database/#struct.ReconcilerStats)
- [`build_queue_stats`](../../../../rust/cloacina/registry/workflow_registry/database/#fn.build_queue_stats)
- [`reconciler_stats`](../../../../rust/cloacina/registry/workflow_registry/database/#fn.reconciler_stats)

## Structs

### <span id="struct.WorkflowRegistryImpl"></span><code style="font-size: 0.85em">struct WorkflowRegistryImpl&lt;S: RegistryStorage&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Complete implementation of the workflow registry.

This registry implementation combines storage backends, package loading,
validation, and task registration to provide a full-featured system for
managing packaged workflows with proper lifecycle management.

#### Methods

##### <span id="WorkflowRegistryImpl.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(storage: S, database: Database) -> Result<Self, RegistryError>
```

Create a new workflow registry implementation.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `storage` | `S` | Storage backend for binary workflow data |
| `database` | `Database` | Database for metadata storage |

**Returns:**

* `Ok(WorkflowRegistryImpl)` - Successfully created registry
* `Err(RegistryError)` - If creation fails

<details>
<summary>Source</summary>

```rust
    pub fn new(storage: S, database: Database) -> Result<Self, RegistryError> {
        let loader = PackageLoader::new().map_err(RegistryError::Loader)?;
        let registrar = TaskRegistrar::new().map_err(RegistryError::Loader)?;

        Ok(Self {
            storage,
            database,
            loader,
            registrar,
            loaded_packages: HashMap::new(),
        })
    }
```

</details>

##### <span id="WorkflowRegistryImpl.loaded_package_count"></span><code>loaded_​package_​count</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn loaded_package_count(&self) -> usize
```

Get the number of currently loaded packages.

<details>
<summary>Source</summary>

```rust
    pub fn loaded_package_count(&self) -> usize {
        self.loaded_packages.len()
    }
```

</details>

##### <span id="WorkflowRegistryImpl.store_package_providers"></span><code>store_​package_​providers</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn store_package_providers(
    &self,
    package_name: &str,
    version: &str,
    target_triple: Option<&str>,
    providers: Vec<(String, String, String, Vec<u8>)>,
) -> Result<(), RegistryError>
```

Persist the bundled constructor providers a build produced for a package (CLOACI-T-0836): one `package_providers` row per provider, each carrying the packed `.cloacina` archive the reconciler unpacks at load. Called by the compiler after a successful build that discovered provider refs. `providers` is `(provider_name, provider_version, runtime, archive_bytes)` where `runtime` is `"wasm"` or `"native"`.

CLOACI-T-0908: `target_triple` is `None` for the PRIMARY compile and
`Some(triple)` for a per-target compiler's rebuild — triple-scoped rows
coexist with the primary, so a second-arch compile never clobbers it.

<details>
<summary>Source</summary>

```rust
    pub async fn store_package_providers(
        &self,
        package_name: &str,
        version: &str,
        target_triple: Option<&str>,
        providers: Vec<(String, String, String, Vec<u8>)>,
    ) -> Result<(), RegistryError> {
        use sha2::{Digest, Sha256};
        let dal = crate::dal::DAL::new(self.database.clone());
        for (provider_name, provider_version, runtime, archive) in providers {
            let mut hasher = Sha256::new();
            hasher.update(&archive);
            let content_hash = hex::encode(hasher.finalize());
            dal.workflow_packages()
                .upsert_provider(
                    package_name,
                    version,
                    None,
                    &provider_name,
                    &provider_version,
                    &content_hash,
                    archive,
                    &runtime,
                    target_triple,
                )
                .await?;
        }
        Ok(())
    }
```

</details>

##### <span id="WorkflowRegistryImpl.total_registered_tasks"></span><code>total_​registered_​tasks</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn total_registered_tasks(&self) -> usize
```

Get the total number of registered tasks across all packages.

<details>
<summary>Source</summary>

```rust
    pub fn total_registered_tasks(&self) -> usize {
        self.loaded_packages.values().map(|tasks| tasks.len()).sum()
    }
```

</details>

##### <span id="WorkflowRegistryImpl.register_workflow_package"></span><code>register_​workflow_​package</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn register_workflow_package(
    &mut self,
    package_data: Vec<u8>,
) -> Result<Uuid, RegistryError>
```

Register a workflow package (alias for register_workflow via the trait).

This is a convenience method that provides the same functionality as
the `register_workflow` trait method.

<details>
<summary>Source</summary>

```rust
    pub async fn register_workflow_package(
        &mut self,
        package_data: Vec<u8>,
    ) -> Result<Uuid, RegistryError> {
        // Use the trait implementation directly
        WorkflowRegistry::register_workflow(self, package_data).await
    }
```

</details>

##### <span id="WorkflowRegistryImpl.get_source_for_build"></span><code>get_​source_​for_​build</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_source_for_build(
    &self,
    package_id: Uuid,
) -> Result<Option<(WorkflowMetadata, Vec<u8>)>, RegistryError>
```

Get the source archive bytes for a package the compiler service has claimed. Unlike `get_workflow_package_by_id`, this does *not* filter by `build_status = 'success'` — the compiler needs to read source for rows currently in `building` state, which the reconciler-facing filter would hide. Still excludes superseded rows.

<details>
<summary>Source</summary>

```rust
    pub async fn get_source_for_build(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(WorkflowMetadata, Vec<u8>)>, RegistryError> {
        let ins = match self.inspect_package_by_id(package_id).await? {
            Some(ins) => ins,
            None => return Ok(None),
        };
        let registry_id = ins.metadata.registry_id.to_string();
        let package_data = match self.storage.retrieve_binary(&registry_id).await? {
            Some(data) => data,
            None => {
                return Err(RegistryError::Internal(
                    "Package metadata exists but binary data is missing".to_string(),
                ));
            }
        };
        Ok(Some((ins.metadata, package_data)))
    }
```

</details>

##### <span id="WorkflowRegistryImpl.get_workflow_source"></span><code>get_​workflow_​source</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_workflow_source(
    &self,
    package_id: Uuid,
) -> Result<Option<(WorkflowMetadata, Vec<WorkflowSourceFile>)>, RegistryError>
```

Extract the human-readable source files from a package's retained `.cloacina` archive for read-only display (CLOACI-T-0750).

The original source tree is always kept in registry storage (the
compiler unpacks it to build), independent of build status — so this
works for `building` and `failed` rows too, not just `success`. The
archive is unpacked in a temp dir and every UTF-8 text file is returned
(path relative to the source root + contents); binary and oversized
files (> `MAX_SOURCE_FILE_BYTES`) are skipped. Returns `None` when no
inspectable package exists for `package_id`.

<details>
<summary>Source</summary>

```rust
    pub async fn get_workflow_source(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(WorkflowMetadata, Vec<WorkflowSourceFile>)>, RegistryError> {
        let (metadata, archive_bytes) = match self.get_source_for_build(package_id).await? {
            Some(pair) => pair,
            None => return Ok(None),
        };

        // Unpacking (bzip2 + tar + filesystem walk) is blocking work; keep it
        // off the async runtime's worker threads.
        let files = tokio::task::spawn_blocking(move || extract_source_files(&archive_bytes))
            .await
            .map_err(|e| {
                RegistryError::Internal(format!("source extraction task panicked: {}", e))
            })??;

        Ok(Some((metadata, files)))
    }
```

</details>

##### <span id="WorkflowRegistryImpl.is_workflow_paused"></span><code>is_​workflow_​paused</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn is_workflow_paused(
    &self,
    name: &str,
) -> Result<bool, RegistryError>
```

Whether the workflow addressed by `name` (its executable `workflow_name` or `package_name`) is paused (CLOACI-T-0749). Unknown workflows are treated as not paused so resolution/“not found” is handled downstream by the executor. Used by the execute chokepoint to refuse new runs.

<details>
<summary>Source</summary>

```rust
    pub async fn is_workflow_paused(&self, name: &str) -> Result<bool, RegistryError> {
        let workflows = self.list_workflows().await?;
        Ok(workflows
            .into_iter()
            .find(|w| w.workflow_name == name || w.package_name == name)
            .map(|w| w.paused)
            .unwrap_or(false))
    }
```

</details>

##### <span id="WorkflowRegistryImpl.get_workflow_declared_params"></span><code>get_​workflow_​declared_​params</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_workflow_declared_params(
    &self,
    name: &str,
) -> Result<Vec<cloacina_api_types::InputSlot>, RegistryError>
```

Declared input params for the workflow addressed by `name` (its `workflow_name` or `package_name`), or empty when the workflow is unknown or declares none (CLOACI-I-0128). The execute chokepoint validates the provided context against these.

<details>
<summary>Source</summary>

```rust
    pub async fn get_workflow_declared_params(
        &self,
        name: &str,
    ) -> Result<Vec<cloacina_api_types::InputSlot>, RegistryError> {
        let workflows = self.list_workflows().await?;
        Ok(workflows
            .into_iter()
            .find(|w| w.workflow_name == name || w.package_name == name)
            .map(|w| w.declared_params)
            .unwrap_or_default())
    }
```

</details>

##### <span id="WorkflowRegistryImpl.workflow_exists"></span><code>workflow_​exists</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn workflow_exists(
    &self,
    name: &str,
) -> Result<bool, RegistryError>
```

Whether a workflow addressed by `name` (its `workflow_name` or `package_name`) is registered in THIS tenant's registry (CLOACI-T-0901). The execute chokepoint uses this to refuse cross-tenant execution: a tenant must not be able to run a workflow it never installed just because another tenant loaded the same name into the process-shared `Runtime`.

<details>
<summary>Source</summary>

```rust
    pub async fn workflow_exists(&self, name: &str) -> Result<bool, RegistryError> {
        let workflows = self.list_workflows().await?;
        Ok(workflows
            .iter()
            .any(|w| w.workflow_name == name || w.package_name == name))
    }
```

</details>

##### <span id="WorkflowRegistryImpl.find_trigger_subscribers"></span><code>find_​trigger_​subscribers</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn find_trigger_subscribers(
    &self,
    trigger_name: &str,
) -> Result<Vec<String>, RegistryError>
```

Workflow names subscribed to `trigger_name` via `#[workflow(triggers = […])]` (CLOACI-T-0777) — the fan-out set for a manual trigger fire. The schedules table only carries the trigger's primary `on` workflow, so subscriptions (which may live in other packages) are resolved here from package metadata.

<details>
<summary>Source</summary>

```rust
    pub async fn find_trigger_subscribers(
        &self,
        trigger_name: &str,
    ) -> Result<Vec<String>, RegistryError> {
        let workflows = self.list_workflows().await?;
        let mut seen = std::collections::HashSet::new();
        Ok(workflows
            .into_iter()
            .filter(|w| w.workflow_triggers.iter().any(|t| t == trigger_name))
            .map(|w| w.workflow_name)
            .filter(|name| seen.insert(name.clone()))
            .collect())
    }
```

</details>

##### <span id="WorkflowRegistryImpl.find_surface_input_slots"></span><code>find_​surface_​input_​slots</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn find_surface_input_slots(
    &self,
    kind: &str,
    name: &str,
) -> Result<Vec<cloacina_api_types::InputSlot>, RegistryError>
```

Declared input slots for a non-workflow injectable surface (`kind` = `"graph"` / `"reactor"` / `"accumulator"`) addressed by `name`, scanned across all registered packages. Empty when the surface is unknown or declares no typed interface (CLOACI-I-0128 T-0758). The operator fire/inject endpoints validate payloads against these (Task E).

<details>
<summary>Source</summary>

```rust
    pub async fn find_surface_input_slots(
        &self,
        kind: &str,
        name: &str,
    ) -> Result<Vec<cloacina_api_types::InputSlot>, RegistryError> {
        let workflows = self.list_workflows().await?;
        for w in workflows {
            for surface in w.declared_surfaces {
                if surface.kind == kind && surface.name == name {
                    return Ok(surface.slots);
                }
            }
        }
        Ok(Vec::new())
    }
```

</details>

##### <span id="WorkflowRegistryImpl.find_package_for_surface"></span><code>find_​package_​for_​surface</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn find_package_for_surface(
    &self,
    kind: &str,
    name: &str,
) -> Result<Option<String>, RegistryError>
```

The `package_name` of the registered package that declares the surface (`kind` = `"graph"` / `"reactor"` / `"accumulator"`) addressed by `name`, scanned across all registered packages (CLOACI-T-0773). Lets the graph health endpoint tell the UI which package's retained source defines a graph's nodes, so node code is one `…/source` fetch away. `None` when no package declares the surface (e.g. a graph with no typed interface). A surface match on the reactor or any accumulator resolves the package.

<details>
<summary>Source</summary>

```rust
    pub async fn find_package_for_surface(
        &self,
        kind: &str,
        name: &str,
    ) -> Result<Option<String>, RegistryError> {
        let workflows = self.list_workflows().await?;
        for w in workflows {
            for surface in &w.declared_surfaces {
                if surface.kind == kind && surface.name == name {
                    return Ok(Some(w.package_name.clone()));
                }
            }
        }
        Ok(None)
    }
```

</details>

##### <span id="WorkflowRegistryImpl.find_accumulator_input_slot"></span><code>find_​accumulator_​input_​slot</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn find_accumulator_input_slot(
    &self,
    name: &str,
) -> Result<Option<cloacina_api_types::InputSlot>, RegistryError>
```

The declared input slot for an accumulator addressed by `name` (CLOACI-I-0128 T-0758). An accumulator's boundary type is carried as a per-source slot inside its graph/reactor surface, so this scans all surfaces' slots for one whose name matches. `None` when unknown or untyped. The accumulator-inject endpoint validates the pushed event against this (Task E).

<details>
<summary>Source</summary>

```rust
    pub async fn find_accumulator_input_slot(
        &self,
        name: &str,
    ) -> Result<Option<cloacina_api_types::InputSlot>, RegistryError> {
        let workflows = self.list_workflows().await?;
        for w in workflows {
            for surface in w.declared_surfaces {
                if let Some(slot) = surface.slots.into_iter().find(|s| s.name == name) {
                    return Ok(Some(slot));
                }
            }
        }
        Ok(None)
    }
```

</details>

##### <span id="WorkflowRegistryImpl.set_workflow_paused"></span><code>set_​workflow_​paused</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn set_workflow_paused(
    &self,
    name: &str,
    paused: bool,
) -> Result<Option<Uuid>, RegistryError>
```

Pause or resume the workflow addressed by `name` (CLOACI-T-0749). Resolves `name` against the active package list (by `workflow_name` or `package_name`) and sets its `paused` flag. Returns the affected package id, or `None` if no matching active workflow exists.

<details>
<summary>Source</summary>

```rust
    pub async fn set_workflow_paused(
        &self,
        name: &str,
        paused: bool,
    ) -> Result<Option<Uuid>, RegistryError> {
        let workflows = self.list_workflows().await?;
        let Some(target) = workflows
            .into_iter()
            .find(|w| w.workflow_name == name || w.package_name == name)
        else {
            return Ok(None);
        };
        self.set_package_paused(target.id, paused).await?;
        Ok(Some(target.id))
    }
```

</details>

##### <span id="WorkflowRegistryImpl.get_workflow_package_by_id"></span><code>get_​workflow_​package_​by_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_workflow_package_by_id(
    &self,
    package_id: Uuid,
) -> Result<Option<(WorkflowMetadata, Vec<u8>)>, RegistryError>
```

Get a workflow package by its UUID.

Returns the package metadata and binary data if found.

<details>
<summary>Source</summary>

```rust
    pub async fn get_workflow_package_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(WorkflowMetadata, Vec<u8>)>, RegistryError> {
        // Get metadata from database
        let (registry_id, metadata, _compiled) =
            match self.get_package_metadata_by_id(package_id).await? {
                Some(data) => data,
                None => return Ok(None),
            };

        // Get binary data from storage
        let package_data = match self.storage.retrieve_binary(&registry_id).await? {
            Some(data) => data,
            None => {
                return Err(RegistryError::Internal(
                    "Package metadata exists but binary data is missing".to_string(),
                ));
            }
        };

        Ok(Some((metadata, package_data)))
    }
```

</details>

##### <span id="WorkflowRegistryImpl.get_workflow_package_by_name"></span><code>get_​workflow_​package_​by_​name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_workflow_package_by_name(
    &self,
    package_name: &str,
    version: &str,
) -> Result<Option<(WorkflowMetadata, Vec<u8>)>, RegistryError>
```

Get a workflow package by name and version.

Returns the package metadata and binary data if found.

<details>
<summary>Source</summary>

```rust
    pub async fn get_workflow_package_by_name(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<(WorkflowMetadata, Vec<u8>)>, RegistryError> {
        // Use the trait implementation and convert the result
        match self.get_workflow(package_name, version).await? {
            Some(loaded) => Ok(Some((loaded.metadata, loaded.package_data))),
            None => Ok(None),
        }
    }
```

</details>

##### <span id="WorkflowRegistryImpl.exists_by_id"></span><code>exists_​by_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn exists_by_id(
    &self,
    package_id: Uuid,
) -> Result<bool, RegistryError>
```

Check if a package exists by ID.

<details>
<summary>Source</summary>

```rust
    pub async fn exists_by_id(&self, package_id: Uuid) -> Result<bool, RegistryError> {
        Ok(self.get_package_metadata_by_id(package_id).await?.is_some())
    }
```

</details>

##### <span id="WorkflowRegistryImpl.exists_by_name"></span><code>exists_​by_​name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn exists_by_name(
    &self,
    package_name: &str,
    version: &str,
) -> Result<bool, RegistryError>
```

Check if a package exists by name and version.

<details>
<summary>Source</summary>

```rust
    pub async fn exists_by_name(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<bool, RegistryError> {
        Ok(self
            .get_package_metadata(package_name, version)
            .await?
            .is_some())
    }
```

</details>

##### <span id="WorkflowRegistryImpl.list_packages"></span><code>list_​packages</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_packages(
    &self,
) -> Result<Vec<WorkflowMetadata>, RegistryError>
```

List all packages in the registry.

Returns metadata for all registered packages.

<details>
<summary>Source</summary>

```rust
    pub async fn list_packages(&self) -> Result<Vec<WorkflowMetadata>, RegistryError> {
        self.list_all_packages().await
    }
```

</details>

##### <span id="WorkflowRegistryImpl.unregister_workflow_package_by_id"></span><code>unregister_​workflow_​package_​by_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn unregister_workflow_package_by_id(
    &mut self,
    package_id: Uuid,
) -> Result<(), RegistryError>
```

Unregister a workflow package by ID.

<details>
<summary>Source</summary>

```rust
    pub async fn unregister_workflow_package_by_id(
        &mut self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        // Get package metadata to find the registry_id for storage cleanup
        let (registry_id, _metadata, _compiled) =
            match self.get_package_metadata_by_id(package_id).await? {
                Some(data) => data,
                None => return Ok(()), // Idempotent - already doesn't exist
            };

        // Unregister tasks from global registry
        if let Some(_namespaces) = self.loaded_packages.remove(&package_id) {
            self.registrar
                .unregister_package_tasks(&package_id.to_string())
                .map_err(RegistryError::Loader)?;
        }

        // Delete metadata from database
        self.delete_package_metadata_by_id(package_id).await?;

        // Delete binary data from storage
        self.storage.delete_binary(&registry_id).await?;

        Ok(())
    }
```

</details>

##### <span id="WorkflowRegistryImpl.unregister_workflow_package_by_name"></span><code>unregister_​workflow_​package_​by_​name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn unregister_workflow_package_by_name(
    &mut self,
    package_name: &str,
    version: &str,
) -> Result<(), RegistryError>
```

Unregister a workflow package by name and version.

<details>
<summary>Source</summary>

```rust
    pub async fn unregister_workflow_package_by_name(
        &mut self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        // Check if package exists first
        if self
            .get_package_metadata(package_name, version)
            .await?
            .is_none()
        {
            return Ok(()); // Idempotent - already doesn't exist
        }

        // Use the trait implementation
        self.unregister_workflow(package_name, version).await
    }
```

</details>
