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
use crate::models::workflow_packages::WorkflowPackage;
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
            BackendType::Postgres => {
                self.store_package_metadata_postgres(registry_id, package_metadata)
                    .await
            }
            BackendType::Sqlite => {
                self.store_package_metadata_sqlite(registry_id, package_metadata)
                    .await
            }
        }
    }

    async fn store_package_metadata_postgres(
        &self,
        registry_id: &str,
        package_metadata: &PackageMetadata,
    ) -> Result<Uuid, RegistryError> {
        use crate::dal::postgres_dal::models::{NewPgWorkflowPackage, PgWorkflowPackage};
        use crate::database::schema::postgres::workflow_packages;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let registry_uuid = Uuid::parse_str(registry_id).map_err(RegistryError::InvalidUuid)?;
        let metadata = serde_json::to_string(package_metadata).map_err(RegistryError::Serialization)?;

        let pg_new = NewPgWorkflowPackage {
            registry_id: registry_uuid,
            package_name: package_metadata.package_name.clone(),
            version: package_metadata.version.clone(),
            description: package_metadata.description.clone(),
            author: package_metadata.author.clone(),
            metadata,
        };

        let package_name_clone = package_metadata.package_name.clone();
        let version_clone = package_metadata.version.clone();

        let pg_package: PgWorkflowPackage = conn
            .interact(move |conn| {
                diesel::insert_into(workflow_packages::table)
                    .values(&pg_new)
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

        Ok(pg_package.id)
    }

    async fn store_package_metadata_sqlite(
        &self,
        registry_id: &str,
        package_metadata: &PackageMetadata,
    ) -> Result<Uuid, RegistryError> {
        use crate::dal::sqlite_dal::models::{
            current_timestamp_string, uuid_to_blob, NewSqliteWorkflowPackage, SqliteWorkflowPackage,
        };
        use crate::database::schema::sqlite::workflow_packages;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let registry_uuid = Uuid::parse_str(registry_id).map_err(RegistryError::InvalidUuid)?;
        let metadata = serde_json::to_string(package_metadata).map_err(RegistryError::Serialization)?;

        // For SQLite, generate UUID and timestamps client-side
        let id = UniversalUuid::new_v4();
        let id_blob = uuid_to_blob(&id.0);
        let now = current_timestamp_string();

        let sqlite_new = NewSqliteWorkflowPackage {
            id: id_blob.clone(),
            registry_id: uuid_to_blob(&registry_uuid),
            package_name: package_metadata.package_name.clone(),
            version: package_metadata.version.clone(),
            description: package_metadata.description.clone(),
            author: package_metadata.author.clone(),
            metadata,
            created_at: now.clone(),
            updated_at: now,
        };

        let package_name_clone = package_metadata.package_name.clone();
        let version_clone = package_metadata.version.clone();

        conn.interact(move |conn| {
            diesel::insert_into(workflow_packages::table)
                .values(&sqlite_new)
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

        Ok(id.0)
    }

    /// Retrieve package metadata from the database.
    pub async fn get_package_metadata(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.get_package_metadata_postgres(package_name, version)
                    .await
            }
            BackendType::Sqlite => {
                self.get_package_metadata_sqlite(package_name, version)
                    .await
            }
        }
    }

    async fn get_package_metadata_postgres(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        use crate::dal::postgres_dal::models::PgWorkflowPackage;
        use crate::database::schema::postgres::workflow_packages;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_name = package_name.to_string();
        let version = version.to_string();

        let pg_package: Option<PgWorkflowPackage> = conn
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

        if let Some(pg_record) = pg_package {
            let metadata: PackageMetadata =
                serde_json::from_str(&pg_record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((pg_record.registry_id.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    async fn get_package_metadata_sqlite(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        use crate::dal::sqlite_dal::models::{blob_to_uuid, SqliteWorkflowPackage};
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

        let sqlite_package: Option<SqliteWorkflowPackage> = conn
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

        if let Some(sqlite_record) = sqlite_package {
            let registry_uuid = blob_to_uuid(&sqlite_record.registry_id)
                .map_err(|e| RegistryError::Database(format!("Invalid registry UUID: {}", e)))?;
            let metadata: PackageMetadata =
                serde_json::from_str(&sqlite_record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((registry_uuid.to_string(), metadata)))
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
            BackendType::Postgres => self.get_package_metadata_by_id_postgres(package_id).await,
            BackendType::Sqlite => self.get_package_metadata_by_id_sqlite(package_id).await,
        }
    }

    async fn get_package_metadata_by_id_postgres(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        use crate::dal::postgres_dal::models::PgWorkflowPackage;
        use crate::database::schema::postgres::workflow_packages;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let pg_package: Option<PgWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::id.eq(&package_id))
                    .first::<PgWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(pg_record) = pg_package {
            let metadata: PackageMetadata =
                serde_json::from_str(&pg_record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((pg_record.registry_id.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    async fn get_package_metadata_by_id_sqlite(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        use crate::dal::sqlite_dal::models::{blob_to_uuid, uuid_to_blob, SqliteWorkflowPackage};
        use crate::database::schema::sqlite::workflow_packages;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_blob = uuid_to_blob(&package_id);

        let sqlite_package: Option<SqliteWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::id.eq(&package_blob))
                    .first::<SqliteWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(sqlite_record) = sqlite_package {
            let registry_uuid = blob_to_uuid(&sqlite_record.registry_id)
                .map_err(|e| RegistryError::Database(format!("Invalid registry UUID: {}", e)))?;
            let metadata: PackageMetadata =
                serde_json::from_str(&sqlite_record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((registry_uuid.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    /// List all packages in the registry.
    pub async fn list_all_packages(&self) -> Result<Vec<WorkflowPackage>, RegistryError> {
        match self.dal.backend() {
            BackendType::Postgres => self.list_all_packages_postgres().await,
            BackendType::Sqlite => self.list_all_packages_sqlite().await,
        }
    }

    async fn list_all_packages_postgres(&self) -> Result<Vec<WorkflowPackage>, RegistryError> {
        use crate::dal::postgres_dal::models::PgWorkflowPackage;
        use crate::database::schema::postgres::workflow_packages;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let pg_packages: Vec<PgWorkflowPackage> = conn
            .interact(move |conn| workflow_packages::table.load::<PgWorkflowPackage>(conn))
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(pg_packages.into_iter().map(Into::into).collect())
    }

    async fn list_all_packages_sqlite(&self) -> Result<Vec<WorkflowPackage>, RegistryError> {
        use crate::dal::sqlite_dal::models::SqliteWorkflowPackage;
        use crate::database::schema::sqlite::workflow_packages;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let sqlite_packages: Vec<SqliteWorkflowPackage> = conn
            .interact(move |conn| workflow_packages::table.load::<SqliteWorkflowPackage>(conn))
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(sqlite_packages.into_iter().map(Into::into).collect())
    }

    /// Delete package metadata from the database.
    pub async fn delete_package_metadata(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.delete_package_metadata_postgres(package_name, version)
                    .await
            }
            BackendType::Sqlite => {
                self.delete_package_metadata_sqlite(package_name, version)
                    .await
            }
        }
    }

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
            BackendType::Postgres => self.delete_package_metadata_by_id_postgres(package_id).await,
            BackendType::Sqlite => self.delete_package_metadata_by_id_sqlite(package_id).await,
        }
    }

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

    async fn delete_package_metadata_by_id_sqlite(
        &self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        use crate::dal::sqlite_dal::models::uuid_to_blob;
        use crate::database::schema::sqlite::workflow_packages;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_blob = uuid_to_blob(&package_id);

        conn.interact(move |conn| {
            diesel::delete(workflow_packages::table.filter(workflow_packages::id.eq(&package_blob)))
                .execute(conn)
        })
        .await
        .map_err(|e| RegistryError::Database(e.to_string()))?
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(())
    }
}
