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

use async_trait::async_trait;
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::io::Read;
use tar::Archive;
use uuid::Uuid;

#[cfg(feature = "postgres")]
use crate::database::schema::postgres::workflow_packages;
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
use crate::database::schema::sqlite::workflow_packages;
use crate::database::universal_types::{current_timestamp, UniversalUuid};
use crate::database::Database;
use crate::models::workflow_packages::{
    NewWorkflowPackage as ModelNewWorkflowPackage, WorkflowPackage,
};
use crate::registry::error::RegistryError;
use crate::registry::loader::{PackageLoader, PackageValidator, TaskRegistrar};
use crate::registry::traits::{RegistryStorage, WorkflowRegistry};
use crate::registry::types::{LoadedWorkflow, WorkflowMetadata, WorkflowPackageId};
use crate::task::TaskNamespace;

use diesel::prelude::*;

/// Complete implementation of the workflow registry.
///
/// This registry implementation combines storage backends, package loading,
/// validation, and task registration to provide a full-featured system for
/// managing packaged workflows with proper lifecycle management.
pub struct WorkflowRegistryImpl<S: RegistryStorage> {
    /// Storage backend for binary data
    storage: S,
    /// Database for metadata storage
    database: Database,
    /// Package loader for metadata extraction
    loader: PackageLoader,
    /// Task registrar for global registry integration
    registrar: TaskRegistrar,
    /// Package validator for safety checks
    validator: PackageValidator,
    /// Map of package IDs to registered task namespaces for cleanup tracking
    loaded_packages: HashMap<Uuid, Vec<TaskNamespace>>,
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

    /// Check if package data is a .cloacina archive (tar.gz format)
    fn is_cloacina_package(data: &[u8]) -> bool {
        // Check for gzip magic number at the start
        data.len() >= 3 && data[0] == 0x1f && data[1] == 0x8b && data[2] == 0x08
    }

    /// Extract .so file from .cloacina package archive
    async fn extract_so_from_cloacina(package_data: &[u8]) -> Result<Vec<u8>, RegistryError> {
        // Create a cursor from the package data
        let cursor = std::io::Cursor::new(package_data);
        let gz_decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(gz_decoder);

        // Look for .so file in the archive
        for entry in archive
            .entries()
            .map_err(|e| RegistryError::ValidationError {
                reason: format!("Failed to read archive entries: {}", e),
            })?
        {
            let mut entry = entry.map_err(|e| RegistryError::ValidationError {
                reason: format!("Failed to read archive entry: {}", e),
            })?;

            let path = entry.path().map_err(|e| RegistryError::ValidationError {
                reason: format!("Failed to get entry path: {}", e),
            })?;

            // Check if this is a dynamic library file (.so on Linux, .dylib on macOS, .dll on Windows)
            if let Some(extension) = path.extension() {
                if extension == "so" || extension == "dylib" || extension == "dll" {
                    let mut library_data = Vec::new();
                    entry.read_to_end(&mut library_data).map_err(|e| {
                        RegistryError::ValidationError {
                            reason: format!("Failed to read library file from archive: {}", e),
                        }
                    })?;
                    return Ok(library_data);
                }
            }
        }

        Err(RegistryError::ValidationError {
            reason: "No dynamic library file (.so/.dylib/.dll) found in .cloacina package"
                .to_string(),
        })
    }

