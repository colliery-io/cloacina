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

//! Complete implementation of the workflow registry.
//!
//! This module provides the `WorkflowRegistryImpl` that combines all registry
//! components - storage, loading, validation, and task registration - into a
//! cohesive system for managing packaged workflows.

mod database;
mod package;

use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

use crate::database::Database;
use crate::registry::error::RegistryError;
use crate::registry::loader::{PackageLoader, PackageValidator, TaskRegistrar};
use crate::registry::traits::{RegistryStorage, WorkflowRegistry};
use crate::registry::types::{LoadedWorkflow, WorkflowMetadata, WorkflowPackageId};
use crate::task::TaskNamespace;

/// Complete implementation of the workflow registry.
///
/// This registry implementation combines storage backends, package loading,
/// validation, and task registration to provide a full-featured system for
/// managing packaged workflows with proper lifecycle management.
pub struct WorkflowRegistryImpl<S: RegistryStorage> {
    /// Storage backend for binary data
    pub(super) storage: S,
    /// Database for metadata storage
    pub(super) database: Database,
    /// Package loader for metadata extraction
    loader: PackageLoader,
    /// Task registrar for global registry integration
    registrar: TaskRegistrar,
    /// Package validator for safety checks
    validator: PackageValidator,
    /// Map of package IDs to registered task namespaces for cleanup tracking
    pub(super) loaded_packages: HashMap<Uuid, Vec<TaskNamespace>>,
}

