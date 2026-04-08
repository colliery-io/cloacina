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

//! Package loading, unloading, and task/workflow registration.

use tracing::{debug, error, info, warn};

use super::{PackageState, RegistryReconciler};
use crate::registry::error::RegistryError;
use crate::registry::types::{WorkflowMetadata, WorkflowPackageId};
use crate::task::{global_task_registry, TaskNamespace};
use crate::workflow::global_workflow_registry;

impl RegistryReconciler {
    /// Load a package into the global registries.
    ///
    /// The `package_data` stored in the registry is a bzip2-compressed tar
    /// containing Rust source and a `package.toml`.  This method:
    /// 1. Writes the archive to a temp file.
    /// 2. Unpacks it via `fidius_core::package::unpack_package`.
    /// 3. Reads `package.toml` for metadata via `load_manifest::<CloacinaMetadata>`.
    /// 4. Compiles the source with `cargo build --lib`.
    /// 5. Reads the compiled cdylib bytes.
    /// 6. Passes them to `register_package_tasks` / `register_package_workflows`.
    pub(super) async fn load_package(
        &self,
        metadata: WorkflowMetadata,
    ) -> Result<(), RegistryError> {
        debug!(
            "Loading package: {} v{}",
            metadata.package_name, metadata.version
        );

        // Get the package archive data from the registry
        let loaded_workflow = self
            .registry
            .get_workflow(&metadata.package_name, &metadata.version)
            .await?
            .ok_or_else(|| RegistryError::PackageNotFound {
                package_name: metadata.package_name.clone(),
                version: metadata.version.clone(),
            })?;

        // --- Step 1: write archive to a temp file ---
        let work_dir = tempfile::TempDir::new().map_err(|e| RegistryError::RegistrationFailed {
            message: format!("Failed to create temp dir: {}", e),
        })?;

        let archive_path = work_dir.path().join(format!(
            "{}-{}.cloacina",
            metadata.package_name, metadata.version
        ));

        tokio::fs::write(&archive_path, &loaded_workflow.package_data)
            .await
            .map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Failed to write archive to temp file: {}", e),
            })?;

        // --- Step 2: unpack archive ---
        let extract_dir = work_dir.path().join("source");
        tokio::fs::create_dir_all(&extract_dir).await.map_err(|e| {
            RegistryError::RegistrationFailed {
                message: format!("Failed to create extract dir: {}", e),
            }
        })?;

        let archive_path_clone = archive_path.clone();
        let extract_dir_clone = extract_dir.clone();
        let source_dir = tokio::task::spawn_blocking(move || {
            fidius_core::package::unpack_package(&archive_path_clone, &extract_dir_clone).map_err(
                |e| RegistryError::RegistrationFailed {
                    message: format!("Failed to unpack source archive: {}", e),
                },
            )
        })
        .await
        .map_err(|e| RegistryError::RegistrationFailed {
            message: format!("spawn_blocking failed during unpack: {}", e),
        })??;

        // --- Step 3: load manifest and validate ---
        let source_dir_clone = source_dir.clone();
        let cloacina_manifest = tokio::task::spawn_blocking(move || {
            fidius_core::package::load_manifest::<cloacina_workflow_plugin::CloacinaMetadata>(
                &source_dir_clone,
            )
            .map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Failed to load package.toml: {}", e),
            })
        })
        .await
        .map_err(|e| RegistryError::RegistrationFailed {
            message: format!("spawn_blocking failed during manifest load: {}", e),
        })??;

        debug!(
            "Package manifest loaded: {} v{} language={}",
            cloacina_manifest.package.name,
            cloacina_manifest.package.version,
            cloacina_manifest.metadata.language
        );

        // --- Step 4, 5, 6: language-specific loading ---
        let (task_namespaces, workflow_name, trigger_names) = if cloacina_manifest.metadata.language
            == "rust"
        {
            // Rust path: compile cdylib, extract metadata via FFI, register
            info!(
                "Step 4: Compiling Rust source for package: {}",
                metadata.package_name
            );
            let lib_path = Self::compile_source_package(&source_dir).await?;
            info!(
                "Step 4: Compilation complete for {}: {}",
                metadata.package_name,
                lib_path.display()
            );

            let library_data = tokio::fs::read(&lib_path).await.map_err(|e| {
                RegistryError::RegistrationFailed {
                    message: format!(
                        "Failed to read compiled library {}: {}",
                        lib_path.display(),
                        e
                    ),
                }
            })?;
            info!(
                "Step 5: Library read ({} bytes) for {}",
                library_data.len(),
                metadata.package_name
            );

            info!("Step 5a: Registering tasks for {}", metadata.package_name);
            let task_namespaces = self
                .register_package_tasks(&metadata, &library_data)
                .await?;
            info!(
                "Step 5b: Registering workflows for {}",
                metadata.package_name
            );
            let workflow_name = self
                .register_package_workflows(&metadata, &library_data)
                .await?;
            let trigger_names =
                self.register_package_triggers(&metadata, &cloacina_manifest.metadata)?;

            (task_namespaces, workflow_name, trigger_names)
        } else if cloacina_manifest.metadata.language == "python" {
            // Python path: extract package, import module, register via PyO3
            debug!("Loading Python package: {}", metadata.package_name);

            // Ensure the Python interpreter is initialized (idempotent — safe to call multiple times)
            pyo3::prepare_freethreaded_python();

            let extracted = tokio::task::spawn_blocking({
                let archive_data = loaded_workflow.package_data.clone();
                let staging = work_dir.path().join("python-staging");
                move || {
                    std::fs::create_dir_all(&staging).map_err(|e| {
                        RegistryError::RegistrationFailed {
                            message: format!("Failed to create Python staging dir: {}", e),
                        }
                    })?;
                    crate::registry::loader::python_loader::extract_python_package(
                        &archive_data,
                        &staging,
                    )
                    .map_err(|e| RegistryError::RegistrationFailed {
                        message: format!("Failed to extract Python package: {}", e),
                    })
                }
            })
            .await
            .map_err(|e| RegistryError::RegistrationFailed {
                message: format!("spawn_blocking failed during Python extraction: {}", e),
            })??;

            let tenant_id = self.config.default_tenant_id.clone();
            let task_namespaces = tokio::task::spawn_blocking({
                let workflow_dir = extracted.workflow_dir.clone();
                let vendor_dir = extracted.vendor_dir.clone();
                let entry_module = extracted.entry_module.clone();
                let package_name = extracted.package_name.clone();
                let workflow_name = extracted.workflow_name.clone();
                let tenant_id = tenant_id.clone();
                move || {
                    crate::python::loader::import_and_register_python_workflow_named(
                        &workflow_dir,
                        &vendor_dir,
                        &entry_module,
                        &package_name,
                        &workflow_name,
                        &tenant_id,
                    )
                    .map_err(|e| RegistryError::RegistrationFailed {
                        message: format!("Python workflow import failed: {}", e),
                    })
                }
            })
            .await
            .map_err(|e| RegistryError::RegistrationFailed {
                message: format!("spawn_blocking failed during Python import: {}", e),
            })??;

            let workflow_name = Some(extracted.workflow_name.clone());

            // Python triggers are registered during import_and_register_python_workflow.
            // Track them from the manifest so they can be unloaded later.
            let trigger_names =
                self.register_package_triggers(&metadata, &cloacina_manifest.metadata)?;

            info!(
                "Python package loaded: {} v{} — {} tasks, workflow '{}'",
                metadata.package_name,
                metadata.version,
                task_namespaces.len(),
                extracted.workflow_name,
            );

            (task_namespaces, workflow_name, trigger_names)
        } else {
            return Err(RegistryError::RegistrationFailed {
                    message: format!(
                        "Unsupported package language '{}' for package {} — only 'rust' and 'python' are supported",
                        cloacina_manifest.metadata.language, metadata.package_name
                    ),
                });
        };

        // --- Step 7: Computation graph routing ---
        let graph_name = if cloacina_manifest.metadata.has_computation_graph() {
            if cloacina_manifest.metadata.language == "rust" {
                // Re-read the library to call get_graph_metadata (method index 2)
                let lib_path = Self::compile_source_package(&source_dir).await?;
                let library_data = tokio::fs::read(&lib_path).await.map_err(|e| {
                    RegistryError::RegistrationFailed {
                        message: format!("Failed to read library for graph metadata: {}", e),
                    }
                })?;

                match self
                    .package_loader
                    .extract_graph_metadata(&library_data)
                    .await
                {
                    Ok(Some(graph_meta)) => {
                        info!(
                            "Computation graph detected: {} (accumulators: {:?})",
                            graph_meta.graph_name,
                            graph_meta
                                .accumulators
                                .iter()
                                .map(|a| &a.name)
                                .collect::<Vec<_>>()
                        );

                        // Merge manifest accumulator configs into FFI defaults
                        let mut graph_meta = graph_meta;
                        for manifest_acc in &cloacina_manifest.metadata.accumulators {
                            if let Some(ffi_acc) = graph_meta
                                .accumulators
                                .iter_mut()
                                .find(|a| a.name == manifest_acc.name)
                            {
                                ffi_acc.accumulator_type = manifest_acc.accumulator_type.clone();
                                ffi_acc.config = manifest_acc.config.clone();
                            }
                        }

                        let scheduler_guard = self.reactive_scheduler.read().await;
                        if let Some(ref scheduler) = *scheduler_guard {
                            let decl =
                                crate::computation_graph::packaging_bridge::build_declaration_from_ffi(
                                    &graph_meta,
                                    library_data.clone(),
                                );
                            if let Err(e) = scheduler.load_graph(decl).await {
                                warn!(
                                    "Failed to load computation graph '{}': {}",
                                    graph_meta.graph_name, e
                                );
                            } else {
                                info!(
                                    "Computation graph '{}' loaded into ReactiveScheduler",
                                    graph_meta.graph_name
                                );
                            }
                        } else {
                            warn!(
                                "Computation graph '{}' detected but no ReactiveScheduler configured",
                                graph_meta.graph_name
                            );
                        }

                        Some(graph_meta.graph_name)
                    }
                    Ok(None) => {
                        debug!(
                            "Package claims computation_graph type but plugin doesn't support get_graph_metadata"
                        );
                        None
                    }
                    Err(e) => {
                        warn!("Failed to extract graph metadata: {}", e);
                        None
                    }
                }
            } else if cloacina_manifest.metadata.language == "python" {
                // Python computation graph: import module, decorators register executor
                if let (Some(ref graph_name), Some(ref entry_module)) = (
                    &cloacina_manifest.metadata.graph_name,
                    &cloacina_manifest.metadata.entry_module,
                ) {
                    let extracted = tokio::task::spawn_blocking({
                        let archive_data = loaded_workflow.package_data.clone();
                        let staging = work_dir.path().join("python-cg-staging");
                        move || {
                            std::fs::create_dir_all(&staging).map_err(|e| {
                                RegistryError::RegistrationFailed {
                                    message: format!("Failed to create staging dir: {}", e),
                                }
                            })?;
                            crate::registry::loader::python_loader::extract_python_package(
                                &archive_data,
                                &staging,
                            )
                            .map_err(|e| {
                                RegistryError::RegistrationFailed {
                                    message: format!("Failed to extract Python CG package: {}", e),
                                }
                            })
                        }
                    })
                    .await
                    .map_err(|e| RegistryError::RegistrationFailed {
                        message: format!(
                            "spawn_blocking failed during Python CG extraction: {}",
                            e
                        ),
                    })??;

                    let gn = graph_name.clone();
                    let em = entry_module.clone();
                    let wd = extracted.workflow_dir.clone();
                    let vd = extracted.vendor_dir.clone();

                    tokio::task::spawn_blocking(move || {
                        pyo3::prepare_freethreaded_python();
                        crate::python::loader::import_python_computation_graph(&wd, &vd, &em, &gn)
                            .map_err(|e| RegistryError::RegistrationFailed {
                                message: format!("Python CG import failed: {}", e),
                            })
                    })
                    .await
                    .map_err(|e| RegistryError::RegistrationFailed {
                        message: format!("spawn_blocking failed during Python CG import: {}", e),
                    })??;

                    info!(
                        "Python computation graph '{}' imported from '{}'",
                        graph_name, entry_module
                    );
                    Some(graph_name.clone())
                } else {
                    warn!("Python computation graph package missing graph_name or entry_module");
                    None
                }
            } else {
                debug!("Unsupported language for computation graph");
                None
            }
        } else {
            None
        };

        // Track the loaded package state
        let package_state = PackageState {
            metadata: metadata.clone(),
            task_namespaces,
            workflow_name,
            trigger_names,
            graph_name,
        };

        let mut loaded_packages = self.loaded_packages.write().await;
        loaded_packages.insert(metadata.id, package_state);

        Ok(())
    }

    /// Unload a package from the global registries
    pub(super) async fn unload_package(
        &self,
        package_id: WorkflowPackageId,
    ) -> Result<(), RegistryError> {
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

        // Unregister triggers from global trigger registry
        if !package_state.trigger_names.is_empty() {
            self.unregister_package_triggers(&package_state.trigger_names);
        }

        // Unload computation graph from reactive scheduler
        if let Some(graph_name) = &package_state.graph_name {
            let scheduler_guard = self.reactive_scheduler.read().await;
            if let Some(ref scheduler) = *scheduler_guard {
                if let Err(e) = scheduler.unload_graph(graph_name).await {
                    warn!("Failed to unload computation graph '{}': {}", graph_name, e);
                }
            }
        }

        info!(
            "Unloaded package: {} v{}",
            package_state.metadata.package_name, package_state.metadata.version
        );

        Ok(())
    }

    /// Register tasks from a package into the global task registry
    pub(super) async fn register_package_tasks(
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
    pub(super) async fn register_package_workflows(
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

        // Check if package has tasks (which means it has a workflow since it was compiled with the macro)
        if !package_metadata.tasks.is_empty() {
            debug!(
                "Package {} has {} tasks - workflow exists since it compiled with packaged_workflow macro",
                metadata.package_name,
                package_metadata.tasks.len()
            );

            // Extract the workflow name from the package metadata
            // The workflow name comes from the #[packaged_workflow(name = "...")] macro
            // Since package_loader::PackageMetadata doesn't have workflow_name field directly,
            // we need to extract it from the task metadata namespaced templates
            let workflow_name = {
                // Extract workflow name from namespaced_id_template
                if let Some(first_task) = package_metadata.tasks.first() {
                    let template = &first_task.namespaced_id_template;
                    debug!("Parsing workflow_name from template: '{}'", template);

                    // Split by "::" and extract the workflow_id part (3rd component)
                    let parts: Vec<&str> = template.split("::").collect();
                    if parts.len() >= 3 {
                        let workflow_part = parts[2];
                        // Handle both {workflow} placeholder and actual workflow_id
                        if workflow_part == "{workflow}" {
                            // This is a template, need to look up actual workflow_id from registered tasks
                            let task_registry = crate::task::global_task_registry();
                            let mut found_id = None;
                            let registry = task_registry.read();
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
                }
            };

            debug!(
                "Using workflow_name '{}' for workflow registration",
                workflow_name
            );

            // Use the actual package name from metadata — the namespaced_id_template
            // contains unresolved placeholders like {pkg} that don't match registered tasks
            let task_package_name = metadata.package_name.clone();

            debug!(
                "Using task_package_name '{}' for task lookup",
                task_package_name
            );

            // Create the workflow directly using host registries (avoid FFI isolation issues)
            let _workflow = self.create_workflow_from_host_registry(
                &task_package_name, // Use the correct package name from task metadata
                &workflow_name,
                &self.config.default_tenant_id,
            )?;

            // Register workflow constructor with global workflow registry
            let workflow_registry = global_workflow_registry();
            let mut registry = workflow_registry.write();

            // Create a constructor that recreates the workflow from host registry each time
            let workflow_name_for_closure = workflow_name.clone();
            let package_name_for_closure = task_package_name.clone(); // Use the correct package name
            let workflow_name_for_closure_static = workflow_name.clone();
            let tenant_id_for_closure = self.config.default_tenant_id.clone();

            registry.insert(
                workflow_name.clone(),
                Box::new(move || {
                    debug!(
                        "Creating workflow instance for {} using host registry",
                        workflow_name_for_closure
                    );

                    // Recreate the workflow from the host task registry each time
                    match Self::create_workflow_from_host_registry_static(
                        &package_name_for_closure,
                        &workflow_name_for_closure_static,
                        &tenant_id_for_closure,
                    ) {
                        Ok(workflow) => workflow,
                        Err(e) => {
                            error!("Failed to create workflow from host registry: {}", e);
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

    /// Create a workflow using the host's global task registry (avoiding FFI isolation)
    pub(super) fn create_workflow_from_host_registry(
        &self,
        package_name: &str,
        workflow_name: &str,
        tenant_id: &str,
    ) -> Result<crate::workflow::Workflow, RegistryError> {
        // Create workflow and add registered tasks from host registry
        let mut workflow = crate::workflow::Workflow::new(workflow_name);
        workflow.set_tenant(tenant_id);
        workflow.set_package(package_name);

        // Add tasks from the host's global task registry
        let task_registry = crate::task::global_task_registry();
        let registry = task_registry.read();

        let mut found_tasks = 0;
        for (namespace, task_constructor) in registry.iter() {
            // Only include tasks from this package, workflow, and tenant
            if namespace.package_name == package_name
                && namespace.workflow_id == workflow_name
                && namespace.tenant_id == tenant_id
            {
                let task = task_constructor();
                workflow
                    .add_task(task)
                    .map_err(|e| RegistryError::RegistrationFailed {
                        message: format!(
                            "Failed to add task {} to workflow: {:?}",
                            namespace.task_id, e
                        ),
                    })?;
                found_tasks += 1;
            }
        }

        debug!(
            "Created workflow '{}' with {} tasks from host registry",
            workflow_name, found_tasks
        );

        // Validate and finalize the workflow
        workflow
            .validate()
            .map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Workflow validation failed: {:?}", e),
            })?;

        Ok(workflow.finalize())
    }

    /// Static version of create_workflow_from_host_registry for use in closures
    pub(super) fn create_workflow_from_host_registry_static(
        package_name: &str,
        workflow_name: &str,
        tenant_id: &str,
    ) -> Result<crate::workflow::Workflow, RegistryError> {
        // Create workflow and add registered tasks from host registry
        let mut workflow = crate::workflow::Workflow::new(workflow_name);
        workflow.set_tenant(tenant_id);
        workflow.set_package(package_name);

        // Add tasks from the host's global task registry
        let task_registry = crate::task::global_task_registry();
        let registry = task_registry.read();

        let mut found_tasks = 0;
        for (namespace, task_constructor) in registry.iter() {
            // Only include tasks from this package, workflow, and tenant
            if namespace.package_name == package_name
                && namespace.workflow_id == workflow_name
                && namespace.tenant_id == tenant_id
            {
                let task = task_constructor();
                workflow
                    .add_task(task)
                    .map_err(|e| RegistryError::RegistrationFailed {
                        message: format!(
                            "Failed to add task {} to workflow: {:?}",
                            namespace.task_id, e
                        ),
                    })?;
                found_tasks += 1;
            }
        }

        debug!(
            "Created workflow '{}' with {} tasks from host registry (static)",
            workflow_name, found_tasks
        );

        // Validate and finalize the workflow
        workflow
            .validate()
            .map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Workflow validation failed: {:?}", e),
            })?;

        Ok(workflow.finalize())
    }

    /// Unregister tasks from the global task registry
    pub(super) async fn unregister_package_tasks(
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
        let mut registry = task_registry.write();

        for namespace in task_namespaces {
            registry.remove(namespace);
            debug!("Unregistered task: {}", namespace);
        }

        Ok(())
    }

    /// Unregister a workflow from the global workflow registry
    pub(super) async fn unregister_package_workflow(
        &self,
        workflow_name: &str,
    ) -> Result<(), RegistryError> {
        let workflow_registry = global_workflow_registry();
        let mut registry = workflow_registry.write();

        registry.remove(workflow_name);
        debug!("Unregistered workflow: {}", workflow_name);

        Ok(())
    }

    /// Verify and track triggers declared in a package's `CloacinaMetadata`.
    ///
    /// The package's trigger implementations are registered by the package itself
    /// via `ctor` when the cdylib is loaded.  This method checks that each
    /// declared trigger actually appeared in the global trigger registry and
    /// tracks its name so it can be removed when the package is unloaded.
    pub(super) fn register_package_triggers(
        &self,
        metadata: &WorkflowMetadata,
        cloacina_metadata: &cloacina_workflow_plugin::CloacinaMetadata,
    ) -> Result<Vec<String>, RegistryError> {
        if cloacina_metadata.triggers.is_empty() {
            return Ok(vec![]);
        }

        let mut tracked_trigger_names = Vec::new();

        for trigger_def in &cloacina_metadata.triggers {
            if crate::trigger::registry::is_trigger_registered(&trigger_def.name) {
                info!(
                    "Trigger '{}' (workflow: {}, interval: {}) registered from package {} v{}",
                    trigger_def.name,
                    trigger_def.workflow,
                    trigger_def.poll_interval,
                    metadata.package_name,
                    metadata.version
                );
                tracked_trigger_names.push(trigger_def.name.clone());
            } else {
                warn!(
                    "Trigger '{}' declared in package.toml for package {} v{} but not found in \
                     registry — the package must provide a Trigger impl (via #[trigger] macro or \
                     manual registration)",
                    trigger_def.name, metadata.package_name, metadata.version
                );
            }
        }

        if !tracked_trigger_names.is_empty() {
            info!(
                "Tracking {} triggers for package {} v{}",
                tracked_trigger_names.len(),
                metadata.package_name,
                metadata.version
            );
        }

        Ok(tracked_trigger_names)
    }

    /// Unregister triggers from the global trigger registry.
    pub(super) fn unregister_package_triggers(&self, trigger_names: &[String]) {
        for name in trigger_names {
            if crate::trigger::deregister_trigger(name) {
                debug!("Deregistered trigger: {}", name);
            } else {
                warn!("Trigger '{}' was not found in registry during unload", name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::reconciler::ReconcilerConfig;
    use crate::registry::workflow_registry::filesystem::FilesystemWorkflowRegistry;
    use serial_test::serial;
    use std::sync::Arc;
    use uuid::Uuid;

    /// Create a minimal RegistryReconciler for testing.
    fn make_test_reconciler() -> RegistryReconciler {
        let registry = Arc::new(FilesystemWorkflowRegistry::new(vec![]));
        let config = ReconcilerConfig::default();
        let (_tx, rx) = tokio::sync::watch::channel(false);
        RegistryReconciler::new(registry, config, rx).expect("Failed to create test reconciler")
    }

    fn make_test_metadata() -> WorkflowMetadata {
        WorkflowMetadata {
            id: Uuid::new_v4(),
            registry_id: Uuid::new_v4(),
            package_name: "test-pkg".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test package".to_string()),
            author: None,
            tasks: vec![],
            schedules: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn make_cloacina_metadata_with_triggers(
        triggers: Vec<cloacina_workflow_plugin::TriggerDefinition>,
    ) -> cloacina_workflow_plugin::CloacinaMetadata {
        cloacina_workflow_plugin::CloacinaMetadata {
            package_type: vec!["workflow".to_string()],
            workflow_name: Some("test-workflow".to_string()),
            graph_name: None,
            language: "python".to_string(),
            description: Some("Test".to_string()),
            author: None,
            requires_python: None,
            entry_module: Some("test.tasks".to_string()),
            triggers,
            reaction_mode: None,
            input_strategy: None,
            accumulators: Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // register_package_triggers tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn register_triggers_with_no_triggers_returns_empty() {
        let reconciler = make_test_reconciler();
        let metadata = make_test_metadata();
        let cloacina_meta = make_cloacina_metadata_with_triggers(vec![]);

        let result = reconciler
            .register_package_triggers(&metadata, &cloacina_meta)
            .unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn register_triggers_tracks_registered_triggers() {
        let reconciler = make_test_reconciler();
        let metadata = make_test_metadata();

        // Pre-register a trigger in the global registry
        let trigger_name = format!("test-trigger-{}", Uuid::new_v4());
        crate::trigger::registry::register_trigger_constructor(trigger_name.clone(), || {
            // Minimal trigger for testing — just need it in the registry
            Arc::new(DummyTrigger {
                name: "dummy".to_string(),
            })
        });

        let cloacina_meta = make_cloacina_metadata_with_triggers(vec![
            cloacina_workflow_plugin::TriggerDefinition {
                name: trigger_name.clone(),
                workflow: "test-workflow".to_string(),
                poll_interval: "5s".to_string(),
                cron_expression: None,
                allow_concurrent: false,
            },
        ]);

        let result = reconciler
            .register_package_triggers(&metadata, &cloacina_meta)
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], trigger_name);

        // Cleanup
        crate::trigger::deregister_trigger(&trigger_name);
    }

    #[tokio::test]
    #[serial]
    async fn register_triggers_skips_unregistered_triggers() {
        let reconciler = make_test_reconciler();
        let metadata = make_test_metadata();

        let trigger_name = format!("nonexistent-trigger-{}", Uuid::new_v4());
        let cloacina_meta = make_cloacina_metadata_with_triggers(vec![
            cloacina_workflow_plugin::TriggerDefinition {
                name: trigger_name.clone(),
                workflow: "test-workflow".to_string(),
                poll_interval: "10s".to_string(),
                cron_expression: None,
                allow_concurrent: false,
            },
        ]);

        let result = reconciler
            .register_package_triggers(&metadata, &cloacina_meta)
            .unwrap();
        // Should be empty because the trigger is not actually registered in the global registry
        assert!(result.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn register_triggers_mixed_registered_and_missing() {
        let reconciler = make_test_reconciler();
        let metadata = make_test_metadata();

        // Register one trigger, leave the other unregistered
        let registered_name = format!("registered-trigger-{}", Uuid::new_v4());
        let missing_name = format!("missing-trigger-{}", Uuid::new_v4());

        crate::trigger::registry::register_trigger_constructor(registered_name.clone(), || {
            Arc::new(DummyTrigger {
                name: "dummy".to_string(),
            })
        });

        let cloacina_meta = make_cloacina_metadata_with_triggers(vec![
            cloacina_workflow_plugin::TriggerDefinition {
                name: registered_name.clone(),
                workflow: "wf1".to_string(),
                poll_interval: "5s".to_string(),
                cron_expression: None,
                allow_concurrent: false,
            },
            cloacina_workflow_plugin::TriggerDefinition {
                name: missing_name.clone(),
                workflow: "wf2".to_string(),
                poll_interval: "10s".to_string(),
                cron_expression: None,
                allow_concurrent: false,
            },
        ]);

        let result = reconciler
            .register_package_triggers(&metadata, &cloacina_meta)
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], registered_name);

        // Cleanup
        crate::trigger::deregister_trigger(&registered_name);
    }

    // -----------------------------------------------------------------------
    // unregister_package_triggers tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn unregister_triggers_removes_from_global_registry() {
        let reconciler = make_test_reconciler();

        let trigger_name = format!("unregister-test-{}", Uuid::new_v4());
        crate::trigger::registry::register_trigger_constructor(trigger_name.clone(), || {
            Arc::new(DummyTrigger {
                name: "dummy".to_string(),
            })
        });

        assert!(crate::trigger::registry::is_trigger_registered(
            &trigger_name
        ));

        reconciler.unregister_package_triggers(std::slice::from_ref(&trigger_name));

        assert!(!crate::trigger::registry::is_trigger_registered(
            &trigger_name
        ));
    }

    #[tokio::test]
    #[serial]
    async fn unregister_triggers_handles_already_removed() {
        let reconciler = make_test_reconciler();

        let trigger_name = format!("already-gone-{}", Uuid::new_v4());
        // Don't register it — just try to unregister
        // Should not panic, just log a warning
        reconciler.unregister_package_triggers(&[trigger_name]);
    }

    #[tokio::test]
    #[serial]
    async fn unregister_triggers_empty_list_is_noop() {
        let reconciler = make_test_reconciler();
        reconciler.unregister_package_triggers(&[]);
    }

    // -----------------------------------------------------------------------
    // unregister_package_workflow tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn unregister_workflow_removes_from_global_registry() {
        let reconciler = make_test_reconciler();

        let workflow_name = format!("test-wf-{}", Uuid::new_v4());

        // Register a workflow constructor
        {
            let registry = global_workflow_registry();
            let mut reg = registry.write();
            let wf_name = workflow_name.clone();
            reg.insert(
                workflow_name.clone(),
                Box::new(move || crate::workflow::Workflow::new(&wf_name)),
            );
        }

        // Verify it's there
        {
            let registry = global_workflow_registry();
            let reg = registry.read();
            assert!(reg.contains_key(&workflow_name));
        }

        // Unregister
        reconciler
            .unregister_package_workflow(&workflow_name)
            .await
            .unwrap();

        // Verify it's gone
        {
            let registry = global_workflow_registry();
            let reg = registry.read();
            assert!(!reg.contains_key(&workflow_name));
        }
    }

    #[tokio::test]
    #[serial]
    async fn unregister_workflow_nonexistent_is_ok() {
        let reconciler = make_test_reconciler();
        // Should succeed even if workflow doesn't exist
        let result = reconciler
            .unregister_package_workflow("does-not-exist")
            .await;
        assert!(result.is_ok());
    }

    // -----------------------------------------------------------------------
    // Dummy trigger for testing
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone)]
    struct DummyTrigger {
        name: String,
    }

    #[async_trait::async_trait]
    impl crate::trigger::Trigger for DummyTrigger {
        fn name(&self) -> &str {
            &self.name
        }

        fn poll_interval(&self) -> std::time::Duration {
            std::time::Duration::from_secs(60)
        }

        fn allow_concurrent(&self) -> bool {
            false
        }

        async fn poll(
            &self,
        ) -> Result<crate::trigger::TriggerResult, crate::trigger::TriggerError> {
            Ok(crate::trigger::TriggerResult::Skip)
        }
    }
}
