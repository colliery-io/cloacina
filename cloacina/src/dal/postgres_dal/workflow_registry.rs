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

use crate::dal::postgres_dal::DAL;
use crate::packaging::extract_manifest_from_package;
use crate::registry::error::RegistryError;
use crate::registry::loader::package_loader::PackageMetadata;
use crate::registry::traits::{RegistryStorage, WorkflowRegistry};
use crate::registry::types::{LoadedWorkflow, WorkflowMetadata, WorkflowPackageId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use super::models::{NewPgWorkflowPackage, NewPgWorkflowRegistryEntry, PgWorkflowPackage, PgWorkflowRegistryEntry};

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
}

/// Complete workflow package data including binary and metadata
#[derive(Debug, Clone)]
pub struct WorkflowPackage {
    pub info: WorkflowPackageInfo,
    pub metadata: PackageMetadata,
    pub binary_data: Vec<u8>,
}

impl WorkflowRegistryDAL {
    /// Create a new WorkflowRegistryDAL instance.
    ///
    /// # Arguments
    ///
    /// * `dal` - The main PostgreSQL DAL for metadata operations
    /// * `storage` - Storage backend for binary package data
    pub fn new(dal: DAL, storage: Arc<dyn RegistryStorage + Send + Sync>) -> Self {
        Self { dal, storage }
    }

