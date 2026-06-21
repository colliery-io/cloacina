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
use crate::registry::types::{WorkflowMetadata, WorkflowTaskNode};

/// Result of inspecting a package — full metadata plus the raw build state.
#[derive(Debug, Clone)]
pub struct InspectedPackage {
    pub metadata: WorkflowMetadata,
    pub build_status: String,
    pub build_error: Option<String>,
}

/// Build the task dependency graph (one node per task, with its upstream
/// dependencies) from the persisted package metadata's task list, so the API
/// and UI can render the full DAG. (CLOACI-T-0663)
pub(super) fn build_task_graph(
    package_metadata: &crate::registry::loader::package_loader::PackageMetadata,
) -> Vec<WorkflowTaskNode> {
    package_metadata
        .tasks
        .iter()
        .map(|t| WorkflowTaskNode {
            id: t.local_id.clone(),
            dependencies: t.dependencies.clone(),
            description: if t.description.trim().is_empty() {
                None
            } else {
                Some(t.description.clone())
            },
            doc_what: t.doc_what.clone(),
            doc_why: t.doc_why.clone(),
        })
        .collect()
}

impl<S: RegistryStorage> WorkflowRegistryImpl<S> {
    /// Store package metadata in the database. Test-only helper; the
    /// production write path is `supersede_and_insert_with_prebuilt`,
    /// which deals with the active/superseded transition correctly.
    #[cfg(test)]
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

    #[cfg(all(feature = "postgres", test))]
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
            compiled_data: None,
            build_status: "pending".to_string(),
            build_error: None,
            build_claimed_at: None,
            compiled_at: None,
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

    #[cfg(all(feature = "sqlite", test))]
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
            compiled_data: None,
            build_status: "pending".to_string(),
            build_error: None,
            build_claimed_at: None,
            compiled_at: None,
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

