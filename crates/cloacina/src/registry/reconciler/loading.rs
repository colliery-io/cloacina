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

//! Package loading, unloading, and task/workflow registration.

use tracing::{debug, error, info};

use super::{PackageState, RegistryReconciler};
use crate::registry::error::RegistryError;
use crate::registry::types::{WorkflowMetadata, WorkflowPackageId};
use crate::task::{global_task_registry, TaskNamespace};
use crate::workflow::global_workflow_registry;

impl RegistryReconciler {
    /// Load a package into the global registries
    pub(super) async fn load_package(
        &self,
        metadata: WorkflowMetadata,
    ) -> Result<(), RegistryError> {
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

            // Get the package name from the first task's metadata (this is the correct package name for task lookup)
            let task_package_name = if let Some(first_task) = package_metadata.tasks.first() {
                // Extract package name from namespaced template: {tenant}::package_name::workflow_id::task_id
                let template = &first_task.namespaced_id_template;
                let parts: Vec<&str> = template.split("::").collect();
                if parts.len() >= 2 {
                    parts[1].to_string() // Get the package_name part
                } else {
                    metadata.package_name.clone() // Fallback to metadata package name
                }
            } else {
                metadata.package_name.clone() // Fallback to metadata package name
            };

            debug!(
                "Using task_package_name '{}' for task lookup (extracted from task metadata)",
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
}
