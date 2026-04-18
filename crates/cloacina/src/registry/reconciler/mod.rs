/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! # Registry Reconciler
//!
//! The Registry Reconciler is responsible for synchronizing the persistent workflow registry
//! state with the in-memory task and workflow registries. It ensures that:
//!
//! - Packages registered in the database are loaded into the global registries
//! - Packages removed from the database are unloaded from the global registries
//! - System restarts properly restore all registered packages
//! - Dynamic package loading/unloading works seamlessly
//!
//! ## Key Components
//!
//! - `RegistryReconciler`: Main reconciliation service
//! - `ReconcilerConfig`: Configuration for reconciliation behavior
//! - `ReconcileResult`: Result of a reconciliation operation
//! - `PackageState`: Tracking loaded package state

mod loading;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::{interval, Interval};
use tracing::{debug, error, info, warn};

use crate::computation_graph::scheduler::ReactiveScheduler;
use crate::registry::error::RegistryError;
use crate::registry::loader::package_loader::PackageLoader;
use crate::registry::loader::task_registrar::TaskRegistrar;
use crate::registry::traits::WorkflowRegistry;
use crate::registry::types::{WorkflowMetadata, WorkflowPackageId};
use crate::task::TaskNamespace;

/// Configuration for the Registry Reconciler
#[derive(Debug, Clone)]
pub struct ReconcilerConfig {
    /// How often to run reconciliation
    pub reconcile_interval: Duration,

    /// Whether to perform startup reconciliation
    pub enable_startup_reconciliation: bool,

    /// Maximum time to wait for a single package load/unload operation
    pub package_operation_timeout: Duration,

    /// Whether to continue reconciliation if individual package operations fail
    pub continue_on_package_error: bool,

    /// Default tenant ID to use for package loading
    pub default_tenant_id: String,
}

impl Default for ReconcilerConfig {
    fn default() -> Self {
        Self {
            reconcile_interval: Duration::from_secs(30),
            enable_startup_reconciliation: true,
            package_operation_timeout: Duration::from_secs(30),
            continue_on_package_error: true,
            default_tenant_id: "public".to_string(),
        }
    }
}

/// Result of a reconciliation operation
#[derive(Debug, Clone)]
pub struct ReconcileResult {
    /// Packages that were loaded during this reconciliation
    pub packages_loaded: Vec<WorkflowPackageId>,

    /// Packages that were unloaded during this reconciliation
    pub packages_unloaded: Vec<WorkflowPackageId>,

    /// Packages that failed to load/unload
    pub packages_failed: Vec<(WorkflowPackageId, String)>,

    /// Total packages currently tracked
    pub total_packages_tracked: usize,

    /// Duration of the reconciliation operation
    pub reconciliation_duration: Duration,
}

impl ReconcileResult {
    /// Check if the reconciliation had any changes
    pub fn has_changes(&self) -> bool {
        !self.packages_loaded.is_empty() || !self.packages_unloaded.is_empty()
    }

    /// Check if the reconciliation had any failures
    pub fn has_failures(&self) -> bool {
        !self.packages_failed.is_empty()
    }
}

/// Tracks the state of loaded packages
#[derive(Debug, Clone)]
pub(super) struct PackageState {
    /// Package metadata
    pub(super) metadata: WorkflowMetadata,

    /// Task namespaces registered for this package
    pub(super) task_namespaces: Vec<TaskNamespace>,

    /// Workflow name registered for this package
    pub(super) workflow_name: Option<String>,

    /// Trigger names registered for this package
    pub(super) trigger_names: Vec<String>,

    /// Computation graph name loaded for this package (if any)
    pub(super) graph_name: Option<String>,
}

/// Status information about the reconciler
#[derive(Debug, Clone)]
pub struct ReconcilerStatus {
    /// Number of packages currently loaded
    pub packages_loaded: usize,

    /// Details about each loaded package
    pub package_details: Vec<PackageStatusDetail>,
}

/// Detailed status information about a loaded package
#[derive(Debug, Clone)]
pub struct PackageStatusDetail {
    /// Package name
    pub package_name: String,

    /// Package version
    pub version: String,

    /// Number of tasks registered
    pub task_count: usize,

    /// Whether a workflow was registered
    pub has_workflow: bool,
}

