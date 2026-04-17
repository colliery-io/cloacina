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

//! Database operations for workflow registry metadata storage.

use diesel::prelude::*;
use uuid::Uuid;

use super::WorkflowRegistryImpl;
use crate::registry::error::RegistryError;
use crate::registry::traits::RegistryStorage;
use crate::registry::types::WorkflowMetadata;

impl<S: RegistryStorage> WorkflowRegistryImpl<S> {
    /// Store package metadata in the database.
    pub(super) async fn store_package_metadata(
        &self,
        registry_id: &str,
        package_metadata: &crate::registry::loader::package_loader::PackageMetadata,
    ) -> Result<Uuid, RegistryError> {
        let registry_uuid = Uuid::parse_str(registry_id).map_err(RegistryError::InvalidUuid)?;
        let metadata =
            serde_json::to_string(package_metadata).map_err(RegistryError::Serialization)?;
        let storage_type = self.storage.storage_type();

        crate::dispatch_backend!(
            self.database.backend(),
            self.store_package_metadata_postgres(
                registry_uuid,
                package_metadata,
                metadata,
                storage_type,
            )
            .await,
            self.store_package_metadata_sqlite(
                registry_uuid,
                package_metadata,
                metadata,
                storage_type,
            )
            .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn store_package_metadata_postgres(
        &self,
        registry_uuid: Uuid,
        package_metadata: &crate::registry::loader::package_loader::PackageMetadata,
        metadata: String,
        storage_type: crate::models::workflow_packages::StorageType,
    ) -> Result<Uuid, RegistryError> {
        use crate::dal::unified::models::NewUnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};

        let conn = self
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();
        let new_package = NewUnifiedWorkflowPackage {
            id,
            registry_id: UniversalUuid(registry_uuid),
            package_name: package_metadata.package_name.clone(),
            version: package_metadata.version.clone(),
            description: package_metadata.description.clone(),
            author: package_metadata.author.clone(),
            metadata,
            storage_type: storage_type.as_str().to_string(),
            created_at: now,
            updated_at: now,
            tenant_id: None,
            content_hash: String::new(),
            superseded: crate::database::universal_types::UniversalBool(false),
        };

        let package_name_for_error = package_metadata.package_name.clone();
        let version_for_error = package_metadata.version.clone();

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
                package_name: package_name_for_error.clone(),
                version: version_for_error.clone(),
            },
            _ => RegistryError::Database(format!("Database error: {}", e)),
        })?;

        Ok(id.0)
    }

    #[cfg(feature = "sqlite")]
    async fn store_package_metadata_sqlite(
        &self,
        registry_uuid: Uuid,
        package_metadata: &crate::registry::loader::package_loader::PackageMetadata,
        metadata: String,
        storage_type: crate::models::workflow_packages::StorageType,
    ) -> Result<Uuid, RegistryError> {
        use crate::dal::unified::models::NewUnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};

        let conn = self
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_package = NewUnifiedWorkflowPackage {
            id,
            registry_id: UniversalUuid(registry_uuid),
            package_name: package_metadata.package_name.clone(),
            version: package_metadata.version.clone(),
            description: package_metadata.description.clone(),
            author: package_metadata.author.clone(),
            metadata,
            storage_type: storage_type.as_str().to_string(),
            created_at: now,
            updated_at: now,
            tenant_id: None,
            content_hash: String::new(),
            superseded: crate::database::universal_types::UniversalBool(false),
        };

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

        Ok(id.0)
    }

    /// Retrieve package metadata from the database.
    pub(super) async fn get_package_metadata(
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
        crate::dispatch_backend!(
            self.database.backend(),
            self.get_package_metadata_postgres(package_name, version)
                .await,
            self.get_package_metadata_sqlite(package_name, version)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_package_metadata_postgres(
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
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;

        let conn = self
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_name = package_name.to_string();
        let version = version.to_string();

        let package_record: Option<UnifiedWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::package_name.eq(&package_name))
                    .filter(workflow_packages::version.eq(&version))
                    .filter(
                        workflow_packages::superseded
                            .eq(crate::database::universal_types::UniversalBool(false)),
                    )
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = package_record {
            let metadata: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((record.registry_id.0.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "sqlite")]
    async fn get_package_metadata_sqlite(
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
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;

        let conn = self
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_name = package_name.to_string();
        let version = version.to_string();

        let package_record: Option<UnifiedWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::package_name.eq(&package_name))
                    .filter(workflow_packages::version.eq(&version))
                    .filter(
                        workflow_packages::superseded
                            .eq(crate::database::universal_types::UniversalBool(false)),
                    )
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = package_record {
            let metadata: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((record.registry_id.0.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    /// List all packages in the registry.
    pub(super) async fn list_all_packages(&self) -> Result<Vec<WorkflowMetadata>, RegistryError> {
        crate::dispatch_backend!(
            self.database.backend(),
            self.list_all_packages_postgres().await,
            self.list_all_packages_sqlite().await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_all_packages_postgres(&self) -> Result<Vec<WorkflowMetadata>, RegistryError> {
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;

        let conn = self
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_records: Vec<UnifiedWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(
                        workflow_packages::superseded
                            .eq(crate::database::universal_types::UniversalBool(false)),
                    )
                    .load::<UnifiedWorkflowPackage>(conn)
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        let mut workflows = Vec::new();
        for record in package_records {
            let package_metadata: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;

            workflows.push(WorkflowMetadata {
                id: record.id.0,
                registry_id: record.registry_id.0,
                package_name: record.package_name,
                version: record.version,
                description: record.description,
                author: record.author,
                tasks: package_metadata
                    .tasks
                    .iter()
                    .map(|t| t.local_id.clone())
                    .collect(),
                schedules: Vec::new(),
                created_at: record.created_at.0,
                updated_at: record.updated_at.0,
            });
        }

        Ok(workflows)
    }

    #[cfg(feature = "sqlite")]
    async fn list_all_packages_sqlite(&self) -> Result<Vec<WorkflowMetadata>, RegistryError> {
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;

        let conn = self
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_records: Vec<UnifiedWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(
                        workflow_packages::superseded
                            .eq(crate::database::universal_types::UniversalBool(false)),
                    )
                    .load::<UnifiedWorkflowPackage>(conn)
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        let mut workflows = Vec::new();
        for record in package_records {
            let package_metadata: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;

            workflows.push(WorkflowMetadata {
                id: record.id.0,
                registry_id: record.registry_id.0,
                package_name: record.package_name,
                version: record.version,
                description: record.description,
                author: record.author,
                tasks: package_metadata
                    .tasks
                    .iter()
                    .map(|t| t.local_id.clone())
                    .collect(),
                schedules: Vec::new(),
                created_at: record.created_at.0,
                updated_at: record.updated_at.0,
            });
        }

        Ok(workflows)
    }

    /// Delete package metadata from the database.
    pub(super) async fn delete_package_metadata(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        crate::dispatch_backend!(
            self.database.backend(),
            self.delete_package_metadata_postgres(package_name, version)
                .await,
            self.delete_package_metadata_sqlite(package_name, version)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn delete_package_metadata_postgres(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        use crate::database::schema::unified::workflow_packages;

        let conn = self
            .database
            .get_postgres_connection()
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
        use crate::database::schema::unified::workflow_packages;

        let conn = self
            .database
            .get_sqlite_connection()
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

    /// Get package metadata by ID.
    pub(super) async fn get_package_metadata_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(String, WorkflowMetadata)>, RegistryError> {
        crate::dispatch_backend!(
            self.database.backend(),
            self.get_package_metadata_by_id_postgres(package_id).await,
            self.get_package_metadata_by_id_sqlite(package_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_package_metadata_by_id_postgres(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(String, WorkflowMetadata)>, RegistryError> {
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        let conn = self
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let pkg_id = UniversalUuid(package_id);
        let package_record: Option<UnifiedWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::id.eq(pkg_id))
                    .filter(
                        workflow_packages::superseded
                            .eq(crate::database::universal_types::UniversalBool(false)),
                    )
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = package_record {
            let package_metadata: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;

            let workflow_metadata = WorkflowMetadata {
                id: record.id.0,
                registry_id: record.registry_id.0,
                package_name: record.package_name,
                version: record.version,
                description: record.description,
                author: record.author,
                tasks: package_metadata
                    .tasks
                    .iter()
                    .map(|t| t.local_id.clone())
                    .collect(),
                schedules: Vec::new(),
                created_at: record.created_at.0,
                updated_at: record.updated_at.0,
            };

            Ok(Some((record.registry_id.0.to_string(), workflow_metadata)))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "sqlite")]
    async fn get_package_metadata_by_id_sqlite(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(String, WorkflowMetadata)>, RegistryError> {
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        let conn = self
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let pkg_id = UniversalUuid(package_id);

        let package_record: Option<UnifiedWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::id.eq(pkg_id))
                    .filter(
                        workflow_packages::superseded
                            .eq(crate::database::universal_types::UniversalBool(false)),
                    )
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = package_record {
            let package_metadata: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;

            let workflow_metadata = WorkflowMetadata {
                id: record.id.0,
                registry_id: record.registry_id.0,
                package_name: record.package_name,
                version: record.version,
                description: record.description,
                author: record.author,
                tasks: package_metadata
                    .tasks
                    .iter()
                    .map(|t| t.local_id.clone())
                    .collect(),
                schedules: Vec::new(),
                created_at: record.created_at.0,
                updated_at: record.updated_at.0,
            };

            Ok(Some((record.registry_id.0.to_string(), workflow_metadata)))
        } else {
            Ok(None)
        }
    }

    /// Look up the active package row for `name`, returning (id, registry_id, content_hash).
    ///
    /// Returns `Ok(None)` if no active row exists. Superseded rows are ignored.
    pub(super) async fn get_active_package_by_name(
        &self,
        package_name: &str,
    ) -> Result<Option<(Uuid, String, String)>, RegistryError> {
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalBool;

        crate::dispatch_backend!(
            self.database.backend(),
            {
                let conn = self
                    .database
                    .get_postgres_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                let name = package_name.to_string();
                let record: Option<UnifiedWorkflowPackage> = conn
                    .interact(move |conn| {
                        workflow_packages::table
                            .filter(workflow_packages::package_name.eq(&name))
                            .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                            .first::<UnifiedWorkflowPackage>(conn)
                            .optional()
                    })
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?
                    .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
                Ok(record.map(|r| (r.id.0, r.registry_id.0.to_string(), r.content_hash)))
            },
            {
                let conn = self
                    .database
                    .get_sqlite_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                let name = package_name.to_string();
                let record: Option<UnifiedWorkflowPackage> = conn
                    .interact(move |conn| {
                        workflow_packages::table
                            .filter(workflow_packages::package_name.eq(&name))
                            .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                            .first::<UnifiedWorkflowPackage>(conn)
                            .optional()
                    })
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?
                    .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
                Ok(record.map(|r| (r.id.0, r.registry_id.0.to_string(), r.content_hash)))
            }
        )
    }

    /// Supersede the current active row for `old_id` (if provided) and insert a new
    /// active row in the same transaction. Returns the new package UUID.
    ///
    /// Used by the upload handler to atomically replace a package version: the old
    /// row is flagged `superseded = true` and a fresh row with the new content is
    /// inserted. The partial unique index `(package_name) WHERE NOT superseded`
    /// guarantees at most one active row per name even under concurrent uploads.
    pub(super) async fn supersede_and_insert(
        &self,
        old_id: Option<Uuid>,
        registry_id: &str,
        package_metadata: &crate::registry::loader::package_loader::PackageMetadata,
        content_hash: &str,
    ) -> Result<Uuid, RegistryError> {
        use crate::dal::unified::models::NewUnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};

        let registry_uuid = Uuid::parse_str(registry_id).map_err(RegistryError::InvalidUuid)?;
        let metadata =
            serde_json::to_string(package_metadata).map_err(RegistryError::Serialization)?;
        let storage_type = self.storage.storage_type();
        let new_id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_row = NewUnifiedWorkflowPackage {
            id: new_id,
            registry_id: UniversalUuid(registry_uuid),
            package_name: package_metadata.package_name.clone(),
            version: package_metadata.version.clone(),
            description: package_metadata.description.clone(),
            author: package_metadata.author.clone(),
            metadata,
            storage_type: storage_type.as_str().to_string(),
            created_at: now,
            updated_at: now,
            tenant_id: None,
            content_hash: content_hash.to_string(),
            superseded: UniversalBool(false),
        };

        let pkg_name = package_metadata.package_name.clone();
        let pkg_version = package_metadata.version.clone();
        let old_uuid = old_id.map(UniversalUuid);

        crate::dispatch_backend!(
            self.database.backend(),
            {
                let conn = self
                    .database
                    .get_postgres_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                conn.interact(move |conn| {
                    conn.transaction::<_, diesel::result::Error, _>(|tx| {
                        if let Some(id) = old_uuid {
                            diesel::update(
                                workflow_packages::table.filter(workflow_packages::id.eq(id)),
                            )
                            .set(workflow_packages::superseded.eq(UniversalBool(true)))
                            .execute(tx)?;
                        }
                        diesel::insert_into(workflow_packages::table)
                            .values(&new_row)
                            .execute(tx)?;
                        Ok(new_id.0)
                    })
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| match e {
                    diesel::result::Error::DatabaseError(
                        diesel::result::DatabaseErrorKind::UniqueViolation,
                        _,
                    ) => RegistryError::PackageExists {
                        package_name: pkg_name.clone(),
                        version: pkg_version.clone(),
                    },
                    _ => RegistryError::Database(format!("Database error: {}", e)),
                })
            },
            {
                let conn = self
                    .database
                    .get_sqlite_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                conn.interact(move |conn| {
                    conn.transaction::<_, diesel::result::Error, _>(|tx| {
                        if let Some(id) = old_uuid {
                            diesel::update(
                                workflow_packages::table.filter(workflow_packages::id.eq(id)),
                            )
                            .set(workflow_packages::superseded.eq(UniversalBool(true)))
                            .execute(tx)?;
                        }
                        diesel::insert_into(workflow_packages::table)
                            .values(&new_row)
                            .execute(tx)?;
                        Ok(new_id.0)
                    })
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| match e {
                    diesel::result::Error::DatabaseError(
                        diesel::result::DatabaseErrorKind::UniqueViolation,
                        _,
                    ) => RegistryError::PackageExists {
                        package_name: pkg_name.clone(),
                        version: pkg_version.clone(),
                    },
                    _ => RegistryError::Database(format!("Database error: {}", e)),
                })
            }
        )
    }

    /// Delete package metadata by ID.
    pub(super) async fn delete_package_metadata_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        crate::dispatch_backend!(
            self.database.backend(),
            self.delete_package_metadata_by_id_postgres(package_id)
                .await,
            self.delete_package_metadata_by_id_sqlite(package_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn delete_package_metadata_by_id_postgres(
        &self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        let conn = self
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let pkg_id = UniversalUuid(package_id);
        conn.interact(move |conn| {
            diesel::delete(workflow_packages::table.filter(workflow_packages::id.eq(pkg_id)))
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
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        let conn = self
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let pkg_id = UniversalUuid(package_id);

        conn.interact(move |conn| {
            diesel::delete(workflow_packages::table.filter(workflow_packages::id.eq(pkg_id)))
                .execute(conn)
        })
        .await
        .map_err(|e| RegistryError::Database(e.to_string()))?
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dal::unified::workflow_registry_storage::UnifiedRegistryStorage;
    use crate::database::Database;
    use crate::registry::loader::package_loader::{PackageMetadata, TaskMetadata};

    #[cfg(feature = "sqlite")]
    async fn create_test_registry() -> WorkflowRegistryImpl<UnifiedRegistryStorage> {
        let url = format!(
            "file:wfreg_test_{}?mode=memory&cache=shared",
            Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        let storage = UnifiedRegistryStorage::new(db.clone());
        WorkflowRegistryImpl::new(storage, db).unwrap()
    }

    #[cfg(feature = "sqlite")]
    fn sample_metadata(name: &str, version: &str) -> PackageMetadata {
        PackageMetadata {
            package_name: name.to_string(),
            version: version.to_string(),
            description: Some("A test package".to_string()),
            author: Some("test-author".to_string()),
            tasks: vec![TaskMetadata {
                index: 0,
                local_id: "my_task".to_string(),
                namespaced_id_template: "{tenant}.{package}.{workflow}.my_task".to_string(),
                dependencies: vec![],
                description: "a task".to_string(),
                source_location: "lib.rs:1".to_string(),
            }],
            graph_data: None,
            architecture: "x86_64".to_string(),
            symbols: vec![],
        }
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_store_and_get_package_metadata() {
        let registry = create_test_registry().await;
        let registry_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("reg-pkg", "1.0.0");

        let pkg_id = registry
            .store_package_metadata(&registry_id, &meta)
            .await
            .unwrap();
        assert_ne!(pkg_id, Uuid::nil());

        let result = registry
            .get_package_metadata("reg-pkg", "1.0.0")
            .await
            .unwrap();
        assert!(result.is_some());
        let (reg_id, retrieved) = result.unwrap();
        assert_eq!(reg_id, registry_id);
        assert_eq!(retrieved.package_name, "reg-pkg");
        assert_eq!(retrieved.version, "1.0.0");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_package_metadata_not_found() {
        let registry = create_test_registry().await;

        let result = registry
            .get_package_metadata("nonexistent", "0.0.0")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_all_packages() {
        let registry = create_test_registry().await;

        // Initially empty
        let list = registry.list_all_packages().await.unwrap();
        assert!(list.is_empty());

        // Store two packages
        let reg_id = Uuid::new_v4().to_string();
        let meta1 = sample_metadata("list-a", "1.0.0");
        let meta2 = sample_metadata("list-b", "2.0.0");
        registry
            .store_package_metadata(&reg_id, &meta1)
            .await
            .unwrap();
        registry
            .store_package_metadata(&reg_id, &meta2)
            .await
            .unwrap();

        let list = registry.list_all_packages().await.unwrap();
        assert_eq!(list.len(), 2);

        let names: Vec<&str> = list.iter().map(|w| w.package_name.as_str()).collect();
        assert!(names.contains(&"list-a"));
        assert!(names.contains(&"list-b"));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_package_metadata() {
        let registry = create_test_registry().await;
        let reg_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("del-pkg", "1.0.0");

        registry
            .store_package_metadata(&reg_id, &meta)
            .await
            .unwrap();

        // Confirm exists
        assert!(registry
            .get_package_metadata("del-pkg", "1.0.0")
            .await
            .unwrap()
            .is_some());

        // Delete
        registry
            .delete_package_metadata("del-pkg", "1.0.0")
            .await
            .unwrap();

        // Confirm gone
        assert!(registry
            .get_package_metadata("del-pkg", "1.0.0")
            .await
            .unwrap()
            .is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_package_metadata_by_id() {
        let registry = create_test_registry().await;
        let reg_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("by-id-pkg", "3.0.0");

        let pkg_id = registry
            .store_package_metadata(&reg_id, &meta)
            .await
            .unwrap();

        let result = registry.get_package_metadata_by_id(pkg_id).await.unwrap();
        assert!(result.is_some());
        let (_, wf_meta) = result.unwrap();
        assert_eq!(wf_meta.package_name, "by-id-pkg");
        assert_eq!(wf_meta.version, "3.0.0");
        assert!(wf_meta.tasks.contains(&"my_task".to_string()));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_package_metadata_by_id_not_found() {
        let registry = create_test_registry().await;

        let result = registry
            .get_package_metadata_by_id(Uuid::new_v4())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_package_metadata_by_id() {
        let registry = create_test_registry().await;
        let reg_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("del-id-pkg", "1.0.0");

        let pkg_id = registry
            .store_package_metadata(&reg_id, &meta)
            .await
            .unwrap();

        registry
            .delete_package_metadata_by_id(pkg_id)
            .await
            .unwrap();

        assert!(registry
            .get_package_metadata_by_id(pkg_id)
            .await
            .unwrap()
            .is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_nonexistent_does_not_error() {
        let registry = create_test_registry().await;

        registry
            .delete_package_metadata("nonexistent", "0.0.0")
            .await
            .unwrap();

        registry
            .delete_package_metadata_by_id(Uuid::new_v4())
            .await
            .unwrap();
    }

    // ========================================================================
    // Package lifecycle tests (T-0497)
    // ========================================================================

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_supersede_and_insert_fresh_name() {
        let registry = create_test_registry().await;
        let reg_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("pkg-a", "1");

        let pkg_id = registry
            .supersede_and_insert(None, &reg_id, &meta, "hash-v1")
            .await
            .unwrap();

        let active = registry
            .get_active_package_by_name("pkg-a")
            .await
            .unwrap()
            .expect("should have active row");
        assert_eq!(active.0, pkg_id);
        assert_eq!(active.2, "hash-v1");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_supersede_and_insert_replaces_old_active() {
        let registry = create_test_registry().await;
        let reg_id_1 = Uuid::new_v4().to_string();
        let reg_id_2 = Uuid::new_v4().to_string();
        let meta_v1 = sample_metadata("pkg-b", "1");
        let meta_v2 = sample_metadata("pkg-b", "2");

        let id_v1 = registry
            .supersede_and_insert(None, &reg_id_1, &meta_v1, "hash-v1")
            .await
            .unwrap();

        let id_v2 = registry
            .supersede_and_insert(Some(id_v1), &reg_id_2, &meta_v2, "hash-v2")
            .await
            .unwrap();
        assert_ne!(id_v1, id_v2);

        // Exactly one active row, and it's the new one.
        let active = registry
            .get_active_package_by_name("pkg-b")
            .await
            .unwrap()
            .expect("should have active row");
        assert_eq!(active.0, id_v2);
        assert_eq!(active.2, "hash-v2");

        // Old row is no longer visible through filtered reads.
        assert!(registry
            .get_package_metadata("pkg-b", "1")
            .await
            .unwrap()
            .is_none());
        assert!(registry
            .get_package_metadata_by_id(id_v1)
            .await
            .unwrap()
            .is_none());

        // New row is visible.
        assert!(registry
            .get_package_metadata("pkg-b", "2")
            .await
            .unwrap()
            .is_some());

        // list_all_packages returns only the active row.
        let list = registry.list_all_packages().await.unwrap();
        let names: Vec<_> = list.iter().filter(|w| w.package_name == "pkg-b").collect();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].id, id_v2);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_partial_unique_rejects_second_active_for_same_name() {
        let registry = create_test_registry().await;
        let reg_id_1 = Uuid::new_v4().to_string();
        let reg_id_2 = Uuid::new_v4().to_string();
        let meta_v1 = sample_metadata("pkg-c", "1");
        let meta_v2 = sample_metadata("pkg-c", "2");

        registry
            .supersede_and_insert(None, &reg_id_1, &meta_v1, "hash-v1")
            .await
            .unwrap();

        // Second insert without superseding the first must fail due to the
        // partial unique index `(package_name) WHERE NOT superseded`.
        let err = registry
            .supersede_and_insert(None, &reg_id_2, &meta_v2, "hash-v2")
            .await
            .expect_err("second active insert should fail");
        assert!(
            matches!(err, RegistryError::PackageExists { .. }),
            "expected PackageExists, got {:?}",
            err
        );
    }
}