    /// Register a complete workflow package from binary data.
    ///
    /// This method extracts metadata from the package, stores the binary data,
    /// and records the metadata in the database.
    ///
    /// # Arguments
    ///
    /// * `package_data` - Binary package data to register
    ///
    /// # Returns
    ///
    /// The UUID of the registered package metadata record
    pub async fn register_workflow_package(
        &mut self,
        package_data: Vec<u8>,
    ) -> Result<Uuid, RegistryError> {
        // Extract manifest from binary data (in-memory)
        let manifest = {
            // Write to temporary file for manifest extraction
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

        let metadata = PackageMetadata {
            package_name: manifest.package.name.clone(),
            version: manifest
                .package
                .workflow_fingerprint
                .clone()
                .unwrap_or(manifest.package.version.clone()),
            description: Some(manifest.package.description.clone()),
            author: manifest.package.author.clone(),
            tasks: manifest
                .tasks
                .into_iter()
                .map(
                    |task| crate::registry::loader::package_loader::TaskMetadata {
                        index: task.index,
                        local_id: task.id.clone(),
                        namespaced_id_template: format!(
                            "{}/{}/{}",
                            "{tenant_id}", manifest.package.name, task.id
                        ),
                        dependencies: task.dependencies,
                        description: task.description,
                        source_location: task.source_location,
                    },
                )
                .collect(),
            graph_data: manifest
                .graph
                .map(|g| serde_json::to_value(g).unwrap_or(serde_json::Value::Null)),
            architecture: manifest.library.architecture,
            symbols: manifest.library.symbols,
        };

        // Execute atomic transaction: store binary data and metadata together
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(format!("Failed to get connection: {}", e)))?;

        let package_data_clone = package_data.clone();
        let metadata_clone = metadata.clone();

        let (registry_id, package_id) = conn
            .interact(move |conn| {
                use crate::database::schema::postgres::{workflow_packages, workflow_registry};
                use diesel::prelude::*;
                use diesel::result::Error;

                conn.transaction::<_, Error, _>(|conn| {
                    // 1. Insert binary data into workflow_registry
                    let new_registry_entry = NewPgWorkflowRegistryEntry {
                        data: package_data_clone,
                    };
                    let registry_entry: PgWorkflowRegistryEntry =
                        diesel::insert_into(workflow_registry::table)
                            .values(&new_registry_entry)
                            .get_result(conn)?;

                    let registry_id_str = registry_entry.id.to_string();

                    // 2. Insert metadata into workflow_packages
                    let metadata_json = serde_json::to_string(&metadata_clone)
                        .map_err(|e| Error::RollbackTransaction)?;

                    let new_package = NewPgWorkflowPackage {
                        registry_id: registry_entry.id,
                        package_name: metadata_clone.package_name.clone(),
                        version: metadata_clone.version.clone(),
                        description: metadata_clone.description.clone(),
                        author: metadata_clone.author.clone(),
                        metadata: metadata_json,
                        storage_type: "database".to_string(), // This DAL always stores binary in database
                    };

                    let package_entry: PgWorkflowPackage =
                        diesel::insert_into(workflow_packages::table)
                            .values(&new_package)
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
                    package_name: metadata.package_name.clone(),
                    version: metadata.version.clone(),
                },
                _ => RegistryError::Database(format!("Database error: {}", e)),
            })?;

        Ok(package_id.into())
    }

    /// Get complete workflow package by ID.
    ///
    /// # Arguments
    ///
    /// * `package_id` - UUID of the package to retrieve
    ///
    /// # Returns
    ///
    /// Complete package data if found, None otherwise
    pub async fn get_workflow_package_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(PackageMetadata, Vec<u8>)>, RegistryError> {
        // Get metadata first
        let metadata_result = self
            .dal
            .workflow_packages()
            .get_package_metadata_by_id(package_id)
            .await?;

        match metadata_result {
            Some((registry_id, metadata)) => {
                // Get binary data directly from workflow_registry table
                let binary_data = {
                    use crate::database::schema::postgres::workflow_registry;
                    use diesel::prelude::*;

                    let conn = self
                        .dal
                        .database
                        .get_connection_with_schema()
                        .await
                        .map_err(|e| {
                            RegistryError::Database(format!("Failed to get connection: {}", e))
                        })?;

                    let registry_uuid =
                        Uuid::parse_str(&registry_id).map_err(RegistryError::InvalidUuid)?;

                    let entry: Option<PgWorkflowRegistryEntry> =
                        conn.interact(move |conn| {
                            workflow_registry::table
                                .filter(workflow_registry::id.eq(&registry_uuid))
                                .first::<PgWorkflowRegistryEntry>(conn)
                                .optional()
                        })
                        .await
                        .map_err(|e| {
                            RegistryError::Database(format!("Database interaction error: {}", e))
                        })?
                        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

                    entry.map(|e| e.data)
                };

                match binary_data {
                    Some(data) => {
                        let registry_uuid =
                            Uuid::parse_str(&registry_id).map_err(RegistryError::InvalidUuid)?;

                        let info = WorkflowPackageInfo {
                            id: package_id,
                            registry_id: registry_uuid,
                            package_name: metadata.package_name.clone(),
                            version: metadata.version.clone(),
                            description: metadata.description.clone(),
                            author: metadata.author.clone(),
                        };

                        Ok(Some((metadata, data)))
                    }
                    None => {
                        // Metadata exists but binary data is missing - inconsistent state
                        Err(RegistryError::Database(format!(
                            "Binary data missing for package {} with registry_id {}",
                            package_id, registry_id
                        )))
                    }
                }
            }
            None => Ok(None),
        }
    }

    /// Get complete workflow package by name and version.
    ///
    /// # Arguments
    ///
    /// * `package_name` - Name of the package
    /// * `version` - Version of the package
    ///
    /// # Returns
    ///
    /// Complete package data if found, None otherwise
    pub async fn get_workflow_package_by_name(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<(PackageMetadata, Vec<u8>)>, RegistryError> {
        // Get metadata first
        let metadata_result = self
            .dal
            .workflow_packages()
            .get_package_metadata(package_name, version)
            .await?;

        match metadata_result {
            Some((registry_id, metadata)) => {
                // Get binary data directly from workflow_registry table
                let binary_data = {
                    use crate::database::schema::postgres::workflow_registry;
                    use diesel::prelude::*;

                    let conn = self
                        .dal
                        .database
                        .get_connection_with_schema()
                        .await
                        .map_err(|e| {
                            RegistryError::Database(format!("Failed to get connection: {}", e))
                        })?;

                    let registry_uuid =
                        Uuid::parse_str(&registry_id).map_err(RegistryError::InvalidUuid)?;

                    let entry: Option<PgWorkflowRegistryEntry> =
                        conn.interact(move |conn| {
                            workflow_registry::table
                                .filter(workflow_registry::id.eq(&registry_uuid))
                                .first::<PgWorkflowRegistryEntry>(conn)
                                .optional()
                        })
                        .await
                        .map_err(|e| {
                            RegistryError::Database(format!("Database interaction error: {}", e))
                        })?
                        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

                    entry.map(|e| e.data)
                };

                match binary_data {
                    Some(data) => {
                        let registry_uuid =
                            Uuid::parse_str(&registry_id).map_err(RegistryError::InvalidUuid)?;

                        // We need to get the package ID - this requires another query
                        // For now, we'll generate a placeholder UUID since this method
                        // doesn't naturally have access to the package ID
                        let package_id = Uuid::new_v4(); // This is a limitation of the current design

                        let info = WorkflowPackageInfo {
                            id: package_id,
                            registry_id: registry_uuid,
                            package_name: metadata.package_name.clone(),
                            version: metadata.version.clone(),
                            description: metadata.description.clone(),
                            author: metadata.author.clone(),
                        };

                        Ok(Some((metadata, data)))
                    }
                    None => {
                        // Metadata exists but binary data is missing - inconsistent state
                        Err(RegistryError::Database(format!(
                            "Binary data missing for package {}/{} with registry_id {}",
                            package_name, version, registry_id
                        )))
                    }
                }
            }
            None => Ok(None),
        }
    }

    /// Unregister a workflow package by ID.
    ///
    /// This removes both the metadata and binary data.
    ///
    /// # Arguments
    ///
    /// * `package_id` - UUID of the package to unregister
    pub async fn unregister_workflow_package_by_id(
        &mut self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        // Get the registry_id first so we can delete the binary data
        let metadata_result = self
            .dal
            .workflow_packages()
            .get_package_metadata_by_id(package_id)
            .await?;

        if let Some((registry_id, _)) = metadata_result {
            // Delete binary data first
            Arc::get_mut(&mut self.storage)
                .unwrap()
                .delete_binary(&registry_id)
                .await?;

            // Delete workflow_registry entry
            use crate::database::schema::postgres::workflow_registry;
            use diesel::prelude::*;
            use uuid::Uuid;

            let conn = self
                .dal
                .database
                .get_connection_with_schema()
                .await
                .map_err(|e| RegistryError::Database(format!("Failed to get connection: {}", e)))?;

            let registry_uuid = Uuid::parse_str(&registry_id)
                .map_err(|e| RegistryError::Database(format!("Invalid registry UUID: {}", e)))?;

            let _ = conn
                .interact(move |conn| {
                    diesel::delete(
                        workflow_registry::table.filter(workflow_registry::id.eq(&registry_uuid)),
                    )
                    .execute(conn)
                })
                .await
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        }

        // Delete metadata (this is idempotent)
        self.dal
            .workflow_packages()
            .delete_package_metadata_by_id(package_id)
            .await?;

        Ok(())
    }

    /// Unregister a workflow package by name and version.
    ///
    /// This removes both the metadata and binary data.
    ///
    /// # Arguments
    ///
    /// * `package_name` - Name of the package to unregister
    /// * `version` - Version of the package to unregister
    pub async fn unregister_workflow_package_by_name(
        &mut self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        // Get the registry_id first so we can delete the binary data
        let metadata_result = self
            .dal
            .workflow_packages()
            .get_package_metadata(package_name, version)
            .await?;

        if let Some((registry_id, _)) = metadata_result {
            // Delete binary data first
            Arc::get_mut(&mut self.storage)
                .unwrap()
                .delete_binary(&registry_id)
                .await?;

            // Delete workflow_registry entry
            use crate::database::schema::postgres::workflow_registry;
            use diesel::prelude::*;
            use uuid::Uuid;

            let conn = self
                .dal
                .database
                .get_connection_with_schema()
                .await
                .map_err(|e| RegistryError::Database(format!("Failed to get connection: {}", e)))?;

            let registry_uuid = Uuid::parse_str(&registry_id)
                .map_err(|e| RegistryError::Database(format!("Invalid registry UUID: {}", e)))?;

            let _ = conn
                .interact(move |conn| {
                    diesel::delete(
                        workflow_registry::table.filter(workflow_registry::id.eq(&registry_uuid)),
                    )
                    .execute(conn)
                })
                .await
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        }

        // Delete metadata (this is idempotent)
        self.dal
            .workflow_packages()
            .delete_package_metadata(package_name, version)
            .await?;

        Ok(())
    }

    /// Check if a workflow package exists by ID.
    ///
    /// # Arguments
    ///
    /// * `package_id` - UUID of the package to check
    ///
    /// # Returns
    ///
    /// True if the package exists, false otherwise
    pub async fn exists_by_id(&self, package_id: Uuid) -> Result<bool, RegistryError> {
        let result = self
            .dal
            .workflow_packages()
            .get_package_metadata_by_id(package_id)
            .await?;
        Ok(result.is_some())
    }

    /// Check if a workflow package exists by name and version.
    ///
    /// # Arguments
    ///
    /// * `package_name` - Name of the package to check
    /// * `version` - Version of the package to check
    ///
    /// # Returns
    ///
    /// True if the package exists, false otherwise
    pub async fn exists_by_name(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<bool, RegistryError> {
        let result = self
            .dal
            .workflow_packages()
            .get_package_metadata(package_name, version)
            .await?;
        Ok(result.is_some())
    }

    /// List basic information about all registered workflow packages.
    ///
    /// # Returns
    ///
    /// Vector of WorkflowPackageInfo for all registered packages
    pub async fn list_packages(&self) -> Result<Vec<WorkflowPackageInfo>, RegistryError> {
        let packages = self.dal.workflow_packages().list_all_packages().await?;

        let mut package_infos = Vec::new();
        for package in packages {
            let registry_uuid = package.registry_id;

            let info = WorkflowPackageInfo {
                id: package.id.into(),
                registry_id: package.registry_id.into(),
                package_name: package.package_name,
                version: package.version,
                description: package.description,
                author: package.author,
            };
            package_infos.push(info);
        }

        Ok(package_infos)
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
        let packages = self.list_packages().await?;

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
                tasks: vec![],          // Would need to extract from metadata
                schedules: vec![],      // Would need to extract from metadata
                created_at: Utc::now(), // WorkflowPackageInfo doesn't have timestamps
                updated_at: Utc::now(), // WorkflowPackageInfo doesn't have timestamps
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
