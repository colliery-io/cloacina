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
    /// CLOACINA-T-1074: why the compiler could not write the package's
    /// metadata document. `None` when it could, or has not tried yet.
    pub describe_error: Option<String>,
}

/// One stored version of a package, for the versions list (CLOACINA-T-1018).
#[derive(Debug, Clone)]
pub struct PackageVersion {
    pub id: Uuid,
    pub package_name: String,
    pub version: String,
    /// In service: not superseded and built.
    pub active: bool,
    pub superseded: bool,
    pub build_status: String,
    pub build_error: Option<String>,
    /// CLOACINA-T-1074: why the compiler could not write this version's
    /// metadata document.
    pub describe_error: Option<String>,
    pub content_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub compiled_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// A package in service that has no metadata document of the current format
/// (CLOACINA-T-1074). See
/// [`WorkflowRegistryImpl::list_undescribed_packages`].
#[derive(Debug, Clone)]
pub struct UndescribedPackage {
    pub id: Uuid,
    pub package_name: String,
    pub version: String,
}

/// What a package row holds about its metadata document (CLOACINA-T-1074).
#[derive(Debug, Clone)]
pub struct StoredDescription {
    /// The document. `None`: not described, or the describe failed.
    pub document: Option<crate::registry::package_description::PackageDescription>,
    /// The format version of the last describe, set also when it failed.
    pub version: Option<u32>,
    /// Why the last describe failed.
    pub describe_error: Option<String>,
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
    /// Wrap this registry's `Database` handle in a lightweight `DAL` so the
    /// `interact_on_backend!` macro (CLOACI-I-0135) can check out the concrete
    /// pooled connection for whichever backend is live. `DAL::new` only clones
    /// the `Database` (an `Arc`-backed pool handle), so this is cheap per call.
    fn dal(&self) -> crate::dal::unified::DAL {
        crate::dal::unified::DAL::new(self.database.clone())
    }

    /// Store package metadata in the database. Test-only helper; the
    /// production write path is `insert_candidate`, which deals with the
    /// candidate/active/superseded transitions correctly.
    #[cfg(test)]
    pub(super) async fn store_package_metadata(
        &self,
        registry_id: &str,
        package_metadata: &crate::registry::loader::package_loader::PackageMetadata,
    ) -> Result<Uuid, RegistryError> {
        use crate::dal::unified::models::NewUnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};

        let registry_uuid = Uuid::parse_str(registry_id).map_err(RegistryError::InvalidUuid)?;
        let metadata =
            serde_json::to_string(package_metadata).map_err(RegistryError::Serialization)?;
        let storage_type = self.storage.storage_type();

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

        let dal = self.dal();
        crate::interact_on_backend!(dal, |conn| {
            diesel::insert_into(workflow_packages::table)
                .values(&new_package)
                .execute(conn)
        })
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
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;

        let package_name = package_name.to_string();
        let version = version.to_string();

        let dal = self.dal();
        let package_record: Option<UnifiedWorkflowPackage> =
            crate::interact_on_backend!(dal, |conn| {
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
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;

        let dal = self.dal();
        let package_records: Vec<UnifiedWorkflowPackage> =
            crate::interact_on_backend!(dal, |conn| {
                workflow_packages::table
                    .filter(
                        workflow_packages::superseded
                            .eq(crate::database::universal_types::UniversalBool(false)),
                    )
                    .filter(workflow_packages::build_status.eq("success"))
                    .load::<UnifiedWorkflowPackage>(conn)
            })
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
                workflow_triggers: package_metadata.workflow_triggers.clone(),
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
        use crate::database::schema::unified::workflow_packages;

        let package_name = package_name.to_string();
        let version = version.to_string();

        let dal = self.dal();
        crate::interact_on_backend!(dal, |conn| {
            diesel::delete(
                workflow_packages::table
                    .filter(workflow_packages::package_name.eq(&package_name))
                    .filter(workflow_packages::version.eq(&version)),
            )
            .execute(conn)
        })
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
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        let pkg_id = UniversalUuid(package_id);
        let dal = self.dal();
        let package_record: Option<UnifiedWorkflowPackage> =
            crate::interact_on_backend!(dal, |conn| {
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
                workflow_triggers: package_metadata.workflow_triggers.clone(),
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

    /// The rows of `name` that are not superseded: the active row
    /// (`build_status = 'success'`) and the candidates (pending / building /
    /// failed). Returns `(id, content_hash, build_status)` for each.
    /// (CLOACINA-T-1018)
    pub(super) async fn get_live_rows_by_name(
        &self,
        package_name: &str,
    ) -> Result<Vec<(Uuid, String, String)>, RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalBool, UniversalUuid};

        let name = package_name.to_string();
        let dal = self.dal();
        let rows: Vec<(UniversalUuid, String, String)> = crate::interact_on_backend!(dal, |conn| {
            workflow_packages::table
                .filter(workflow_packages::package_name.eq(&name))
                .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                .select((
                    workflow_packages::id,
                    workflow_packages::content_hash,
                    workflow_packages::build_status,
                ))
                .load(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(rows.into_iter().map(|(id, h, st)| (id.0, h, st)).collect())
    }

    /// Mark the candidates of `name` (not superseded, not built) superseded.
    /// The active row is not touched. Returns the number of rows changed.
    /// (CLOACINA-T-1018)
    pub(super) async fn abandon_candidates(
        &self,
        package_name: &str,
    ) -> Result<usize, RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalBool, UniversalTimestamp};

        let name = package_name.to_string();
        let now = UniversalTimestamp::now();
        let dal = self.dal();
        crate::interact_on_backend!(dal, |conn| {
            diesel::update(
                workflow_packages::table
                    .filter(workflow_packages::package_name.eq(&name))
                    .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                    .filter(workflow_packages::build_status.ne("success")),
            )
            .set((
                workflow_packages::superseded.eq(UniversalBool(true)),
                workflow_packages::updated_at.eq(now),
            ))
            .execute(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
    }

    /// Insert an uploaded package as a candidate. Returns the new package UUID.
    ///
    /// One transaction (CLOACINA-T-1018):
    /// - earlier candidates of the name (not superseded, not built) are marked
    ///   superseded: a newer upload replaces an upload that is not in service;
    /// - the new row is inserted `pending`. The active row is NOT touched: it
    ///   stays in service until [`Self::mark_build_success_with_docs`] swaps
    ///   the two, and stays for good when the build fails.
    ///
    /// With `prebuilt` (content-hash artifact reuse, T-0523) the new row is
    /// born `success`, so the swap happens here: the active row is marked
    /// superseded in the same transaction.
    ///
    /// The partial unique index `(package_name) WHERE NOT superseded AND
    /// build_status = 'success'` keeps at most one active row per name.
    pub(super) async fn insert_candidate(
        &self,
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

        let born_active = prebuilt.is_some();
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
        let name_in_tx = pkg_name.clone();

        let dal = self.dal();
        crate::interact_on_backend!(dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|tx| {
                let live = workflow_packages::table
                    .filter(workflow_packages::package_name.eq(&name_in_tx))
                    .filter(workflow_packages::superseded.eq(UniversalBool(false)));
                if born_active {
                    // Already built: this row is active at once, so every
                    // other live row of the name is retired.
                    diesel::update(live)
                        .set((
                            workflow_packages::superseded.eq(UniversalBool(true)),
                            workflow_packages::updated_at.eq(now),
                        ))
                        .execute(tx)?;
                } else {
                    diesel::update(live.filter(workflow_packages::build_status.ne("success")))
                        .set((
                            workflow_packages::superseded.eq(UniversalBool(true)),
                            workflow_packages::updated_at.eq(now),
                        ))
                        .execute(tx)?;
                }
                diesel::insert_into(workflow_packages::table)
                    .values(&new_row)
                    .execute(tx)?;
                Ok(new_id.0)
            })
        })
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

    /// Every stored version of `package_name`, newest upload first, with its
    /// build state and whether it is the active one. Superseded and failed
    /// rows are included. Does not read the compiled bytes. (CLOACINA-T-1018)
    pub async fn list_package_versions(
        &self,
        package_name: &str,
    ) -> Result<Vec<PackageVersion>, RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};

        type Row = (
            UniversalUuid,
            String,
            String,
            Option<String>,
            UniversalBool,
            String,
            UniversalTimestamp,
            Option<UniversalTimestamp>,
            Option<String>,
        );

        let name = package_name.to_string();
        let dal = self.dal();
        let rows: Vec<Row> = crate::interact_on_backend!(dal, |conn| {
            workflow_packages::table
                .filter(workflow_packages::package_name.eq(&name))
                .order(workflow_packages::created_at.desc())
                .select((
                    workflow_packages::id,
                    workflow_packages::version,
                    workflow_packages::build_status,
                    workflow_packages::build_error,
                    workflow_packages::superseded,
                    workflow_packages::content_hash,
                    workflow_packages::created_at,
                    workflow_packages::compiled_at,
                    workflow_packages::describe_error,
                ))
                .load(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    version,
                    build_status,
                    build_error,
                    superseded,
                    content_hash,
                    created,
                    built,
                    describe_error,
                )| {
                    let superseded = superseded.is_true();
                    PackageVersion {
                        id: id.0,
                        package_name: package_name.to_string(),
                        version,
                        active: !superseded && build_status == "success",
                        superseded,
                        build_status,
                        build_error,
                        describe_error,
                        content_hash,
                        created_at: created.0,
                        compiled_at: built.map(|t| t.0),
                    }
                },
            )
            .collect())
    }

    /// Make an earlier built version of `package_name` the active one, with no
    /// rebuild. Returns the id of the row that is now active.
    ///
    /// One transaction (CLOACINA-T-1018): the target must be
    /// `build_status = 'success'`. The active row and any candidates of the
    /// name are marked superseded, then the target is set `superseded = false`.
    /// The target takes the paused flag of the row it replaces, so a paused
    /// workflow stays paused. The reconciler loads the target on its next pass.
    ///
    /// Errors: `PackageNotFound` when the version does not exist,
    /// `ValidationError` when it was never built.
    pub async fn activate_package_version(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Uuid, RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};

        enum Outcome {
            NotFound,
            NotBuilt(String),
            Active(Uuid),
        }

        let name = package_name.to_string();
        let ver = version.to_string();
        let now = UniversalTimestamp::now();
        let dal = self.dal();
        let outcome: Outcome = crate::interact_on_backend!(dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|tx| {
                let target: Option<(UniversalUuid, String, UniversalBool)> =
                    workflow_packages::table
                        .filter(workflow_packages::package_name.eq(&name))
                        .filter(workflow_packages::version.eq(&ver))
                        .select((
                            workflow_packages::id,
                            workflow_packages::build_status,
                            workflow_packages::superseded,
                        ))
                        .first(tx)
                        .optional()?;
                let Some((target_id, status, superseded)) = target else {
                    return Ok(Outcome::NotFound);
                };
                if status != "success" {
                    return Ok(Outcome::NotBuilt(status));
                }
                if !superseded.is_true() {
                    // Already the active row. Nothing to do.
                    return Ok(Outcome::Active(target_id.0));
                }

                let replaced: Option<(UniversalBool, Option<UniversalTimestamp>)> =
                    workflow_packages::table
                        .filter(workflow_packages::package_name.eq(&name))
                        .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                        .filter(workflow_packages::build_status.eq("success"))
                        .select((workflow_packages::paused, workflow_packages::paused_at))
                        .first(tx)
                        .optional()?;

                // Retire the active row and the candidates first: the unique
                // index allows one active row per name.
                diesel::update(
                    workflow_packages::table
                        .filter(workflow_packages::package_name.eq(&name))
                        .filter(workflow_packages::superseded.eq(UniversalBool(false))),
                )
                .set((
                    workflow_packages::superseded.eq(UniversalBool(true)),
                    workflow_packages::updated_at.eq(now),
                ))
                .execute(tx)?;

                let target_row =
                    workflow_packages::table.filter(workflow_packages::id.eq(target_id));
                match replaced {
                    Some((paused, paused_at)) => diesel::update(target_row)
                        .set((
                            workflow_packages::superseded.eq(UniversalBool(false)),
                            workflow_packages::paused.eq(paused),
                            workflow_packages::paused_at.eq(paused_at),
                            workflow_packages::updated_at.eq(now),
                        ))
                        .execute(tx)?,
                    None => diesel::update(target_row)
                        .set((
                            workflow_packages::superseded.eq(UniversalBool(false)),
                            workflow_packages::updated_at.eq(now),
                        ))
                        .execute(tx)?,
                };
                Ok(Outcome::Active(target_id.0))
            })
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        match outcome {
            Outcome::Active(id) => Ok(id),
            Outcome::NotFound => Err(RegistryError::PackageNotFound {
                package_name: package_name.to_string(),
                version: version.to_string(),
            }),
            Outcome::NotBuilt(status) => Err(RegistryError::ValidationError {
                reason: format!(
                    "version {version} of {package_name} has build status '{status}': \
                     only a version with a successful build can be made active"
                ),
            }),
        }
    }

    /// Inspect a package by ID — returns metadata plus `build_status` /
    /// `build_error` regardless of build outcome. Unlike `get_package_metadata_by_id`
    /// this does not filter by `build_status = 'success'`, so operators can
    /// surface pending / building / failed rows through `package inspect`.
    ///
    /// Only `superseded = false` rows are returned: the active row or a
    /// candidate. `list_package_versions` gives the superseded history.
    pub async fn inspect_package_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<Option<InspectedPackage>, RegistryError> {
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalBool, UniversalUuid};

        let pkg_id = UniversalUuid(package_id);

        let dal = self.dal();
        let record: Option<UnifiedWorkflowPackage> = crate::interact_on_backend!(dal, |conn| {
            workflow_packages::table
                .filter(workflow_packages::id.eq(pkg_id))
                .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                .first::<UnifiedWorkflowPackage>(conn)
                .optional()
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

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
                workflow_triggers: package_metadata.workflow_triggers.clone(),
            },
            build_status: record.build_status,
            build_error: record.build_error,
            describe_error: record.describe_error,
        }))
    }

