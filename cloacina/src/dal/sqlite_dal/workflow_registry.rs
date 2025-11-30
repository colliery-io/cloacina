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

//! High-level workflow registry DAL for complete package management.
//!
//! This module provides a comprehensive DAL for workflow package operations that
//! coordinates both metadata storage and binary storage backends.

use std::sync::Arc;
use uuid::Uuid;

use crate::dal::sqlite_dal::DAL;
use crate::packaging::extract_manifest_from_package;
use crate::registry::error::RegistryError;
use crate::registry::loader::package_loader::PackageMetadata;
use crate::registry::traits::{RegistryStorage, WorkflowRegistry};
use crate::registry::types::{LoadedWorkflow, WorkflowMetadata, WorkflowPackageId};
use async_trait::async_trait;

/// High-level workflow registry DAL that coordinates metadata and binary storage.
///
/// This DAL provides a complete interface for workflow package management,
/// handling both the extraction of metadata and coordination with storage backends.
pub struct WorkflowRegistryDAL {
    /// The main DAL for metadata operations
    dal: DAL,
    /// Storage backend for binary package data
    storage: Arc<dyn RegistryStorage + Send + Sync>,
}

/// Basic information about a registered workflow package
#[derive(Debug, Clone)]
pub struct WorkflowPackageInfo {
    pub id: Uuid,
    pub registry_id: Uuid,
    pub package_name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl WorkflowRegistryDAL {
    /// Create a new workflow registry DAL.
    ///
    /// # Arguments
    ///
    /// * `dal` - The main DAL for metadata operations
    /// * `storage` - Storage backend for binary package data
    pub fn new(dal: DAL, storage: Arc<dyn RegistryStorage + Send + Sync>) -> Self {
        Self { dal, storage }
    }

    /// Register a complete workflow package.
    ///
    /// This method handles the entire registration process:
    /// 1. Extracts manifest from package data
    /// 2. Uses workflow fingerprint as version
    /// 3. Stores binary data in storage backend
    /// 4. Stores metadata in database with proper linking
    ///
    /// # Arguments
    ///
    /// * `package_data` - Raw bytes of the .cloacina package file
    ///
    /// # Returns
    ///
    /// * `Ok(Uuid)` - The package UUID for future operations
    /// * `Err(RegistryError)` - If registration fails
    pub async fn register_workflow_package(
        &mut self,
        package_data: Vec<u8>,
    ) -> Result<Uuid, RegistryError> {
        // 1. Extract manifest from package to get metadata with fingerprint
        let manifest = {
            // Create a temporary file for manifest extraction
            let temp_file = tempfile::NamedTempFile::new().map_err(|e| {
                RegistryError::Internal(format!("Failed to create temp file: {}", e))
            })?;

            std::fs::write(temp_file.path(), &package_data).map_err(|e| {
                RegistryError::Internal(format!("Failed to write temp file: {}", e))
            })?;

            extract_manifest_from_package(&temp_file.path().to_path_buf()).map_err(|e| {
                RegistryError::Internal(format!("Failed to extract manifest: {}", e))
            })?
        };

        // 2. Create PackageMetadata from manifest using fingerprint as version
        let package_metadata = PackageMetadata {
            package_name: manifest.package.name.clone(),
            version: manifest
                .package
                .workflow_fingerprint
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            description: Some(manifest.package.description.clone()),
            author: manifest.package.author.clone(),
            tasks: vec![], // We don't need task details for registry storage
            graph_data: None,
            architecture: manifest.library.architecture.clone(),
            symbols: vec![], // Not needed for registry
        };

        // 3. Check if package already exists
        if self
            .exists_by_name(&package_metadata.package_name, &package_metadata.version)
            .await?
        {
            return Err(RegistryError::PackageExists {
                package_name: package_metadata.package_name,
                version: package_metadata.version,
            });
        }

        // 4. Execute atomic transaction: store binary data and metadata together
        let conn = self
            .dal
            .database
            .pool()
            .get()
            .await
            .map_err(|e| RegistryError::Database(format!("Failed to get connection: {}", e)))?;

        let package_data_clone = package_data.clone();
        let package_metadata_clone = package_metadata.clone();

        let (registry_id, package_id) = conn
            .interact(move |conn| {
                use crate::database::schema::sqlite::{workflow_packages, workflow_registry};
                use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
                use crate::models::workflow_packages::NewWorkflowPackage;
                use crate::models::workflow_registry::NewWorkflowRegistryEntry;
                use diesel::prelude::*;
                use diesel::result::Error;

                conn.transaction::<_, Error, _>(|conn| {
                    // 1. Insert binary data into workflow_registry with explicit ID and timestamp
                    let registry_id = UniversalUuid::new_v4();
                    let now = UniversalTimestamp::now();
                    let new_registry_entry = NewWorkflowRegistryEntry::new(package_data_clone);

                    let registry_entry: crate::models::workflow_registry::WorkflowRegistryEntry =
                        diesel::insert_into(workflow_registry::table)
                            .values((
                                workflow_registry::id.eq(&registry_id),
                                &new_registry_entry,
                                workflow_registry::created_at.eq(&now),
                            ))
                            .get_result(conn)?;

                    let registry_id_str = registry_entry.id.to_string();

                    // 2. Insert metadata into workflow_packages with explicit ID and timestamps
                    let metadata_json = serde_json::to_string(&package_metadata_clone)
                        .map_err(|_| Error::RollbackTransaction)?;

                    let package_id = UniversalUuid::new_v4();
                    let new_package = NewWorkflowPackage::new(
                        registry_entry.id,
                        package_metadata_clone.package_name.clone(),
                        package_metadata_clone.version.clone(),
                        package_metadata_clone.description.clone(),
                        package_metadata_clone.author.clone(),
                        metadata_json,
                    );

                    let package_entry: crate::models::workflow_packages::WorkflowPackage =
                        diesel::insert_into(workflow_packages::table)
                            .values((
                                workflow_packages::id.eq(&package_id),
                                &new_package,
                                workflow_packages::created_at.eq(&now),
                                workflow_packages::updated_at.eq(&now),
                            ))
                            .get_result(conn)?;

                    Ok((registry_id_str, package_entry.id))
                })
            })
            .await
            .map_err(|e| RegistryError::Database(format!("Database interaction error: {}", e)))?
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

        Ok(package_id.into())
    }

    /// Retrieve a workflow package by UUID.
    ///
    /// # Arguments
    ///
    /// * `package_id` - UUID of the package to retrieve
    ///
    /// # Returns
    ///
    /// * `Ok(Some((PackageMetadata, Vec<u8>)))` - Package metadata and binary data
    /// * `Ok(None)` - Package not found
    /// * `Err(RegistryError)` - If retrieval fails
    pub async fn get_workflow_package_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(PackageMetadata, Vec<u8>)>, RegistryError> {
        // 1. Get package metadata from database
        let package_record = self
            .dal
            .workflow_packages()
            .get_package_metadata_by_id(package_id)
            .await?;

        let (registry_id, package_metadata) = match package_record {
            Some(data) => data,
            None => return Ok(None),
        };

        // 2. Get binary data directly from workflow_registry table
        let binary_data = {
            use crate::database::schema::sqlite::workflow_registry;
            use diesel::prelude::*;

            let conn =
                self.dal.database.pool().get().await.map_err(|e| {
                    RegistryError::Database(format!("Failed to get connection: {}", e))
                })?;

            let registry_uuid =
                Uuid::parse_str(&registry_id).map_err(RegistryError::InvalidUuid)?;
            let registry_universal_uuid =
                crate::database::universal_types::UniversalUuid::from(registry_uuid);

            let entry: Option<crate::models::workflow_registry::WorkflowRegistryEntry> = conn
                .interact(move |conn| {
                    workflow_registry::table
                        .filter(workflow_registry::id.eq(&registry_universal_uuid))
                        .first::<crate::models::workflow_registry::WorkflowRegistryEntry>(conn)
                        .optional()
                })
                .await
                .map_err(|e| RegistryError::Database(format!("Database interaction error: {}", e)))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

            entry.map(|e| e.data)
        };

        match binary_data {
            Some(data) => Ok(Some((package_metadata, data))),
            None => Err(RegistryError::Internal(
                "Package metadata exists but binary data is missing".to_string(),
            )),
        }
    }

    /// Retrieve a workflow package by name and version.
    ///
    /// # Arguments
    ///
    /// * `package_name` - Name of the package
    /// * `version` - Version of the package (fingerprint)
    ///
    /// # Returns
    ///
    /// * `Ok(Some((PackageMetadata, Vec<u8>)))` - Package metadata and binary data
    /// * `Ok(None)` - Package not found
    /// * `Err(RegistryError)` - If retrieval fails
    pub async fn get_workflow_package_by_name(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<(PackageMetadata, Vec<u8>)>, RegistryError> {
        // 1. Get package metadata from database
        let package_record = self
            .dal
            .workflow_packages()
            .get_package_metadata(package_name, version)
            .await?;

        let (registry_id, package_metadata) = match package_record {
            Some(data) => data,
            None => return Ok(None),
        };

        // DEBUG: Log the registry_id we got from workflow_packages
        tracing::debug!("Got registry_id from workflow_packages: '{}'", registry_id);

        // 2. Get binary data directly from workflow_registry table
        let binary_data = {
            use crate::database::schema::sqlite::workflow_registry;
            use diesel::prelude::*;

            let conn =
                self.dal.database.pool().get().await.map_err(|e| {
                    RegistryError::Database(format!("Failed to get connection: {}", e))
                })?;

            // DEBUG: Try to parse the registry_id as UUID
            tracing::debug!("Attempting to parse registry_id '{}' as UUID", registry_id);
            let registry_uuid = Uuid::parse_str(&registry_id).map_err(|e| {
                tracing::error!(
                    "Failed to parse registry_id '{}' as UUID: {}",
                    registry_id,
                    e
                );
                RegistryError::InvalidUuid(e)
            })?;
            tracing::debug!("Successfully parsed UUID: {}", registry_uuid);
            let registry_universal_uuid =
                crate::database::universal_types::UniversalUuid::from(registry_uuid);

            tracing::debug!(
                "Querying workflow_registry table for UUID: {}",
                registry_universal_uuid
            );

            let entry: Option<crate::models::workflow_registry::WorkflowRegistryEntry> = conn
                .interact(move |conn| {
                    workflow_registry::table
                        .filter(workflow_registry::id.eq(&registry_universal_uuid))
                        .first::<crate::models::workflow_registry::WorkflowRegistryEntry>(conn)
                        .optional()
                })
                .await
                .map_err(|e| RegistryError::Database(format!("Database interaction error: {}", e)))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

            tracing::debug!("Query result: entry found = {}", entry.is_some());
            if let Some(ref entry) = entry {
                tracing::debug!("Entry found with data length: {}", entry.data.len());
            }

            entry.map(|e| e.data)
        };

        match binary_data {
            Some(data) => Ok(Some((package_metadata, data))),
            None => Err(RegistryError::Internal(
                "Package metadata exists but binary data is missing".to_string(),
            )),
        }
    }

    /// Unregister a workflow package by UUID.
    ///
    /// # Arguments
    ///
    /// * `package_id` - UUID of the package to unregister
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Package successfully unregistered
    /// * `Err(RegistryError)` - If unregistration fails
    pub async fn unregister_workflow_package_by_id(
        &mut self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        // 1. Get registry_id from metadata (if exists)
        let package_record = self
            .dal
            .workflow_packages()
            .get_package_metadata_by_id(package_id)
            .await?;

        if let Some((registry_id, _)) = package_record {
            // 2. Delete binary data from storage
            Arc::get_mut(&mut self.storage)
                .unwrap()
                .delete_binary(&registry_id)
                .await
                .map_err(|e| {
                    RegistryError::Storage(crate::registry::error::StorageError::Backend(format!(
                        "Failed to delete binary data: {}",
                        e
                    )))
                })?;

            // 3. Delete workflow_registry entry
            use crate::database::schema::sqlite::workflow_registry;
            use diesel::prelude::*;
            use uuid::Uuid;

            let conn =
                self.dal.database.pool().get().await.map_err(|e| {
                    RegistryError::Database(format!("Failed to get connection: {}", e))
                })?;

            let registry_uuid = Uuid::parse_str(&registry_id)
                .map_err(|e| RegistryError::Database(format!("Invalid registry UUID: {}", e)))?;
            let uuid_param = crate::database::universal_types::UniversalUuid::from(registry_uuid);

            let _ = conn
                .interact(move |conn| {
                    diesel::delete(
                        workflow_registry::table.filter(workflow_registry::id.eq(uuid_param)),
                    )
                    .execute(conn)
                })
                .await
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        }

        // 4. Delete metadata from database (this is idempotent)
        self.dal
            .workflow_packages()
            .delete_package_metadata_by_id(package_id)
            .await?;

        Ok(())
    }

    /// Unregister a workflow package by name and version.
    ///
    /// # Arguments
    ///
    /// * `package_name` - Name of the package
    /// * `version` - Version of the package (fingerprint)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Package successfully unregistered
    /// * `Err(RegistryError)` - If unregistration fails
    pub async fn unregister_workflow_package_by_name(
        &mut self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        // 1. Get registry_id from metadata (if exists)
        let package_record = self
            .dal
            .workflow_packages()
            .get_package_metadata(package_name, version)
            .await?;

        if let Some((registry_id, _)) = package_record {
            // 2. Delete binary data from storage
            Arc::get_mut(&mut self.storage)
                .unwrap()
                .delete_binary(&registry_id)
                .await
                .map_err(|e| {
                    RegistryError::Storage(crate::registry::error::StorageError::Backend(format!(
                        "Failed to delete binary data: {}",
                        e
                    )))
                })?;

            // 3. Delete workflow_registry entry
            use crate::database::schema::sqlite::workflow_registry;
            use diesel::prelude::*;
            use uuid::Uuid;

            let conn =
                self.dal.database.pool().get().await.map_err(|e| {
                    RegistryError::Database(format!("Failed to get connection: {}", e))
                })?;

            let registry_uuid = Uuid::parse_str(&registry_id)
                .map_err(|e| RegistryError::Database(format!("Invalid registry UUID: {}", e)))?;
            let uuid_param = crate::database::universal_types::UniversalUuid::from(registry_uuid);

            let _ = conn
                .interact(move |conn| {
                    diesel::delete(
                        workflow_registry::table.filter(workflow_registry::id.eq(uuid_param)),
                    )
                    .execute(conn)
                })
                .await
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        }

        // 4. Delete metadata from database (this is idempotent)
        self.dal
            .workflow_packages()
            .delete_package_metadata(package_name, version)
            .await?;

        Ok(())
    }

    /// Check if a workflow package exists by UUID.
    ///
    /// # Arguments
    ///
    /// * `package_id` - UUID of the package to check
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - True if package exists, false otherwise
    /// * `Err(RegistryError)` - If check fails
    pub async fn exists_by_id(&self, package_id: Uuid) -> Result<bool, RegistryError> {
        let exists = self
            .dal
            .workflow_packages()
            .get_package_metadata_by_id(package_id)
            .await?
            .is_some();
        Ok(exists)
    }

    /// Check if a workflow package exists by name and version.
    ///
    /// # Arguments
    ///
    /// * `package_name` - Name of the package
    /// * `version` - Version of the package (fingerprint)
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - True if package exists, false otherwise
    /// * `Err(RegistryError)` - If check fails
    pub async fn exists_by_name(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<bool, RegistryError> {
        let exists = self
            .dal
            .workflow_packages()
            .get_package_metadata(package_name, version)
            .await?
            .is_some();
        Ok(exists)
    }

    /// List all registered workflow packages.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<WorkflowPackageInfo>)` - List of all registered packages
    /// * `Err(RegistryError)` - If listing fails
    pub async fn list_packages(&self) -> Result<Vec<WorkflowPackageInfo>, RegistryError> {
        let packages = self.dal.workflow_packages().list_all_packages().await?;

        let package_info = packages
            .into_iter()
            .map(|pkg| WorkflowPackageInfo {
                id: pkg.id.into(),
                registry_id: pkg.registry_id.into(),
                package_name: pkg.package_name,
                version: pkg.version,
                description: pkg.description,
                author: pkg.author,
                created_at: pkg.created_at.into(),
                updated_at: pkg.updated_at.into(),
            })
            .collect();

        Ok(package_info)
    }
}

/// Implementation of WorkflowRegistry trait for WorkflowRegistryDAL
///
/// This bridges the new DAL system with the existing WorkflowRegistry trait
/// used by the reconciler and other parts of the system.
#[async_trait]
impl WorkflowRegistry for WorkflowRegistryDAL {
    async fn register_workflow(
        &mut self,
        package_data: Vec<u8>,
    ) -> Result<WorkflowPackageId, RegistryError> {
        self.register_workflow_package(package_data).await
    }

    async fn get_workflow(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<LoadedWorkflow>, RegistryError> {
        tracing::debug!(
            "WorkflowRegistry trait: get_workflow called for {}:{}",
            package_name,
            version
        );
        match self
            .get_workflow_package_by_name(package_name, version)
            .await?
        {
            Some((package_metadata, package_data)) => {
                // Convert PackageMetadata to WorkflowMetadata
                let workflow_metadata = WorkflowMetadata {
                    id: Uuid::new_v4(),          // We don't have this in PackageMetadata, generate one
                    registry_id: Uuid::new_v4(), // Same issue, would need to look this up
                    package_name: package_metadata.package_name,
                    version: package_metadata.version,
                    description: package_metadata.description,
                    author: package_metadata.author,
                    tasks: package_metadata
                        .tasks
                        .into_iter()
                        .map(|task| task.local_id)
                        .collect(),
                    schedules: vec![], // PackageMetadata doesn't have schedules currently
                    created_at: chrono::Utc::now(), // Not available in PackageMetadata
                    updated_at: chrono::Utc::now(), // Not available in PackageMetadata
                };

                Ok(Some(LoadedWorkflow::new(workflow_metadata, package_data)))
            }
            None => Ok(None),
        }
    }

    async fn list_workflows(&self) -> Result<Vec<WorkflowMetadata>, RegistryError> {
        tracing::debug!("WorkflowRegistry trait: list_workflows called");
        let packages = self.list_packages().await?;
        tracing::debug!(
            "WorkflowRegistry trait: list_workflows found {} packages",
            packages.len()
        );

        // Convert WorkflowPackageInfo to WorkflowMetadata
        let workflows = packages
            .into_iter()
            .map(|pkg| WorkflowMetadata {
                id: pkg.id,
                registry_id: pkg.registry_id,
                package_name: pkg.package_name,
                version: pkg.version,
                description: pkg.description,
                author: pkg.author,
                tasks: vec![],     // Would need to extract from metadata
                schedules: vec![], // Would need to extract from metadata
                created_at: pkg.created_at,
                updated_at: pkg.updated_at,
            })
            .collect();

        Ok(workflows)
    }

    async fn unregister_workflow(
        &mut self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        self.unregister_workflow_package_by_name(package_name, version)
            .await
    }
}