    /// Store package metadata in the database.
    async fn store_package_metadata(
        &self,
        registry_id: &str,
        package_metadata: &crate::registry::loader::package_loader::PackageMetadata,
    ) -> Result<Uuid, RegistryError> {
        #[cfg(feature = "postgres")]
        let conn = self
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let conn = self
            .database
            .pool()
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let registry_uuid = Uuid::parse_str(registry_id).map_err(RegistryError::InvalidUuid)?;
        let metadata =
            serde_json::to_string(package_metadata).map_err(RegistryError::Serialization)?;

        let new_package = ModelNewWorkflowPackage::new(
            UniversalUuid::from(registry_uuid),
            package_metadata.package_name.clone(),
            package_metadata.version.clone(),
            package_metadata.description.clone(),
            package_metadata.author.clone(),
            metadata,
        );

        // Following DAL pattern: manually generate UUID and timestamps
        let id = UniversalUuid::new_v4();
        let now = current_timestamp();

        // Insert with explicit values following DAL pattern
        conn.interact(move |conn| {
            diesel::insert_into(workflow_packages::table)
                .values((
                    workflow_packages::id.eq(&id),
                    workflow_packages::registry_id.eq(&new_package.registry_id),
                    workflow_packages::package_name.eq(&new_package.package_name),
                    workflow_packages::version.eq(&new_package.version),
                    workflow_packages::description.eq(&new_package.description),
                    workflow_packages::author.eq(&new_package.author),
                    workflow_packages::metadata.eq(&new_package.metadata),
                    workflow_packages::created_at.eq(&now),
                    workflow_packages::updated_at.eq(&now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| RegistryError::Database(e.to_string()))?
        .map_err(|e| match e {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _info,
            ) => RegistryError::PackageExists {
                package_name: package_metadata.package_name.clone(),
                version: package_metadata.version.clone(),
            },
            _ => RegistryError::Database(format!("Database error: {}", e)),
        })?;

        Ok(id.into())
    }

    /// Retrieve package metadata from the database.
    async fn get_package_metadata(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<
        Option<(
            String,
            crate::registry::loader::package_loader::PackageMetadata,
        )>,
        RegistryError,
    > {
        #[cfg(feature = "postgres")]
        let conn = self
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let conn = self
            .database
            .pool()
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_name = package_name.to_string();
        let version = version.to_string();

        let package_record: Option<WorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::package_name.eq(&package_name))
                    .filter(workflow_packages::version.eq(&version))
                    .first::<WorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = package_record {
            // Deserialize package metadata from JSON string
            let metadata: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((record.registry_id.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    /// List all packages in the registry.
    async fn list_all_packages(&self) -> Result<Vec<WorkflowMetadata>, RegistryError> {
        #[cfg(feature = "postgres")]
        let conn = self
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let conn = self
            .database
            .pool()
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_records: Vec<WorkflowPackage> = conn
            .interact(move |conn| workflow_packages::table.load::<WorkflowPackage>(conn))
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        let mut workflows = Vec::new();
        for record in package_records {
            // Deserialize package metadata from JSON to extract tasks and schedules
            let package_metadata: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;

            workflows.push(WorkflowMetadata {
                id: record.id.into(),
                registry_id: record.registry_id.into(),
                package_name: record.package_name,
                version: record.version,
                description: record.description,
                author: record.author,
                tasks: package_metadata
                    .tasks
                    .iter()
                    .map(|t| t.local_id.clone())
                    .collect(),
                schedules: Vec::new(), // Could be extracted from graph_data in the future
                created_at: record.created_at.into(),
                updated_at: record.updated_at.into(),
            });
        }

        Ok(workflows)
    }

    /// Delete package metadata from the database.
    async fn delete_package_metadata(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        #[cfg(feature = "postgres")]
        let conn = self
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let conn = self
            .database
            .pool()
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_name = package_name.to_string();
        let version = version.to_string();

        conn.interact(move |conn| {
            diesel::delete(
                workflow_packages::table
                    .filter(workflow_packages::package_name.eq(&package_name))
                    .filter(workflow_packages::version.eq(&version)),
            )
            .execute(conn)
        })
        .await
        .map_err(|e| RegistryError::Database(e.to_string()))?
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(())
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
        if let Some(_) = self
            .get_package_metadata(&package_metadata.package_name, &package_metadata.version)
            .await?
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
    use super::*;
    use crate::registry::storage::FilesystemRegistryStorage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_registry_creation() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FilesystemRegistryStorage::new(temp_dir.path()).unwrap();

        // Note: This test would need a proper database setup
        // For now, we'll just test the storage creation part
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_registry_metrics() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FilesystemRegistryStorage::new(temp_dir.path()).unwrap();

        // This would need a database for full testing
        // For now just test that we can create the storage
        assert!(temp_dir.path().exists());
    }
}