/// Registry Reconciler for synchronizing database state with in-memory registries
pub struct RegistryReconciler {
    /// Reference to the workflow registry for database operations
    pub(super) registry: Arc<dyn WorkflowRegistry>,

    /// Configuration for reconciliation behavior
    pub(super) config: ReconcilerConfig,

    /// Optional runtime handle. When set, the reconciler pushes
    /// newly-loaded/unloaded registrations through the runtime so executors
    /// looking up tasks/workflows/triggers/CGs/stream backends stay in sync
    /// with dynamic package loads.
    pub(super) runtime: Option<Arc<crate::Runtime>>,

    /// Tracking of currently loaded packages
    pub(super) loaded_packages: Arc<tokio::sync::RwLock<HashMap<WorkflowPackageId, PackageState>>>,

    /// Package loader for extracting metadata from .so files
    pub(super) package_loader: PackageLoader,

    /// Task registrar for managing dynamic task registration
    pub(super) task_registrar: TaskRegistrar,

    /// Shutdown signal receiver
    shutdown_rx: watch::Receiver<bool>,

    /// Reconciliation interval timer
    interval: Interval,

    /// Optional reactive scheduler for computation graph packages.
    /// Shared reference so it can be set after construction.
    reactive_scheduler: Arc<tokio::sync::RwLock<Option<Arc<ReactiveScheduler>>>>,
}

