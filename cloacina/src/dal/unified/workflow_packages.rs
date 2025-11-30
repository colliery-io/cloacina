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

//! Unified Workflow Packages DAL with runtime backend selection
//!
//! This module provides CRUD operations for WorkflowPackage entities that work with
//! both PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.

use super::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::database::BackendType;
use crate::models::workflow_packages::{NewWorkflowPackage, WorkflowPackage};
use crate::registry::error::RegistryError;
use crate::registry::loader::package_loader::PackageMetadata;
use diesel::prelude::*;
use uuid::Uuid;

/// Data access layer for workflow package operations with runtime backend selection.
#[derive(Clone)]
pub struct WorkflowPackagesDAL<'a> {
    dal: &'a DAL,
}

impl<'a> WorkflowPackagesDAL<'a> {
    /// Creates a new WorkflowPackagesDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Store package metadata in the database.
    pub async fn store_package_metadata(
        &self,
        registry_id: &str,
        package_metadata: &PackageMetadata,
    ) -> Result<Uuid, RegistryError> {
        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => {
                self.store_package_metadata_postgres(registry_id, package_metadata)
                    .await
            }
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => {
                self.store_package_metadata_sqlite(registry_id, package_metadata)
                    .await
            }
        }
    }

    #[cfg(feature = "postgres")]
    async fn store_package_metadata_postgres(
        &self,
        registry_id: &str,
        package_metadata: &PackageMetadata,
    ) -> Result<Uuid, RegistryError> {
        use crate::database::schema::postgres::workflow_packages;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let registry_uuid = Uuid::parse_str(registry_id).map_err(RegistryError::InvalidUuid)?;
        let registry_universal_uuid = UniversalUuid::from(registry_uuid);
        let metadata = serde_json::to_string(package_metadata).map_err(RegistryError::Serialization)?;

        let new_package = NewWorkflowPackage::new(
            registry_universal_uuid,
            package_metadata.package_name.clone(),
            package_metadata.version.clone(),
            package_metadata.description.clone(),
            package_metadata.author.clone(),
            metadata,
        );

        let package_name_clone = package_metadata.package_name.clone();
        let version_clone = package_metadata.version.clone();

        let inserted_package: WorkflowPackage = conn
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
                    package_name: package_name_clone,
                    version: version_clone,
                },
                _ => RegistryError::Database(format!("Database error: {}", e)),
            })?;

        Ok(inserted_package.id.into())
    }

    #[cfg(feature = "sqlite")]
    async fn store_package_metadata_sqlite(
        &self,
        registry_id: &str,
        package_metadata: &PackageMetadata,
    ) -> Result<Uuid, RegistryError> {
        use crate::database::schema::sqlite::workflow_packages;
        use crate::database::universal_types::current_timestamp;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let registry_uuid = Uuid::parse_str(registry_id).map_err(RegistryError::InvalidUuid)?;
        let metadata = serde_json::to_string(package_metadata).map_err(RegistryError::Serialization)?;

        let new_package = NewWorkflowPackage::new(
            UniversalUuid::from(registry_uuid),
            package_metadata.package_name.clone(),
            package_metadata.version.clone(),
            package_metadata.description.clone(),
            package_metadata.author.clone(),
            metadata,
        );

        // For SQLite, generate UUID and timestamps client-side
        let id = UniversalUuid::new_v4();
        let now = current_timestamp();

        let package_name_clone = package_metadata.package_name.clone();
        let version_clone = package_metadata.version.clone();

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
                package_name: package_name_clone,
                version: version_clone,
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
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => {
                self.get_package_metadata_postgres(package_name, version)
                    .await
            }
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => {
                self.get_package_metadata_sqlite(package_name, version)
                    .await
            }
        }
    }

    #[cfg(feature = "postgres")]
    async fn get_package_metadata_postgres(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        use crate::database::schema::postgres::workflow_packages;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
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
            let metadata: PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((record.registry_id.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "sqlite")]
    async fn get_package_metadata_sqlite(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        use crate::database::schema::sqlite::workflow_packages;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
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
            let metadata: PackageMetadata =
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
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => self.get_package_metadata_by_id_postgres(package_id).await,
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => self.get_package_metadata_by_id_sqlite(package_id).await,
        }
    }

    #[cfg(feature = "postgres")]
    async fn get_package_metadata_by_id_postgres(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        use crate::database::schema::postgres::workflow_packages;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_record: Option<WorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::id.eq(&package_id))
                    .first::<WorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = package_record {
            let metadata: PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((record.registry_id.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "sqlite")]
    async fn get_package_metadata_by_id_sqlite(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        use crate::database::schema::sqlite::workflow_packages;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
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
            let metadata: PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((record.registry_id.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    /// List all packages in the registry.
    pub async fn list_all_packages(&self) -> Result<Vec<WorkflowPackage>, RegistryError> {
        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => self.list_all_packages_postgres().await,
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => self.list_all_packages_sqlite().await,
        }
    }

    #[cfg(feature = "postgres")]
    async fn list_all_packages_postgres(&self) -> Result<Vec<WorkflowPackage>, RegistryError> {
        use crate::database::schema::postgres::workflow_packages;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_records: Vec<WorkflowPackage> = conn
            .interact(move |conn| workflow_packages::table.load::<WorkflowPackage>(conn))
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(package_records)
    }

    #[cfg(feature = "sqlite")]
    async fn list_all_packages_sqlite(&self) -> Result<Vec<WorkflowPackage>, RegistryError> {
        use crate::database::schema::sqlite::workflow_packages;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
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
        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => {
                self.delete_package_metadata_postgres(package_name, version)
                    .await
            }
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => {
                self.delete_package_metadata_sqlite(package_name, version)
                    .await
            }
        }
    }

    #[cfg(feature = "postgres")]
    async fn delete_package_metadata_postgres(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        use crate::database::schema::postgres::workflow_packages;

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

    #[cfg(feature = "sqlite")]
    async fn delete_package_metadata_sqlite(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        use crate::database::schema::sqlite::workflow_packages;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
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
    pub async fn delete_package_metadata_by_id(&self, package_id: Uuid) -> Result<(), RegistryError> {
        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => self.delete_package_metadata_by_id_postgres(package_id).await,
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => self.delete_package_metadata_by_id_sqlite(package_id).await,
        }
    }

    #[cfg(feature = "postgres")]
    async fn delete_package_metadata_by_id_postgres(
        &self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        use crate::database::schema::postgres::workflow_packages;

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

    #[cfg(feature = "sqlite")]
    async fn delete_package_metadata_by_id_sqlite(
        &self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        use crate::database::schema::sqlite::workflow_packages;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
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
