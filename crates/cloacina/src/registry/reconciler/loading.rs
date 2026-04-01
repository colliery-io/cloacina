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

        // --- Step 4 & 5: compile (Rust) and read cdylib ---
        let library_data = if cloacina_manifest.metadata.language == "rust" {
            debug!(
                "Compiling Rust source for package: {}",
                metadata.package_name
            );
            let lib_path = Self::compile_source_package(&source_dir).await?;

            tokio::fs::read(&lib_path)
                .await
                .map_err(|e| RegistryError::RegistrationFailed {
                    message: format!(
                        "Failed to read compiled library {}: {}",
                        lib_path.display(),
                        e
                    ),
                })?
        } else {
            return Err(RegistryError::RegistrationFailed {
                message: format!(
                    "Unsupported package language '{}' for package {} — only 'rust' is supported by this loader",
                    cloacina_manifest.metadata.language, metadata.package_name
                ),
            });
        };

        // --- Step 6: register tasks and workflows ---
        let task_namespaces = self
            .register_package_tasks(&metadata, &library_data)
            .await?;
        let workflow_name = self
            .register_package_workflows(&metadata, &library_data)
            .await?;

        // Register triggers from the manifest metadata
        let trigger_names =
            self.register_package_triggers(&metadata, &cloacina_manifest.metadata)?;

        // Track the loaded package state
        let package_state = PackageState {
            metadata: metadata.clone(),
            task_namespaces,
            workflow_name,
            trigger_names,
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
