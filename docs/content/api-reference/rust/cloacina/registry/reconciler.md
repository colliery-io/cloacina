# cloacina::registry::reconciler <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>




## Structs

### `cloacina::registry::reconciler::ReconcilerConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::registry::reconciler::ReconcileResult`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Result of a reconciliation operation

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `packages_loaded` | `Vec < WorkflowPackageId >` | Packages that were loaded during this reconciliation |
| `packages_unloaded` | `Vec < WorkflowPackageId >` | Packages that were unloaded during this reconciliation |
| `packages_failed` | `Vec < (WorkflowPackageId , String) >` | Packages that failed to load/unload |
| `total_packages_tracked` | `usize` | Total packages currently tracked |
| `reconciliation_duration` | `Duration` | Duration of the reconciliation operation |

#### Methods

##### `has_changes` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn has_changes (& self) -> bool
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



##### `has_failures` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn has_failures (& self) -> bool
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





### `cloacina::registry::reconciler::PackageState`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


**Derives:** `Debug`, `Clone`

Tracks the state of loaded packages

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `metadata` | `WorkflowMetadata` | Package metadata |
| `task_namespaces` | `Vec < TaskNamespace >` | Task namespaces registered for this package |
| `workflow_name` | `Option < String >` | Workflow name registered for this package |
| `trigger_names` | `Vec < String >` | Trigger names registered for this package |
| `graph_name` | `Option < String >` | Computation graph name loaded for this package (if any) |



### `cloacina::registry::reconciler::ReconcilerStatus`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Status information about the reconciler

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `packages_loaded` | `usize` | Number of packages currently loaded |
| `package_details` | `Vec < PackageStatusDetail >` | Details about each loaded package |



### `cloacina::registry::reconciler::PackageStatusDetail`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Detailed status information about a loaded package

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package_name` | `String` | Package name |
| `version` | `String` | Package version |
| `task_count` | `usize` | Number of tasks registered |
| `has_workflow` | `bool` | Whether a workflow was registered |



### `cloacina::registry::reconciler::RegistryReconciler`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Registry Reconciler for synchronizing database state with in-memory registries

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `registry` | `Arc < dyn WorkflowRegistry >` | Reference to the workflow registry for database operations |
| `config` | `ReconcilerConfig` | Configuration for reconciliation behavior |
| `loaded_packages` | `Arc < tokio :: sync :: RwLock < HashMap < WorkflowPackageId , PackageState > > >` | Tracking of currently loaded packages |
| `package_loader` | `PackageLoader` | Package loader for extracting metadata from .so files |
| `task_registrar` | `TaskRegistrar` | Task registrar for managing dynamic task registration |
| `shutdown_rx` | `watch :: Receiver < bool >` | Shutdown signal receiver |
| `interval` | `Interval` | Reconciliation interval timer |
| `graph_scheduler` | `Arc < tokio :: sync :: RwLock < Option < Arc < ComputationGraphScheduler > > > >` | Optional graph scheduler for computation graph packages.
Shared reference so it can be set after construction. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (registry : Arc < dyn WorkflowRegistry > , config : ReconcilerConfig , shutdown_rx : watch :: Receiver < bool > ,) -> Result < Self , RegistryError >
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
            loaded_packages: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            package_loader,
            task_registrar,
            shutdown_rx,
            interval,
            graph_scheduler: Arc::new(tokio::sync::RwLock::new(None)),
        })
    }
```

</details>



##### `with_graph_scheduler` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_graph_scheduler (self , scheduler : Arc < ComputationGraphScheduler >) -> Self
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



##### `set_graph_scheduler_slot` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn set_graph_scheduler_slot (& mut self , slot : Arc < tokio :: sync :: RwLock < Option < Arc < ComputationGraphScheduler > > > > ,)
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



##### `start_reconciliation_loop` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn start_reconciliation_loop (mut self) -> Result < () , RegistryError >
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



##### `reconcile` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn reconcile (& self) -> Result < ReconcileResult , RegistryError >
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

        // Determine what needs to be loaded and unloaded
        let packages_to_load: Vec<_> = db_package_ids
            .difference(&loaded_package_ids)
            .cloned()
            .collect();

        let packages_to_unload: Vec<_> = loaded_package_ids
            .difference(&db_package_ids)
            .cloned()
            .collect();

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



##### `shutdown_cleanup` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn shutdown_cleanup (& self) -> Result < () , RegistryError >
```

Perform cleanup operations during shutdown

<details>
<summary>Source</summary>

```rust
    async fn shutdown_cleanup(&self) -> Result<(), RegistryError> {
        info!("Performing Registry Reconciler shutdown cleanup");

        // Optionally unload all packages during shutdown
        // For now, we'll just log the current state
        let loaded_packages = self.loaded_packages.read().await;
        if !loaded_packages.is_empty() {
            info!(
                "Shutdown with {} packages still loaded",
                loaded_packages.len()
            );
            for (package_id, state) in loaded_packages.iter() {
                debug!(
                    "Loaded package on shutdown: {} - {} v{}",
                    package_id, state.metadata.package_name, state.metadata.version
                );
            }
        }

        Ok(())
    }
```

</details>



##### `get_status` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn get_status (& self) -> ReconcilerStatus
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