impl RegistryReconciler {
    /// Create a new Registry Reconciler
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
            reactive_scheduler: Arc::new(tokio::sync::RwLock::new(None)),
        })
    }

    /// Attach a Runtime to this reconciler. Package load/unload operations
    /// will push registrations through the runtime so executors see the
    /// same view as the reconciler.
    pub fn with_runtime(mut self, runtime: Arc<crate::Runtime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Set the reactive scheduler for computation graph package routing.
    pub fn with_reactive_scheduler(self, scheduler: Arc<ReactiveScheduler>) -> Self {
        // Use try_write since this is called during initialization (not async)
        if let Ok(mut lock) = self.reactive_scheduler.try_write() {
            *lock = Some(scheduler);
        }
        self
    }

    /// Replace the reactive scheduler slot with a shared reference from the runner.
    /// This allows the runner to inject the scheduler after construction.
    pub fn set_reactive_scheduler_slot(
        &mut self,
        slot: Arc<tokio::sync::RwLock<Option<Arc<ReactiveScheduler>>>>,
    ) {
        self.reactive_scheduler = slot;
    }

    /// Start the background reconciliation loop
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

    /// Perform a single reconciliation operation
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

    /// Perform cleanup operations during shutdown
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

    /// Get the current reconciliation status
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn test_reconciler_config_default() {
        let config = ReconcilerConfig::default();
        assert_eq!(config.reconcile_interval, Duration::from_secs(30));
        assert!(config.enable_startup_reconciliation);
        assert_eq!(config.package_operation_timeout, Duration::from_secs(30));
        assert!(config.continue_on_package_error);
        assert_eq!(config.default_tenant_id, "public");
    }

    #[test]
    fn test_reconcile_result_methods() {
        let result = ReconcileResult {
            packages_loaded: vec![Uuid::new_v4()],
            packages_unloaded: vec![],
            packages_failed: vec![],
            total_packages_tracked: 1,
            reconciliation_duration: Duration::from_millis(100),
        };

        assert!(result.has_changes());
        assert!(!result.has_failures());

        let result_no_changes = ReconcileResult {
            packages_loaded: vec![],
            packages_unloaded: vec![],
            packages_failed: vec![(Uuid::new_v4(), "error".to_string())],
            total_packages_tracked: 0,
            reconciliation_duration: Duration::from_millis(50),
        };

        assert!(!result_no_changes.has_changes());
        assert!(result_no_changes.has_failures());
    }

    #[test]
    fn test_reconciler_status() {
        let status = ReconcilerStatus {
            packages_loaded: 2,
            package_details: vec![
                PackageStatusDetail {
                    package_name: "pkg1".to_string(),
                    version: "1.0.0".to_string(),
                    task_count: 3,
                    has_workflow: true,
                },
                PackageStatusDetail {
                    package_name: "pkg2".to_string(),
                    version: "2.0.0".to_string(),
                    task_count: 1,
                    has_workflow: false,
                },
            ],
        };

        assert_eq!(status.packages_loaded, 2);
        assert_eq!(status.package_details.len(), 2);
        assert_eq!(status.package_details[0].package_name, "pkg1");
        assert!(status.package_details[0].has_workflow);
        assert!(!status.package_details[1].has_workflow);
    }

    #[test]
    fn test_reconciler_config_custom_values() {
        let config = ReconcilerConfig {
            reconcile_interval: Duration::from_secs(60),
            enable_startup_reconciliation: false,
            package_operation_timeout: Duration::from_secs(120),
            continue_on_package_error: false,
            default_tenant_id: "tenant-42".to_string(),
        };

        assert_eq!(config.reconcile_interval, Duration::from_secs(60));
        assert!(!config.enable_startup_reconciliation);
        assert_eq!(config.package_operation_timeout, Duration::from_secs(120));
        assert!(!config.continue_on_package_error);
        assert_eq!(config.default_tenant_id, "tenant-42");
    }

    #[test]
    fn test_reconcile_result_no_changes_no_failures() {
        let result = ReconcileResult {
            packages_loaded: vec![],
            packages_unloaded: vec![],
            packages_failed: vec![],
            total_packages_tracked: 5,
            reconciliation_duration: Duration::from_millis(10),
        };

        assert!(!result.has_changes());
        assert!(!result.has_failures());
        assert_eq!(result.total_packages_tracked, 5);
    }

    #[test]
    fn test_reconcile_result_unloaded_counts_as_change() {
        let result = ReconcileResult {
            packages_loaded: vec![],
            packages_unloaded: vec![Uuid::new_v4()],
            packages_failed: vec![],
            total_packages_tracked: 0,
            reconciliation_duration: Duration::from_millis(20),
        };

        assert!(result.has_changes());
        assert!(!result.has_failures());
    }

    #[test]
    fn test_reconcile_result_both_loaded_and_unloaded() {
        let result = ReconcileResult {
            packages_loaded: vec![Uuid::new_v4(), Uuid::new_v4()],
            packages_unloaded: vec![Uuid::new_v4()],
            packages_failed: vec![(Uuid::new_v4(), "timeout".to_string())],
            total_packages_tracked: 3,
            reconciliation_duration: Duration::from_secs(2),
        };

        assert!(result.has_changes());
        assert!(result.has_failures());
        assert_eq!(result.packages_loaded.len(), 2);
        assert_eq!(result.packages_unloaded.len(), 1);
        assert_eq!(result.packages_failed.len(), 1);
    }

    #[test]
    fn test_package_status_detail_fields() {
        let detail = PackageStatusDetail {
            package_name: "my-workflow".to_string(),
            version: "2.3.1".to_string(),
            task_count: 7,
            has_workflow: true,
        };

        assert_eq!(detail.package_name, "my-workflow");
        assert_eq!(detail.version, "2.3.1");
        assert_eq!(detail.task_count, 7);
        assert!(detail.has_workflow);
    }

    #[test]
    fn test_reconciler_status_empty() {
        let status = ReconcilerStatus {
            packages_loaded: 0,
            package_details: vec![],
        };

        assert_eq!(status.packages_loaded, 0);
        assert!(status.package_details.is_empty());
    }

    #[test]
    fn test_reconciler_config_clone() {
        let config = ReconcilerConfig::default();
        let cloned = config.clone();
        assert_eq!(config.reconcile_interval, cloned.reconcile_interval);
        assert_eq!(
            config.enable_startup_reconciliation,
            cloned.enable_startup_reconciliation
        );
        assert_eq!(config.default_tenant_id, cloned.default_tenant_id);
    }

    #[test]
    fn test_reconcile_result_clone() {
        let id = Uuid::new_v4();
        let result = ReconcileResult {
            packages_loaded: vec![id],
            packages_unloaded: vec![],
            packages_failed: vec![],
            total_packages_tracked: 1,
            reconciliation_duration: Duration::from_millis(50),
        };

        let cloned = result.clone();
        assert_eq!(cloned.packages_loaded, vec![id]);
        assert_eq!(cloned.reconciliation_duration, Duration::from_millis(50));
    }
}