impl<S: RegistryStorage> WorkflowRegistryImpl<S> {
    /// Create a new workflow registry implementation.
    ///
    /// # Arguments
    ///
    /// * `storage` - Storage backend for binary workflow data
    /// * `database` - Database for metadata storage
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowRegistryImpl)` - Successfully created registry
    /// * `Err(RegistryError)` - If creation fails
    pub fn new(storage: S, database: Database) -> Result<Self, RegistryError> {
        let loader = PackageLoader::new().map_err(RegistryError::Loader)?;
        let registrar = TaskRegistrar::new().map_err(RegistryError::Loader)?;
        let validator = PackageValidator::new().map_err(RegistryError::Loader)?;

        Ok(Self {
            storage,
            database,
            loader,
            registrar,
            validator,
            loaded_packages: HashMap::new(),
        })
    }

    /// Create a registry with strict validation enabled.
    pub fn with_strict_validation(storage: S, database: Database) -> Result<Self, RegistryError> {
        let loader = PackageLoader::new().map_err(RegistryError::Loader)?;
        let registrar = TaskRegistrar::new().map_err(RegistryError::Loader)?;
        let validator = PackageValidator::strict().map_err(RegistryError::Loader)?;

        Ok(Self {
            storage,
            database,
            loader,
            registrar,
            validator,
            loaded_packages: HashMap::new(),
        })
    }

    /// Get the number of currently loaded packages.
    pub fn loaded_package_count(&self) -> usize {
        self.loaded_packages.len()
    }

    /// Get the total number of registered tasks across all packages.
    pub fn total_registered_tasks(&self) -> usize {
        self.loaded_packages.values().map(|tasks| tasks.len()).sum()
    }

    // ========================================================================
    // Public convenience methods for tests and direct usage
    // ========================================================================

    /// Register a workflow package (alias for register_workflow via the trait).
    ///
    /// This is a convenience method that provides the same functionality as
    /// the `register_workflow` trait method.
    pub async fn register_workflow_package(
        &mut self,
        package_data: Vec<u8>,
    ) -> Result<Uuid, RegistryError> {
        // Use the trait implementation directly
        WorkflowRegistry::register_workflow(self, package_data).await
    }

    /// Get a workflow package by its UUID.
    ///
    /// Returns the package metadata and binary data if found.
    pub async fn get_workflow_package_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(WorkflowMetadata, Vec<u8>)>, RegistryError> {
        // Get metadata from database
        let (registry_id, metadata) = match self.get_package_metadata_by_id(package_id).await? {
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

    /// Get a workflow package by name and version.
    ///
    /// Returns the package metadata and binary data if found.
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

    /// Check if a package exists by ID.
    pub async fn exists_by_id(&self, package_id: Uuid) -> Result<bool, RegistryError> {
        Ok(self.get_package_metadata_by_id(package_id).await?.is_some())
    }

    /// Check if a package exists by name and version.
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

    /// List all packages in the registry.
    ///
    /// Returns metadata for all registered packages.
    pub async fn list_packages(&self) -> Result<Vec<WorkflowMetadata>, RegistryError> {
        self.list_all_packages().await
    }

    /// Unregister a workflow package by ID.
    pub async fn unregister_workflow_package_by_id(
        &mut self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        // Get package metadata to find the registry_id for storage cleanup
        let (registry_id, _metadata) = match self.get_package_metadata_by_id(package_id).await? {
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

    /// Unregister a workflow package by name and version.
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
}

#[async_trait]
impl<S: RegistryStorage + Send + Sync> WorkflowRegistry for WorkflowRegistryImpl<S> {
    async fn register_workflow(
        &mut self,
        package_data: Vec<u8>,
    ) -> Result<WorkflowPackageId, RegistryError> {
        // 1. Check if this is a .cloacina package
        let is_cloacina = Self::is_cloacina_package(&package_data);

        // 2. Extract .so file for validation if needed
        let so_data = if is_cloacina {
            Self::extract_so_from_cloacina(&package_data).await?
        } else {
            package_data.clone()
        };

        // 3. Validate the extracted .so file
        let validation_result = self
            .validator
            .validate_package(&so_data, None)
            .await
            .map_err(RegistryError::Loader)?;

        if !validation_result.is_valid {
            return Err(RegistryError::ValidationError {
                reason: validation_result.errors.join("; "),
            });
        }

        // 4. Extract metadata from the package
        let package_metadata = if is_cloacina {
            // For .cloacina packages, extract metadata directly from the archive
            self.loader
                .extract_metadata(&package_data)
                .await
                .map_err(RegistryError::Loader)?
        } else {
            // For raw .so files, we need to create a simple PackageLoader that handles raw files
            // For now, return an error as we haven't implemented raw .so support in the new PackageLoader
            return Err(RegistryError::ValidationError {
                reason:
                    "Raw .so file registration not yet supported. Please use .cloacina packages."
                        .to_string(),
            });
        };

        // 4. Check if package already exists
        if self
            .get_package_metadata(&package_metadata.package_name, &package_metadata.version)
            .await?
            .is_some()
        {
            return Err(RegistryError::PackageExists {
                package_name: package_metadata.package_name,
                version: package_metadata.version,
            });
        }

        // 5. Store original package data in registry storage (.cloacina or .so)
        let registry_id = self.storage.store_binary(package_data).await?;

        // 6. Store metadata in database
        let package_id = self
            .store_package_metadata(&registry_id, &package_metadata)
            .await?;

        // 7. Register tasks with the global registry using .so data
        let registered_namespaces = self
            .registrar
            .register_package_tasks(
                &package_id.to_string(),
                &so_data,
                &package_metadata,
                Some("public"), // Default tenant
            )
            .await
            .map_err(RegistryError::Loader)?;

        // 7. Track loaded state
        self.loaded_packages
            .insert(package_id, registered_namespaces);

        Ok(package_id)
    }

    async fn get_workflow(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<LoadedWorkflow>, RegistryError> {
        // 1. Get metadata from database
        let (registry_id, package_metadata) =
            match self.get_package_metadata(package_name, version).await? {
                Some(data) => data,
                None => return Ok(None),
            };

        // 2. Retrieve binary data from storage
        let package_data = match self.storage.retrieve_binary(&registry_id).await? {
            Some(data) => data,
            None => {
                return Err(RegistryError::Internal(
                    "Package metadata exists but binary data is missing".to_string(),
                ));
            }
        };

        // 3. Create loaded workflow
        let workflow_metadata = WorkflowMetadata {
            id: Uuid::new_v4(), // This should be the actual package ID from the database
            registry_id: Uuid::parse_str(&registry_id).map_err(RegistryError::InvalidUuid)?,
            package_name: package_metadata.package_name.clone(),
            version: package_metadata.version.clone(),
            description: package_metadata.description.clone(),
            author: package_metadata.author.clone(),
            tasks: package_metadata
                .tasks
                .iter()
                .map(|t| t.local_id.clone())
                .collect(),
            schedules: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        Ok(Some(LoadedWorkflow {
            metadata: workflow_metadata,
            package_data,
        }))
    }

    async fn list_workflows(&self) -> Result<Vec<WorkflowMetadata>, RegistryError> {
        self.list_all_packages().await
    }

    async fn unregister_workflow(
        &mut self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        // 1. Get package metadata to find the package ID
        let (registry_id, _) = self
            .get_package_metadata(package_name, version)
            .await?
            .ok_or_else(|| RegistryError::PackageNotFound {
                package_name: package_name.to_string(),
                version: version.to_string(),
            })?;

        // 2. Find the package ID to unregister tasks
        let package_uuid = Uuid::parse_str(&registry_id).map_err(RegistryError::InvalidUuid)?;

        // 3. Unregister tasks from global registry
        if let Some(_namespaces) = self.loaded_packages.remove(&package_uuid) {
            self.registrar
                .unregister_package_tasks(&package_uuid.to_string())
                .map_err(RegistryError::Loader)?;
        }

        // 4. Delete metadata from database (this will cascade to registry storage via foreign key)
        self.delete_package_metadata(package_name, version).await?;

        // 5. Delete binary data from storage
        self.storage.delete_binary(&registry_id).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::registry::storage::FilesystemRegistryStorage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_registry_creation() {
        let temp_dir = TempDir::new().unwrap();
        let _storage = FilesystemRegistryStorage::new(temp_dir.path()).unwrap();

        // Note: This test would need a proper database setup
        // For now, we'll just test the storage creation part
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_registry_metrics() {
        let temp_dir = TempDir::new().unwrap();
        let _storage = FilesystemRegistryStorage::new(temp_dir.path()).unwrap();

        // This would need a database for full testing
        // For now just test that we can create the storage
        assert!(temp_dir.path().exists());
    }
}
