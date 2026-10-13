---
title: "cloacina::registry::reconciler"
description: "API reference of the Rust module cloacina::registry::reconciler."
editUrl: false
sidebar:
  order: 167
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Registry Reconciler**

The Registry Reconciler is responsible for synchronizing the persistent workflow registry
state with the in-memory task and workflow registries. It ensures that:

- Packages registered in the database are loaded into the global registries
- Packages removed from the database are unloaded from the global registries
- System restarts properly restore all registered packages
- Dynamic package loading/unloading works seamlessly

**Key Components**

- `RegistryReconciler`: Main reconciliation service
- `ReconcilerConfig`: Configuration for reconciliation behavior
- `ReconcileResult`: Result of a reconciliation operation
- `PackageState`: Tracking loaded package state

## Structs

### <span id="struct.ReconcilerConfig"></span><code style="font-size: 0.85em">struct ReconcilerConfig</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Configuration for the Registry Reconciler

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `reconcile_interval` | `Duration` | How often to run reconciliation |
| `enable_startup_reconciliation` | `bool` | Whether to perform startup reconciliation |
| `package_operation_timeout` | `Duration` | Maximum time to wait for a single package load/unload operation |
| `continue_on_package_error` | `bool` | Whether to continue reconciliation if individual package operations fail |
| `default_tenant_id` | `String` | Default tenant ID to use for package loading |
| `require_signatures` | `bool` | Defense-in-depth: when true, the reconciler refuses to load any `workflow_packages` row that has no companion `package_signatures` row (CLOACI-T-0571). The upload route is the strong gate (CLOACI-I-0103); this catches direct DB inserts that bypass it. |
| `verification_org_id` | `Option<crate::UniversalUuid>` | Trusted org UUID used as the `org_id` field on the audit-log entry when a signature-existence check fails. None when `require_signatures` is false. Mirrors `cloacina-server`'s `--verification-org-id` CLI flag. |

### <span id="struct.ReconcileResult"></span><code style="font-size: 0.85em">struct ReconcileResult</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Result of a reconciliation operation

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `packages_loaded` | `Vec<WorkflowPackageId>` | Packages that were loaded during this reconciliation |
| `packages_unloaded` | `Vec<WorkflowPackageId>` | Packages that were unloaded during this reconciliation |
| `packages_failed` | `Vec<(WorkflowPackageId, String)>` | Packages that failed to load/unload |
| `total_packages_tracked` | `usize` | Total packages currently tracked |
| `reconciliation_duration` | `Duration` | Duration of the reconciliation operation |

#### Methods

##### <span id="ReconcileResult.has_changes"></span><code>has_​changes</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn has_changes(&self) -> bool
```

Check if the reconciliation had any changes

<details>
<summary>Source</summary>

```rust
    pub fn has_changes(&self) -> bool {
        !self.packages_loaded.is_empty() || !self.packages_unloaded.is_empty()
    }
```

</details>

##### <span id="ReconcileResult.has_failures"></span><code>has_​failures</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn has_failures(&self) -> bool
```

Check if the reconciliation had any failures

<details>
<summary>Source</summary>

```rust
    pub fn has_failures(&self) -> bool {
        !self.packages_failed.is_empty()
    }
```

</details>

### <span id="struct.ReconcilerStatus"></span><code style="font-size: 0.85em">struct ReconcilerStatus</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Status information about the reconciler

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `packages_loaded` | `usize` | Number of packages currently loaded |
| `package_details` | `Vec<PackageStatusDetail>` | Details about each loaded package |

### <span id="struct.PackageStatusDetail"></span><code style="font-size: 0.85em">struct PackageStatusDetail</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Detailed status information about a loaded package

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package_name` | `String` | Package name |
| `version` | `String` | Package version |
| `task_count` | `usize` | Number of tasks registered |
| `has_workflow` | `bool` | Whether a workflow was registered |

### <span id="struct.RegistryReconciler"></span><code style="font-size: 0.85em">struct RegistryReconciler</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Registry Reconciler for synchronizing database state with in-memory registries

#### Methods

##### <span id="RegistryReconciler.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    registry: Arc<dyn WorkflowRegistry>,
    config: ReconcilerConfig,
    shutdown_rx: watch::Receiver<bool>,
) -> Result<Self, RegistryError>
```

