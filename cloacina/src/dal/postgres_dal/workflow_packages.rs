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

//! PostgreSQL DAL for workflow packages metadata operations.
//!
//! This module provides PostgreSQL-specific data access operations for workflow
//! package metadata, following the established DAL patterns.

use diesel::prelude::*;
use uuid::Uuid;

use crate::dal::postgres_dal::DAL;
use crate::database::schema::postgres::workflow_packages;
use crate::registry::error::RegistryError;
use super::models::{NewPgWorkflowPackage, PgWorkflowPackage};

/// PostgreSQL DAL for workflow packages metadata operations.
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
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let registry_uuid = Uuid::parse_str(registry_id).map_err(RegistryError::InvalidUuid)?;
        let metadata =
            serde_json::to_string(package_metadata).map_err(RegistryError::Serialization)?;

        let new_package = NewPgWorkflowPackage {
            registry_id: registry_uuid,
            package_name: package_metadata.package_name.clone(),
            version: package_metadata.version.clone(),
            description: package_metadata.description.clone(),
            author: package_metadata.author.clone(),
            metadata,
            storage_type: "database".to_string(), // This DAL always stores binary in database
        };

        // Insert using NewPgWorkflowPackage model, let PostgreSQL handle UUID and timestamps
        let inserted_package: PgWorkflowPackage = conn
            .interact(move |conn| {
                diesel::insert_into(workflow_packages::table)
                    .values(&new_package)
                    .get_result(conn)
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

        Ok(inserted_package.id)
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
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_name = package_name.to_string();
        let version = version.to_string();

        let package_record: Option<PgWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::package_name.eq(&package_name))
                    .filter(workflow_packages::version.eq(&version))
                    .first::<PgWorkflowPackage>(conn)
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
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_record: Option<PgWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::id.eq(&package_id))
                    .first::<PgWorkflowPackage>(conn)
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
    pub async fn list_all_packages(&self) -> Result<Vec<crate::models::workflow_packages::WorkflowPackage>, RegistryError> {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_records: Vec<PgWorkflowPackage> = conn
            .interact(move |conn| workflow_packages::table.load::<PgWorkflowPackage>(conn))
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(package_records.into_iter().map(|pg| pg.into()).collect())
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
            .get_connection_with_schema()
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
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::delete(workflow_packages::table.filter(workflow_packages::id.eq(&package_id)))
                .execute(conn)
        })
        .await
        .map_err(|e| RegistryError::Database(e.to_string()))?
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(())
    }
}
