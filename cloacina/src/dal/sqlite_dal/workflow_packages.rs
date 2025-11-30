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

//! SQLite DAL for workflow packages metadata operations.
//!
//! This module provides SQLite-specific data access operations for workflow
//! package metadata, following the established DAL patterns.

use diesel::prelude::*;
use uuid::Uuid;

use super::models::{NewSqliteWorkflowPackage, SqliteWorkflowPackage, uuid_to_blob, current_timestamp_string, blob_to_uuid};
use crate::dal::sqlite_dal::DAL;
use crate::database::schema::sqlite::workflow_packages;
use crate::models::workflow_packages::WorkflowPackage;
use crate::registry::error::RegistryError;

/// SQLite DAL for workflow packages metadata operations.
pub struct WorkflowPackagesDAL<'a> {
    /// Reference to the main DAL instance
    pub dal: &'a DAL,
}

impl<'a> WorkflowPackagesDAL<'a> {
    /// Store package metadata in the database.
    pub async fn store_package_metadata(
        &self,
        registry_id: &str,
        package_metadata: &crate::registry::loader::package_loader::PackageMetadata,
    ) -> Result<Uuid, RegistryError> {
        let conn = self
            .dal
            .pool
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let registry_uuid = Uuid::parse_str(registry_id).map_err(RegistryError::InvalidUuid)?;
        let metadata =
            serde_json::to_string(package_metadata).map_err(RegistryError::Serialization)?;

        // Generate UUID client-side
        let id = Uuid::new_v4();
        let id_blob = uuid_to_blob(&id);
        let registry_id_blob = uuid_to_blob(&registry_uuid);
        let now = current_timestamp_string();

        let new_package = NewSqliteWorkflowPackage {
            id: id_blob,
            registry_id: registry_id_blob,
            package_name: package_metadata.package_name.clone(),
            version: package_metadata.version.clone(),
            description: package_metadata.description.clone(),
            author: package_metadata.author.clone(),
            metadata,
            created_at: now.clone(),
            updated_at: now,
        };

        // Insert with explicit values following DAL pattern
        conn.interact(move |conn| {
            diesel::insert_into(workflow_packages::table)
                .values(&new_package)
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

        Ok(id)
    }

    /// Retrieve package metadata from the database.
    pub async fn get_package_metadata(
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
        let conn = self
            .dal
            .pool
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_name = package_name.to_string();
        let version = version.to_string();

        let package_record: Option<SqliteWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::package_name.eq(&package_name))
                    .filter(workflow_packages::version.eq(&version))
                    .first::<SqliteWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = package_record {
            // Convert blob to UUID
            let registry_uuid = blob_to_uuid(&record.registry_id)
                .map_err(|e| RegistryError::Database(format!("Invalid registry UUID: {}", e)))?;
            let registry_id_string = registry_uuid.to_string();

            // DEBUG: Log the registry_id and its string representation
            tracing::debug!(
                "Found package record with registry_id: {:?}",
                registry_id_string
            );
            tracing::debug!("registry_id.to_string() = '{}'", registry_id_string);

            // Deserialize package metadata from JSON string
            let metadata: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((registry_id_string, metadata)))
        } else {
            Ok(None)
        }
    }

    /// Retrieve package metadata by UUID from the database.
    pub async fn get_package_metadata_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<
        Option<(
            String,
            crate::registry::loader::package_loader::PackageMetadata,
        )>,
        RegistryError,
    > {
        let conn = self
            .dal
            .pool
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_id_blob = uuid_to_blob(&package_id);

        let package_record: Option<SqliteWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::id.eq(&package_id_blob))
                    .first::<SqliteWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = package_record {
            // Convert blob to UUID
            let registry_uuid = blob_to_uuid(&record.registry_id)
                .map_err(|e| RegistryError::Database(format!("Invalid registry UUID: {}", e)))?;

            // Deserialize package metadata from JSON string
            let metadata: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((registry_uuid.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    /// List all packages in the registry.
    pub async fn list_all_packages(&self) -> Result<Vec<WorkflowPackage>, RegistryError> {
        let conn = self
            .dal
            .pool
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_records: Vec<SqliteWorkflowPackage> = conn
            .interact(move |conn| workflow_packages::table.load::<SqliteWorkflowPackage>(conn))
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        // Convert to domain types
        let domain_packages: Vec<WorkflowPackage> = package_records
            .into_iter()
            .map(|p| p.into())
            .collect();

        Ok(domain_packages)
    }

    /// Delete package metadata from the database.
    pub async fn delete_package_metadata(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        let conn = self
            .dal
            .pool
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

    /// Delete package metadata by UUID from the database.
    pub async fn delete_package_metadata_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        let conn = self
            .dal
            .pool
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_id_blob = uuid_to_blob(&package_id);

        conn.interact(move |conn| {
            diesel::delete(workflow_packages::table.filter(workflow_packages::id.eq(&package_id_blob)))
                .execute(conn)
        })
        .await
        .map_err(|e| RegistryError::Database(e.to_string()))?
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(())
    }
}