Create a new Registry Reconciler

<details>
<summary>Source</summary>

```rust
    pub fn new(
        registry: Arc<dyn WorkflowRegistry>,
        config: ReconcilerConfig,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Result<Self, RegistryError> {
        let interval = interval(config.reconcile_interval);

        let package_loader = PackageLoader::new().map_err(RegistryError::Loader)?;
        let shared_cache = package_loader.handle_cache();

        let task_registrar =
            TaskRegistrar::with_handle_cache(shared_cache).map_err(RegistryError::Loader)?;

        Ok(Self {
            registry,
            config,
            runtime: None,
            loaded_packages: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            package_loader,
            task_registrar,
            shutdown_rx,
            interval,
            graph_scheduler: Arc::new(tokio::sync::RwLock::new(None)),
            cron_registrar: None,
            recompile_requested: std::sync::Mutex::new(std::collections::HashSet::new()),
        })
    }
```

</details>

##### <span id="RegistryReconciler.with_runtime"></span><code>with_​runtime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_runtime(mut self, runtime: Arc<crate::Runtime>) -> Self
```

Attach a Runtime to this reconciler. Package load/unload operations will push registrations through the runtime so executors see the same view as the reconciler.

<details>
<summary>Source</summary>

```rust
    pub fn with_runtime(mut self, runtime: Arc<crate::Runtime>) -> Self {
        self.runtime = Some(runtime);
        self
    }
```

</details>

##### <span id="RegistryReconciler.with_graph_scheduler"></span><code>with_​graph_​scheduler</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_graph_scheduler(
    self,
    scheduler: Arc<ComputationGraphScheduler>,
) -> Self
```

Set the graph scheduler for computation graph package routing.

<details>
<summary>Source</summary>

```rust
    pub fn with_graph_scheduler(self, scheduler: Arc<ComputationGraphScheduler>) -> Self {
        // Use try_write since this is called during initialization (not async)
        if let Ok(mut lock) = self.graph_scheduler.try_write() {
            *lock = Some(scheduler);
        }
        self
    }
```

</details>

##### <span id="RegistryReconciler.set_graph_scheduler_slot"></span><code>set_​graph_​scheduler_​slot</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn set_graph_scheduler_slot(
    &mut self,
    slot: Arc<tokio::sync::RwLock<Option<Arc<ComputationGraphScheduler>>>>,
)
```

Replace the graph scheduler slot with a shared reference from the runner. This allows the runner to inject the scheduler after construction.

<details>
<summary>Source</summary>

```rust
    pub fn set_graph_scheduler_slot(
        &mut self,
        slot: Arc<tokio::sync::RwLock<Option<Arc<ComputationGraphScheduler>>>>,
    ) {
        self.graph_scheduler = slot;
    }
```

</details>

##### <span id="RegistryReconciler.with_cron_registrar"></span><code>with_​cron_​registrar</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_cron_registrar(
    mut self,
    registrar: Arc<dyn CronWorkflowRegistrar>,
) -> Self
```

Attach a cron registrar that the reconciler will use to install cron schedules for each `#[trigger(cron = ...)]` declaration in loaded packages, and to drop them on unload. Builder-style counterpart for callers that wire the reconciler in one chained expression.

<details>
<summary>Source</summary>

```rust
    pub fn with_cron_registrar(mut self, registrar: Arc<dyn CronWorkflowRegistrar>) -> Self {
        self.cron_registrar = Some(registrar);
        self
    }
```

</details>

##### <span id="RegistryReconciler.set_cron_registrar"></span><code>set_​cron_​registrar</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn set_cron_registrar(
    &mut self,
    registrar: Arc<dyn CronWorkflowRegistrar>,
)
```

Inject a cron registrar after construction (mirrors `set_graph_scheduler_slot`). Used by `DefaultRunner` setup because the runner needs to construct the reconciler before it can build the cron registrar (the registrar holds runner-owned resources).

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_registrar(&mut self, registrar: Arc<dyn CronWorkflowRegistrar>) {
        self.cron_registrar = Some(registrar);
    }
```