    /// Delete package metadata by ID.
    pub(super) async fn delete_package_metadata_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        let pkg_id = UniversalUuid(package_id);
        let dal = self.dal();
        crate::interact_on_backend!(dal, |conn| {
            diesel::delete(workflow_packages::table.filter(workflow_packages::id.eq(pkg_id)))
                .execute(conn)
        })
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
        // KEPT AS EXPLICIT TWINS (CLOACI-I-0135): the two arms genuinely
        // diverge — the Postgres claim uses `FOR UPDATE SKIP LOCKED`
        // (`.for_update().skip_locked()`) so concurrent compiler instances
        // don't block on each other's claims; SQLite has no such construct and
        // relies on its single-writer lock instead. Not a backend-agnostic
        // body, so it cannot fold into `interact_on_backend!`.
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
        self.mark_build_success_with_docs(
            package_id,
            compiled,
            std::collections::HashMap::new(),
            Vec::new(),
            Vec::new(),
        )
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
        declared_params: Vec<cloacina_api_types::InputSlot>,
        declared_surfaces: Vec<cloacina_api_types::DeclaredSurface>,
    ) -> Result<(), RegistryError> {
        // Extract the authoritative metadata (executable `workflow_name`, task
        // list, graph data, triggers) from the freshly compiled cdylib and
        // merge it into the stored row's `metadata` JSON. At upload time the
        // row only carries manifest-derived fields (no workflow_name, empty
        // tasks); the cdylib is the only place the real workflow name lives.
        // Persisting it here is what lets the UI/API resolve and execute a
        // workflow by name when package name ≠ workflow name.
        // (CLOACI-T-0671 / CLOACI-T-0663)
        let merged_metadata_json: Option<String> = match self
            .extract_and_merge_build_metadata(
                package_id,
                &compiled,
                &task_docs,
                &declared_params,
                &declared_surfaces,
            )
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

        // No document: this path opens the library in the calling process
        // (embedded use and tests). The compiler service uses
        // `mark_build_success_described`.
        self.finish_build(package_id, compiled, merged_metadata_json, None)
            .await
    }

    /// Record a successful build with the full metadata document that the
    /// compiler's describe child wrote (CLOACINA-T-1074).
    ///
    /// This does not open the compiled library: the fields of the row's
    /// `metadata` JSON that the library used to be opened for (workflow name,
    /// task list, graph data, subscriptions, declared params and surfaces)
    /// are taken from `document`. `document` is stored in `metadata_document`
    /// with its format version, and `describe_error` is cleared.
    ///
    /// `task_docs`, `declared_params` and `declared_surfaces` are the
    /// compiler's parse of the package source; the caller has already put
    /// them into `document` with
    /// [`PackageDescription::overlay_source_parse`](crate::registry::package_description::PackageDescription::overlay_source_parse).
    ///
    /// The swap of the active version is the one of
    /// [`Self::mark_build_success_with_docs`].
    pub async fn mark_build_success_described(
        &self,
        package_id: Uuid,
        compiled: Vec<u8>,
        document: &crate::registry::package_description::PackageDescription,
        task_docs: std::collections::HashMap<
            String,
            crate::registry::loader::package_loader::TaskDocs,
        >,
        declared_params: Vec<cloacina_api_types::InputSlot>,
        declared_surfaces: Vec<cloacina_api_types::DeclaredSurface>,
    ) -> Result<(), RegistryError> {
        let merged = self
            .merge_described_build_metadata(
                package_id,
                document,
                &task_docs,
                &declared_params,
                &declared_surfaces,
            )
            .await?;
        let document_json =
            serde_json::to_string(document).map_err(RegistryError::Serialization)?;
        self.finish_build(
            package_id,
            compiled,
            merged,
            Some((document_json, document.format_version as i32)),
        )
        .await
    }

    /// The row's `metadata` JSON after a described build: the stored JSON
    /// with the code-derived fields replaced from `document`. `None` when the
    /// row is gone.
    async fn merge_described_build_metadata(
        &self,
        package_id: Uuid,
        document: &crate::registry::package_description::PackageDescription,
        task_docs: &std::collections::HashMap<
            String,
            crate::registry::loader::package_loader::TaskDocs,
        >,
        source_declared_params: &[cloacina_api_types::InputSlot],
        source_declared_surfaces: &[cloacina_api_types::DeclaredSurface],
    ) -> Result<Option<String>, RegistryError> {
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        let pid = UniversalUuid(package_id);
        let dal = self.dal();
        let record: Option<UnifiedWorkflowPackage> = crate::interact_on_backend!(dal, |conn| {
            use diesel::prelude::*;
            workflow_packages::table
                .filter(workflow_packages::id.eq(pid))
                .first::<UnifiedWorkflowPackage>(conn)
                .optional()
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        let Some(record) = record else {
            return Ok(None);
        };
        let mut merged: crate::registry::loader::package_loader::PackageMetadata =
            serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;

        if document.language == "python" {
            // As before the document existed: a Python row gets the source
            // parse at build, and its task list at load
            // (`persist_task_graph_db`).
            merged.declared_params = source_declared_params.to_vec();
            merged.declared_surfaces = source_declared_surfaces.to_vec();
        } else {
            // CLOACI-T-0736: the compiled code is authoritative for
            // workflow_name; a manifest value that disagrees is replaced.
            if !merged.workflow_name.is_empty()
                && !document.workflow_name.is_empty()
                && merged.workflow_name != document.workflow_name
            {
                tracing::warn!(
                    manifest = %merged.workflow_name,
                    code = %document.workflow_name,
                    package = %record.package_name,
                    "package.toml workflow_name disagrees with the compiled code — \
                     the code value wins (drop the manifest line; it is optional)"
                );
            }
            document.merge_into_legacy(&mut merged);
        }
        merged.task_docs = task_docs.clone();

        let json = serde_json::to_string(&merged).map_err(RegistryError::Serialization)?;
        Ok(Some(json))
    }

    /// Write a finished build: the compiled bytes, the `metadata` JSON when
    /// there is a new one, the metadata document when there is one, and the
    /// swap of the active version.
    async fn finish_build(
        &self,
        package_id: Uuid,
        compiled: Vec<u8>,
        merged_metadata_json: Option<String>,
        document: Option<(String, i32)>,
    ) -> Result<(), RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{
            UniversalBinary, UniversalTimestamp, UniversalUuid,
        };

        let pid = UniversalUuid(package_id);
        let now = UniversalTimestamp::now();
        let bytes = UniversalBinary::new(compiled);
        let meta = merged_metadata_json;
        // A build with no document leaves the row "not described": a document
        // of an earlier build of the row does not describe the new bytes.
        let (document_json, document_version) = match document {
            Some((json, version)) => (Some(json), Some(version)),
            None => (None, None),
        };
        let dal = self.dal();
        // CLOACINA-T-1018: the swap. One transaction retires the active row of
        // the same name and makes this row `success`. Until here the old
        // version was active and loaded; a failed build never gets here.
        let updated: usize = crate::interact_on_backend!(dal, |conn| {
            use crate::database::universal_types::UniversalBool;
            use diesel::prelude::*;
            let swap = conn.transaction::<usize, diesel::result::Error, _>(|tx| {
                // Only a live claim may swap: `building` and not superseded.
                // A candidate that a newer upload or an activation replaced
                // is superseded, so its late build cannot become active.
                let name: Option<String> = workflow_packages::table
                    .filter(workflow_packages::id.eq(pid))
                    .filter(workflow_packages::build_status.eq("building"))
                    .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                    .select(workflow_packages::package_name)
                    .first(tx)
                    .optional()?;
                let Some(name) = name else {
                    return Ok(0);
                };
                // Retire the active row first: the unique index allows one
                // active row per name.
                diesel::update(
                    workflow_packages::table
                        .filter(workflow_packages::package_name.eq(&name))
                        .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                        .filter(workflow_packages::build_status.eq("success"))
                        .filter(workflow_packages::id.ne(pid)),
                )
                .set((
                    workflow_packages::superseded.eq(UniversalBool(true)),
                    workflow_packages::updated_at.eq(now),
                ))
                .execute(tx)?;
                let target = workflow_packages::table
                    .filter(workflow_packages::id.eq(pid))
                    .filter(workflow_packages::build_status.eq("building"))
                    .filter(workflow_packages::superseded.eq(UniversalBool(false)));
                let n = match meta {
                    Some(meta) => diesel::update(target)
                        .set((
                            workflow_packages::build_status.eq("success"),
                            workflow_packages::compiled_data.eq(Some(bytes)),
                            workflow_packages::compiled_at.eq(Some(now)),
                            workflow_packages::build_error.eq::<Option<String>>(None),
                            workflow_packages::metadata.eq(meta),
                            workflow_packages::metadata_document.eq(document_json),
                            workflow_packages::metadata_document_version.eq(document_version),
                            workflow_packages::describe_error.eq::<Option<String>>(None),
                        ))
                        .execute(tx)?,
                    None => diesel::update(target)
                        .set((
                            workflow_packages::build_status.eq("success"),
                            workflow_packages::compiled_data.eq(Some(bytes)),
                            workflow_packages::compiled_at.eq(Some(now)),
                            workflow_packages::build_error.eq::<Option<String>>(None),
                            workflow_packages::metadata_document.eq(document_json),
                            workflow_packages::metadata_document_version.eq(document_version),
                            workflow_packages::describe_error.eq::<Option<String>>(None),
                        ))
                        .execute(tx)?,
                };
                if n == 0 {
                    // The claim was lost between the read and the write: undo
                    // the retire so the old version stays active.
                    return Err(diesel::result::Error::RollbackTransaction);
                }
                Ok(n)
            });
            match swap {
                Err(diesel::result::Error::RollbackTransaction) => Ok(0),
                other => other,
            }
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        if updated == 0 {
            return Err(RegistryError::Database(format!(
                "stale build claim on package {pid}: row no longer in 'building' state \
                 or replaced by a newer upload — this build does not become active"
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

        let dal = self.dal();
        let updated: usize = crate::interact_on_backend!(dal, |conn| {
            use diesel::prelude::*;
            diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
                .set((
                    workflow_packages::paused.eq(paused_val),
                    workflow_packages::paused_at.eq(paused_at_val),
                    workflow_packages::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

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
        source_declared_params: &[cloacina_api_types::InputSlot],
        source_declared_surfaces: &[cloacina_api_types::DeclaredSurface],
    ) -> Result<Option<String>, RegistryError> {
        use crate::dal::unified::models::UnifiedWorkflowPackage;
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        if compiled.is_empty() {
            // Pure-Python packages have no cdylib to extract from. CLOACI-T-0760:
            // if the compiler parsed declared params from the Python source,
            // persist them onto the stored (manifest-derived) metadata — this is
            // the Python parity carry path. CLOACI-T-0754: the docstring
            // what/why parse rides the same carry (persist_task_graph_db
            // re-merges it when it writes the task list at load).
            if source_declared_params.is_empty()
                && source_declared_surfaces.is_empty()
                && task_docs.is_empty()
            {
                return Ok(None);
            }
            let pid = UniversalUuid(package_id);
            let dal = self.dal();
            let record: Option<UnifiedWorkflowPackage> = crate::interact_on_backend!(dal, |conn| {
                use diesel::prelude::*;
                workflow_packages::table
                    .filter(workflow_packages::id.eq(pid))
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
            let record = match record {
                Some(r) => r,
                None => return Ok(None),
            };
            let mut merged: crate::registry::loader::package_loader::PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            merged.declared_params = source_declared_params.to_vec();
            // CLOACI-T-0770: carry source-parsed CG boundary surfaces too.
            merged.declared_surfaces = source_declared_surfaces.to_vec();
            // CLOACI-T-0754: preserve the raw docstring parse — the Python
            // task list doesn't exist yet (written at load by
            // persist_task_graph_db, which merges docs from this map).
            merged.task_docs = task_docs.clone();
            let json = serde_json::to_string(&merged).map_err(RegistryError::Serialization)?;
            return Ok(Some(json));
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
        let dal = self.dal();
        let record: Option<UnifiedWorkflowPackage> = crate::interact_on_backend!(dal, |conn| {
            use diesel::prelude::*;
            workflow_packages::table
                .filter(workflow_packages::id.eq(pid))
                .first::<UnifiedWorkflowPackage>(conn)
                .optional()
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        let record = match record {
            Some(r) => r,
            None => return Ok(None),
        };

        // Start from the existing metadata (preserves package_name, version,
        // description, author) and overwrite the cdylib-derived fields.
        let mut merged: crate::registry::loader::package_loader::PackageMetadata =
            serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;

        // CLOACI-T-0736: the COMPILED CODE is authoritative for
        // workflow_name — the manifest value is at most a pre-build hint.
        // A disagreement is the T-0666 drift class: never silently prefer
        // the manifest; adopt the code value and warn loudly so the author
        // fixes (or deletes) the stale manifest line.
        if !merged.workflow_name.is_empty()
            && !extracted.workflow_name.is_empty()
            && merged.workflow_name != extracted.workflow_name
        {
            tracing::warn!(
                manifest = %merged.workflow_name,
                code = %extracted.workflow_name,
                package = %record.package_name,
                "package.toml workflow_name disagrees with the compiled code —                  the code value wins (drop the manifest line; it is optional)"
            );
        }
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
        // CLOACI-T-0754: also persist the RAW doc map. Python packages have no
        // tasks at build time (no cdylib) — their task list is written later by
        // persist_task_graph_db, which re-merges docs from this map.
        merged.task_docs = task_docs.clone();

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
        let dal = self.dal();
        let record: Option<UnifiedWorkflowPackage> = crate::interact_on_backend!(dal, |conn| {
            use diesel::prelude::*;
            workflow_packages::table
                .filter(workflow_packages::id.eq(pid))
                .first::<UnifiedWorkflowPackage>(conn)
                .optional()
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

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
                // CLOACI-T-0754: re-merge the compiler's build-time doc parse
                // (stored on the metadata by mark_build_success_with_docs) so
                // Python docstring what/why survive the load-time task rewrite.
                doc_what: merged.task_docs.get(id).and_then(|d| d.what.clone()),
                doc_why: merged.task_docs.get(id).and_then(|d| d.why.clone()),
            })
            .collect();

        let json = serde_json::to_string(&merged).map_err(RegistryError::Serialization)?;

        let dal = self.dal();
        let updated: usize = crate::interact_on_backend!(dal, |conn| {
            use diesel::prelude::*;
            diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
                .set(workflow_packages::metadata.eq(json))
                .execute(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

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
        let dal = self.dal();
        let updated: usize = crate::interact_on_backend!(dal, |conn| {
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
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        if updated == 0 {
            return Err(RegistryError::Database(format!(
                "stale build claim on package {pid}: row no longer in 'building' state \
                 — another compiler must have raced this mark_build_failed"
            )));
        }
        Ok(())
    }

    /// Record that a built package is refused at load (CLOACINA-T-1027): its
    /// task names an agent pool the tenant does not have, or requests more
    /// than one agent of its pool can hold. The row goes from `success` to
    /// `failed` with the reason in `build_error`, which is what the package
    /// routes and `cloacinactl package inspect` show for a failed build.
    ///
    /// Only a `success` row is changed: a build in work is not touched.
    /// Returns `true` when the row was changed.
    pub async fn refuse_package(
        &self,
        package_id: Uuid,
        reason: &str,
    ) -> Result<bool, RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        let pid = UniversalUuid(package_id);
        let reason = reason.to_string();
        let dal = self.dal();
        let updated: usize = crate::interact_on_backend!(dal, |conn| {
            use diesel::prelude::*;
            diesel::update(
                workflow_packages::table
                    .filter(workflow_packages::id.eq(pid))
                    .filter(workflow_packages::build_status.eq("success")),
            )
            .set((
                workflow_packages::build_status.eq("failed"),
                workflow_packages::build_error.eq(Some(reason)),
            ))
            .execute(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(updated > 0)
    }

    /// Flip a previously-built package back to `pending` so the compiler
    /// rebuilds it from its retained source (CLOACI-T-0835 — the post-upgrade
    /// recompile signal). Used by the reconciler when a load fails with a
    /// STALE-ARTIFACT error (fidius ABI bump, `CloacinaPlugin` interface-hash
    /// mismatch): the recorded artifact predates the host's expected ABI, and
    /// only a rebuild fixes it.
    ///
    /// Guarded to `success`/`failed` rows only — an in-flight `building` claim
    /// is never stomped. Returns `true` when the row was flipped.
    pub async fn request_recompile(&self, package_id: Uuid) -> Result<bool, RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        let pid = UniversalUuid(package_id);
        let dal = self.dal();
        let updated: usize = crate::interact_on_backend!(dal, |conn| {
            use diesel::prelude::*;
            diesel::update(
                workflow_packages::table
                    .filter(workflow_packages::id.eq(pid))
                    .filter(workflow_packages::build_status.eq_any(["success", "failed"])),
            )
            .set((
                workflow_packages::build_status.eq("pending"),
                workflow_packages::build_error.eq(None::<String>),
                workflow_packages::build_claimed_at
                    .eq(None::<crate::database::universal_types::UniversalTimestamp>),
            ))
            .execute(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(updated > 0)
    }

    // ========================================================================
    // The metadata document (CLOACINA-T-1074)
    // ========================================================================

    /// The packages in service that have no metadata document of this build's
    /// format: active rows (`success`, not superseded) whose
    /// `metadata_document_version` is NULL or older than
    /// [`PACKAGE_DESCRIPTION_VERSION`](crate::registry::package_description::PACKAGE_DESCRIPTION_VERSION).
    /// Oldest upload first, at most `limit` rows.
    ///
    /// These are packages built before the document existed, packages whose
    /// build was taken over from an identical upload, and packages a build
    /// recorded with no document. A row whose describe failed has the version
    /// of that try, so it is not listed again until the format changes or the
    /// package is built again.
    pub async fn list_undescribed_packages(
        &self,
        limit: i64,
    ) -> Result<Vec<UndescribedPackage>, RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalBool, UniversalUuid};

        let current = crate::registry::package_description::PACKAGE_DESCRIPTION_VERSION as i32;
        let dal = self.dal();
        let rows: Vec<(UniversalUuid, String, String)> = crate::interact_on_backend!(dal, |conn| {
            use diesel::prelude::*;
            workflow_packages::table
                .filter(workflow_packages::build_status.eq("success"))
                .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                .filter(
                    workflow_packages::metadata_document_version
                        .is_null()
                        .or(workflow_packages::metadata_document_version.lt(current)),
                )
                .order(workflow_packages::created_at.asc())
                .limit(limit)
                .select((
                    workflow_packages::id,
                    workflow_packages::package_name,
                    workflow_packages::version,
                ))
                .load(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(rows
            .into_iter()
            .map(|(id, package_name, version)| UndescribedPackage {
                id: id.0,
                package_name,
                version,
            })
            .collect())
    }

    /// The compiled library of a package row, by id. `None` when the row is
    /// gone or holds no library (a Python package holds an empty one).
    pub async fn get_compiled_data_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<Option<Vec<u8>>, RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalBinary, UniversalUuid};

        let pid = UniversalUuid(package_id);
        let dal = self.dal();
        let data: Option<Option<UniversalBinary>> = crate::interact_on_backend!(dal, |conn| {
            use diesel::prelude::*;
            workflow_packages::table
                .filter(workflow_packages::id.eq(pid))
                .select(workflow_packages::compiled_data)
                .first(conn)
                .optional()
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(data.flatten().map(|b| b.into_inner()))
    }

    /// Store the metadata document of a package that is already built
    /// (CLOACINA-T-1074): the describe of a package from before the document
    /// existed. Only a `success` row is written; the build state, the library
    /// and the `metadata` JSON are not changed. Returns `true` when the row
    /// was written.
    pub async fn store_package_description(
        &self,
        package_id: Uuid,
        document: &crate::registry::package_description::PackageDescription,
    ) -> Result<bool, RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        let pid = UniversalUuid(package_id);
        let json = serde_json::to_string(document).map_err(RegistryError::Serialization)?;
        let version = document.format_version as i32;
        let dal = self.dal();
        let updated: usize = crate::interact_on_backend!(dal, |conn| {
            use diesel::prelude::*;
            diesel::update(
                workflow_packages::table
                    .filter(workflow_packages::id.eq(pid))
                    .filter(workflow_packages::build_status.eq("success")),
            )
            .set((
                workflow_packages::metadata_document.eq(Some(json)),
                workflow_packages::metadata_document_version.eq(Some(version)),
                workflow_packages::describe_error.eq::<Option<String>>(None),
            ))
            .execute(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(updated > 0)
    }

    /// Record that a built package could not be described (CLOACINA-T-1074):
    /// `describe_error` gets the reason and `metadata_document_version` the
    /// format version of the try, so the package is not tried again by the
    /// same compiler version. A document of an earlier format is removed: it
    /// does not describe the package in the current format. The build state
    /// is not changed. Returns `true` when the row was written.
    pub async fn store_describe_error(
        &self,
        package_id: Uuid,
        error: &str,
    ) -> Result<bool, RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        const MAX_ERR: usize = 64 * 1024;
        let reason = if error.len() > MAX_ERR {
            let mut start = error.len() - MAX_ERR;
            while !error.is_char_boundary(start) {
                start += 1;
            }
            error[start..].to_string()
        } else {
            error.to_string()
        };
        let pid = UniversalUuid(package_id);
        let version = crate::registry::package_description::PACKAGE_DESCRIPTION_VERSION as i32;
        let dal = self.dal();
        let updated: usize = crate::interact_on_backend!(dal, |conn| {
            use diesel::prelude::*;
            diesel::update(
                workflow_packages::table
                    .filter(workflow_packages::id.eq(pid))
                    .filter(workflow_packages::build_status.eq("success")),
            )
            .set((
                workflow_packages::metadata_document.eq::<Option<String>>(None),
                workflow_packages::metadata_document_version.eq(Some(version)),
                workflow_packages::describe_error.eq(Some(reason)),
            ))
            .execute(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(updated > 0)
    }

    /// The stored metadata document of a package row, with the reason of a
    /// failed describe. `None` when the row does not exist.
    pub async fn get_package_description(
        &self,
        package_id: Uuid,
    ) -> Result<Option<StoredDescription>, RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::UniversalUuid;

        let pid = UniversalUuid(package_id);
        let dal = self.dal();
        let row: Option<(Option<String>, Option<i32>, Option<String>)> =
            crate::interact_on_backend!(dal, |conn| {
                use diesel::prelude::*;
                workflow_packages::table
                    .filter(workflow_packages::id.eq(pid))
                    .select((
                        workflow_packages::metadata_document,
                        workflow_packages::metadata_document_version,
                        workflow_packages::describe_error,
                    ))
                    .first(conn)
                    .optional()
            })
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        let Some((json, version, describe_error)) = row else {
            return Ok(None);
        };
        let document = match json {
            Some(json) => Some(serde_json::from_str(&json).map_err(RegistryError::Serialization)?),
            None => None,
        };
        Ok(Some(StoredDescription {
            document,
            version: version.map(|v| v as u32),
            describe_error,
        }))
    }

    /// Refresh `build_claimed_at` so the stale-build sweeper doesn't reset us.
    /// No-op if the row is no longer in `building` state.
    pub async fn heartbeat_build(&self, package_id: Uuid) -> Result<(), RegistryError> {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};

        let pid = UniversalUuid(package_id);
        let now = UniversalTimestamp::now();
        let dal = self.dal();
        crate::interact_on_backend!(dal, |conn| {
            use diesel::prelude::*;
            diesel::update(
                workflow_packages::table
                    .filter(workflow_packages::id.eq(pid))
                    .filter(workflow_packages::build_status.eq("building")),
            )
            .set(workflow_packages::build_claimed_at.eq(Some(now)))
            .execute(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(())
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
        let dal = self.dal();
        let n: usize = crate::interact_on_backend!(dal, |conn| {
            use diesel::prelude::*;
            diesel::update(
                workflow_packages::table
                    .filter(workflow_packages::build_status.eq("building"))
                    .filter(workflow_packages::build_claimed_at.lt(Some(cutoff))),
            )
            .set((
                workflow_packages::build_status.eq("pending"),
                workflow_packages::build_claimed_at.eq::<Option<UniversalTimestamp>>(None),
                workflow_packages::build_error
                    .eq(Some("(reset after stale heartbeat)".to_string())),
            ))
            .execute(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(n)
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
        let dal = self.dal();
        let record: Option<UnifiedWorkflowPackage> = crate::interact_on_backend!(dal, |conn| {
            use diesel::prelude::*;
            workflow_packages::table
                .filter(workflow_packages::content_hash.eq(&hash))
                .filter(workflow_packages::build_status.eq("success"))
                .filter(workflow_packages::compiled_data.is_not_null())
                .order(workflow_packages::compiled_at.desc())
                .first::<UnifiedWorkflowPackage>(conn)
                .optional()
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(record.and_then(|r| r.compiled_data.map(|b| (r.id.0, b.into_inner()))))
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

/// Build-queue telemetry over a raw [`Database`](crate::database::Database) handle, independent of the
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

    let dal = crate::dal::unified::DAL::new(database.clone());
    crate::interact_on_backend!(dal, |conn| {
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
        // CLOACINA-T-1074: active packages with no metadata document of the
        // current format. The compiler describes them from the stored build.
        let undescribed =
            workflow_packages::table
                .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                .filter(workflow_packages::build_status.eq("success"))
                .filter(workflow_packages::metadata_document_version.is_null().or(
                    workflow_packages::metadata_document_version.lt(
                        crate::registry::package_description::PACKAGE_DESCRIPTION_VERSION as i32,
                    ),
                ))
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
        Ok::<_, diesel::result::Error>(BuildQueueStats {
            pending: pending as u64,
            building: building as u64,
            undescribed: undescribed as u64,
            last_success_at: last_success.and_then(|r| r.compiled_at.map(|t| t.0)),
            last_failure_at: last_failure.map(|r| r.updated_at.0),
            heartbeat_at: heartbeat_row.and_then(|r| r.build_claimed_at.map(|t| t.0)),
        })
    })
    .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
}

/// Snapshot of the build queue for the compiler's status endpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BuildQueueStats {
    pub pending: u64,
    pub building: u64,
    /// CLOACINA-T-1074: packages in service with no metadata document of the
    /// current format. The compiler describes them from the stored build, so
    /// they are work for it like a pending build.
    pub undescribed: u64,
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

/// Compute [`ReconcilerStats`] over a raw [`Database`](crate::database::Database) handle.
pub async fn reconciler_stats(
    database: &crate::database::Database,
) -> Result<ReconcilerStats, RegistryError> {
    use crate::dal::unified::models::UnifiedWorkflowPackage;
    use crate::database::schema::unified::workflow_packages;
    use crate::database::universal_types::UniversalBool;

    let dal = crate::dal::unified::DAL::new(database.clone());
    crate::interact_on_backend!(dal, |conn| {
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
        Ok::<_, diesel::result::Error>(ReconcilerStats {
            built: built as u64,
            failed: failed as u64,
            last_built_at: last_built.and_then(|r| r.compiled_at.map(|t| t.0)),
        })
    })
    .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
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
            task_docs: Default::default(),
            python_platforms: None,
        }
    }

    /// CLOACI-T-0905 (multi-arch): `get_compiled_data_for_target` selects the
    /// per-target artifact for exactly `(package, version, triple)` — a simulated
    /// second arch gets ITS bytes, a triple with no artifact gets `None` (the
    /// caller then falls back to the primary build), and another VERSION's
    /// artifact never satisfies this version's load.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_compiled_data_for_target_is_version_and_triple_scoped() {
        use crate::registry::traits::WorkflowRegistry as _;

        let url = format!(
            "file:wfreg_target_test_{}?mode=memory&cache=shared",
            Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        let storage = UnifiedRegistryStorage::new(db.clone());
        let registry = WorkflowRegistryImpl::new(storage, db.clone()).unwrap();

        // A success package whose PRIMARY build is the compiler host's arch.
        let registry_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("arch-pkg", "1.0.0");
        let pkg_id = registry
            .store_package_metadata(&registry_id, &meta)
            .await
            .unwrap();
        registry.claim_next_build().await.unwrap();
        registry
            .mark_build_success(pkg_id, b"primary-arch-bytes".to_vec())
            .await
            .unwrap();

        // Simulated second arch: the per-target compiler stored ITS cdylib.
        let dal = crate::dal::DAL::new(db);
        dal.workflow_packages()
            .upsert_artifact(
                "arch-pkg",
                "1.0.0",
                None,
                "testarch-testos",
                "digest-testarch",
                b"testarch-bytes".to_vec(),
            )
            .await
            .unwrap();
        // A DIFFERENT VERSION's artifact for the same triple — must never leak
        // into 1.0.0's selection.
        dal.workflow_packages()
            .upsert_artifact(
                "arch-pkg",
                "2.0.0",
                None,
                "otherarch-testos",
                "digest-otherarch-v2",
                b"v2-otherarch-bytes".to_vec(),
            )
            .await
            .unwrap();

        // The simulated arch loads ITS OWN cdylib.
        let got = registry
            .get_compiled_data_for_target("arch-pkg", "1.0.0", "testarch-testos")
            .await
            .unwrap();
        assert_eq!(got.as_deref(), Some(b"testarch-bytes".as_slice()));

        // No artifact for this triple+version → None (caller falls back to the
        // primary build) — including when only ANOTHER version has that triple.
        assert!(
            registry
                .get_compiled_data_for_target("arch-pkg", "1.0.0", "missing-arch")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            registry
                .get_compiled_data_for_target("arch-pkg", "1.0.0", "otherarch-testos")
                .await
                .unwrap()
                .is_none()
        );
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

    /// CLOACI-T-0754: Python packages have NO tasks at build time (no cdylib) —
    /// their task list is written later by persist_task_graph_db. The compiler's
    /// docstring parse must survive that rewrite: mark_build_success_with_docs
    /// stores the raw docs map on the metadata, and the load-time task rewrite
    /// re-merges it. Mirrors the real py flow: upload (tasks empty) → build with
    /// docs → reconciler persists the task graph → docs surface per task.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_python_task_docs_survive_load_time_task_rewrite() {
        use crate::registry::loader::package_loader::TaskDocs;

        let registry = create_test_registry().await;
        let registry_id = Uuid::new_v4().to_string();
        // Python-style row: no tasks at upload/build time.
        let mut meta = sample_metadata("py-docs-pkg", "1.0.0");
        meta.tasks = vec![];
        let pkg_id = registry
            .store_package_metadata(&registry_id, &meta)
            .await
            .unwrap();
        registry.claim_next_build().await.unwrap();

        let mut docs = std::collections::HashMap::new();
        docs.insert(
            "prepare".to_string(),
            TaskDocs {
                what: Some("Stage the batch".to_string()),
                why: Some("Bad seeds fail the run".to_string()),
            },
        );
        registry
            .mark_build_success_with_docs(pkg_id, Vec::new(), docs, Vec::new(), Vec::new())
            .await
            .unwrap();

        // The reconciler writes the task graph at load (the step that previously
        // clobbered docs with None).
        registry
            .persist_task_graph_db(
                pkg_id.into(),
                vec![
                    ("prepare".to_string(), vec![]),
                    ("undocumented".to_string(), vec!["prepare".to_string()]),
                ],
            )
            .await
            .unwrap();

        let (_, retrieved, _) = registry
            .get_package_metadata("py-docs-pkg", "1.0.0")
            .await
            .unwrap()
            .unwrap();
        let prepare = retrieved
            .tasks
            .iter()
            .find(|t| t.local_id == "prepare")
            .expect("prepare task persisted");
        assert_eq!(prepare.doc_what.as_deref(), Some("Stage the batch"));
        assert_eq!(prepare.doc_why.as_deref(), Some("Bad seeds fail the run"));
        let undoc = retrieved
            .tasks
            .iter()
            .find(|t| t.local_id == "undocumented")
            .unwrap();
        assert!(undoc.doc_what.is_none() && undoc.doc_why.is_none());
    }

    /// CLOACINA-T-1073: the flow of a compiler that vendors a Python package.
    /// The sets are stored during the build, the row's metadata says where the
    /// package runs, and both survive the build-success write and the task
    /// rewrite at load. A loader gets the set of its platform and no other.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_python_vendor_sets_survive_build_success_and_load() {
        use crate::registry::traits::WorkflowRegistry;

        let registry = create_test_registry().await;
        let mut meta = sample_metadata("py-lock-pkg", "1.0.0");
        meta.tasks = vec![];
        let pkg_id = registry
            .store_package_metadata(&Uuid::new_v4().to_string(), &meta)
            .await
            .unwrap();
        registry.claim_next_build().await.unwrap();

        let linux = vec!["aarch64-linux".to_string(), "x86_64-linux".to_string()];
        registry
            .store_python_vendor(
                pkg_id,
                "py-lock-pkg",
                "1.0.0",
                vec![
                    ("aarch64-linux".to_string(), b"arm-set".to_vec()),
                    ("x86_64-linux".to_string(), b"x86-set".to_vec()),
                ],
                Some(linux.clone()),
            )
            .await
            .unwrap();
        registry
            .mark_build_success_with_docs(
                pkg_id,
                Vec::new(),
                std::collections::HashMap::new(),
                Vec::new(),
                Vec::new(),
            )
            .await
            .unwrap();
        registry
            .persist_task_graph_db(pkg_id.into(), vec![("prepare".to_string(), vec![])])
            .await
            .unwrap();

        let (_, retrieved, _) = registry
            .get_package_metadata("py-lock-pkg", "1.0.0")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.python_platforms, Some(linux.clone()));
        assert_eq!(
            registry
                .python_vendor_platforms("py-lock-pkg", "1.0.0")
                .await
                .unwrap(),
            linux
        );
        assert_eq!(
            registry
                .get_python_vendor("py-lock-pkg", "1.0.0", "x86_64-linux")
                .await
                .unwrap(),
            Some(b"x86-set".to_vec())
        );
        assert_eq!(
            registry
                .get_python_vendor("py-lock-pkg", "1.0.0", "aarch64-macos")
                .await
                .unwrap(),
            None
        );
        // The set is not the library of the platform.
        assert_eq!(
            registry
                .get_compiled_data_for_target("py-lock-pkg", "1.0.0", "x86_64-linux")
                .await
                .unwrap(),
            None
        );

        // A later build of the same version that vendors nothing leaves no set.
        registry
            .clear_python_vendor("py-lock-pkg", "1.0.0")
            .await
            .unwrap();
        assert_eq!(
            registry
                .get_python_vendor("py-lock-pkg", "1.0.0", "x86_64-linux")
                .await
                .unwrap(),
            None
        );
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
        assert!(
            registry
                .list_all_packages()
                .await
                .unwrap()
                .iter()
                .all(|w| !w.paused)
        );

        // Pause by name → flag set everywhere the gate / API read it.
        let affected = registry
            .set_workflow_paused("pause-pkg", true)
            .await
            .unwrap();
        assert_eq!(affected, Some(pkg_id));
        assert!(registry.is_workflow_paused("pause-pkg").await.unwrap());
        assert!(
            registry
                .list_all_packages()
                .await
                .unwrap()
                .iter()
                .any(|w| w.package_name == "pause-pkg" && w.paused)
        );

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
        assert!(
            registry
                .get_package_metadata("del-pkg", "1.0.0")
                .await
                .unwrap()
                .is_some()
        );

        // Delete
        registry
            .delete_package_metadata("del-pkg", "1.0.0")
            .await
            .unwrap();

        // Confirm gone
        assert!(
            registry
                .get_package_metadata("del-pkg", "1.0.0")
                .await
                .unwrap()
                .is_none()
        );
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

        assert!(
            registry
                .get_package_metadata_by_id(pkg_id)
                .await
                .unwrap()
                .is_none()
        );
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

    /// Test helper: upload a candidate and run its build to `success`.
    #[cfg(feature = "sqlite")]
    async fn upload_and_build(
        registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
        name: &str,
        version: &str,
        hash: &str,
    ) -> Uuid {
        let id = registry
            .insert_candidate(
                &Uuid::new_v4().to_string(),
                &sample_metadata(name, version),
                hash,
                None,
            )
            .await
            .unwrap();
        let claimed = registry.claim_next_build().await.unwrap().expect("claim");
        assert_eq!(claimed.id, id);
        registry.mark_build_success(id, Vec::new()).await.unwrap();
        id
    }

    /// Test helper: `(superseded, build_status)` of a row, read with no filter.
    #[cfg(feature = "sqlite")]
    async fn row_state(
        registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
        name: &str,
        id: Uuid,
    ) -> (bool, String) {
        let v = registry
            .list_package_versions(name)
            .await
            .unwrap()
            .into_iter()
            .find(|v| v.id == id)
            .expect("row exists");
        (v.superseded, v.build_status)
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_insert_candidate_fresh_name() {
        let registry = create_test_registry().await;
        let reg_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("pkg-a", "1");

        let pkg_id = registry
            .insert_candidate(&reg_id, &meta, "hash-v1", None)
            .await
            .unwrap();

        let live = registry.get_live_rows_by_name("pkg-a").await.unwrap();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].0, pkg_id);
        assert_eq!(live[0].1, "hash-v1");
        assert_eq!(live[0].2, "pending");
        // A candidate is not in service.
        assert!(registry.list_all_packages().await.unwrap().is_empty());
    }

    /// CLOACINA-T-1018: the old version stays active until the build of the
    /// new one succeeds; the build success swaps the two.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_old_version_stays_active_until_new_build_succeeds() {
        let registry = create_test_registry().await;
        let id_v1 = upload_and_build(&registry, "pkg-b", "1", "hash-v1").await;

        let id_v2 = registry
            .insert_candidate(
                &Uuid::new_v4().to_string(),
                &sample_metadata("pkg-b", "2"),
                "hash-v2",
                None,
            )
            .await
            .unwrap();
        assert_ne!(id_v1, id_v2);

        // Uploaded, not built: v1 is still the one the reconciler loads.
        let list = registry.list_all_packages().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id_v1);
        assert_eq!(
            row_state(&registry, "pkg-b", id_v1).await,
            (false, "success".into())
        );
        assert_eq!(
            row_state(&registry, "pkg-b", id_v2).await,
            (false, "pending".into())
        );

        // Building: still v1.
        let claimed = registry
            .claim_next_build()
            .await
            .unwrap()
            .expect("claim v2");
        assert_eq!(claimed.id, id_v2);
        let list = registry.list_all_packages().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id_v1);
        assert!(
            registry
                .get_package_metadata("pkg-b", "1")
                .await
                .unwrap()
                .is_some()
        );

        // Build success: the swap.
        registry
            .mark_build_success(id_v2, Vec::new())
            .await
            .unwrap();
        let list = registry.list_all_packages().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id_v2);
        assert_eq!(
            row_state(&registry, "pkg-b", id_v1).await,
            (true, "success".into())
        );
        assert_eq!(
            row_state(&registry, "pkg-b", id_v2).await,
            (false, "success".into())
        );

        // The old row is no longer visible through the active reads.
        assert!(
            registry
                .get_package_metadata("pkg-b", "1")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            registry
                .get_package_metadata_by_id(id_v1)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            registry
                .get_package_metadata("pkg-b", "2")
                .await
                .unwrap()
                .is_some()
        );
    }

    /// CLOACINA-T-1018: a failed build leaves the old version in service.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_failed_build_leaves_old_version_active() {
        let registry = create_test_registry().await;
        let id_v1 = upload_and_build(&registry, "pkg-f", "1", "hash-v1").await;

        let id_v2 = registry
            .insert_candidate(
                &Uuid::new_v4().to_string(),
                &sample_metadata("pkg-f", "2"),
                "hash-v2",
                None,
            )
            .await
            .unwrap();
        registry
            .claim_next_build()
            .await
            .unwrap()
            .expect("claim v2");
        registry
            .mark_build_failed(id_v2, "error[E0425]")
            .await
            .unwrap();

        let list = registry.list_all_packages().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id_v1);
        assert_eq!(
            row_state(&registry, "pkg-f", id_v1).await,
            (false, "success".into())
        );
        assert_eq!(
            row_state(&registry, "pkg-f", id_v2).await,
            (false, "failed".into())
        );
        // The failed candidate can still be inspected by id.
        let ins = registry
            .inspect_package_by_id(id_v2)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ins.build_status, "failed");

        // A fixed upload replaces the failed candidate and then swaps in.
        let id_v3 = upload_and_build(&registry, "pkg-f", "3", "hash-v3").await;
        assert_eq!(
            row_state(&registry, "pkg-f", id_v2).await,
            (true, "failed".into())
        );
        assert_eq!(
            row_state(&registry, "pkg-f", id_v1).await,
            (true, "success".into())
        );
        let list = registry.list_all_packages().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id_v3);
    }

    /// CLOACINA-T-1018: a newer upload replaces a candidate that is not built,
    /// and the late build of the replaced candidate cannot become active.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_newer_upload_abandons_candidate_and_its_late_build() {
        let registry = create_test_registry().await;
        let id_v1 = upload_and_build(&registry, "pkg-n", "1", "hash-v1").await;

        let id_v2 = registry
            .insert_candidate(
                &Uuid::new_v4().to_string(),
                &sample_metadata("pkg-n", "2"),
                "hash-v2",
                None,
            )
            .await
            .unwrap();
        // The compiler claims v2, then v3 arrives.
        registry
            .claim_next_build()
            .await
            .unwrap()
            .expect("claim v2");
        let id_v3 = registry
            .insert_candidate(
                &Uuid::new_v4().to_string(),
                &sample_metadata("pkg-n", "3"),
                "hash-v3",
                None,
            )
            .await
            .unwrap();
        assert_eq!(
            row_state(&registry, "pkg-n", id_v2).await,
            (true, "building".into())
        );

        // The late success of v2 is refused, and v1 stays active.
        let err = registry
            .mark_build_success(id_v2, Vec::new())
            .await
            .expect_err("an abandoned build must not become active");
        assert!(format!("{err}").contains("stale build claim"), "{err}");
        let list = registry.list_all_packages().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id_v1);

        // v3 builds and swaps in.
        let claimed = registry
            .claim_next_build()
            .await
            .unwrap()
            .expect("claim v3");
        assert_eq!(claimed.id, id_v3);
        registry
            .mark_build_success(id_v3, Vec::new())
            .await
            .unwrap();
        let list = registry.list_all_packages().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id_v3);
    }

    /// The partial unique index keeps one active row per name: a second
    /// built row of a name cannot be made active beside the first.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_partial_unique_rejects_second_active_for_same_name() {
        use crate::database::schema::unified::workflow_packages;
        use crate::database::universal_types::{UniversalBool, UniversalUuid};

        let registry = create_test_registry().await;
        let id_v1 = upload_and_build(&registry, "pkg-c", "1", "hash-v1").await;
        let _id_v2 = upload_and_build(&registry, "pkg-c", "2", "hash-v2").await;

        // Force v1 back to "not superseded" with no swap: the index refuses.
        let dal = registry.dal();
        let pid = UniversalUuid(id_v1);
        let res: Result<usize, diesel::result::Error> = crate::interact_on_backend!(dal, |conn| {
            Ok::<_, diesel::result::Error>(
                diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
                    .set(workflow_packages::superseded.eq(UniversalBool(false)))
                    .execute(conn),
            )
        })
        .unwrap();
        assert!(
            matches!(
                res,
                Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _
                ))
            ),
            "expected a unique violation, got {:?}",
            res
        );

        // The same version number cannot be uploaded again.
        let err = registry
            .insert_candidate(
                &Uuid::new_v4().to_string(),
                &sample_metadata("pkg-c", "2"),
                "hash-other",
                None,
            )
            .await
            .expect_err("version numbers are single-use");
        assert!(
            matches!(err, RegistryError::PackageExists { .. }),
            "{err:?}"
        );
    }

    /// CLOACINA-T-1018: the versions list shows every row, and `activate`
    /// makes an earlier built version the active one in one step.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_versions_and_activate_earlier_version() {
        let registry = create_test_registry().await;
        let id_v1 = upload_and_build(&registry, "pkg-r", "1", "hash-v1").await;
        let id_v2 = upload_and_build(&registry, "pkg-r", "2", "hash-v2").await;
        // A candidate that waits while the operator goes back.
        let id_v3 = registry
            .insert_candidate(
                &Uuid::new_v4().to_string(),
                &sample_metadata("pkg-r", "3"),
                "hash-v3",
                None,
            )
            .await
            .unwrap();
        // v2 is paused: the pause must survive the activation.
        assert!(registry.set_package_paused(id_v2, true).await.unwrap());

        let versions = registry.list_package_versions("pkg-r").await.unwrap();
        assert_eq!(versions.len(), 3);
        let by_id = |id: Uuid| versions.iter().find(|v| v.id == id).unwrap().clone();
        assert!(!by_id(id_v1).active && by_id(id_v1).superseded);
        assert!(by_id(id_v2).active && !by_id(id_v2).superseded);
        assert!(!by_id(id_v3).active && by_id(id_v3).build_status == "pending");
        assert!(
            registry
                .list_package_versions("no-such")
                .await
                .unwrap()
                .is_empty()
        );

        // Go back to version 1.
        let active = registry
            .activate_package_version("pkg-r", "1")
            .await
            .unwrap();
        assert_eq!(active, id_v1);
        let list = registry.list_all_packages().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id_v1);
        assert!(
            list[0].paused,
            "the paused flag moves with the active version"
        );
        assert_eq!(
            row_state(&registry, "pkg-r", id_v2).await,
            (true, "success".into())
        );
        // The candidate was abandoned: its build cannot undo the activation.
        assert_eq!(
            row_state(&registry, "pkg-r", id_v3).await,
            (true, "pending".into())
        );
        assert!(registry.claim_next_build().await.unwrap().is_none());

        // Activating the active version changes nothing.
        let again = registry
            .activate_package_version("pkg-r", "1")
            .await
            .unwrap();
        assert_eq!(again, id_v1);

        // And forward again to version 2.
        let active = registry
            .activate_package_version("pkg-r", "2")
            .await
            .unwrap();
        assert_eq!(active, id_v2);
        assert_eq!(
            row_state(&registry, "pkg-r", id_v1).await,
            (true, "success".into())
        );

        // A version that does not exist, and one that was never built.
        let err = registry
            .activate_package_version("pkg-r", "9")
            .await
            .unwrap_err();
        assert!(
            matches!(err, RegistryError::PackageNotFound { .. }),
            "{err:?}"
        );
        let err = registry
            .activate_package_version("pkg-r", "3")
            .await
            .unwrap_err();
        assert!(
            matches!(err, RegistryError::ValidationError { .. }),
            "{err:?}"
        );
        let list = registry.list_all_packages().await.unwrap();
        assert_eq!(list[0].id, id_v2, "a refused activation changes nothing");
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
            .insert_candidate(&reg_id, &meta, "hash-claim", None)
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
            .insert_candidate(&reg_id, &meta, "hash-ok", None)
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
            .insert_candidate(&reg_id, &meta, "hash-bad", None)
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
            .insert_candidate(&reg_id, &meta, "hash-hb", None)
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
            .insert_candidate(&reg_id, &meta, "hash-stale", None)
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
        assert!(
            registry
                .find_success_by_hash("abc")
                .await
                .unwrap()
                .is_none()
        );

        let pkg_id = registry
            .insert_candidate(&reg_id, &meta, "abc", None)
            .await
            .unwrap();

        // Still no match — row is pending, not success.
        assert!(
            registry
                .find_success_by_hash("abc")
                .await
                .unwrap()
                .is_none()
        );

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
    async fn test_insert_candidate_with_prebuilt_skips_queue() {
        let registry = create_test_registry().await;
        let reg_id_a = Uuid::new_v4().to_string();
        let reg_id_b = Uuid::new_v4().to_string();
        let meta_a = sample_metadata("prebuilt-a", "1");
        let meta_b = sample_metadata("prebuilt-b", "1");

        let prebuilt = vec![0xCA, 0xFE, 0xBA, 0xBE];
        // Insert package A as already-successful via the prebuilt path.
        registry
            .insert_candidate(&reg_id_a, &meta_a, "shared-hash", Some(prebuilt.clone()))
            .await
            .unwrap();

        // It's already success, so claim_next_build returns None.
        assert!(registry.claim_next_build().await.unwrap().is_none());

        // Insert package B as pending (normal path) so we can confirm the
        // queue still works when no prebuilt is supplied.
        registry
            .insert_candidate(&reg_id_b, &meta_b, "other-hash", None)
            .await
            .unwrap();
        assert!(registry.claim_next_build().await.unwrap().is_some());
    }

    // ========================================================================
    // The metadata document (CLOACINA-T-1074)
    // ========================================================================

    /// A document as the describe child writes it for a Rust library with two
    /// tasks, a trigger subscription and a declared secret.
    #[cfg(feature = "sqlite")]
    fn sample_document(
        workflow_name: &str,
    ) -> crate::registry::package_description::PackageDescription {
        use crate::registry::package_description::{PackageDescription, RustLibraryMetadata};
        use cloacina_workflow_plugin::types::{PackageTasksMetadata, TaskMetadataEntry};

        let task = |index: u32, id: &str, deps: &[&str]| TaskMetadataEntry {
            index,
            id: id.to_string(),
            namespaced_id_template: format!("{{tenant}}::pkg::{workflow_name}::{id}"),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            description: format!("{id} step"),
            source_location: "src/lib.rs".to_string(),
            trigger_rules: "{\"type\":\"Always\"}".to_string(),
            requires_handle: false,
            retry_policy: String::new(),
            timeout_seconds: Some(30),
            pool: Some("large".to_string()),
            cpu: None,
            memory: Some("2Gi".to_string()),
            inline: false,
        };
        PackageDescription::from_rust_library(RustLibraryMetadata {
            tasks: PackageTasksMetadata {
                workflow_name: workflow_name.to_string(),
                package_name: "pkg".to_string(),
                package_description: None,
                package_author: None,
                workflow_fingerprint: None,
                graph_data_json: None,
                tasks: vec![task(0, "extract", &[]), task(1, "load", &["extract"])],
                triggers: vec!["nightly".to_string()],
            },
            declared_params: vec![cloacina_api_types::InputSlot::secret("warehouse")],
            declared_surfaces: Vec::new(),
            graph: None,
            reactors: Vec::new(),
            triggers: Vec::new(),
            triggerless_graphs: Vec::new(),
            constructors: Vec::new(),
        })
    }

    /// A described build stores the document, and fills the row's `metadata`
    /// from it. The compiled bytes here are not a library: nothing opens them.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_described_build_stores_the_document_and_opens_no_library() {
        use crate::registry::package_description::PACKAGE_DESCRIPTION_VERSION;

        let registry = create_test_registry().await;
        let mut manifest_meta = sample_metadata("described-pkg", "1.0.0");
        manifest_meta.workflow_name = String::new();
        manifest_meta.tasks = Vec::new();
        let id = registry
            .insert_candidate(
                &Uuid::new_v4().to_string(),
                &manifest_meta,
                "h-described",
                None,
            )
            .await
            .unwrap();
        registry.claim_next_build().await.unwrap().expect("claim");

        let mut document = sample_document("etl_workflow");
        let docs = std::collections::HashMap::from([(
            "load".to_string(),
            crate::registry::loader::package_loader::TaskDocs {
                what: Some("loads".to_string()),
                why: None,
            },
        )]);
        document.overlay_source_parse(&docs, &[], &[]);

        registry
            .mark_build_success_described(
                id,
                b"not a library: a described build never opens it".to_vec(),
                &document,
                docs,
                Vec::new(),
                Vec::new(),
            )
            .await
            .expect("a described build needs no loadable library");

        let stored = registry
            .get_package_description(id)
            .await
            .unwrap()
            .expect("row");
        assert_eq!(stored.version, Some(PACKAGE_DESCRIPTION_VERSION));
        assert!(stored.describe_error.is_none());
        let stored_doc = stored.document.expect("document");
        assert_eq!(
            serde_json::to_value(&stored_doc).unwrap(),
            serde_json::to_value(&document).unwrap()
        );
        assert_eq!(stored_doc.tasks[1].pool.as_deref(), Some("large"));
        assert_eq!(stored_doc.tasks[1].timeout_seconds, Some(30));
        assert_eq!(stored_doc.declared_secrets, vec!["warehouse"]);

        // The fields the library used to be opened for are in `metadata`.
        let inspected = registry
            .inspect_package_by_id(id)
            .await
            .unwrap()
            .expect("row");
        assert_eq!(inspected.build_status, "success");
        assert_eq!(inspected.metadata.workflow_name, "etl_workflow");
        assert_eq!(inspected.metadata.tasks, vec!["extract", "load"]);
        assert_eq!(
            inspected.metadata.task_graph[1].dependencies,
            vec!["extract"]
        );
        assert_eq!(
            inspected.metadata.task_graph[1].doc_what.as_deref(),
            Some("loads")
        );
        assert_eq!(inspected.metadata.workflow_triggers, vec!["nightly"]);
        assert_eq!(inspected.metadata.declared_params.len(), 1);
        assert!(inspected.describe_error.is_none());

        assert!(
            registry
                .list_undescribed_packages(10)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(registry.build_queue_stats().await.unwrap().undescribed, 0);
    }

    /// A package in service with no document is listed until it is described,
    /// or until a describe failed for it. The build state does not change.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_undescribed_packages_are_listed_until_described_or_marked() {
        use crate::registry::package_description::PACKAGE_DESCRIPTION_VERSION;

        let registry = create_test_registry().await;
        // Built the old way: no document.
        let ok_id = upload_and_build(&registry, "old-ok", "1", "h-old-ok").await;
        let bad_id = upload_and_build(&registry, "old-bad", "1", "h-old-bad").await;
        // A candidate that is not built is not listed.
        let pending_id = registry
            .insert_candidate(
                &Uuid::new_v4().to_string(),
                &sample_metadata("old-pending", "1"),
                "h-old-pending",
                None,
            )
            .await
            .unwrap();

        let listed: std::collections::HashSet<Uuid> = registry
            .list_undescribed_packages(10)
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(listed, std::collections::HashSet::from([ok_id, bad_id]));
        assert_eq!(
            registry.list_undescribed_packages(1).await.unwrap().len(),
            1
        );
        assert_eq!(registry.build_queue_stats().await.unwrap().undescribed, 2);

        // Neither write touches a row that is not built.
        assert!(
            !registry
                .store_package_description(pending_id, &sample_document("wf"))
                .await
                .unwrap()
        );
        assert!(
            !registry
                .store_describe_error(pending_id, "not built")
                .await
                .unwrap()
        );

        assert!(
            registry
                .store_package_description(ok_id, &sample_document("old_ok_workflow"))
                .await
                .unwrap()
        );
        assert!(
            registry
                .store_describe_error(bad_id, "the library does not load")
                .await
                .unwrap()
        );

        let ok = registry
            .get_package_description(ok_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            ok.document.expect("document").workflow_name,
            "old_ok_workflow"
        );
        assert!(ok.describe_error.is_none());

        let bad = registry
            .get_package_description(bad_id)
            .await
            .unwrap()
            .unwrap();
        assert!(bad.document.is_none());
        assert_eq!(bad.version, Some(PACKAGE_DESCRIPTION_VERSION));
        assert_eq!(
            bad.describe_error.as_deref(),
            Some("the library does not load")
        );

        // Both stay in service; the mark is on the inspect and versions reads.
        for (name, id) in [("old-ok", ok_id), ("old-bad", bad_id)] {
            assert_eq!(
                row_state(&registry, name, id).await,
                (false, "success".to_string())
            );
        }
        let inspected = registry
            .inspect_package_by_id(bad_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            inspected.describe_error.as_deref(),
            Some("the library does not load")
        );
        let versions = registry.list_package_versions("old-bad").await.unwrap();
        assert_eq!(
            versions[0].describe_error.as_deref(),
            Some("the library does not load")
        );

        assert!(
            registry
                .list_undescribed_packages(10)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(registry.build_queue_stats().await.unwrap().undescribed, 0);

        // A new build of the row with no document makes it "not described"
        // again: the document of the earlier bytes is gone.
        registry.request_recompile(ok_id).await.unwrap();
        registry.claim_next_build().await.unwrap().expect("claim");
        registry
            .mark_build_success(ok_id, Vec::new())
            .await
            .unwrap();
        let rebuilt = registry
            .get_package_description(ok_id)
            .await
            .unwrap()
            .unwrap();
        assert!(rebuilt.document.is_none() && rebuilt.version.is_none());
    }
}
