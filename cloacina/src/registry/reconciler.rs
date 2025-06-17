/*
 *  Copyright 2025 Colliery Software
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

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::{interval, Interval};
use tracing::{debug, error, info, warn};

use crate::registry::error::RegistryError;
use crate::registry::loader::package_loader::PackageLoader;
use crate::registry::loader::task_registrar::TaskRegistrar;
use crate::registry::traits::WorkflowRegistry;
use crate::registry::types::{WorkflowMetadata, WorkflowPackageId};
use crate::task::{global_task_registry, TaskNamespace};
use crate::workflow::global_workflow_registry;

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
struct PackageState {
    /// Package metadata
    metadata: WorkflowMetadata,

    /// Task namespaces registered for this package
    task_namespaces: Vec<TaskNamespace>,

    /// Workflow name registered for this package
    workflow_name: Option<String>,
}

/// Registry Reconciler for synchronizing database state with in-memory registries
pub struct RegistryReconciler {
    /// Reference to the workflow registry for database operations
    registry: Arc<dyn WorkflowRegistry>,

    /// Configuration for reconciliation behavior
    config: ReconcilerConfig,

    /// Tracking of currently loaded packages
    loaded_packages: Arc<tokio::sync::RwLock<HashMap<WorkflowPackageId, PackageState>>>,

    /// Package loader for extracting metadata from .so files
    package_loader: PackageLoader,

    /// Task registrar for managing dynamic task registration
    task_registrar: TaskRegistrar,

    /// Shutdown signal receiver
    shutdown_rx: watch::Receiver<bool>,

    /// Reconciliation interval timer
    interval: Interval,
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

        let task_registrar = TaskRegistrar::new().map_err(RegistryError::Loader)?;

        Ok(Self {
            registry,
            config,
            loaded_packages: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            package_loader,
            task_registrar,
            shutdown_rx,
            interval,
        })
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
        for package_id in packages_to_load {
            // Find the package metadata in db_packages
            if let Some(package_metadata) = db_packages.iter().find(|p| p.id == package_id) {
                match self.load_package(package_metadata.clone()).await {
                    Ok(()) => {
                        result.packages_loaded.push(package_id);
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
                        result.packages_failed.push((package_id, error_msg));

                        if !self.config.continue_on_package_error {
                            return Err(e);
                        }
                    }
                }
            } else {
                let error_msg = format!("Package {} not found in database during load", package_id);
                error!("{}", error_msg);
                result.packages_failed.push((package_id, error_msg));
            }
        }

        // Update total packages tracked
        let loaded_packages = self.loaded_packages.read().await;
        result.total_packages_tracked = loaded_packages.len();
        drop(loaded_packages);

        result.reconciliation_duration = start_time.elapsed();

        Ok(result)
    }

    /// Load a package into the global registries
    async fn load_package(&self, metadata: WorkflowMetadata) -> Result<(), RegistryError> {
        debug!(
            "Loading package: {} v{}",
            metadata.package_name, metadata.version
        );

        // Get the package binary data from the registry
        let loaded_workflow = self
            .registry
            .get_workflow(&metadata.package_name, &metadata.version)
            .await?
            .ok_or_else(|| RegistryError::PackageNotFound {
                package_name: metadata.package_name.clone(),
                version: metadata.version.clone(),
            })?;

        // Extract package metadata and register tasks
        // This would use the package loader to load the .so file and register tasks/workflows
        // For now, we'll create a placeholder implementation

        // Extract library data from .cloacina archive if needed
        let is_cloacina = self.is_cloacina_package(&loaded_workflow.package_data);
        debug!(
            "Package data format check: is_cloacina={}, data_len={}, first_bytes={:02x?}",
            is_cloacina,
            loaded_workflow.package_data.len(),
            &loaded_workflow.package_data[..std::cmp::min(10, loaded_workflow.package_data.len())]
        );

        let library_data = if is_cloacina {
            debug!(
                "Extracting library from .cloacina archive for package: {}",
                metadata.package_name
            );
            self.extract_library_from_cloacina(&loaded_workflow.package_data)
                .await?
        } else {
            debug!(
                "Using raw library data for package: {}",
                metadata.package_name
            );
            loaded_workflow.package_data.clone()
        };

        let task_namespaces = self
            .register_package_tasks(&metadata, &library_data)
            .await?;
        let workflow_name = self
            .register_package_workflows(&metadata, &library_data)
            .await?;

        // Track the loaded package state
        let package_state = PackageState {
            metadata: metadata.clone(),
            task_namespaces,
            workflow_name,
        };

        let mut loaded_packages = self.loaded_packages.write().await;
        loaded_packages.insert(metadata.id, package_state);

        Ok(())
    }

    /// Unload a package from the global registries
    async fn unload_package(&self, package_id: WorkflowPackageId) -> Result<(), RegistryError> {
        debug!("Unloading package: {}", package_id);

        // Get the package state to know what to unload
        let mut loaded_packages = self.loaded_packages.write().await;
        let package_state =
            loaded_packages
                .remove(&package_id)
                .ok_or_else(|| RegistryError::PackageNotFound {
                    package_name: package_id.to_string(),
                    version: "unknown".to_string(),
                })?;
        drop(loaded_packages);

        // Unregister tasks from global task registry
        self.unregister_package_tasks(package_id, &package_state.task_namespaces)
            .await?;

        // Unregister workflow from global workflow registry
        if let Some(workflow_name) = &package_state.workflow_name {
            self.unregister_package_workflow(workflow_name).await?;
        }

        info!(
            "Unloaded package: {} v{}",
            package_state.metadata.package_name, package_state.metadata.version
        );

        Ok(())
    }

    /// Register tasks from a package into the global task registry
    async fn register_package_tasks(
        &self,
        metadata: &WorkflowMetadata,
        package_data: &[u8],
    ) -> Result<Vec<TaskNamespace>, RegistryError> {
        debug!(
            "Loading tasks for package: {} v{}",
            metadata.package_name, metadata.version
        );

        // Extract metadata from the .so file using PackageLoader
        let package_metadata = self
            .package_loader
            .extract_metadata(package_data)
            .await
            .map_err(RegistryError::Loader)?;

        debug!(
            "Package {} contains {} tasks",
            package_metadata.package_name,
            package_metadata.tasks.len()
        );

        // Register tasks using TaskRegistrar
        let package_id = metadata.id.to_string();
        let tenant_id = Some(self.config.default_tenant_id.as_str());

        let task_namespaces = self
            .task_registrar
            .register_package_tasks(&package_id, package_data, &package_metadata, tenant_id)
            .await
            .map_err(RegistryError::Loader)?;

        info!(
            "Successfully registered {} tasks for package {} v{}",
            task_namespaces.len(),
            metadata.package_name,
            metadata.version
        );

        Ok(task_namespaces)
    }

    /// Register workflows from a package into the global workflow registry
    async fn register_package_workflows(
        &self,
        metadata: &WorkflowMetadata,
        package_data: &[u8],
    ) -> Result<Option<String>, RegistryError> {
        debug!(
            "Loading workflows for package: {} v{}",
            metadata.package_name, metadata.version
        );

        // Extract metadata from the .so file using PackageLoader
        let package_metadata = self
            .package_loader
            .extract_metadata(package_data)
            .await
            .map_err(RegistryError::Loader)?;

        // Check if package has workflow data
        if let Some(ref graph_data) = package_metadata.graph_data {
            debug!(
                "Package {} has workflow graph data with {} tasks",
                metadata.package_name,
                graph_data
                    .get("metadata")
                    .and_then(|m| m.get("task_count"))
                    .and_then(|c| c.as_u64())
                    .unwrap_or(0)
            );

            // Extract the workflow_id from package metadata by parsing the namespaced_id_template
            // Template format: {tenant}::package_name::workflow_id::task_id
            let workflow_id = if let Some(first_task) = package_metadata.tasks.first() {
                let template = &first_task.namespaced_id_template;
                debug!("Parsing workflow_id from template: '{}'", template);

                // Split by "::" and extract the workflow_id part (3rd component)
                let parts: Vec<&str> = template.split("::").collect();
                if parts.len() >= 3 {
                    let workflow_part = parts[2];
                    // Handle both {workflow} placeholder and actual workflow_id
                    if workflow_part == "{workflow}" {
                        // This is a template, need to look up actual workflow_id from registered tasks
                        let task_registry = crate::task::global_task_registry();
                        let mut found_id = None;
                        if let Ok(registry) = task_registry.read() {
                            for (namespace, _) in registry.iter() {
                                if namespace.package_name == metadata.package_name
                                    && namespace.tenant_id == self.config.default_tenant_id
                                {
                                    debug!(
                                        "Found registered task with workflow_id: '{}'",
                                        namespace.workflow_id
                                    );
                                    found_id = Some(namespace.workflow_id.clone());
                                    break;
                                }
                            }
                        }
                        // Use found ID or fallback
                        found_id.unwrap_or_else(|| metadata.package_name.clone())
                    } else {
                        // This is the actual workflow_id
                        workflow_part.to_string()
                    }
                } else {
                    debug!("Template format unexpected, using package name as fallback");
                    metadata.package_name.clone()
                }
            } else {
                debug!("No tasks in package metadata, using package name as fallback");
                metadata.package_name.clone()
            };

            debug!(
                "Using workflow_id '{}' for FFI constructor (extracted from task metadata)",
                workflow_id
            );

            // Use workflow_id as the workflow name for registration
            let workflow_name = workflow_id.clone();

            // Call the workflow constructor FFI function from the package
            let _workflow = self.create_workflow_from_package(
                package_data,
                &self.config.default_tenant_id,
                &workflow_id,
            )?;

            // Register workflow constructor with global workflow registry
            let workflow_registry = global_workflow_registry();
            let mut registry =
                workflow_registry
                    .write()
                    .map_err(|e| RegistryError::RegistrationFailed {
                        message: format!("Failed to access workflow registry: {}", e),
                    })?;

            // Create a constructor that calls the package's FFI function
            let workflow_name_for_closure = workflow_name.clone();
            let package_data_for_closure = package_data.to_vec();
            let tenant_id_for_closure = self.config.default_tenant_id.clone();
            let workflow_id_for_closure = workflow_id.clone();

            registry.insert(
                workflow_name.clone(),
                Box::new(move || {
                    debug!(
                        "Creating workflow instance for {} using FFI constructor",
                        workflow_name_for_closure
                    );

                    // We could store the workflow and clone it, but for now we'll create a new one each time
                    // In a real implementation, we might want to cache the created workflow
                    match Self::call_workflow_constructor_ffi(
                        &package_data_for_closure,
                        &tenant_id_for_closure,
                        &workflow_id_for_closure,
                    ) {
                        Ok(workflow) => workflow,
                        Err(e) => {
                            error!("Failed to create workflow from FFI constructor: {}", e);
                            // Fallback to empty workflow
                            crate::workflow::Workflow::new(&workflow_name_for_closure)
                        }
                    }
                }),
            );

            info!(
                "Registered workflow '{}' for package {} v{}",
                workflow_name, metadata.package_name, metadata.version
            );

            Ok(Some(workflow_name))
        } else {
            debug!(
                "Package {} has no workflow data - registering as task-only package",
                metadata.package_name
            );
            Ok(None)
        }
    }

    /// Create a workflow from a package using its FFI constructor function
    fn create_workflow_from_package(
        &self,
        package_data: &[u8],
        tenant_id: &str,
        workflow_id: &str,
    ) -> Result<crate::workflow::Workflow, RegistryError> {
        Self::call_workflow_constructor_ffi(package_data, tenant_id, workflow_id)
    }

    /// Call the cloacina_create_workflow FFI function from a loaded package
    fn call_workflow_constructor_ffi(
        package_data: &[u8],
        tenant_id: &str,
        workflow_id: &str,
    ) -> Result<crate::workflow::Workflow, RegistryError> {
        use libloading::Library;

        // Write package data to a temporary file for loading
        let temp_dir = tempfile::TempDir::new().map_err(|e| {
            RegistryError::Loader(crate::registry::error::LoaderError::TempDirectory {
                error: e.to_string(),
            })
        })?;

        let library_extension = if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "windows") {
            "dll"
        } else {
            "so"
        };

        let temp_path = temp_dir
            .path()
            .join(format!("workflow_package.{}", library_extension));
        std::fs::write(&temp_path, package_data).map_err(|e| {
            RegistryError::Loader(crate::registry::error::LoaderError::FileSystem {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })
        })?;

        // Load the dynamic library
        let lib = unsafe {
            Library::new(&temp_path).map_err(|e| {
                RegistryError::Loader(crate::registry::error::LoaderError::LibraryLoad {
                    path: temp_path.to_string_lossy().to_string(),
                    error: e.to_string(),
                })
            })?
        };

        // Get the workflow constructor function
        let create_workflow = unsafe {
            lib.get::<unsafe extern "C" fn(
                *const std::os::raw::c_char,
                *const std::os::raw::c_char,
            ) -> *const crate::workflow::Workflow>(b"cloacina_create_workflow")
                .map_err(|e| {
                    RegistryError::Loader(crate::registry::error::LoaderError::SymbolNotFound {
                        symbol: "cloacina_create_workflow".to_string(),
                        error: e.to_string(),
                    })
                })?
        };

        // Convert Rust strings to C strings
        let tenant_id_cstring = std::ffi::CString::new(tenant_id).map_err(|e| {
            RegistryError::Loader(crate::registry::error::LoaderError::MetadataExtraction {
                reason: format!("Invalid tenant_id string: {}", e),
            })
        })?;

        let workflow_id_cstring = std::ffi::CString::new(workflow_id).map_err(|e| {
            RegistryError::Loader(crate::registry::error::LoaderError::MetadataExtraction {
                reason: format!("Invalid workflow_id string: {}", e),
            })
        })?;

        // Call the constructor function with tenant and workflow context
        let workflow_ptr =
            unsafe { create_workflow(tenant_id_cstring.as_ptr(), workflow_id_cstring.as_ptr()) };
        if workflow_ptr.is_null() {
            return Err(RegistryError::Loader(
                crate::registry::error::LoaderError::MetadataExtraction {
                    reason: "Workflow constructor returned null pointer".to_string(),
                },
            ));
        }

        // Convert the pointer back to a Rust object
        // Note: This takes ownership of the workflow, so we need to make sure
        // the FFI function allocated it properly with Box::into_raw
        let workflow = unsafe { Box::from_raw(workflow_ptr as *mut crate::workflow::Workflow) };

        Ok(*workflow)
    }

    /// Unregister tasks from the global task registry
    async fn unregister_package_tasks(
        &self,
        package_id: WorkflowPackageId,
        task_namespaces: &[TaskNamespace],
    ) -> Result<(), RegistryError> {
        // First unregister from the task registrar (which handles dynamic library cleanup)
        let package_id_str = package_id.to_string();
        self.task_registrar
            .unregister_package_tasks(&package_id_str)
            .map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Failed to unregister package tasks: {}", e),
            })?;

        // Then unregister from the global task registry
        let task_registry = global_task_registry();
        let mut registry =
            task_registry
                .write()
                .map_err(|e| RegistryError::RegistrationFailed {
                    message: format!("Failed to access task registry for unregistration: {}", e),
                })?;

        for namespace in task_namespaces {
            registry.remove(namespace);
            debug!("Unregistered task: {}", namespace);
        }

        Ok(())
    }

    /// Unregister a workflow from the global workflow registry
    async fn unregister_package_workflow(&self, workflow_name: &str) -> Result<(), RegistryError> {
        let workflow_registry = global_workflow_registry();
        let mut registry =
            workflow_registry
                .write()
                .map_err(|e| RegistryError::RegistrationFailed {
                    message: format!(
                        "Failed to access workflow registry for unregistration: {}",
                        e
                    ),
                })?;

        registry.remove(workflow_name);
        debug!("Unregistered workflow: {}", workflow_name);

        Ok(())
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

    /// Check if package data is a .cloacina archive
    fn is_cloacina_package(&self, package_data: &[u8]) -> bool {
        // Check for gzip magic number at the start
        package_data.len() >= 3
            && package_data[0] == 0x1f
            && package_data[1] == 0x8b
            && package_data[2] == 0x08
    }

    /// Extract library file data from a .cloacina archive
    async fn extract_library_from_cloacina(
        &self,
        package_data: &[u8],
    ) -> Result<Vec<u8>, RegistryError> {
        use flate2::read::GzDecoder;
        use std::io::Read;
        use tar::Archive;

        debug!(
            "Starting library extraction from .cloacina archive, data length: {}",
            package_data.len()
        );

        // Get platform-specific library extension
        let library_extension = if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "windows") {
            "dll"
        } else {
            "so"
        };

        debug!("Looking for library with extension: {}", library_extension);

        // Extract library file synchronously to avoid Send issues
        let library_data = tokio::task::spawn_blocking({
            let package_data = package_data.to_vec();
            let library_extension = library_extension.to_string();
            move || -> Result<Vec<u8>, RegistryError> {
                debug!("Starting spawn_blocking task for library extraction");

                // Create a cursor from the archive data
                let cursor = std::io::Cursor::new(package_data);
                debug!("Created cursor from package data");

                let gz_decoder = GzDecoder::new(cursor);
                debug!("Created GzDecoder");

                let mut archive = Archive::new(gz_decoder);
                debug!("Created Archive from GzDecoder");

                // Look for a library file in the archive
                debug!("Starting to iterate through archive entries");
                for entry_result in archive.entries().map_err(|e| {
                    debug!("Error reading archive entries: {}", e);
                    RegistryError::Loader(crate::registry::error::LoaderError::FileSystem {
                        path: "archive".to_string(),
                        error: format!("Failed to read archive entries: {}", e),
                    })
                })? {
                    let mut entry = entry_result.map_err(|e| {
                        RegistryError::Loader(crate::registry::error::LoaderError::FileSystem {
                            path: "archive".to_string(),
                            error: format!("Failed to read archive entry: {}", e),
                        })
                    })?;

                    let path = entry.path().map_err(|e| {
                        RegistryError::Loader(crate::registry::error::LoaderError::FileSystem {
                            path: "archive".to_string(),
                            error: format!("Failed to get entry path: {}", e),
                        })
                    })?;

                    // Check if this is a library file with the correct extension
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.ends_with(&format!(".{}", library_extension)) {
                            // Store path info before borrowing entry mutably
                            let path_string = path.to_string_lossy().to_string();

                            // Read the library file data
                            let mut file_data = Vec::new();
                            entry.read_to_end(&mut file_data).map_err(|e| {
                                RegistryError::Loader(
                                    crate::registry::error::LoaderError::FileSystem {
                                        path: path_string,
                                        error: format!(
                                            "Failed to read library file from archive: {}",
                                            e
                                        ),
                                    },
                                )
                            })?;

                            return Ok(file_data);
                        }
                    }
                }

                Err(RegistryError::Loader(
                    crate::registry::error::LoaderError::MetadataExtraction {
                        reason: format!(
                            "No library file with extension '{}' found in archive",
                            library_extension
                        ),
                    },
                ))
            }
        })
        .await
        .map_err(|e| {
            RegistryError::Loader(crate::registry::error::LoaderError::FileSystem {
                path: "spawn_blocking".to_string(),
                error: format!("Failed to spawn blocking task: {}", e),
            })
        })??;

        Ok(library_data)
    }
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
}