</details>

##### <span id="RegistryReconciler.start_reconciliation_loop"></span><code>start_​reconciliation_​loop</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn start_reconciliation_loop(
    mut self) -> Result<(,
) -> Result<(), RegistryError>
```

Start the background reconciliation loop

<details>
<summary>Source</summary>

```rust
    pub async fn start_reconciliation_loop(mut self) -> Result<(), RegistryError> {
        info!(
            "Starting Registry Reconciler with interval {:?}",
            self.config.reconcile_interval
        );

        // Perform startup reconciliation if enabled
        if self.config.enable_startup_reconciliation {
            info!("Performing startup reconciliation");
            match self.reconcile().await {
                Ok(result) => {
                    info!(
                        "Startup reconciliation completed: {} loaded, {} unloaded, {} failed",
                        result.packages_loaded.len(),
                        result.packages_unloaded.len(),
                        result.packages_failed.len()
                    );
                }
                Err(e) => {
                    error!("Startup reconciliation failed: {}", e);
                    if !self.config.continue_on_package_error {
                        return Err(e);
                    }
                }
            }
        }

        // Main reconciliation loop
        loop {
            tokio::select! {
                _ = self.interval.tick() => {
                    debug!("Running periodic reconciliation");
                    match self.reconcile().await {
                        Ok(result) => {
                            if result.has_changes() {
                                info!(
                                    "Reconciliation completed: {} loaded, {} unloaded",
                                    result.packages_loaded.len(),
                                    result.packages_unloaded.len()
                                );
                            } else {
                                debug!("Reconciliation completed with no changes");
                            }

                            if result.has_failures() {
                                warn!("Reconciliation had {} failures", result.packages_failed.len());
                                for (package_id, error) in &result.packages_failed {
                                    warn!("Package {} failed: {}", package_id, error);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Reconciliation failed: {}", e);
                            if !self.config.continue_on_package_error {
                                return Err(e);
                            }
                        }
                    }
                }
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        info!("Registry Reconciler shutdown requested");
                        break;
                    }
                }
            }
        }

        // Perform cleanup on shutdown
        info!("Registry Reconciler shutting down");
        self.shutdown_cleanup().await?;

        Ok(())
    }
```

</details>

##### <span id="RegistryReconciler.reconcile"></span><code>reconcile</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn reconcile(&self) -> Result<ReconcileResult, RegistryError>
```

Perform a single reconciliation operation

<details>
<summary>Source</summary>

