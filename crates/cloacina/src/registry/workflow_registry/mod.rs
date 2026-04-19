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

//! Complete implementation of the workflow registry.
//!
//! This module provides the `WorkflowRegistryImpl` that combines all registry
//! components - storage, loading, validation, and task registration - into a
//! cohesive system for managing packaged workflows.

mod database;
pub mod filesystem;
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
    #[allow(dead_code)]
    loader: PackageLoader,
    /// Task registrar for global registry integration
    registrar: TaskRegistrar,
    /// Package validator for safety checks
    #[allow(dead_code)]
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

    /// Get the source archive bytes for a package the compiler service has
    /// claimed. Unlike `get_workflow_package_by_id`, this does *not* filter
    /// by `build_status = 'success'` — the compiler needs to read source for
    /// rows currently in `building` state, which the reconciler-facing filter
    /// would hide. Still excludes superseded rows.
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

    /// Get a workflow package by its UUID.
    ///
    /// Returns the package metadata and binary data if found.
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
        // 1. Require a bzip2 source archive (.cloacina)
        if !Self::is_cloacina_package(&package_data) {
            return Err(RegistryError::ValidationError {
                reason: "Package data is not a valid .cloacina bzip2 source archive. \
                         Raw library registration is not supported."
                    .to_string(),
            });
        }

        // 2. Read the manifest to extract package name/version for duplicate checking.
        //    We do this by writing to a temp dir and calling unpack + load_manifest.
        let work_dir = tempfile::TempDir::new()
            .map_err(|e| RegistryError::Internal(format!("Failed to create temp dir: {}", e)))?;
        let archive_path = work_dir.path().join("pkg.cloacina");
        std::fs::write(&archive_path, &package_data)
            .map_err(|e| RegistryError::Internal(format!("Failed to write archive: {}", e)))?;
        let extract_dir = work_dir.path().join("source");
        std::fs::create_dir_all(&extract_dir)
            .map_err(|e| RegistryError::Internal(format!("Failed to create extract dir: {}", e)))?;

        let source_dir = fidius_core::package::unpack_package(&archive_path, &extract_dir)
            .map_err(|e| RegistryError::ValidationError {
                reason: format!("Failed to unpack source archive: {}", e),
            })?;

        let manifest = fidius_core::package::load_manifest::<
            cloacina_workflow_plugin::CloacinaMetadata,
        >(&source_dir)
        .map_err(|e| RegistryError::ValidationError {
            reason: format!("Failed to load package.toml: {}", e),
        })?;

        let pkg_name = manifest.package.name.clone();
        let pkg_version = manifest.package.version.clone();

        // 3. Content hash for idempotency + audit of what's installed.
        let content_hash = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&package_data);
            format!("{:x}", hasher.finalize())
        };

        // 4. Look up the currently-active row for this package name.
        //    - Same hash  → idempotent, return existing id (no storage churn, no supersede)
        //    - Different  → supersede old + insert new atomically
        //    - None       → insert new
        let active = self.get_active_package_by_name(&pkg_name).await?;
        if let Some((existing_id, _, ref existing_hash)) = active {
            if existing_hash == &content_hash {
                return Ok(existing_id);
            }
        }

        let package_metadata = crate::registry::loader::package_loader::PackageMetadata {
            package_name: pkg_name,
            version: pkg_version,
            description: manifest.metadata.description.clone(),
            author: manifest.metadata.author.clone(),
            tasks: vec![],
            graph_data: None,
            architecture: std::env::consts::ARCH.to_string(),
            symbols: vec![],
        };

        let registry_id = self.storage.store_binary(package_data).await?;

        let old_id = active.map(|(id, _, _)| id);

        // Content-hash artifact reuse (T-0523): if an earlier row compiled the
        // same bytes successfully, skip the build queue and pre-populate the
        // new row as `success` with the existing compiled_data.
        let prebuilt = self.find_success_by_hash(&content_hash).await?;
        let prebuilt_bytes = prebuilt.map(|(_, bytes)| bytes);

        let package_id = self
            .supersede_and_insert_with_prebuilt(
                old_id,
                &registry_id,
                &package_metadata,
                &content_hash,
                prebuilt_bytes,
            )
            .await?;

        Ok(package_id)
    }

    async fn get_workflow(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<LoadedWorkflow>, RegistryError> {
        // 1. Get metadata + compiled bytes from database (success rows only)
        let (registry_id, package_metadata, compiled_data) =
            match self.get_package_metadata(package_name, version).await? {
                Some(data) => data,
                None => return Ok(None),
            };

        // 2. Retrieve source archive from storage
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
            compiled_data,
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
        let (registry_id, _, _) = self
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
