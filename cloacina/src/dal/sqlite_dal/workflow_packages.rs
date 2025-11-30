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

use crate::dal::sqlite_dal::DAL;
use crate::database::schema::sqlite::workflow_packages;
use crate::database::universal_types::{current_timestamp, UniversalUuid};
use crate::models::workflow_packages::{
    NewWorkflowPackage as ModelNewWorkflowPackage, WorkflowPackage,
};
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
            // DEBUG: Log the registry_id and its string representation
            tracing::debug!(
                "Found package record with registry_id: {:?}",
                record.registry_id
            );
            let registry_id_string = record.registry_id.to_string();
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
            .database
            .pool()
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_uuid = UniversalUuid::from(package_id);

        let package_record: Option<WorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::id.eq(&package_uuid))
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
    pub async fn list_all_packages(&self) -> Result<Vec<WorkflowPackage>, RegistryError> {
        let conn = self
            .dal
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

        Ok(package_records)
    }

    /// Delete package metadata from the database.
    pub async fn delete_package_metadata(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        let conn = self
            .dal
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

    /// Delete package metadata by UUID from the database.
    pub async fn delete_package_metadata_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        let conn = self
            .dal
            .database
            .pool()
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_uuid = UniversalUuid::from(package_id);

        conn.interact(move |conn| {
            diesel::delete(workflow_packages::table.filter(workflow_packages::id.eq(&package_uuid)))
                .execute(conn)
        })
        .await
        .map_err(|e| RegistryError::Database(e.to_string()))?
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(())
    }
}