```rust
    pub async fn reconcile(&self) -> Result<ReconcileResult, RegistryError> {
        let start_time = std::time::Instant::now();

        // Get all packages from the database
        let db_packages = self.registry.list_workflows().await?;
        let db_package_ids: HashSet<WorkflowPackageId> = db_packages.iter().map(|p| p.id).collect();

        // Get currently loaded packages
        let loaded_packages = self.loaded_packages.read().await;
        let loaded_package_ids: HashSet<WorkflowPackageId> =
            loaded_packages.keys().cloned().collect();
        drop(loaded_packages);

        // Determine what needs to be loaded and unloaded.
        //
        // T-0553 follow-up: sort `packages_to_load` by registration
        // timestamp (`created_at`) so cross-package binding order is
        // deterministic. The HashSet difference produces an arbitrary
        // iteration order, which broke cross-package fan-out (subscriber
        // loading before publisher → "no such reactor is loaded"). For
        // unloads, sort REVERSE by created_at so dependents tear down
        // before publishers — this complements the per-package reverse
        // step pipeline (workflows → CGs → reactors → triggers → tasks)
        // by also reversing across packages.
        let mut packages_to_load: Vec<_> = db_package_ids
            .difference(&loaded_package_ids)
            .cloned()
            .collect();
        packages_to_load.sort_by_key(|id| {
            db_packages
                .iter()
                .find(|p| p.id == *id)
                .map(|p| p.created_at)
                .unwrap_or_else(chrono::Utc::now)
        });

        let mut packages_to_unload: Vec<_> = loaded_package_ids
            .difference(&db_package_ids)
            .cloned()
            .collect();
        // Best-effort reverse-creation-order unload using whatever metadata
        // the loaded_packages map still holds; fall back to package_id as
        // a stable tiebreaker.
        {
            let snapshot = self.loaded_packages.read().await;
            packages_to_unload.sort_by(|a, b| {
                let a_t = snapshot.get(a).map(|s| s.metadata.created_at);
                let b_t = snapshot.get(b).map(|s| s.metadata.created_at);
                b_t.cmp(&a_t).then_with(|| b.cmp(a))
            });
        }

        debug!(
            "Reconciliation: {} packages to load, {} to unload",
            packages_to_load.len(),
            packages_to_unload.len()
        );

        let mut result = ReconcileResult {
            packages_loaded: Vec::new(),
            packages_unloaded: Vec::new(),
            packages_failed: Vec::new(),
            total_packages_tracked: 0,
            reconciliation_duration: Duration::ZERO,
        };

        // Unload packages that are no longer in the database
        for package_id in packages_to_unload {
            match self.unload_package(package_id).await {
                Ok(()) => {
                    result.packages_unloaded.push(package_id);
                    info!("Unloaded package: {}", package_id);
                }
                Err(e) => {
                    let error_msg = format!("Failed to unload package {}: {}", package_id, e);
                    error!("{}", error_msg);
                    result.packages_failed.push((package_id, error_msg));

                    if !self.config.continue_on_package_error {
                        return Err(e);
                    }
                }
            }
        }

        // Load packages that are new in the database
        info!(
            "Reconciler: {} package(s) to load: {:?}",
            packages_to_load.len(),
            packages_to_load
        );
        for (pkg_idx, package_id) in packages_to_load.iter().enumerate() {
            info!(
                "Reconciler: starting package {}/{} (id={})",
                pkg_idx + 1,
                packages_to_load.len(),
                package_id
            );
            // Find the package metadata in db_packages
            if let Some(package_metadata) = db_packages.iter().find(|p| p.id == *package_id) {
                info!(
                    "Reconciler: loading {} v{} (id={})",
                    package_metadata.package_name, package_metadata.version, package_id
                );
                match self.load_package(package_metadata.clone()).await {
                    Ok(()) => {
                        result.packages_loaded.push(*package_id);
                        info!(
                            "Loaded package: {} v{}",
                            package_metadata.package_name, package_metadata.version
                        );
                    }
                    Err(e) => {
                        let error_msg = format!(
                            "Failed to load package {} ({}:{}): {}",
                            package_id, package_metadata.package_name, package_metadata.version, e
                        );
                        error!("{}", error_msg);

                        // CLOACI-T-0835: a STALE-ARTIFACT failure (the cdylib
                        // was built against an older plugin ABI / interface
                        // version than this host expects) is fixable by a
                        // rebuild from retained source — signal the compiler
                        // by flipping the row back to `pending`. Once per
                        // package per process, so a rebuild that comes back
                        // still-stale doesn't ping-pong forever.
                        if is_stale_artifact_error(&error_msg)
                            && self.recompile_requested.lock().unwrap().insert(*package_id)
                        {
                            match self.registry.request_recompile(*package_id).await {
                                Ok(true) => warn!(
                                    "Package {} v{} has a STALE compiled artifact (ABI/interface \
                                     mismatch) — requested a recompile from retained source; it \
                                     will reload once the compiler rebuilds it",
                                    package_metadata.package_name, package_metadata.version
                                ),
                                Ok(false) => debug!(
                                    "Stale artifact for {} v{}, but this registry cannot \
                                     schedule recompiles",
                                    package_metadata.package_name, package_metadata.version
                                ),
                                Err(req_err) => warn!(
                                    "Failed to request recompile for {} v{}: {}",
                                    package_metadata.package_name,
                                    package_metadata.version,
                                    req_err
                                ),
                            }
                        }

                        result.packages_failed.push((*package_id, error_msg));

                        if !self.config.continue_on_package_error {
                            return Err(e);
                        }
                    }
                }
            } else {
                let error_msg = format!("Package {} not found in database during load", package_id);
                error!("{}", error_msg);
                result.packages_failed.push((*package_id, error_msg));
            }
        }

        // Update total packages tracked
        let loaded_packages = self.loaded_packages.read().await;
        result.total_packages_tracked = loaded_packages.len();
        drop(loaded_packages);

        result.reconciliation_duration = start_time.elapsed();

        Ok(result)
    }
```