    /// Retrieve package metadata + compiled artifact for a successfully-built package.
    ///
    /// Filters to `superseded = false AND build_status = 'success'` so pending /
    /// building / failed rows are invisible to the reconciler. Returns the
    /// registry_id (source archive key), decoded package metadata, and the
    /// compiled cdylib bytes (None for pure-Python packages).
    pub(super) async fn get_package_metadata(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<
        Option<(
            String,
            crate::registry::loader::package_loader::PackageMetadata,
            Option<Vec<u8>>,
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
            Option<Vec<u8>>,
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
                    .filter(workflow_packages::build_status.eq("success"))
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = package_record {
            let metadata: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            let compiled = record.compiled_data.map(|b| b.into_inner());
            Ok(Some((record.registry_id.0.to_string(), metadata, compiled)))
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
            Option<Vec<u8>>,
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
                    .filter(workflow_packages::build_status.eq("success"))
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = package_record {
            let metadata: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            let compiled = record.compiled_data.map(|b| b.into_inner());
            Ok(Some((record.registry_id.0.to_string(), metadata, compiled)))
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
                    .filter(workflow_packages::build_status.eq("success"))
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
                workflow_name: if package_metadata.workflow_name.is_empty() {
                    record.package_name.clone()
                } else {
                    package_metadata.workflow_name.clone()
                },
                package_name: record.package_name,
                version: record.version,
                description: record.description,
                author: record.author,
                tasks: package_metadata
                    .tasks
                    .iter()
                    .map(|t| t.local_id.clone())
                    .collect(),
                task_graph: build_task_graph(&package_metadata),
                schedules: Vec::new(),
                created_at: record.created_at.0,
                updated_at: record.updated_at.0,
                paused: record.paused.is_true(),
                declared_params: package_metadata.declared_params.clone(),
                declared_surfaces: package_metadata.declared_surfaces.clone(),
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
                    .filter(workflow_packages::build_status.eq("success"))
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
                workflow_name: if package_metadata.workflow_name.is_empty() {
                    record.package_name.clone()
                } else {
                    package_metadata.workflow_name.clone()
                },
                package_name: record.package_name,
                version: record.version,
                description: record.description,
                author: record.author,
                tasks: package_metadata
                    .tasks
                    .iter()
                    .map(|t| t.local_id.clone())
                    .collect(),
                task_graph: build_task_graph(&package_metadata),
                schedules: Vec::new(),
                created_at: record.created_at.0,
                updated_at: record.updated_at.0,
                paused: record.paused.is_true(),
                declared_params: package_metadata.declared_params.clone(),
                declared_surfaces: package_metadata.declared_surfaces.clone(),
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

    /// Get package metadata + compiled artifact by ID for a successfully-built package.
    ///
    /// Filters to `superseded = false AND build_status = 'success'`. Returns the
    /// registry_id (source archive key), decoded metadata, and compiled cdylib
    /// bytes (None for pure-Python packages).
    pub(super) async fn get_package_metadata_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(String, WorkflowMetadata, Option<Vec<u8>>)>, RegistryError> {
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
    ) -> Result<Option<(String, WorkflowMetadata, Option<Vec<u8>>)>, RegistryError> {
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
                    .filter(workflow_packages::build_status.eq("success"))
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = package_record {
            let package_metadata: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;

            let compiled = record.compiled_data.map(|b| b.into_inner());
            let workflow_metadata = WorkflowMetadata {
                id: record.id.0,
                registry_id: record.registry_id.0,
                workflow_name: if package_metadata.workflow_name.is_empty() {
                    record.package_name.clone()
                } else {
                    package_metadata.workflow_name.clone()
                },
                package_name: record.package_name,
                version: record.version,
                description: record.description,
                author: record.author,
                tasks: package_metadata
                    .tasks
                    .iter()
                    .map(|t| t.local_id.clone())
                    .collect(),
                task_graph: build_task_graph(&package_metadata),
                schedules: Vec::new(),
                created_at: record.created_at.0,
                updated_at: record.updated_at.0,
                paused: record.paused.is_true(),
                declared_params: package_metadata.declared_params.clone(),
                declared_surfaces: package_metadata.declared_surfaces.clone(),
            };

            Ok(Some((
                record.registry_id.0.to_string(),
                workflow_metadata,
                compiled,
            )))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "sqlite")]
    async fn get_package_metadata_by_id_sqlite(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(String, WorkflowMetadata, Option<Vec<u8>>)>, RegistryError> {
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
                    .filter(workflow_packages::build_status.eq("success"))
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = package_record {
            let package_metadata: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;

            let compiled = record.compiled_data.map(|b| b.into_inner());
            let workflow_metadata = WorkflowMetadata {
                id: record.id.0,
                registry_id: record.registry_id.0,
                workflow_name: if package_metadata.workflow_name.is_empty() {
                    record.package_name.clone()
                } else {
                    package_metadata.workflow_name.clone()
                },
                package_name: record.package_name,
                version: record.version,
                description: record.description,
                author: record.author,
                tasks: package_metadata
                    .tasks
                    .iter()
                    .map(|t| t.local_id.clone())
                    .collect(),
                task_graph: build_task_graph(&package_metadata),
                schedules: Vec::new(),
                created_at: record.created_at.0,
                updated_at: record.updated_at.0,
                paused: record.paused.is_true(),
                declared_params: package_metadata.declared_params.clone(),
                declared_surfaces: package_metadata.declared_surfaces.clone(),
            };

            Ok(Some((
                record.registry_id.0.to_string(),
                workflow_metadata,
                compiled,
            )))
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
    /// Used by tests to exercise the supersede-and-insert flow without the
    /// prebuilt-artifact path. Production code calls
    /// `supersede_and_insert_with_prebuilt` directly with a `None` artifact.
    /// The partial unique index `(package_name) WHERE NOT superseded`
    /// guarantees at most one active row per name even under concurrent
    /// uploads.
    #[cfg(test)]
    pub(super) async fn supersede_and_insert(
        &self,
        old_id: Option<Uuid>,
        registry_id: &str,
        package_metadata: &crate::registry::loader::package_loader::PackageMetadata,
        content_hash: &str,
    ) -> Result<Uuid, RegistryError> {
        self.supersede_and_insert_with_prebuilt(
            old_id,
            registry_id,
            package_metadata,
            content_hash,
            None,
        )
        .await
    }

    /// Same as `supersede_and_insert` but optionally pre-populates
    /// `compiled_data` + `build_status = 'success'` for the new row. Used by
    /// the content-hash artifact reuse path (T-0523): when a prior row with
    /// the same content_hash already has a compiled artifact, the new row
    /// skips the build queue.
    pub(super) async fn supersede_and_insert_with_prebuilt(
        &self,
        old_id: Option<Uuid>,
        registry_id: &str,
        package_metadata: &crate::registry::loader::package_loader::PackageMetadata,
        content_hash: &str,
        prebuilt: Option<Vec<u8>>,
    ) -> Result<Uuid, RegistryError> {
        use crate::dal::unified::models::NewUnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{
            UniversalBinary, UniversalBool, UniversalTimestamp, UniversalUuid,
        };

        let registry_uuid = Uuid::parse_str(registry_id).map_err(RegistryError::InvalidUuid)?;
        let metadata =
            serde_json::to_string(package_metadata).map_err(RegistryError::Serialization)?;
        let storage_type = self.storage.storage_type();
        let new_id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let (build_status, compiled_data, compiled_at) = match prebuilt {
            Some(bytes) => (
                "success".to_string(),
                Some(UniversalBinary::new(bytes)),
                Some(now),
            ),
            None => ("pending".to_string(), None, None),
        };

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
            compiled_data,
            build_status,
            build_error: None,
            build_claimed_at: None,
            compiled_at,
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

    /// Inspect a package by ID — returns metadata plus `build_status` /
    /// `build_error` regardless of build outcome. Unlike `get_package_metadata_by_id`
    /// this does not filter by `build_status = 'success'`, so operators can
    /// surface pending / building / failed rows through `package inspect`.
    ///
    /// Only `superseded = false` rows are returned — superseded history is
    /// out of scope here.
    pub async fn inspect_package_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<Option<InspectedPackage>, RegistryError> {
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalBool, UniversalUuid};

        let pkg_id = UniversalUuid(package_id);

        let record: Option<UnifiedWorkflowPackage> = crate::dispatch_backend!(
            self.database.backend(),
            {
                let conn = self
                    .database
                    .get_postgres_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                conn.interact(move |conn| {
                    workflow_packages::table
                        .filter(workflow_packages::id.eq(pkg_id))
                        .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                        .first::<UnifiedWorkflowPackage>(conn)
                        .optional()
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
            },
            {
                let conn = self
                    .database
                    .get_sqlite_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                conn.interact(move |conn| {
                    workflow_packages::table
                        .filter(workflow_packages::id.eq(pkg_id))
                        .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                        .first::<UnifiedWorkflowPackage>(conn)
                        .optional()
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
            }
        );

        let Some(record) = record else {
            return Ok(None);
        };

        let package_metadata: crate::registry::loader::package_loader::PackageMetadata =
            serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;

        Ok(Some(InspectedPackage {
            metadata: WorkflowMetadata {
                id: record.id.0,
                registry_id: record.registry_id.0,
                workflow_name: if package_metadata.workflow_name.is_empty() {
                    record.package_name.clone()
                } else {
                    package_metadata.workflow_name.clone()
                },
                package_name: record.package_name,
                version: record.version,
                description: record.description,
                author: record.author,
                tasks: package_metadata
                    .tasks
                    .iter()
                    .map(|t| t.local_id.clone())
                    .collect(),
                task_graph: build_task_graph(&package_metadata),
                schedules: Vec::new(),
                created_at: record.created_at.0,
                updated_at: record.updated_at.0,
                paused: record.paused.is_true(),
                declared_params: package_metadata.declared_params.clone(),
                declared_surfaces: package_metadata.declared_surfaces.clone(),
            },
            build_status: record.build_status,
            build_error: record.build_error,
        }))
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

    // ========================================================================
    // Build queue (CLOACI-I-0097)
    // ========================================================================

    /// A pending build claimed by the compiler. Contains everything the
    /// compiler needs to fetch the source and produce a cdylib.
    pub async fn claim_next_build(&self) -> Result<Option<ClaimedBuild>, RegistryError> {
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalBool, UniversalTimestamp};

        let now = UniversalTimestamp::now();
        crate::dispatch_backend!(
            self.database.backend(),
            {
                let conn = self
                    .database
                    .get_postgres_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                let claimed: Option<UnifiedWorkflowPackage> = conn
                    .interact(move |conn| {
                        conn.transaction::<_, diesel::result::Error, _>(|tx| {
                            use diesel::prelude::*;
                            // FOR UPDATE SKIP LOCKED so concurrent compiler
                            // instances don't block on each other's claims.
                            let candidate: Option<UnifiedWorkflowPackage> =
                                workflow_packages::table
                                    .filter(workflow_packages::build_status.eq("pending"))
                                    .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                                    .order(workflow_packages::created_at.asc())
                                    .limit(1)
                                    .for_update()
                                    .skip_locked()
                                    .first::<UnifiedWorkflowPackage>(tx)
                                    .optional()?;
                            let Some(mut row) = candidate else {
                                return Ok(None);
                            };
                            diesel::update(
                                workflow_packages::table.filter(workflow_packages::id.eq(row.id)),
                            )
                            .set((
                                workflow_packages::build_status.eq("building"),
                                workflow_packages::build_claimed_at.eq(Some(now)),
                                workflow_packages::build_error.eq::<Option<String>>(None),
                            ))
                            .execute(tx)?;
                            row.build_status = "building".to_string();
                            row.build_claimed_at = Some(now);
                            row.build_error = None;
                            Ok(Some(row))
                        })
                    })
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?
                    .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
                Ok(claimed.map(Into::into))
            },
            {
                let conn = self
                    .database
                    .get_sqlite_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                let claimed: Option<UnifiedWorkflowPackage> = conn
                    .interact(move |conn| {
                        conn.transaction::<_, diesel::result::Error, _>(|tx| {
                            use diesel::prelude::*;
                            let candidate: Option<UnifiedWorkflowPackage> =
                                workflow_packages::table
                                    .filter(workflow_packages::build_status.eq("pending"))
                                    .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                                    .order(workflow_packages::created_at.asc())
                                    .limit(1)
                                    .first::<UnifiedWorkflowPackage>(tx)
                                    .optional()?;
                            let Some(mut row) = candidate else {
                                return Ok(None);
                            };
                            diesel::update(
                                workflow_packages::table.filter(workflow_packages::id.eq(row.id)),
                            )
                            .set((
                                workflow_packages::build_status.eq("building"),
                                workflow_packages::build_claimed_at.eq(Some(now)),
                                workflow_packages::build_error.eq::<Option<String>>(None),
                            ))
                            .execute(tx)?;
                            row.build_status = "building".to_string();
                            row.build_claimed_at = Some(now);
                            row.build_error = None;
                            Ok(Some(row))
                        })
                    })
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?
                    .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
                Ok(claimed.map(Into::into))
            }
        )
    }

    /// Record a successful build. Writes the compiled bytes and transitions
    /// the row to `success`.
    /// Mark a claimed build as successful. Returns `Err(StaleClaim)` if the
    /// row is no longer in `building` state — defends against the clobber
    /// race COR-16 calls out: two compilers racing on the same row would
    /// otherwise both succeed, with the second overwriting the first.
    ///
    /// **Note on the guard column (COR-16):** the spec asked for filtering
    /// by `compiler_instance_id`, but that column doesn't exist on
    /// `workflow_packages` today (it's an in-memory / audit-event id only).
    /// Adding it requires a migration that's out of scope for this bundle.
    /// Filtering by `build_status = 'building'` is the cheaper guard that
    /// still closes the race: `claim_next_build` is the only path that
    /// flips `pending → building`, and it does so with `FOR UPDATE SKIP
    /// LOCKED`, so only one caller can win the `building` state at a time.
    /// Once we transition to `success`, the second caller's filter misses.
    pub async fn mark_build_success(
        &self,
        package_id: Uuid,
        compiled: Vec<u8>,
    ) -> Result<(), RegistryError> {
        self.mark_build_success_with_docs(package_id, compiled, std::collections::HashMap::new())
            .await
    }

    /// Like [`Self::mark_build_success`] but also overlays compiler-parsed
    /// per-task "what & why" documentation (CLOACI-T-0752) onto the persisted
    /// metadata. `task_docs` is keyed by task local id; unmatched ids are
    /// ignored and tasks absent from the map keep `None` docs.
    pub async fn mark_build_success_with_docs(
        &self,
        package_id: Uuid,
        compiled: Vec<u8>,
        task_docs: std::collections::HashMap<
            String,
            crate::registry::loader::package_loader::TaskDocs,
        >,
    ) -> Result<(), RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{
            UniversalBinary, UniversalTimestamp, UniversalUuid,
        };

        let pid = UniversalUuid(package_id);
        let now = UniversalTimestamp::now();

        // Extract the authoritative metadata (executable `workflow_name`, task
        // list, graph data, triggers) from the freshly compiled cdylib and
        // merge it into the stored row's `metadata` JSON. At upload time the
        // row only carries manifest-derived fields (no workflow_name, empty
        // tasks); the cdylib is the only place the real workflow name lives.
        // Persisting it here is what lets the UI/API resolve and execute a
        // workflow by name when package name ≠ workflow name.
        // (CLOACI-T-0671 / CLOACI-T-0663)
        let merged_metadata_json: Option<String> = match self
            .extract_and_merge_build_metadata(package_id, &compiled, &task_docs)
            .await
        {
            Ok(json) => json,
            Err(e) => {
                // Metadata extraction is best-effort: a failure here must not
                // block recording the successful build (the compiled bytes are
                // still valid and loadable). Log and proceed without updating
                // the metadata column.
                tracing::warn!(
                    %package_id,
                    error = %e,
                    "failed to extract/merge build metadata; recording build success without metadata update"
                );
                None
            }
        };

        let bytes = UniversalBinary::new(compiled);
        let updated: usize = crate::dispatch_backend!(
            self.database.backend(),
            {
                let conn = self
                    .database
                    .get_postgres_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                let meta = merged_metadata_json.clone();
                conn.interact(move |conn| {
                    use diesel::prelude::*;
                    let target = workflow_packages::table
                        .filter(workflow_packages::id.eq(pid))
                        .filter(workflow_packages::build_status.eq("building"));
                    match meta {
                        Some(meta) => diesel::update(target)
                            .set((
                                workflow_packages::build_status.eq("success"),
                                workflow_packages::compiled_data.eq(Some(bytes)),
                                workflow_packages::compiled_at.eq(Some(now)),
                                workflow_packages::build_error.eq::<Option<String>>(None),
                                workflow_packages::metadata.eq(meta),
                            ))
                            .execute(conn),
                        None => diesel::update(target)
                            .set((
                                workflow_packages::build_status.eq("success"),
                                workflow_packages::compiled_data.eq(Some(bytes)),
                                workflow_packages::compiled_at.eq(Some(now)),
                                workflow_packages::build_error.eq::<Option<String>>(None),
                            ))
                            .execute(conn),
                    }
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
            },
            {
                let conn = self
                    .database
                    .get_sqlite_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                let meta = merged_metadata_json.clone();
                conn.interact(move |conn| {
                    use diesel::prelude::*;
                    let target = workflow_packages::table
                        .filter(workflow_packages::id.eq(pid))
                        .filter(workflow_packages::build_status.eq("building"));
                    match meta {
                        Some(meta) => diesel::update(target)
                            .set((
                                workflow_packages::build_status.eq("success"),
                                workflow_packages::compiled_data.eq(Some(bytes)),
                                workflow_packages::compiled_at.eq(Some(now)),
                                workflow_packages::build_error.eq::<Option<String>>(None),
                                workflow_packages::metadata.eq(meta),
                            ))
                            .execute(conn),
                        None => diesel::update(target)
                            .set((
                                workflow_packages::build_status.eq("success"),
                                workflow_packages::compiled_data.eq(Some(bytes)),
                                workflow_packages::compiled_at.eq(Some(now)),
                                workflow_packages::build_error.eq::<Option<String>>(None),
                            ))
                            .execute(conn),
                    }
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
            }
        );
        if updated == 0 {
            return Err(RegistryError::Database(format!(
                "stale build claim on package {pid}: row no longer in 'building' state \
                 — another compiler must have raced this mark_build_success"
            )));
        }
        Ok(())
    }

    /// Set the `paused` flag on a package row by id (CLOACI-T-0749). Returns
    /// `true` if a row was updated. A paused package refuses new executions at
    /// the execute chokepoint; in-flight executions are unaffected.
    pub async fn set_package_paused(
        &self,
        package_id: Uuid,
        paused: bool,
    ) -> Result<bool, RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};

        let pid = UniversalUuid(package_id);
        let now = UniversalTimestamp::now();
        let paused_val = UniversalBool::from(paused);
        let paused_at_val: Option<UniversalTimestamp> = if paused { Some(now) } else { None };

        let updated: usize = crate::dispatch_backend!(
            self.database.backend(),
            {
                let conn = self
                    .database
                    .get_postgres_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                conn.interact(move |conn| {
                    use diesel::prelude::*;
                    diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
                        .set((
                            workflow_packages::paused.eq(paused_val),
                            workflow_packages::paused_at.eq(paused_at_val),
                            workflow_packages::updated_at.eq(now),
                        ))
                        .execute(conn)
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
            },
            {
                let conn = self
                    .database
                    .get_sqlite_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                conn.interact(move |conn| {
                    use diesel::prelude::*;
                    diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
                        .set((
                            workflow_packages::paused.eq(paused_val),
                            workflow_packages::paused_at.eq(paused_at_val),
                            workflow_packages::updated_at.eq(now),
                        ))
                        .execute(conn)
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
            }
        );

        Ok(updated > 0)
    }

    /// Extract metadata from freshly compiled cdylib bytes and merge the
    /// authoritative fields (workflow name, tasks, graph data, triggers) into
    /// the row's stored `PackageMetadata`, returning the re-serialized JSON.
    ///
    /// Returns `Ok(None)` when there is nothing to persist (no compiled bytes,
    /// or the cdylib carries no extractable metadata) so the caller skips the
    /// metadata column update. The package's identity fields (package_name,
    /// version, description, author) are preserved from the existing row;
    /// only the cdylib-derived fields are overwritten. (CLOACI-T-0671 / T-0663)
    async fn extract_and_merge_build_metadata(
        &self,
        package_id: Uuid,
        compiled: &[u8],
        task_docs: &std::collections::HashMap<
            String,
            crate::registry::loader::package_loader::TaskDocs,
        >,
    ) -> Result<Option<String>, RegistryError> {
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        if compiled.is_empty() {
            return Ok(None);
        }

        // Pull the authoritative fields out of the compiled library.
        let loader = crate::registry::loader::package_loader::PackageLoader::new()
            .map_err(|e| RegistryError::Internal(format!("loader init failed: {e}")))?;
        let extracted = loader
            .extract_metadata(compiled)
            .await
            .map_err(|e| RegistryError::Internal(format!("metadata extraction failed: {e}")))?;

        // Read the row's existing stored metadata so we keep identity fields.
        let pid = UniversalUuid(package_id);
        let record: Option<UnifiedWorkflowPackage> = crate::dispatch_backend!(
            self.database.backend(),
            {
                let conn = self
                    .database
                    .get_postgres_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                conn.interact(move |conn| {
                    use diesel::prelude::*;
                    workflow_packages::table
                        .filter(workflow_packages::id.eq(pid))
                        .first::<UnifiedWorkflowPackage>(conn)
                        .optional()
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
            },
            {
                let conn = self
                    .database
                    .get_sqlite_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                conn.interact(move |conn| {
                    use diesel::prelude::*;
                    workflow_packages::table
                        .filter(workflow_packages::id.eq(pid))
                        .first::<UnifiedWorkflowPackage>(conn)
                        .optional()
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
            }
        );

        let record = match record {
            Some(r) => r,
            None => return Ok(None),
        };

        // Start from the existing metadata (preserves package_name, version,
        // description, author) and overwrite the cdylib-derived fields.
        let mut merged: crate::registry::loader::package_loader::PackageMetadata =
            serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;

        merged.workflow_name = extracted.workflow_name;
        merged.tasks = extracted.tasks;
        merged.graph_data = extracted.graph_data;
        merged.architecture = extracted.architecture;
        merged.symbols = extracted.symbols;
        merged.workflow_triggers = extracted.workflow_triggers;
        // CLOACI-I-0128: persist the declared input params from the cdylib's
        // input-interface entrypoint.
        merged.declared_params = extracted.declared_params;
        // CLOACI-I-0128 (T-0758): persist the non-workflow surface interfaces
        // (graph/reactor/accumulator) for injection validation.
        merged.declared_surfaces = extracted.declared_surfaces;

        // CLOACI-T-0752: overlay compiler-parsed doc-comment "what & why" onto
        // the cdylib-extracted task list, matched by local id. FFI metadata
        // carries no docs, so this is the only place they land.
        if !task_docs.is_empty() {
            for task in merged.tasks.iter_mut() {
                if let Some(docs) = task_docs.get(&task.local_id) {
                    task.doc_what = docs.what.clone();
                    task.doc_why = docs.why.clone();
                }
            }
        }

        let json = serde_json::to_string(&merged).map_err(RegistryError::Serialization)?;
        Ok(Some(json))
    }

    /// Persist a task list (local ids + dependency edges) into the row's stored
    /// `PackageMetadata`. Used by the Python load path, which has no cdylib to
    /// extract from — the reconciler supplies the task graph captured from the
    /// scoped Runtime. (CLOACI-T-0672)
    pub(super) async fn persist_task_graph_db(
        &self,
        package_id: Uuid,
        tasks: Vec<(String, Vec<String>)>,
    ) -> Result<(), RegistryError> {
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;
        use crate::registry::loader::package_loader::TaskMetadata;

        let pid = UniversalUuid(package_id);

        // Read the existing metadata so we preserve identity + workflow_name.
        let record: Option<UnifiedWorkflowPackage> = crate::dispatch_backend!(
            self.database.backend(),
            {
                let conn = self
                    .database
                    .get_postgres_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                conn.interact(move |conn| {
                    use diesel::prelude::*;
                    workflow_packages::table
                        .filter(workflow_packages::id.eq(pid))
                        .first::<UnifiedWorkflowPackage>(conn)
                        .optional()
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
            },
            {
                let conn = self
                    .database
                    .get_sqlite_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                conn.interact(move |conn| {
                    use diesel::prelude::*;
                    workflow_packages::table
                        .filter(workflow_packages::id.eq(pid))
                        .first::<UnifiedWorkflowPackage>(conn)
                        .optional()
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
            }
        );

        let record = match record {
            Some(r) => r,
            None => return Ok(()),
        };

        let mut merged: crate::registry::loader::package_loader::PackageMetadata =
            serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;

        let workflow = if merged.workflow_name.is_empty() {
            merged.package_name.clone()
        } else {
            merged.workflow_name.clone()
        };
        merged.tasks = tasks
            .iter()
            .enumerate()
            .map(|(i, (id, deps))| TaskMetadata {
                index: i as u32,
                local_id: id.clone(),
                namespaced_id_template: format!(
                    "{{tenant}}::{}::{}::{}",
                    merged.package_name, workflow, id
                ),
                dependencies: deps.clone(),
                description: String::new(),
                source_location: String::new(),
                // Python doc carry is a deferred follow-up (CLOACI-T-0752).
                doc_what: None,
                doc_why: None,
            })
            .collect();

        let json = serde_json::to_string(&merged).map_err(RegistryError::Serialization)?;

        let updated: usize = crate::dispatch_backend!(
            self.database.backend(),
            {
                let conn = self
                    .database
                    .get_postgres_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                let json = json.clone();
                conn.interact(move |conn| {
                    use diesel::prelude::*;
                    diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
                        .set(workflow_packages::metadata.eq(json))
                        .execute(conn)
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
            },
            {
                let conn = self
                    .database
                    .get_sqlite_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                let json = json.clone();
                conn.interact(move |conn| {
                    use diesel::prelude::*;
                    diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
                        .set(workflow_packages::metadata.eq(json))
                        .execute(conn)
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
            }
        );

        if updated == 0 {
            tracing::warn!(%package_id, "persist_task_graph_db updated no rows");
        }
        Ok(())
    }

    /// Record a failed build. Returns `Err(StaleClaim)` if the row is no
    /// longer in `building` state — same race-defence as
    /// [`Self::mark_build_success`] (COR-16).
    pub async fn mark_build_failed(
        &self,
        package_id: Uuid,
        error: &str,
    ) -> Result<(), RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        const MAX_ERR: usize = 64 * 1024;
        let truncated = if error.len() > MAX_ERR {
            let start = error.len() - MAX_ERR;
            error[start..].to_string()
        } else {
            error.to_string()
        };
        let pid = UniversalUuid(package_id);
        let updated: usize = crate::dispatch_backend!(
            self.database.backend(),
            {
                let conn = self
                    .database
                    .get_postgres_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                conn.interact(move |conn| {
                    use diesel::prelude::*;
                    diesel::update(
                        workflow_packages::table
                            .filter(workflow_packages::id.eq(pid))
                            .filter(workflow_packages::build_status.eq("building")),
                    )
                    .set((
                        workflow_packages::build_status.eq("failed"),
                        workflow_packages::build_error.eq(Some(truncated)),
                    ))
                    .execute(conn)
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
            },
            {
                let conn = self
                    .database
                    .get_sqlite_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                conn.interact(move |conn| {
                    use diesel::prelude::*;
                    diesel::update(
                        workflow_packages::table
                            .filter(workflow_packages::id.eq(pid))
                            .filter(workflow_packages::build_status.eq("building")),
                    )
                    .set((
                        workflow_packages::build_status.eq("failed"),
                        workflow_packages::build_error.eq(Some(truncated)),
                    ))
                    .execute(conn)
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?
            }
        );
        if updated == 0 {
            return Err(RegistryError::Database(format!(
                "stale build claim on package {pid}: row no longer in 'building' state \
                 — another compiler must have raced this mark_build_failed"
            )));
        }
        Ok(())
    }

    /// Refresh `build_claimed_at` so the stale-build sweeper doesn't reset us.
    /// No-op if the row is no longer in `building` state.
    pub async fn heartbeat_build(&self, package_id: Uuid) -> Result<(), RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};

        let pid = UniversalUuid(package_id);
        let now = UniversalTimestamp::now();
        crate::dispatch_backend!(
            self.database.backend(),
            {
                let conn = self
                    .database
                    .get_postgres_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                conn.interact(move |conn| {
                    use diesel::prelude::*;
                    diesel::update(
                        workflow_packages::table
                            .filter(workflow_packages::id.eq(pid))
                            .filter(workflow_packages::build_status.eq("building")),
                    )
                    .set(workflow_packages::build_claimed_at.eq(Some(now)))
                    .execute(conn)
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
                Ok(())
            },
            {
                let conn = self
                    .database
                    .get_sqlite_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                conn.interact(move |conn| {
                    use diesel::prelude::*;
                    diesel::update(
                        workflow_packages::table
                            .filter(workflow_packages::id.eq(pid))
                            .filter(workflow_packages::build_status.eq("building")),
                    )
                    .set(workflow_packages::build_claimed_at.eq(Some(now)))
                    .execute(conn)
                })
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
                Ok(())
            }
        )
    }

    /// Reset rows stuck in `building` whose last heartbeat is older than
    /// `stale_threshold`. Returns the number of rows reset.
    pub async fn sweep_stale_builds(
        &self,
        stale_threshold: std::time::Duration,
    ) -> Result<usize, RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalTimestamp;

        let cutoff = UniversalTimestamp(
            chrono::Utc::now()
                - chrono::Duration::from_std(stale_threshold)
                    .unwrap_or_else(|_| chrono::Duration::seconds(60)),
        );
        crate::dispatch_backend!(
            self.database.backend(),
            {
                let conn = self
                    .database
                    .get_postgres_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                let n: usize = conn
                    .interact(move |conn| {
                        use diesel::prelude::*;
                        diesel::update(
                            workflow_packages::table
                                .filter(workflow_packages::build_status.eq("building"))
                                .filter(workflow_packages::build_claimed_at.lt(Some(cutoff))),
                        )
                        .set((
                            workflow_packages::build_status.eq("pending"),
                            workflow_packages::build_claimed_at
                                .eq::<Option<UniversalTimestamp>>(None),
                            workflow_packages::build_error
                                .eq(Some("(reset after stale heartbeat)".to_string())),
                        ))
                        .execute(conn)
                    })
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?
                    .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
                Ok(n)
            },
            {
                let conn = self
                    .database
                    .get_sqlite_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                let n: usize = conn
                    .interact(move |conn| {
                        use diesel::prelude::*;
                        diesel::update(
                            workflow_packages::table
                                .filter(workflow_packages::build_status.eq("building"))
                                .filter(workflow_packages::build_claimed_at.lt(Some(cutoff))),
                        )
                        .set((
                            workflow_packages::build_status.eq("pending"),
                            workflow_packages::build_claimed_at
                                .eq::<Option<UniversalTimestamp>>(None),
                            workflow_packages::build_error
                                .eq(Some("(reset after stale heartbeat)".to_string())),
                        ))
                        .execute(conn)
                    })
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?
                    .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
                Ok(n)
            }
        )
    }

    /// Look up the most recently-compiled artifact for `content_hash`, across
    /// all rows including superseded ones. Returns `(row_id, compiled_bytes)`
    /// when found. Used by the upload handler to skip the build queue when an
    /// identical artifact already exists.
    pub(super) async fn find_success_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<(Uuid, Vec<u8>)>, RegistryError> {
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;

        let hash = hash.to_string();
        crate::dispatch_backend!(
            self.database.backend(),
            {
                let conn = self
                    .database
                    .get_postgres_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                let hash_inner = hash.clone();
                let record: Option<UnifiedWorkflowPackage> = conn
                    .interact(move |conn| {
                        use diesel::prelude::*;
                        workflow_packages::table
                            .filter(workflow_packages::content_hash.eq(&hash_inner))
                            .filter(workflow_packages::build_status.eq("success"))
                            .filter(workflow_packages::compiled_data.is_not_null())
                            .order(workflow_packages::compiled_at.desc())
                            .first::<UnifiedWorkflowPackage>(conn)
                            .optional()
                    })
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?
                    .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
                Ok(record.and_then(|r| r.compiled_data.map(|b| (r.id.0, b.into_inner()))))
            },
            {
                let conn = self
                    .database
                    .get_sqlite_connection()
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?;
                let hash_inner = hash.clone();
                let record: Option<UnifiedWorkflowPackage> = conn
                    .interact(move |conn| {
                        use diesel::prelude::*;
                        workflow_packages::table
                            .filter(workflow_packages::content_hash.eq(&hash_inner))
                            .filter(workflow_packages::build_status.eq("success"))
                            .filter(workflow_packages::compiled_data.is_not_null())
                            .order(workflow_packages::compiled_at.desc())
                            .first::<UnifiedWorkflowPackage>(conn)
                            .optional()
                    })
                    .await
                    .map_err(|e| RegistryError::Database(e.to_string()))?
                    .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
                Ok(record.and_then(|r| r.compiled_data.map(|b| (r.id.0, b.into_inner()))))
            }
        )
    }

    /// Summary telemetry for the compiler service's `/v1/status` endpoint.
    ///
    /// Returns pending/building counts (superseded rows excluded) plus the
    /// most recent success/failure/heartbeat timestamps across all rows.
    /// Timestamps are computed by loading one row per bucket ordered by the
    /// relevant column — DbTimestamp doesn't support `MAX` aggregation, so
    /// `ORDER BY … DESC LIMIT 1` is the simplest portable substitute.
    pub async fn build_queue_stats(&self) -> Result<BuildQueueStats, RegistryError> {
        build_queue_stats(&self.database).await
    }
}

/// Build-queue telemetry over a raw [`Database`] handle, independent of the
/// registry's package loader. Powers both the compiler's `/v1/status` (via
/// [`WorkflowRegistryImpl::build_queue_stats`]) and the server's operator
/// compiler-status endpoint (CLOACI-I-0124), which has a `Database` but no
/// reason to construct a full registry (and its FFI loader) per request.
pub async fn build_queue_stats(
    database: &crate::database::Database,
) -> Result<BuildQueueStats, RegistryError> {
    use crate::dal::unified::models::UnifiedWorkflowPackage;
    use crate::database::schema::unified::workflow_packages;
    use crate::database::universal_types::UniversalBool;

    crate::dispatch_backend!(
        database.backend(),
        {
            let conn = database
                .get_postgres_connection()
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?;
            conn.interact(
                move |conn| -> Result<BuildQueueStats, diesel::result::Error> {
                    let pending = workflow_packages::table
                        .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                        .filter(workflow_packages::build_status.eq("pending"))
                        .count()
                        .get_result::<i64>(conn)?;
                    let building = workflow_packages::table
                        .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                        .filter(workflow_packages::build_status.eq("building"))
                        .count()
                        .get_result::<i64>(conn)?;
                    let last_success: Option<UnifiedWorkflowPackage> = workflow_packages::table
                        .filter(workflow_packages::build_status.eq("success"))
                        .order(workflow_packages::compiled_at.desc())
                        .first::<UnifiedWorkflowPackage>(conn)
                        .optional()?;
                    let last_failure: Option<UnifiedWorkflowPackage> = workflow_packages::table
                        .filter(workflow_packages::build_status.eq("failed"))
                        .order(workflow_packages::updated_at.desc())
                        .first::<UnifiedWorkflowPackage>(conn)
                        .optional()?;
                    let heartbeat_row: Option<UnifiedWorkflowPackage> = workflow_packages::table
                        .filter(workflow_packages::build_status.eq("building"))
                        .order(workflow_packages::build_claimed_at.desc())
                        .first::<UnifiedWorkflowPackage>(conn)
                        .optional()?;
                    Ok(BuildQueueStats {
                        pending: pending as u64,
                        building: building as u64,
                        last_success_at: last_success.and_then(|r| r.compiled_at.map(|t| t.0)),
                        last_failure_at: last_failure.map(|r| r.updated_at.0),
                        heartbeat_at: heartbeat_row.and_then(|r| r.build_claimed_at.map(|t| t.0)),
                    })
                },
            )
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
        },
        {
            let conn = database
                .get_sqlite_connection()
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?;
            conn.interact(
                move |conn| -> Result<BuildQueueStats, diesel::result::Error> {
                    let pending = workflow_packages::table
                        .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                        .filter(workflow_packages::build_status.eq("pending"))
                        .count()
                        .get_result::<i64>(conn)?;
                    let building = workflow_packages::table
                        .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                        .filter(workflow_packages::build_status.eq("building"))
                        .count()
                        .get_result::<i64>(conn)?;
                    let last_success: Option<UnifiedWorkflowPackage> = workflow_packages::table
                        .filter(workflow_packages::build_status.eq("success"))
                        .order(workflow_packages::compiled_at.desc())
                        .first::<UnifiedWorkflowPackage>(conn)
                        .optional()?;
                    let last_failure: Option<UnifiedWorkflowPackage> = workflow_packages::table
                        .filter(workflow_packages::build_status.eq("failed"))
                        .order(workflow_packages::updated_at.desc())
                        .first::<UnifiedWorkflowPackage>(conn)
                        .optional()?;
                    let heartbeat_row: Option<UnifiedWorkflowPackage> = workflow_packages::table
                        .filter(workflow_packages::build_status.eq("building"))
                        .order(workflow_packages::build_claimed_at.desc())
                        .first::<UnifiedWorkflowPackage>(conn)
                        .optional()?;
                    Ok(BuildQueueStats {
                        pending: pending as u64,
                        building: building as u64,
                        last_success_at: last_success.and_then(|r| r.compiled_at.map(|t| t.0)),
                        last_failure_at: last_failure.map(|r| r.updated_at.0),
                        heartbeat_at: heartbeat_row.and_then(|r| r.build_claimed_at.map(|t| t.0)),
                    })
                },
            )
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
        }
    )
}

/// Snapshot of the build queue for the compiler's status endpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BuildQueueStats {
    pub pending: u64,
    pub building: u64,
    pub last_success_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_failure_at: Option<chrono::DateTime<chrono::Utc>>,
    pub heartbeat_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Package-availability snapshot for the reconciler tile (CLOACI-T-0718 /
/// absorbs T-0717): how many packages built successfully and are available to
/// load, how many failed, and when the most recent successful build landed.
/// Counts the active (non-superseded) rows. Independent of the registry's
/// package loader — powers the server's ops-metrics publisher.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReconcilerStats {
    pub built: u64,
    pub failed: u64,
    pub last_built_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Compute [`ReconcilerStats`] over a raw [`Database`] handle.
pub async fn reconciler_stats(
    database: &crate::database::Database,
) -> Result<ReconcilerStats, RegistryError> {
    use crate::dal::unified::models::UnifiedWorkflowPackage;
    use crate::database::schema::unified::workflow_packages;
    use crate::database::universal_types::UniversalBool;

    macro_rules! query {
        ($conn:expr) => {{
            $conn
                .interact(
                    move |conn| -> Result<ReconcilerStats, diesel::result::Error> {
                        let built = workflow_packages::table
                            .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                            .filter(workflow_packages::build_status.eq("success"))
                            .count()
                            .get_result::<i64>(conn)?;
                        let failed = workflow_packages::table
                            .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                            .filter(workflow_packages::build_status.eq("failed"))
                            .count()
                            .get_result::<i64>(conn)?;
                        let last_built: Option<UnifiedWorkflowPackage> = workflow_packages::table
                            .filter(workflow_packages::build_status.eq("success"))
                            .order(workflow_packages::compiled_at.desc())
                            .first::<UnifiedWorkflowPackage>(conn)
                            .optional()?;
                        Ok(ReconcilerStats {
                            built: built as u64,
                            failed: failed as u64,
                            last_built_at: last_built.and_then(|r| r.compiled_at.map(|t| t.0)),
                        })
                    },
                )
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
        }};
    }

    crate::dispatch_backend!(
        database.backend(),
        {
            let conn = database
                .get_postgres_connection()
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?;
            query!(conn)
        },
        {
            let conn = database
                .get_sqlite_connection()
                .await
                .map_err(|e| RegistryError::Database(e.to_string()))?;
            query!(conn)
        }
    )
}

/// A build row claimed by the compiler. Everything the compiler needs to
/// locate the source and write back results.
#[derive(Debug, Clone)]
pub struct ClaimedBuild {
    pub id: Uuid,
    pub registry_id: Uuid,
    pub package_name: String,
    pub version: String,
    pub metadata: String,
}

impl From<crate::dal::unified::models::UnifiedWorkflowPackage> for ClaimedBuild {
    fn from(u: crate::dal::unified::models::UnifiedWorkflowPackage) -> Self {
        ClaimedBuild {
            id: u.id.0,
            registry_id: u.registry_id.0,
            package_name: u.package_name,
            version: u.version,
            metadata: u.metadata,
        }
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
            workflow_name: name.to_string(),
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
                doc_what: None,
                doc_why: None,
            }],
            graph_data: None,
            architecture: "x86_64".to_string(),
            symbols: vec![],
            workflow_triggers: vec![],
            declared_params: vec![],
            declared_surfaces: vec![],
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

        // Filtered reads require build_status = 'success'
        registry.claim_next_build().await.unwrap();
        registry
            .mark_build_success(pkg_id, Vec::new())
            .await
            .unwrap();

        let result = registry
            .get_package_metadata("reg-pkg", "1.0.0")
            .await
            .unwrap();
        assert!(result.is_some());
        let (reg_id, retrieved, _compiled) = result.unwrap();
        assert_eq!(reg_id, registry_id);
        assert_eq!(retrieved.package_name, "reg-pkg");
        assert_eq!(retrieved.version, "1.0.0");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_workflow_pause_resume() {
        // CLOACI-T-0749: pause/resume a workflow by name and observe the flag
        // through list_all_packages + is_workflow_paused (the execute gate's
        // source of truth).
        let registry = create_test_registry().await;
        let registry_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("pause-pkg", "1.0.0");
        let pkg_id = registry
            .store_package_metadata(&registry_id, &meta)
            .await
            .unwrap();
        registry.claim_next_build().await.unwrap();
        registry
            .mark_build_success(pkg_id, Vec::new())
            .await
            .unwrap();

        // Not paused on registration.
        assert!(!registry.is_workflow_paused("pause-pkg").await.unwrap());
        assert!(registry
            .list_all_packages()
            .await
            .unwrap()
            .iter()
            .all(|w| !w.paused));

        // Pause by name → flag set everywhere the gate / API read it.
        let affected = registry
            .set_workflow_paused("pause-pkg", true)
            .await
            .unwrap();
        assert_eq!(affected, Some(pkg_id));
        assert!(registry.is_workflow_paused("pause-pkg").await.unwrap());
        assert!(registry
            .list_all_packages()
            .await
            .unwrap()
            .iter()
            .any(|w| w.package_name == "pause-pkg" && w.paused));

        // Resume clears it.
        registry
            .set_workflow_paused("pause-pkg", false)
            .await
            .unwrap();
        assert!(!registry.is_workflow_paused("pause-pkg").await.unwrap());

        // Unknown workflow: not paused, and set is a no-op returning None.
        assert!(!registry.is_workflow_paused("nope").await.unwrap());
        assert_eq!(
            registry.set_workflow_paused("nope", true).await.unwrap(),
            None
        );
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

        // Store two packages and mark both as successfully built.
        let reg_id = Uuid::new_v4().to_string();
        let meta1 = sample_metadata("list-a", "1.0.0");
        let meta2 = sample_metadata("list-b", "2.0.0");
        let id1 = registry
            .store_package_metadata(&reg_id, &meta1)
            .await
            .unwrap();
        let id2 = registry
            .store_package_metadata(&reg_id, &meta2)
            .await
            .unwrap();
        for id in [id1, id2] {
            registry.claim_next_build().await.unwrap();
            registry.mark_build_success(id, Vec::new()).await.unwrap();
        }

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

        let pkg_id = registry
            .store_package_metadata(&reg_id, &meta)
            .await
            .unwrap();
        registry.claim_next_build().await.unwrap();
        registry
            .mark_build_success(pkg_id, Vec::new())
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
        registry.claim_next_build().await.unwrap();
        registry
            .mark_build_success(pkg_id, Vec::new())
            .await
            .unwrap();

        let result = registry.get_package_metadata_by_id(pkg_id).await.unwrap();
        assert!(result.is_some());
        let (_, wf_meta, _compiled) = result.unwrap();
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
        registry.claim_next_build().await.unwrap();
        registry
            .mark_build_success(id_v1, Vec::new())
            .await
            .unwrap();

        let id_v2 = registry
            .supersede_and_insert(Some(id_v1), &reg_id_2, &meta_v2, "hash-v2")
            .await
            .unwrap();
        assert_ne!(id_v1, id_v2);
        registry.claim_next_build().await.unwrap();
        registry
            .mark_build_success(id_v2, Vec::new())
            .await
            .unwrap();

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

    // ========================================================================
    // Build queue tests (CLOACI-I-0097)
    // ========================================================================

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_claim_next_build_returns_pending_row() {
        let registry = create_test_registry().await;
        let reg_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("claim-pkg", "1");
        let pkg_id = registry
            .supersede_and_insert(None, &reg_id, &meta, "hash-claim")
            .await
            .unwrap();

        let claimed = registry.claim_next_build().await.unwrap().expect("row");
        assert_eq!(claimed.id, pkg_id);
        assert_eq!(claimed.package_name, "claim-pkg");

        // Second claim sees nothing — it's `building` now.
        assert!(registry.claim_next_build().await.unwrap().is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_mark_build_success_flips_state_and_writes_bytes() {
        let registry = create_test_registry().await;
        let reg_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("ok-pkg", "1");
        let pkg_id = registry
            .supersede_and_insert(None, &reg_id, &meta, "hash-ok")
            .await
            .unwrap();
        registry.claim_next_build().await.unwrap();

        registry
            .mark_build_success(pkg_id, vec![0xAA, 0xBB, 0xCC])
            .await
            .unwrap();

        // Row now reachable again via the normal read methods
        let found = registry
            .get_package_metadata("ok-pkg", "1")
            .await
            .unwrap()
            .expect("should still be visible");
        assert_eq!(found.0, reg_id);
        assert_eq!(found.2, Some(vec![0xAA, 0xBB, 0xCC]));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_mark_build_failed_writes_error() {
        let registry = create_test_registry().await;
        let reg_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("bad-pkg", "1");
        let pkg_id = registry
            .supersede_and_insert(None, &reg_id, &meta, "hash-bad")
            .await
            .unwrap();
        registry.claim_next_build().await.unwrap();
        registry
            .mark_build_failed(pkg_id, "compile error on line 42")
            .await
            .unwrap();
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_heartbeat_updates_claim_timestamp_only_while_building() {
        let registry = create_test_registry().await;
        let reg_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("hb-pkg", "1");
        let pkg_id = registry
            .supersede_and_insert(None, &reg_id, &meta, "hash-hb")
            .await
            .unwrap();

        // Not yet building → heartbeat is no-op (doesn't error).
        registry.heartbeat_build(pkg_id).await.unwrap();

        // Claim → building → heartbeat updates.
        registry.claim_next_build().await.unwrap();
        registry.heartbeat_build(pkg_id).await.unwrap();

        // Mark success → heartbeat no-ops again.
        registry.mark_build_success(pkg_id, vec![]).await.unwrap();
        registry.heartbeat_build(pkg_id).await.unwrap();
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_sweep_stale_builds_resets_old_rows() {
        let registry = create_test_registry().await;
        let reg_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("stale-pkg", "1");
        registry
            .supersede_and_insert(None, &reg_id, &meta, "hash-stale")
            .await
            .unwrap();
        registry.claim_next_build().await.unwrap();

        // Threshold of zero means every `building` row looks stale.
        let n = registry
            .sweep_stale_builds(std::time::Duration::from_secs(0))
            .await
            .unwrap();
        assert_eq!(n, 1, "expected one row reset to pending");

        // Next claim picks it up again.
        let re_claimed = registry.claim_next_build().await.unwrap();
        assert!(re_claimed.is_some());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_find_success_by_hash_returns_matching_artifact() {
        let registry = create_test_registry().await;
        let reg_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("hash-reuse-pkg", "1");

        // No match when nothing has been built.
        assert!(registry
            .find_success_by_hash("abc")
            .await
            .unwrap()
            .is_none());

        let pkg_id = registry
            .supersede_and_insert(None, &reg_id, &meta, "abc")
            .await
            .unwrap();

        // Still no match — row is pending, not success.
        assert!(registry
            .find_success_by_hash("abc")
            .await
            .unwrap()
            .is_none());

        registry.claim_next_build().await.unwrap();
        registry
            .mark_build_success(pkg_id, vec![0xDE, 0xAD, 0xBE, 0xEF])
            .await
            .unwrap();

        let hit = registry
            .find_success_by_hash("abc")
            .await
            .unwrap()
            .expect("should find the artifact");
        assert_eq!(hit.0, pkg_id);
        assert_eq!(hit.1, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_supersede_and_insert_with_prebuilt_skips_queue() {
        let registry = create_test_registry().await;
        let reg_id_a = Uuid::new_v4().to_string();
        let reg_id_b = Uuid::new_v4().to_string();
        let meta_a = sample_metadata("prebuilt-a", "1");
        let meta_b = sample_metadata("prebuilt-b", "1");

        let prebuilt = vec![0xCA, 0xFE, 0xBA, 0xBE];
        // Insert package A as already-successful via the prebuilt path.
        registry
            .supersede_and_insert_with_prebuilt(
                None,
                &reg_id_a,
                &meta_a,
                "shared-hash",
                Some(prebuilt.clone()),
            )
            .await
            .unwrap();

        // It's already success, so claim_next_build returns None.
        assert!(registry.claim_next_build().await.unwrap().is_none());

        // Insert package B as pending (normal path) so we can confirm the
        // queue still works when no prebuilt is supplied.
        registry
            .supersede_and_insert(None, &reg_id_b, &meta_b, "other-hash")
            .await
            .unwrap();
        assert!(registry.claim_next_build().await.unwrap().is_some());
    }
}