</details>

##### <span id="RegistryReconciler.get_status"></span><code>get_​status</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_status(&self) -> ReconcilerStatus
```

Get the current reconciliation status

<details>
<summary>Source</summary>

```rust
    pub async fn get_status(&self) -> ReconcilerStatus {
        let loaded_packages = self.loaded_packages.read().await;

        ReconcilerStatus {
            packages_loaded: loaded_packages.len(),
            package_details: loaded_packages
                .values()
                .map(|state| PackageStatusDetail {
                    package_name: state.metadata.package_name.clone(),
                    version: state.metadata.version.clone(),
                    task_count: state.task_namespaces.len(),
                    has_workflow: state.workflow_name.is_some(),
                })
                .collect(),
        }
    }
```

</details>

## Traits

### <span id="trait.CronWorkflowRegistrar"></span><code style="font-size: 0.85em">trait CronWorkflowRegistrar</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Trait the reconciler uses to register and unregister cron workflow schedules at package load/unload time. Implementations live on the runner side (the standalone daemon and the embedded `cloacina-server` runner both implement this against their cron scheduler / DAL). Decoupling this from the reconciler lets cron registration ride the same `reconcile()` lifecycle as every other primitive, instead of relying on the daemon's bespoke post-reconcile hook (which never fired in server mode).

#### Methods

##### <span id="CronWorkflowRegistrar.register_cron_workflow"></span><code>register_​cron_​workflow</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn register_cron_workflow(
    &self,
    workflow_name: &str,
    cron_expression: &str,
    timezone: &str,
) -> Result<String, String>
```

Create a cron schedule for a workflow. Returns an opaque schedule ID the reconciler hands back to [`unregister_cron_workflow`](../../../../rust/cloacina/registry/reconciler/#CronWorkflowRegistrar.unregister_cron_workflow) on unload.

<details>
<summary>Source</summary>

```rust
    async fn register_cron_workflow(
```

</details>

##### <span id="CronWorkflowRegistrar.unregister_cron_workflow"></span><code>unregister_​cron_​workflow</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn unregister_cron_workflow(
    &self) -> Result<(,
    schedule_id: &str) -> Result<(,
) -> Result<(), String>
```

Drop a cron schedule by the ID returned from [`register_cron_workflow`](../../../../rust/cloacina/registry/reconciler/#CronWorkflowRegistrar.register_cron_workflow).

<details>
<summary>Source</summary>

```rust
    async fn unregister_cron_workflow(&self, schedule_id: &str) -> Result<(), String>;
```

</details>

##### <span id="CronWorkflowRegistrar.register_poll_trigger"></span><code>register_​poll_​trigger</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn register_poll_trigger(
    &self,
    trigger_name: &str,
    workflow_name: &str,
    poll_interval_ms: i32,
    allow_concurrent: bool,
) -> Result<String, String>
```

Create a `trigger`-type schedule row for a custom-poll (non-cron) trigger. The in-memory `Trigger` impl is registered separately by the reconciler's runtime step; this persists the row the trigger scheduler polls (`get_enabled_triggers`) and the read API surfaces in the Triggers view (CLOACI-I-0124 / WS-6). Returns an opaque schedule ID dropped via [`unregister_cron_workflow`](../../../../rust/cloacina/registry/reconciler/#CronWorkflowRegistrar.unregister_cron_workflow) on unload (both schedule kinds are deleted by id). The default impl degrades loudly so a registrar without trigger support fails visibly rather than silently dropping poll triggers.

<details>
<summary>Source</summary>

```rust
    async fn register_poll_trigger(
        &self,
        trigger_name: &str,
        workflow_name: &str,
        poll_interval_ms: i32,
        allow_concurrent: bool,
    ) -> Result<String, String> {
        let _ = (
            trigger_name,
            workflow_name,
            poll_interval_ms,
            allow_concurrent,
        );
        Err("poll-trigger scheduling not supported by this registrar".to_string())
    }
```

</details>
