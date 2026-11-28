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

//! Unified Workflow Packages DAL with runtime backend selection
//!
//! This module provides CRUD operations for WorkflowPackage entities that work with
//! both PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.

use super::DAL;
use super::models::{NewUnifiedWorkflowPackage, UnifiedWorkflowPackage};
use crate::database::schema::unified::{workflow_packages, workflow_registry};
use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
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
        storage_type: crate::models::workflow_packages::StorageType,
        tenant_id: Option<&str>,
    ) -> Result<Uuid, RegistryError> {
        let registry_uuid = Uuid::parse_str(registry_id).map_err(RegistryError::InvalidUuid)?;
        let metadata =
            serde_json::to_string(package_metadata).map_err(RegistryError::Serialization)?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let tenant_id_owned = tenant_id.map(|s| s.to_string());
        let new_unified = NewUnifiedWorkflowPackage {
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
            tenant_id: tenant_id_owned,
            content_hash: String::new(),
            superseded: UniversalBool(false),
            compiled_data: None,
            build_status: "pending".to_string(),
            build_error: None,
            build_claimed_at: None,
            compiled_at: None,
        };

        let package_name_clone = package_metadata.package_name.clone();
        let version_clone = package_metadata.version.clone();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(workflow_packages::table)
                .values(&new_unified)
                .execute(conn)
        })
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
        let package_name = package_name.to_string();
        let version = version.to_string();

        let result: Option<UnifiedWorkflowPackage> =
            crate::interact_on_backend!(self.dal, |conn| {
                workflow_packages::table
                    .filter(workflow_packages::package_name.eq(&package_name))
                    .filter(workflow_packages::version.eq(&version))
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = result {
            let metadata: PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((record.registry_id.0.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    /// Retrieve package metadata by UUID from the database.
    pub async fn get_package_metadata_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        let id = UniversalUuid(package_id);
        let result: Option<UnifiedWorkflowPackage> =
            crate::interact_on_backend!(self.dal, |conn| {
                workflow_packages::table
                    .filter(workflow_packages::id.eq(id))
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = result {
            let metadata: PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((record.registry_id.0.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    /// List all packages in the registry.
    pub async fn list_all_packages(&self) -> Result<Vec<WorkflowPackage>, RegistryError> {
        let results: Vec<UnifiedWorkflowPackage> = crate::interact_on_backend!(self.dal, |conn| {
            workflow_packages::table.load::<UnifiedWorkflowPackage>(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Write the version pin of an execution (CLOACINA-T-1018) in a
    /// transaction that first takes a share lock on the pinned version's row
    /// (CLOACINA-T-1130), so a delete of that version cannot run between its
    /// in-use count and its delete with this pin unseen. Returns `false`, and
    /// writes no pin, when the row is gone: a delete committed first.
    async fn write_pin_if_version_exists(
        &self,
        pin: crate::dal::unified::models::WorkflowExecutionPackagePin,
    ) -> Result<bool, RegistryError> {
        use crate::database::schema::unified::workflow_execution_package_pins as pins;

        #[derive(diesel::QueryableByName)]
        struct Found {
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            n: i64,
        }

        let lock = row_locks(self.dal);
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let present = if lock {
                    // `FOR SHARE` cannot go with an aggregate: lock the row,
                    // then count what was locked.
                    diesel::sql_query(
                        "SELECT COUNT(*) AS n FROM (SELECT id FROM workflow_packages \
                         WHERE package_name = $1 AND version = $2 FOR SHARE) AS locked",
                    )
                    .bind::<diesel::sql_types::Text, _>(&pin.package_name)
                    .bind::<diesel::sql_types::Text, _>(&pin.version)
                    .get_result::<Found>(conn)?
                    .n > 0
                } else {
                    true
                };
                if !present {
                    return Ok(false);
                }
                diesel::insert_into(pins::table)
                    .values(&pin)
                    .on_conflict_do_nothing()
                    .execute(conn)?;
                Ok(true)
            })
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
    }

    /// Delete a package version: its `workflow_packages` row and what was built
    /// for it, the `package_artifacts` rows (a library per target, a vendored
    /// Python set per platform) and the `package_providers` rows, in one
    /// transaction (CLOACINA-T-1098).
    ///
    /// Refused with [`RegistryError::PackageInUse`], and nothing removed, while
    /// an execution that has not finished has a version pin on it
    /// (CLOACINA-T-1018): its later tasks are dispatched from this row and its
    /// artifacts. A pin of a finished execution, or of an execution that
    /// retention removed (CLOACINA-T-1023), does not count.
    pub async fn delete_package_metadata(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        use crate::dal::unified::retention::TERMINAL_EXECUTION_STATUSES;
        use crate::database::schema::unified::{
            package_artifacts, package_providers, workflow_execution_package_pins as pins,
            workflow_executions,
        };

        let (pn, ver) = (package_name.to_string(), version.to_string());
        let lock = row_locks(self.dal);
        let in_use: i64 = crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                // CLOACINA-T-1130: lock the version's row before the count. A
                // pin write takes a share lock on it first, so a pin is either
                // committed before this count or written after the delete,
                // when the row is gone and the write makes no pin.
                if lock {
                    diesel::sql_query(
                        "SELECT id FROM workflow_packages \
                         WHERE package_name = $1 AND version = $2 FOR UPDATE",
                    )
                    .bind::<diesel::sql_types::Text, _>(&pn)
                    .bind::<diesel::sql_types::Text, _>(&ver)
                    .execute(conn)?;
                }
                let pinned: Vec<UniversalUuid> = pins::table
                    .filter(pins::package_name.eq(&pn))
                    .filter(pins::version.eq(&ver))
                    .select(pins::workflow_execution_id)
                    .load(conn)?;
                // Pins of finished executions stay (nothing deletes them), so
                // the ids are read in chunks to keep under the bind limits.
                let mut in_use: i64 = 0;
                for chunk in pinned.chunks(500) {
                    in_use += workflow_executions::table
                        .filter(workflow_executions::id.eq_any(chunk.to_vec()))
                        .filter(
                            workflow_executions::status
                                .ne_all(TERMINAL_EXECUTION_STATUSES.to_vec()),
                        )
                        .count()
                        .get_result::<i64>(conn)?;
                }
                if in_use > 0 {
                    return Ok(in_use);
                }
                diesel::delete(
                    package_artifacts::table
                        .filter(package_artifacts::package_name.eq(&pn))
                        .filter(package_artifacts::version.eq(&ver)),
                )
                .execute(conn)?;
                diesel::delete(
                    package_providers::table
                        .filter(package_providers::package_name.eq(&pn))
                        .filter(package_providers::version.eq(&ver)),
                )
                .execute(conn)?;
                diesel::delete(
                    workflow_packages::table
                        .filter(workflow_packages::package_name.eq(&pn))
                        .filter(workflow_packages::version.eq(&ver)),
                )
                .execute(conn)?;
                Ok(0)
            })
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if in_use > 0 {
            return Err(RegistryError::PackageInUse {
                package_name: package_name.to_string(),
                version: version.to_string(),
                active_count: in_use as usize,
            });
        }
        Ok(())
    }

    /// Delete a package version by the UUID of its row, as
    /// [`Self::delete_package_metadata`] does. A row that is not there is not
    /// an error.
    pub async fn delete_package_metadata_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        let id = UniversalUuid(package_id);

        let key: Option<(String, String)> = crate::interact_on_backend!(self.dal, |conn| {
            workflow_packages::table
                .filter(workflow_packages::id.eq(id))
                .select((workflow_packages::package_name, workflow_packages::version))
                .first::<(String, String)>(conn)
                .optional()
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        match key {
            Some((name, version)) => self.delete_package_metadata(&name, &version).await,
            None => Ok(()),
        }
    }

    // ────────────────────────────────────────────────────────────────────
    // CLOACI-T-0631: agent fleet artifact fetch.
    // ────────────────────────────────────────────────────────────────────

    /// Look up a compiled package by its `content_hash` digest and return
    /// the raw cdylib bytes. Used by `GET /v1/agent/artifact/{digest}` so
    /// DB-less agents can fetch the artifact referenced in a work packet.
    ///
    /// Returns `Ok(None)` if no successful build matches the digest. The
    /// partial index `idx_wfp_content_hash_success` (postgres migration 022)
    /// keeps this lookup cheap on the hot path.
    pub async fn get_compiled_data_by_content_hash(
        &self,
        content_hash: &str,
    ) -> Result<Option<Vec<u8>>, RegistryError> {
        let content_hash_owned = content_hash.to_string();
        let result: Option<UnifiedWorkflowPackage> =
            crate::interact_on_backend!(self.dal, |conn| {
                workflow_packages::table
                    .filter(workflow_packages::content_hash.eq(&content_hash_owned))
                    .filter(workflow_packages::build_status.eq("success"))
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        let primary = result.and_then(|r| r.compiled_data.map(|b| b.into_inner()));
        if primary.is_some() {
            return Ok(primary);
        }
        // CLOACI-T-0780: the digest may name a per-target artifact (multi-arch),
        // not the primary build — fall back to package_artifacts.
        self.get_artifact_data_by_content_hash(content_hash).await
    }

    /// CLOACI-T-0780 (multi-arch): the content-hash of the per-target artifact for
    /// `(package, version, target_triple)`, or `None` if there's no build for
    /// that target. Dispatch prefers this over the primary build so each agent
    /// gets its arch.
    ///
    /// CLOACINA-T-1018: the version is part of the key. The caller passes the
    /// version of the row it dispatches (the active one, or the one an
    /// execution is pinned to), so an agent of a second architecture never gets
    /// the library of another version.
    pub async fn get_artifact_digest_for_target(
        &self,
        package_name: &str,
        version: &str,
        target_triple: &str,
    ) -> Result<Option<String>, RegistryError> {
        use crate::database::schema::unified::package_artifacts;
        let (pn, ver, tt) = (
            package_name.to_string(),
            version.to_string(),
            target_triple.to_string(),
        );
        crate::interact_on_backend!(self.dal, |conn| {
            package_artifacts::table
                .filter(package_artifacts::package_name.eq(pn))
                .filter(package_artifacts::version.eq(ver))
                .filter(package_artifacts::target_triple.eq(tt))
                .order(package_artifacts::created_at.desc())
                .select(package_artifacts::content_hash)
                .first::<String>(conn)
                .optional()
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
    }

    /// CLOACI-T-0905 (multi-arch, version-scoped): the cdylib bytes of the
    /// per-target artifact for `(package, version, target_triple)`, or `None` when
    /// no build exists for that target. The reconciler loads a SPECIFIC package
    /// version, so the version must participate in selection — otherwise an old
    /// version's artifact could satisfy a new version's load.
    pub async fn get_artifact_data_for_target(
        &self,
        package_name: &str,
        version: &str,
        target_triple: &str,
    ) -> Result<Option<Vec<u8>>, RegistryError> {
        use crate::database::schema::unified::package_artifacts;
        let (pn, ver, tt) = (
            package_name.to_string(),
            version.to_string(),
            target_triple.to_string(),
        );
        let bytes: Option<crate::database::universal_types::UniversalBinary> =
            crate::interact_on_backend!(self.dal, |conn| {
                package_artifacts::table
                    .filter(package_artifacts::package_name.eq(pn))
                    .filter(package_artifacts::version.eq(ver))
                    .filter(package_artifacts::target_triple.eq(tt))
                    .select(package_artifacts::compiled_data)
                    .first::<crate::database::universal_types::UniversalBinary>(conn)
                    .optional()
            })
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(bytes.map(|b| b.into_inner()))
    }

    /// CLOACI-T-0780: the set of target triples this package version has a
    /// per-target artifact for. The fleet uses this (∪ the host primary) to only
    /// dispatch a package to an agent whose arch it actually has a cdylib for —
    /// so an agent is never handed a WorkPacket it would fail-closed refuse.
    /// CLOACINA-T-1018: scoped to `version`, like the digest lookup.
    pub async fn get_artifact_triples_for_package(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Vec<String>, RegistryError> {
        use crate::database::schema::unified::package_artifacts;
        let (pn, ver) = (package_name.to_string(), version.to_string());
        crate::interact_on_backend!(self.dal, |conn| {
            package_artifacts::table
                .filter(package_artifacts::package_name.eq(pn))
                .filter(package_artifacts::version.eq(ver))
                .select(package_artifacts::target_triple)
                .distinct()
                .load::<String>(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
    }

    /// CLOACI-T-0780: cdylib bytes of a per-target artifact by content-hash — the
    /// fallback `get_compiled_data_by_content_hash` uses so the agent artifact
    /// endpoint can serve per-arch builds (not just the primary).
    pub async fn get_artifact_data_by_content_hash(
        &self,
        content_hash: &str,
    ) -> Result<Option<Vec<u8>>, RegistryError> {
        use crate::database::schema::unified::package_artifacts;
        let ch = content_hash.to_string();
        let bytes: Option<crate::database::universal_types::UniversalBinary> =
            crate::interact_on_backend!(self.dal, |conn| {
                package_artifacts::table
                    .filter(package_artifacts::content_hash.eq(ch))
                    .select(package_artifacts::compiled_data)
                    .first::<crate::database::universal_types::UniversalBinary>(conn)
                    .optional()
            })
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(bytes.map(|b| b.into_inner()))
    }

    /// CLOACI-T-0780: store (replace) the per-target artifact for
    /// `(package, version, tenant, target_triple)`. Called by a target-scoped
    /// compiler. Delete-then-insert keeps it idempotent across rebuilds.
    pub async fn upsert_artifact(
        &self,
        package_name: &str,
        version: &str,
        tenant_id: Option<&str>,
        target_triple: &str,
        content_hash: &str,
        compiled_data: Vec<u8>,
    ) -> Result<(), RegistryError> {
        use crate::database::schema::unified::package_artifacts;
        use crate::database::universal_types::{
            UniversalBinary, UniversalTimestamp, UniversalUuid,
        };
        let new = crate::dal::unified::models::NewPackageArtifact {
            id: UniversalUuid::new_v4(),
            package_name: package_name.to_string(),
            version: version.to_string(),
            tenant_id: tenant_id.map(|s| s.to_string()),
            target_triple: target_triple.to_string(),
            content_hash: content_hash.to_string(),
            compiled_data: UniversalBinary::new(compiled_data),
            created_at: UniversalTimestamp::now(),
        };
        let (pn, ver, tt) = (
            package_name.to_string(),
            version.to_string(),
            target_triple.to_string(),
        );
        let tid = tenant_id.map(|s| s.to_string());
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let mut del = diesel::delete(package_artifacts::table)
                    .filter(package_artifacts::package_name.eq(&pn))
                    .filter(package_artifacts::version.eq(&ver))
                    .filter(package_artifacts::target_triple.eq(&tt))
                    .into_boxed();
                del = match &tid {
                    Some(t) => del.filter(package_artifacts::tenant_id.eq(t.clone())),
                    None => del.filter(package_artifacts::tenant_id.is_null()),
                };
                del.execute(conn)?;
                diesel::insert_into(package_artifacts::table)
                    .values(&new)
                    .execute(conn)?;
                Ok(())
            })
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
    }

    /// CLOACINA-T-1073: make the vendored sets of a Python package version be
    /// exactly `sets` (platform, digest, archive). One row per platform, under
    /// the key `python-vendor:<platform>`. The rows of the version that are not
    /// in `sets` are removed, so a build with no vendored set (the package
    /// ships `vendor/`, or has no dependencies) leaves none behind.
    pub async fn replace_python_vendor_artifacts(
        &self,
        package_name: &str,
        version: &str,
        sets: Vec<(String, String, Vec<u8>)>,
    ) -> Result<(), RegistryError> {
        use crate::database::schema::unified::package_artifacts;
        use crate::database::universal_types::UniversalBinary;
        use cloacina_workflow_plugin::python_vendor::{
            VENDOR_ARTIFACT_PREFIX, vendor_artifact_key,
        };
        let rows: Vec<crate::dal::unified::models::NewPackageArtifact> = sets
            .into_iter()
            .map(
                |(platform, digest, data)| crate::dal::unified::models::NewPackageArtifact {
                    id: UniversalUuid::new_v4(),
                    package_name: package_name.to_string(),
                    version: version.to_string(),
                    tenant_id: None,
                    target_triple: vendor_artifact_key(&platform),
                    content_hash: digest,
                    compiled_data: UniversalBinary::new(data),
                    created_at: UniversalTimestamp::now(),
                },
            )
            .collect();
        let (pn, ver) = (package_name.to_string(), version.to_string());
        let pattern = format!("{VENDOR_ARTIFACT_PREFIX}%");
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                diesel::delete(package_artifacts::table)
                    .filter(package_artifacts::package_name.eq(&pn))
                    .filter(package_artifacts::version.eq(&ver))
                    .filter(package_artifacts::target_triple.like(&pattern))
                    .execute(conn)?;
                for row in &rows {
                    diesel::insert_into(package_artifacts::table)
                        .values(row)
                        .execute(conn)?;
                }
                Ok(())
            })
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
    }

    /// CLOACINA-T-1073: the platforms that a Python package version has a
    /// vendored set for, sorted.
    pub async fn python_vendor_platforms(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Vec<String>, RegistryError> {
        use crate::database::schema::unified::package_artifacts;
        use cloacina_workflow_plugin::python_vendor::{
            VENDOR_ARTIFACT_PREFIX, vendor_artifact_platform,
        };
        let (pn, ver) = (package_name.to_string(), version.to_string());
        let pattern = format!("{VENDOR_ARTIFACT_PREFIX}%");
        let keys: Vec<String> = crate::interact_on_backend!(self.dal, |conn| {
            package_artifacts::table
                .filter(package_artifacts::package_name.eq(pn))
                .filter(package_artifacts::version.eq(ver))
                .filter(package_artifacts::target_triple.like(pattern))
                .select(package_artifacts::target_triple)
                .load::<String>(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        let mut platforms: Vec<String> = keys
            .iter()
            .filter_map(|k| vendor_artifact_platform(k).map(str::to_string))
            .collect();
        platforms.sort();
        platforms.dedup();
        Ok(platforms)
    }

    /// CLOACINA-T-1073: the vendored set of a Python package version for
    /// `platform`, or `None` when the compiler stored none for it. The set of
    /// another platform never serves: environment markers make the sets differ
    /// even when no wheel holds compiled code.
    pub async fn get_python_vendor_artifact(
        &self,
        package_name: &str,
        version: &str,
        platform: &str,
    ) -> Result<Option<Vec<u8>>, RegistryError> {
        use cloacina_workflow_plugin::python_vendor::vendor_artifact_key;
        self.get_artifact_data_for_target(package_name, version, &vendor_artifact_key(platform))
            .await
    }

    /// CLOACINA-T-1073: write `python_platforms` into the stored metadata of a
    /// package row: the platforms a compiler vendored compiled code for, or
    /// `None` for any platform. Fleet dispatch reads it
    /// ([`DispatchPackage::python_platforms`]).
    pub async fn set_python_platforms(
        &self,
        package_id: Uuid,
        platforms: Option<Vec<String>>,
    ) -> Result<(), RegistryError> {
        let pid = UniversalUuid(package_id);
        let stored: Option<String> = crate::interact_on_backend!(self.dal, |conn| {
            workflow_packages::table
                .filter(workflow_packages::id.eq(pid))
                .select(workflow_packages::metadata)
                .first::<String>(conn)
                .optional()
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        let Some(stored) = stored else {
            return Ok(());
        };
        let mut metadata: PackageMetadata =
            serde_json::from_str(&stored).map_err(RegistryError::Serialization)?;
        if metadata.python_platforms == platforms {
            return Ok(());
        }
        metadata.python_platforms = platforms;
        let json = serde_json::to_string(&metadata).map_err(RegistryError::Serialization)?;
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
                .set(workflow_packages::metadata.eq(json))
                .execute(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(())
    }

    /// CLOACI-T-0836: upsert one bundled constructor provider for a consumer
    /// package. `provider_data` is the provider's packed `.cloacina` archive
    /// (arch-neutral WASM) that the reconciler unpacks into a `providers/` tree at
    /// load. Delete+insert in a transaction (mirrors [`Self::upsert_artifact`]) so
    /// a rebuild replaces the previous bundle for the same (package, provider).
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_provider(
        &self,
        package_name: &str,
        version: &str,
        tenant_id: Option<&str>,
        provider_name: &str,
        provider_version: &str,
        content_hash: &str,
        provider_data: Vec<u8>,
        runtime: &str,
        target_triple: Option<&str>,
    ) -> Result<(), RegistryError> {
        use crate::database::schema::unified::package_providers;
        use crate::database::universal_types::{
            UniversalBinary, UniversalTimestamp, UniversalUuid,
        };
        let new = crate::dal::unified::models::NewPackageProvider {
            id: UniversalUuid::new_v4(),
            package_name: package_name.to_string(),
            version: version.to_string(),
            tenant_id: tenant_id.map(|s| s.to_string()),
            provider_name: provider_name.to_string(),
            provider_version: provider_version.to_string(),
            content_hash: content_hash.to_string(),
            provider_data: UniversalBinary::new(provider_data),
            created_at: UniversalTimestamp::now(),
            target_triple: target_triple.map(|s| s.to_string()),
            runtime: runtime.to_string(),
        };
        let (pn, ver, prov) = (
            package_name.to_string(),
            version.to_string(),
            provider_name.to_string(),
        );
        let tid = tenant_id.map(|s| s.to_string());
        let tt = target_triple.map(|s| s.to_string());
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                // CLOACI-T-0908: the delete-filter is TRIPLE-scoped so the
                // primary (NULL) row and per-arch rows COEXIST — previously a
                // second-arch per-target compile clobbered the primary build.
                let mut del = diesel::delete(package_providers::table)
                    .filter(package_providers::package_name.eq(&pn))
                    .filter(package_providers::version.eq(&ver))
                    .filter(package_providers::provider_name.eq(&prov))
                    .into_boxed();
                del = match &tid {
                    Some(t) => del.filter(package_providers::tenant_id.eq(t.clone())),
                    None => del.filter(package_providers::tenant_id.is_null()),
                };
                del = match &tt {
                    Some(t) => del.filter(package_providers::target_triple.eq(t.clone())),
                    None => del.filter(package_providers::target_triple.is_null()),
                };
                del.execute(conn)?;
                diesel::insert_into(package_providers::table)
                    .values(&new)
                    .execute(conn)?;
                Ok(())
            })
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
    }

    /// CLOACI-T-0836: every bundled provider row for a consumer package (the set
    /// the reconciler unpacks into `providers/` before resolving `constructor!`
    /// nodes). Empty for packages that use no constructors.
    pub async fn get_providers_for_package(
        &self,
        package_name: &str,
        version: &str,
        tenant_id: Option<&str>,
    ) -> Result<Vec<crate::dal::unified::models::PackageProvider>, RegistryError> {
        use crate::database::schema::unified::package_providers;
        let (pn, ver) = (package_name.to_string(), version.to_string());
        let tid = tenant_id.map(|s| s.to_string());
        crate::interact_on_backend!(self.dal, |conn| {
            let mut q = package_providers::table
                .filter(package_providers::package_name.eq(&pn))
                .filter(package_providers::version.eq(&ver))
                .into_boxed();
            q = match &tid {
                Some(t) => q.filter(package_providers::tenant_id.eq(t.clone())),
                None => q.filter(package_providers::tenant_id.is_null()),
            };
            q.select(crate::dal::unified::models::PackageProvider::as_select())
                .load(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
    }

    /// CLOACI-T-0908 (producer): active (success, non-superseded) packages whose
    /// PRIMARY rows include a NATIVE provider but which lack a per-arch provider
    /// row for `target_triple` — the provider twin of the artifact scan below and
    /// [`find_packages_missing_target_artifact`](Self::find_packages_missing_target_artifact).
    /// The per-target compiler unions this with the artifact scan; rebuilding the
    /// package (from retained source, on the target arch) re-bundles its native
    /// providers and stores triple-keyed rows. WASM providers are arch-neutral and
    /// never appear here.
    pub async fn find_packages_missing_target_provider(
        &self,
        target_triple: &str,
        tenant_id: Option<&str>,
        name_filter: Option<&str>,
    ) -> Result<
        Vec<(
            crate::database::universal_types::UniversalUuid,
            String,
            String,
        )>,
        RegistryError,
    > {
        use crate::database::schema::unified::{package_providers, workflow_packages};
        use crate::database::universal_types::{UniversalBool, UniversalUuid};

        let (tid, nf, tt) = (
            tenant_id.map(|s| s.to_string()),
            name_filter.map(|s| s.to_string()),
            target_triple.to_string(),
        );
        // 1. Active success packages (optionally one name) in tenant scope.
        let success: Vec<(UniversalUuid, String, String)> =
            crate::interact_on_backend!(self.dal, |conn| {
                let mut q = workflow_packages::table
                    .filter(workflow_packages::build_status.eq("success"))
                    .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                    .into_boxed();
                q = match &tid {
                    Some(t) => q.filter(workflow_packages::tenant_id.eq(t.clone())),
                    None => q.filter(workflow_packages::tenant_id.is_null()),
                };
                if let Some(n) = &nf {
                    q = q.filter(workflow_packages::package_name.eq(n.clone()));
                }
                q.select((
                    workflow_packages::id,
                    workflow_packages::package_name,
                    workflow_packages::version,
                ))
                .load(conn)
            })
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        // 2. Keep packages with a primary NATIVE provider row missing its
        //    per-arch twin. Row counts are tiny; filter in Rust for clarity.
        let mut missing = Vec::new();
        for (id, name, version) in success {
            let (pn, ver, tt2) = (name.clone(), version.clone(), tt.clone());
            let rows: Vec<(String, Option<String>, String)> =
                crate::interact_on_backend!(self.dal, |conn| {
                    package_providers::table
                        .filter(package_providers::package_name.eq(&pn))
                        .filter(package_providers::version.eq(&ver))
                        .filter(package_providers::tenant_id.is_null())
                        .filter(
                            package_providers::target_triple
                                .is_null()
                                .or(package_providers::target_triple.eq(&tt2)),
                        )
                        .select((
                            package_providers::provider_name,
                            package_providers::target_triple,
                            package_providers::runtime,
                        ))
                        .load(conn)
                })
                .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
            let needs = rows.iter().any(|(prov, triple, runtime)| {
                triple.is_none()
                    && runtime == "native"
                    && !rows
                        .iter()
                        .any(|(p2, t2, _)| p2 == prov && t2.as_deref() == Some(tt.as_str()))
            });
            if needs {
                missing.push((id, name, version));
            }
        }
        Ok(missing)
    }

    /// CLOACI-T-0780 (producer): active (success, non-superseded) packages in this
    /// tenant scope that LACK a per-target artifact for `target_triple` — the set a
    /// `--build-target` compiler scan-and-fills. `name_filter` limits the scan to a
    /// single package (keeps the emulated demo build cheap). Returns (id, name,
    /// version) so the caller can `execute_build(id)` then `upsert_artifact`.
    pub async fn find_packages_missing_target_artifact(
        &self,
        target_triple: &str,
        tenant_id: Option<&str>,
        name_filter: Option<&str>,
    ) -> Result<
        Vec<(
            crate::database::universal_types::UniversalUuid,
            String,
            String,
        )>,
        RegistryError,
    > {
        use crate::database::schema::unified::{package_artifacts, workflow_packages};
        use crate::database::universal_types::{UniversalBool, UniversalUuid};
        // 1. Active success packages (optionally a single name) in tenant scope.
        let (tid, nf) = (
            tenant_id.map(|s| s.to_string()),
            name_filter.map(|s| s.to_string()),
        );
        let success: Vec<(UniversalUuid, String, String)> =
            crate::interact_on_backend!(self.dal, |conn| {
                let mut q = workflow_packages::table
                    .filter(workflow_packages::build_status.eq("success"))
                    .filter(workflow_packages::superseded.eq(UniversalBool(false)))
                    .into_boxed();
                q = match &tid {
                    Some(t) => q.filter(workflow_packages::tenant_id.eq(t.clone())),
                    None => q.filter(workflow_packages::tenant_id.is_null()),
                };
                if let Some(n) = &nf {
                    q = q.filter(workflow_packages::package_name.eq(n.clone()));
                }
                q.select((
                    workflow_packages::id,
                    workflow_packages::package_name,
                    workflow_packages::version,
                ))
                .load::<(UniversalUuid, String, String)>(conn)
            })
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        // 2. (package_name, version) that already have an artifact for this triple.
        let tt = target_triple.to_string();
        let existing: Vec<(String, String)> = crate::interact_on_backend!(self.dal, |conn| {
            package_artifacts::table
                .filter(package_artifacts::target_triple.eq(tt))
                .select((package_artifacts::package_name, package_artifacts::version))
                .load::<(String, String)>(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        let have: std::collections::HashSet<(String, String)> = existing.into_iter().collect();
        Ok(success
            .into_iter()
            .filter(|(_, n, v)| !have.contains(&(n.clone(), v.clone())))
            .collect())
    }

    /// Resolve the **active** content-hash (digest) for a package name within a
    /// tenant scope — the `FleetExecutor` (CLOACI-T-0633) uses this to build a
    /// work packet's `ArtifactRef` from the task's package.
    ///
    /// "Active" = `build_status = 'success'` AND `NOT superseded`. Both filters
    /// are load-bearing: dropping `superseded` would route a stale cdylib to
    /// agents after a re-upload; dropping the status filter would point at an
    /// unbuilt/failed row. Returns `Ok(None)` if no active row matches.
    pub async fn get_active_content_hash_for_package(
        &self,
        package_name: &str,
        tenant_id: Option<&str>,
    ) -> Result<Option<String>, RegistryError> {
        let package_name = package_name.to_string();
        let tenant_id = tenant_id.map(|s| s.to_string());
        let result: Option<UnifiedWorkflowPackage> =
            crate::interact_on_backend!(self.dal, |conn| {
                let base = workflow_packages::table
                    .filter(workflow_packages::package_name.eq(package_name))
                    .filter(workflow_packages::build_status.eq("success"))
                    .filter(
                        workflow_packages::superseded
                            .eq(crate::database::universal_types::UniversalBool(false)),
                    );
                match tenant_id {
                    Some(t) => base
                        .filter(workflow_packages::tenant_id.eq(t))
                        .first::<UnifiedWorkflowPackage>(conn)
                        .optional(),
                    None => base
                        .filter(workflow_packages::tenant_id.is_null())
                        .first::<UnifiedWorkflowPackage>(conn)
                        .optional(),
                }
            })
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(result.map(|r| r.content_hash))
    }

    // ────────────────────────────────────────────────────────────────────
    // CLOACI-T-0716: agent fleet — Python source fetch + language dispatch.
    // ────────────────────────────────────────────────────────────────────

    /// `(content_hash, language)` for the **active** build of a package, where
    /// `language` is `"python"` when the build produced no cdylib
    /// (`compiled_data` empty — the compiler short-circuits pure-Python packages
    /// to an empty artifact) and `"rust"` otherwise. The `FleetExecutor` stamps
    /// `language` into the `WorkPacket` so the agent loads the package the right
    /// way (dlopen vs PyO3 import). Same active-row filter as
    /// `get_active_content_hash_for_package`.
    pub async fn get_active_dispatch_for_package(
        &self,
        package_name: &str,
        tenant_id: Option<&str>,
    ) -> Result<Option<(String, String)>, RegistryError> {
        let row = self
            .get_active_row_for_package(package_name, tenant_id)
            .await?;
        Ok(row.map(|r| {
            let UnifiedWorkflowPackage {
                content_hash,
                compiled_data,
                ..
            } = r;
            let has_cdylib = compiled_data
                .map(|b| !b.into_inner().is_empty())
                .unwrap_or(false);
            let language = if has_cdylib { "rust" } else { "python" }.to_string();
            (content_hash, language)
        }))
    }

    /// The active build of a package as a [`DispatchPackage`]: the row id and
    /// version together with the digest and the language. Same active-row
    /// filter as `get_active_content_hash_for_package`. (CLOACINA-T-1018)
    pub async fn get_active_dispatch(
        &self,
        package_name: &str,
        tenant_id: Option<&str>,
    ) -> Result<Option<DispatchPackage>, RegistryError> {
        let row = self
            .get_active_row_for_package(package_name, tenant_id)
            .await?;
        Ok(row.map(DispatchPackage::from_row))
    }

    /// The package version that a task of `workflow_execution_id` runs on
    /// (CLOACINA-T-1018).
    ///
    /// The rule: an execution runs all its tasks on one version of a package.
    /// The first task of the execution that is dispatched pins the version that
    /// is active at that moment (a row in `workflow_execution_package_pins`).
    /// Each later task reads the pin and gets that version, also after a newer
    /// version became active and the pinned one is superseded.
    ///
    /// Returns `Ok(None)` when there is no pin and no active version. Returns
    /// an error when the pinned version is no longer built or was deleted: the
    /// execution does not move to another version.
    pub async fn resolve_dispatch_for_execution(
        &self,
        workflow_execution_id: UniversalUuid,
        package_name: &str,
        tenant_id: Option<&str>,
    ) -> Result<Option<DispatchPackage>, RegistryError> {
        use crate::dal::unified::models::WorkflowExecutionPackagePin;

        let pinned = match self
            .get_execution_pin(workflow_execution_id, package_name)
            .await?
        {
            Some(pin) => pin,
            None => {
                // A version deleted between the read of the active row and the
                // pin write is not pinned (CLOACINA-T-1130): read the active
                // row again.
                let mut tries = 0;
                let active = loop {
                    let Some(active) = self
                        .get_active_row_for_package(package_name, tenant_id)
                        .await?
                    else {
                        return Ok(None);
                    };
                    let pin = WorkflowExecutionPackagePin {
                        workflow_execution_id,
                        package_name: package_name.to_string(),
                        package_id: active.id,
                        version: active.version.clone(),
                        created_at: UniversalTimestamp::now(),
                    };
                    if self.write_pin_if_version_exists(pin).await? {
                        break active;
                    }
                    tries += 1;
                    if tries == 3 {
                        return Err(RegistryError::Internal(format!(
                            "the active version of package `{}` was deleted while execution {} \
                             was pinned to it, {} times",
                            package_name, workflow_execution_id.0, tries
                        )));
                    }
                };
                // Read back: another task of the execution may have pinned first.
                let stored = self
                    .get_execution_pin(workflow_execution_id, package_name)
                    .await?
                    .ok_or_else(|| {
                        RegistryError::Internal(format!(
                            "version pin of execution {} for package `{}` was not stored",
                            workflow_execution_id.0, package_name
                        ))
                    })?;
                if stored.package_id == active.id {
                    return Ok(Some(DispatchPackage::from_row(active)));
                }
                stored
            }
        };

        let package_id = pinned.package_id;
        let row: Option<UnifiedWorkflowPackage> = crate::interact_on_backend!(self.dal, |conn| {
            workflow_packages::table
                .filter(workflow_packages::id.eq(package_id))
                .filter(workflow_packages::build_status.eq("success"))
                .first::<UnifiedWorkflowPackage>(conn)
                .optional()
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        match row {
            Some(row) => Ok(Some(DispatchPackage::from_row(row))),
            None => Err(RegistryError::Internal(format!(
                "execution {} is pinned to version {} of package `{}`, which is no longer \
                 built or was deleted",
                workflow_execution_id.0, pinned.version, package_name
            ))),
        }
    }

    /// The version pin of an execution for a package, if one is recorded.
    pub async fn get_execution_pin(
        &self,
        workflow_execution_id: UniversalUuid,
        package_name: &str,
    ) -> Result<Option<crate::dal::unified::models::WorkflowExecutionPackagePin>, RegistryError>
    {
        use crate::dal::unified::models::WorkflowExecutionPackagePin;
        use crate::database::schema::unified::workflow_execution_package_pins as pins;
        let name = package_name.to_string();
        crate::interact_on_backend!(self.dal, |conn| {
            pins::table
                .filter(pins::workflow_execution_id.eq(workflow_execution_id))
                .filter(pins::package_name.eq(name))
                .select(WorkflowExecutionPackagePin::as_select())
                .first(conn)
                .optional()
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
    }

    async fn get_active_row_for_package(
        &self,
        package_name: &str,
        tenant_id: Option<&str>,
    ) -> Result<Option<UnifiedWorkflowPackage>, RegistryError> {
        let package_name = package_name.to_string();
        let tenant_id = tenant_id.map(|s| s.to_string());
        crate::interact_on_backend!(self.dal, |conn| {
            let base = workflow_packages::table
                .filter(workflow_packages::package_name.eq(package_name))
                .filter(workflow_packages::build_status.eq("success"))
                .filter(workflow_packages::superseded.eq(UniversalBool(false)));
            match tenant_id {
                Some(t) => base
                    .filter(workflow_packages::tenant_id.eq(t))
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional(),
                None => base
                    .filter(workflow_packages::tenant_id.is_null())
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional(),
            }
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
    }

    /// Source `.cloacina` archive bytes for the package whose active build has
    /// `content_hash == digest`. The registry (`workflow_registry.data`) holds
    /// the uploaded archive — for a Python package this is the importable
    /// `workflow/` + `vendor/` tree (`compiled_data` is empty). Served by
    /// `GET /v1/agent/source/{digest}` so a DB-less agent can fetch + import a
    /// Python package (CLOACI-T-0716).
    pub async fn get_package_archive_by_content_hash(
        &self,
        content_hash: &str,
    ) -> Result<Option<Vec<u8>>, RegistryError> {
        use crate::database::universal_types::UniversalBinary;
        let content_hash = content_hash.to_string();
        // Two single-table lookups (package row → registry_id → archive),
        // avoiding a cross-table join the unified schema doesn't wire up.
        let result: Option<UniversalBinary> = crate::interact_on_backend!(self.dal, |conn| {
            let pkg: Option<UnifiedWorkflowPackage> = workflow_packages::table
                .filter(workflow_packages::content_hash.eq(content_hash))
                .filter(workflow_packages::build_status.eq("success"))
                .first::<UnifiedWorkflowPackage>(conn)
                .optional()?;
            match pkg {
                Some(p) => workflow_registry::table
                    .filter(workflow_registry::id.eq(p.registry_id))
                    .select(workflow_registry::data)
                    .first::<UniversalBinary>(conn)
                    .optional(),
                None => Ok(None),
            }
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(result.map(|b| b.into_inner()))
    }

    /// Resolve a successfully-built package's `(package_name, version)` from its
    /// content hash. CLOACI-T-0838: agents identify packages by artifact digest
    /// (content-addressed), while `package_providers` is keyed by name+version —
    /// this is the bridge the `/v1/agent/providers/{digest}` route uses.
    pub async fn get_package_name_version_by_content_hash(
        &self,
        content_hash: &str,
    ) -> Result<Option<(String, String)>, RegistryError> {
        let content_hash = content_hash.to_string();
        let result: Option<(String, String)> = crate::interact_on_backend!(self.dal, |conn| {
            workflow_packages::table
                .filter(workflow_packages::content_hash.eq(content_hash))
                .filter(workflow_packages::build_status.eq("success"))
                .select((workflow_packages::package_name, workflow_packages::version))
                .first::<(String, String)>(conn)
                .optional()
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;
        Ok(result)
    }
}

/// What fleet dispatch needs to know about the package version a task runs on
/// (CLOACINA-T-1018).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchPackage {
    /// The `workflow_packages` row.
    pub package_id: Uuid,
    pub version: String,
    /// Digest of the primary build (the uploaded archive's content hash).
    pub content_hash: String,
    /// `"python"` when the build produced no cdylib, else `"rust"`.
    pub language: String,
    /// CLOACINA-T-1048: the platforms (`<arch>-<os>`, the form of an agent's
    /// `target_triple`) a Python package can run on. `None` means any
    /// platform: the package has no vendored dependencies with compiled code.
    /// Always `None` for a Rust package: its platforms are its per-target
    /// artifacts.
    pub python_platforms: Option<Vec<String>>,
    /// CLOACINA-T-1081: the names of the secrets this version declares, from
    /// its stored metadata document. `None` when the row has no document (not
    /// described yet, or the describe failed), or the document does not read.
    pub declared_secrets: Option<Vec<String>>,
}

impl DispatchPackage {
    fn from_row(row: UnifiedWorkflowPackage) -> Self {
        let has_cdylib = row
            .compiled_data
            .map(|b| !b.into_inner().is_empty())
            .unwrap_or(false);
        let python_platforms = if has_cdylib {
            None
        } else {
            python_platforms_of_metadata(&row.metadata)
        };
        let declared_secrets = row
            .metadata_document
            .as_deref()
            .and_then(declared_secrets_of_document);
        Self {
            package_id: row.id.0,
            version: row.version,
            content_hash: row.content_hash,
            language: if has_cdylib { "rust" } else { "python" }.to_string(),
            python_platforms,
            declared_secrets,
        }
    }
}

/// CLOACINA-T-1081: the `declared_secrets` of a stored metadata document.
/// `None` when the text is not a JSON object. A document with no
/// `declared_secrets` key declares none.
pub fn declared_secrets_of_document(document_json: &str) -> Option<Vec<String>> {
    let value: serde_json::Value = serde_json::from_str(document_json).ok()?;
    let object = value.as_object()?;
    Some(
        object
            .get("declared_secrets")
            .and_then(|v| v.as_array())
            .map(|list| {
                list.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
    )
}

/// CLOACINA-T-1048: the `python_platforms` of a stored `metadata` JSON.
/// `None` (any platform) when the key is absent, null, empty or not readable.
pub fn python_platforms_of_metadata(metadata_json: &str) -> Option<Vec<String>> {
    let value: serde_json::Value = serde_json::from_str(metadata_json).ok()?;
    let list: Vec<String> = value
        .get("python_platforms")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    if list.is_empty() { None } else { Some(list) }
}

/// CLOACI-T-0908 (consumer): pick the provider rows THIS host should stage from
/// the full row set of a package — for each `provider_name`, prefer the row whose
/// `target_triple` matches `target_triple` exactly (a per-arch NATIVE build),
/// else fall back to the primary (`target_triple` NULL) row. Rows for OTHER
/// triples are never returned. Pure function so the reconciler, the agent route,
/// and tests share one selection semantic.
pub fn select_provider_rows_for_target(
    rows: Vec<crate::dal::unified::models::PackageProvider>,
    target_triple: &str,
) -> Vec<crate::dal::unified::models::PackageProvider> {
    let mut names: Vec<String> = Vec::new();
    for r in &rows {
        if !names.contains(&r.provider_name) {
            names.push(r.provider_name.clone());
        }
    }
    names
        .into_iter()
        .filter_map(|name| {
            rows.iter()
                .find(|r| {
                    r.provider_name == name && r.target_triple.as_deref() == Some(target_triple)
                })
                .or_else(|| {
                    rows.iter()
                        .find(|r| r.provider_name == name && r.target_triple.is_none())
                })
                .cloned()
        })
        .collect()
}

/// `true` when the backend takes row locks (`FOR UPDATE`, `FOR SHARE`):
/// Postgres. SQLite runs one writer at a time and has no row locks.
fn row_locks(dal: &DAL) -> bool {
    #[cfg(feature = "postgres")]
    {
        matches!(dal.backend(), crate::database::BackendType::Postgres)
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = dal;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "sqlite")]
    use crate::database::Database;
    #[cfg(feature = "sqlite")]
    use crate::registry::loader::package_loader::{PackageMetadata, TaskMetadata};

    #[cfg(feature = "sqlite")]
    async fn unique_dal() -> DAL {
        let url = format!(
            "file:wfpkg_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
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
                local_id: "task1".to_string(),
                namespaced_id_template: "{tenant}.{package}.{workflow}.task1".to_string(),
                dependencies: vec![],
                description: "test task".to_string(),
                source_location: "test.rs:1".to_string(),
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

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_store_and_get_package_metadata() {
        let dal = unique_dal().await;
        let registry_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("my-package", "1.0.0");

        let pkg_id = dal
            .workflow_packages()
            .store_package_metadata(
                &registry_id,
                &meta,
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();

        assert_ne!(pkg_id, Uuid::nil());

        // Retrieve by name and version
        let result = dal
            .workflow_packages()
            .get_package_metadata("my-package", "1.0.0")
            .await
            .unwrap();
        assert!(result.is_some());
        let (reg_id, retrieved_meta) = result.unwrap();
        assert_eq!(reg_id, registry_id);
        assert_eq!(retrieved_meta.package_name, "my-package");
        assert_eq!(retrieved_meta.version, "1.0.0");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_package_metadata_not_found() {
        let dal = unique_dal().await;

        let result = dal
            .workflow_packages()
            .get_package_metadata("nonexistent", "0.0.0")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_package_metadata_by_id() {
        let dal = unique_dal().await;
        let registry_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("id-lookup", "2.0.0");

        let pkg_id = dal
            .workflow_packages()
            .store_package_metadata(
                &registry_id,
                &meta,
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();

        let result = dal
            .workflow_packages()
            .get_package_metadata_by_id(pkg_id)
            .await
            .unwrap();
        assert!(result.is_some());
        let (_, retrieved_meta) = result.unwrap();
        assert_eq!(retrieved_meta.package_name, "id-lookup");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_package_metadata_by_id_not_found() {
        let dal = unique_dal().await;

        let result = dal
            .workflow_packages()
            .get_package_metadata_by_id(Uuid::new_v4())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_all_packages() {
        let dal = unique_dal().await;
        let registry_id = Uuid::new_v4().to_string();

        // Initially empty
        let list = dal.workflow_packages().list_all_packages().await.unwrap();
        assert!(list.is_empty());

        // Store two packages
        let meta1 = sample_metadata("pkg-a", "1.0.0");
        let meta2 = sample_metadata("pkg-b", "1.0.0");
        dal.workflow_packages()
            .store_package_metadata(
                &registry_id,
                &meta1,
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();
        dal.workflow_packages()
            .store_package_metadata(
                &registry_id,
                &meta2,
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();

        let list = dal.workflow_packages().list_all_packages().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_package_metadata() {
        let dal = unique_dal().await;
        let registry_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("to-delete", "1.0.0");

        dal.workflow_packages()
            .store_package_metadata(
                &registry_id,
                &meta,
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();

        // Confirm it exists
        let result = dal
            .workflow_packages()
            .get_package_metadata("to-delete", "1.0.0")
            .await
            .unwrap();
        assert!(result.is_some());

        // Delete it
        dal.workflow_packages()
            .delete_package_metadata("to-delete", "1.0.0")
            .await
            .unwrap();

        // Confirm it is gone
        let result = dal
            .workflow_packages()
            .get_package_metadata("to-delete", "1.0.0")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_package_metadata_by_id() {
        let dal = unique_dal().await;
        let registry_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("delete-by-id", "1.0.0");

        let pkg_id = dal
            .workflow_packages()
            .store_package_metadata(
                &registry_id,
                &meta,
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();

        // Delete by ID
        dal.workflow_packages()
            .delete_package_metadata_by_id(pkg_id)
            .await
            .unwrap();

        // Confirm it is gone
        let result = dal
            .workflow_packages()
            .get_package_metadata_by_id(pkg_id)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_nonexistent_does_not_error() {
        let dal = unique_dal().await;

        // Deleting something that does not exist should succeed
        dal.workflow_packages()
            .delete_package_metadata("nonexistent", "0.0.0")
            .await
            .unwrap();

        dal.workflow_packages()
            .delete_package_metadata_by_id(Uuid::new_v4())
            .await
            .unwrap();
    }
    // ────────────────────────────────────────────────────────────────────
    // CLOACINA-T-1018: version pins and the per-target lookup.
    // ────────────────────────────────────────────────────────────────────

    /// Test helper: store a row and set its build state with plain updates.
    #[cfg(feature = "sqlite")]
    async fn store_built(
        dal: &DAL,
        name: &str,
        version: &str,
        hash: &str,
        compiled: Vec<u8>,
    ) -> Uuid {
        use crate::database::universal_types::UniversalBinary;
        let id = dal
            .workflow_packages()
            .store_package_metadata(
                &Uuid::new_v4().to_string(),
                &sample_metadata(name, version),
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();
        let (pid, hash) = (UniversalUuid(id), hash.to_string());
        crate::interact_on_backend!(dal, |conn| {
            diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
                .set((
                    workflow_packages::build_status.eq("success"),
                    workflow_packages::content_hash.eq(hash),
                    workflow_packages::compiled_data.eq(Some(UniversalBinary::new(compiled))),
                ))
                .execute(conn)
        })
        .unwrap();
        id
    }

    #[cfg(feature = "sqlite")]
    async fn set_superseded(dal: &DAL, id: Uuid, superseded: bool) {
        let pid = UniversalUuid(id);
        crate::interact_on_backend!(dal, |conn| {
            diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
                .set(workflow_packages::superseded.eq(UniversalBool(superseded)))
                .execute(conn)
        })
        .unwrap();
    }

    #[test]
    fn test_python_platforms_of_metadata() {
        assert_eq!(python_platforms_of_metadata("{}"), None);
        assert_eq!(
            python_platforms_of_metadata(r#"{"python_platforms":null}"#),
            None
        );
        assert_eq!(
            python_platforms_of_metadata(r#"{"python_platforms":[]}"#),
            None
        );
        assert_eq!(python_platforms_of_metadata("not json"), None);
        assert_eq!(
            python_platforms_of_metadata(
                r#"{"python_platforms":["aarch64-linux","x86_64-linux"]}"#
            ),
            Some(vec![
                "aarch64-linux".to_string(),
                "x86_64-linux".to_string()
            ])
        );
    }

    /// CLOACINA-T-1048: dispatch gets the platforms of a Python package with
    /// native vendored dependencies, and "any" for one without, and for a
    /// Rust package.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_dispatch_package_carries_python_platforms() {
        use crate::database::universal_types::UniversalBinary;
        let dal = unique_dal().await;
        let wp = dal.workflow_packages();

        async fn store(dal: &DAL, name: &str, platforms: Option<Vec<String>>, compiled: Vec<u8>) {
            let mut meta = sample_metadata(name, "1.0.0");
            meta.python_platforms = platforms;
            let id = dal
                .workflow_packages()
                .store_package_metadata(
                    &Uuid::new_v4().to_string(),
                    &meta,
                    crate::models::workflow_packages::StorageType::Database,
                    None,
                )
                .await
                .unwrap();
            let (pid, hash) = (UniversalUuid(id), format!("hash-{name}"));
            crate::interact_on_backend!(dal, |conn| {
                diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
                    .set((
                        workflow_packages::build_status.eq("success"),
                        workflow_packages::content_hash.eq(hash),
                        workflow_packages::compiled_data.eq(Some(UniversalBinary::new(compiled))),
                    ))
                    .execute(conn)
            })
            .unwrap();
        }

        let linux = vec!["aarch64-linux".to_string(), "x86_64-linux".to_string()];
        store(&dal, "py-native", Some(linux.clone()), Vec::new()).await;
        store(&dal, "py-pure", None, Vec::new()).await;
        store(&dal, "rust-pkg", Some(linux.clone()), vec![1, 2, 3]).await;

        let native = wp
            .get_active_dispatch("py-native", None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(native.language, "python");
        assert_eq!(native.python_platforms, Some(linux));

        let pure = wp
            .get_active_dispatch("py-pure", None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(pure.python_platforms, None);

        let rust = wp
            .get_active_dispatch("rust-pkg", None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(rust.language, "rust");
        assert_eq!(rust.python_platforms, None);
    }

    /// CLOACINA-T-1073: the vendored sets of a Python package version are
    /// stored one per platform, replaced as a whole, and never read as the
    /// library of a platform.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_python_vendor_artifacts_are_stored_per_platform() {
        let dal = unique_dal().await;
        let wp = dal.workflow_packages();
        let set = |p: &str, b: &[u8]| (p.to_string(), format!("digest-{p}"), b.to_vec());

        wp.replace_python_vendor_artifacts(
            "etl",
            "1.0.0",
            vec![set("x86_64-linux", b"x86"), set("aarch64-linux", b"arm")],
        )
        .await
        .unwrap();
        wp.replace_python_vendor_artifacts("etl", "2.0.0", vec![set("x86_64-linux", b"v2")])
            .await
            .unwrap();

        assert_eq!(
            wp.python_vendor_platforms("etl", "1.0.0").await.unwrap(),
            vec!["aarch64-linux".to_string(), "x86_64-linux".to_string()]
        );
        assert_eq!(
            wp.get_python_vendor_artifact("etl", "1.0.0", "aarch64-linux")
                .await
                .unwrap(),
            Some(b"arm".to_vec())
        );
        // Version is part of the key.
        assert_eq!(
            wp.get_python_vendor_artifact("etl", "2.0.0", "x86_64-linux")
                .await
                .unwrap(),
            Some(b"v2".to_vec())
        );
        // A platform with no set gets none: another platform's set never serves.
        assert_eq!(
            wp.get_python_vendor_artifact("etl", "2.0.0", "aarch64-linux")
                .await
                .unwrap(),
            None
        );
        // The row is not the library of the platform: the reconciler and
        // dispatch ask by the bare triple.
        assert_eq!(
            wp.get_artifact_data_for_target("etl", "1.0.0", "x86_64-linux")
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            wp.get_artifact_digest_for_target("etl", "1.0.0", "x86_64-linux")
                .await
                .unwrap(),
            None
        );

        // A library row of the same version is left alone by a replace.
        wp.upsert_artifact("etl", "1.0.0", None, "x86_64-linux", "lib", b"lib".to_vec())
            .await
            .unwrap();
        wp.replace_python_vendor_artifacts("etl", "1.0.0", vec![set("x86_64-linux", b"new")])
            .await
            .unwrap();
        assert_eq!(
            wp.python_vendor_platforms("etl", "1.0.0").await.unwrap(),
            vec!["x86_64-linux".to_string()]
        );
        assert_eq!(
            wp.get_python_vendor_artifact("etl", "1.0.0", "x86_64-linux")
                .await
                .unwrap(),
            Some(b"new".to_vec())
        );
        assert_eq!(
            wp.get_artifact_data_for_target("etl", "1.0.0", "x86_64-linux")
                .await
                .unwrap(),
            Some(b"lib".to_vec())
        );

        // A build with nothing to vendor removes the sets.
        wp.replace_python_vendor_artifacts("etl", "1.0.0", Vec::new())
            .await
            .unwrap();
        assert!(
            wp.python_vendor_platforms("etl", "1.0.0")
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            wp.python_vendor_platforms("etl", "2.0.0")
                .await
                .unwrap()
                .len(),
            1
        );
    }

    /// CLOACINA-T-1073: the compiler records where a vendored set with
    /// compiled code runs, and dispatch reads it.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_python_vendor_set_python_platforms_reaches_dispatch() {
        let dal = unique_dal().await;
        let wp = dal.workflow_packages();
        let id = wp
            .store_package_metadata(
                &Uuid::new_v4().to_string(),
                &sample_metadata("py-lock", "1.0.0"),
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();
        let pid = UniversalUuid(id);
        crate::interact_on_backend!(dal, |conn| {
            diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
                .set((
                    workflow_packages::build_status.eq("success"),
                    workflow_packages::content_hash.eq("hash-py-lock"),
                ))
                .execute(conn)
        })
        .unwrap();

        let linux = vec!["aarch64-linux".to_string(), "x86_64-linux".to_string()];
        wp.set_python_platforms(id, Some(linux.clone()))
            .await
            .unwrap();
        let got = wp
            .get_active_dispatch("py-lock", None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.python_platforms, Some(linux));

        wp.set_python_platforms(id, None).await.unwrap();
        let got = wp
            .get_active_dispatch("py-lock", None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.python_platforms, None);

        // A row that is gone is not an error.
        wp.set_python_platforms(Uuid::new_v4(), None).await.unwrap();
    }

    /// CLOACINA-T-1081: dispatch reads the declared secrets of the version
    /// from its stored metadata document; no document reads as `None`.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_declared_secrets_reach_dispatch() {
        let dal = unique_dal().await;
        let wp = dal.workflow_packages();
        let id = store_built(&dal, "declares", "1.0.0", "digest-d", vec![1]).await;
        assert_eq!(
            wp.get_active_dispatch("declares", None)
                .await
                .unwrap()
                .unwrap()
                .declared_secrets,
            None
        );

        let pid = UniversalUuid(id);
        let document = serde_json::json!({
            "format_version": 1,
            "declared_secrets": ["warehouse", "api"],
        })
        .to_string();
        crate::interact_on_backend!(dal, |conn| {
            diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
                .set(workflow_packages::metadata_document.eq(Some(document)))
                .execute(conn)
        })
        .unwrap();
        assert_eq!(
            wp.get_active_dispatch("declares", None)
                .await
                .unwrap()
                .unwrap()
                .declared_secrets,
            Some(vec!["warehouse".to_string(), "api".to_string()])
        );

        assert_eq!(declared_secrets_of_document("{}"), Some(Vec::new()));
        assert_eq!(declared_secrets_of_document("not json"), None);
    }

    /// An execution runs all its tasks on the version its first task was
    /// dispatched on. A new execution gets the new version.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_execution_stays_on_the_version_it_started_with() {
        let dal = unique_dal().await;
        let wp = dal.workflow_packages();
        let v1 = store_built(&dal, "pinned", "1.0.0", "digest-v1", vec![1]).await;

        let running = UniversalUuid::new_v4();
        let first = wp
            .resolve_dispatch_for_execution(running, "pinned", None)
            .await
            .unwrap()
            .expect("an active version");
        assert_eq!(first.package_id, v1);
        assert_eq!(first.version, "1.0.0");
        assert_eq!(first.content_hash, "digest-v1");
        assert_eq!(first.language, "rust");

        // Version 2 becomes active while the execution runs.
        set_superseded(&dal, v1, true).await;
        let v2 = store_built(&dal, "pinned", "2.0.0", "digest-v2", vec![2]).await;
        let active = wp
            .get_active_dispatch("pinned", None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(active.package_id, v2);

        // The next task of the running execution still gets version 1.
        let later = wp
            .resolve_dispatch_for_execution(running, "pinned", None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(later, first);
        // The artifact of the superseded version is still served by digest.
        assert_eq!(
            wp.get_compiled_data_by_content_hash("digest-v1")
                .await
                .unwrap(),
            Some(vec![1])
        );

        // An execution that starts now runs on version 2.
        let fresh = wp
            .resolve_dispatch_for_execution(UniversalUuid::new_v4(), "pinned", None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fresh.package_id, v2);
        assert_eq!(fresh.content_hash, "digest-v2");

        // Going back to version 1 does not move an execution pinned to 2.
        let on_v2 = UniversalUuid::new_v4();
        wp.resolve_dispatch_for_execution(on_v2, "pinned", None)
            .await
            .unwrap()
            .unwrap();
        set_superseded(&dal, v2, true).await;
        set_superseded(&dal, v1, false).await;
        let still = wp
            .resolve_dispatch_for_execution(on_v2, "pinned", None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(still.package_id, v2);
    }

    /// No pin and no active version: nothing to dispatch. A pinned version
    /// that was deleted is an error, not a move to another version.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_pin_with_no_active_version_and_with_deleted_version() {
        let dal = unique_dal().await;
        let wp = dal.workflow_packages();
        let exec = UniversalUuid::new_v4();

        // Only a pending row: no active version, and no pin is written.
        wp.store_package_metadata(
            &Uuid::new_v4().to_string(),
            &sample_metadata("gone", "0.1.0"),
            crate::models::workflow_packages::StorageType::Database,
            None,
        )
        .await
        .unwrap();
        assert!(
            wp.resolve_dispatch_for_execution(exec, "gone", None)
                .await
                .unwrap()
                .is_none()
        );
        assert!(wp.get_execution_pin(exec, "gone").await.unwrap().is_none());

        let v1 = store_built(&dal, "gone", "1.0.0", "digest-gone", Vec::new()).await;
        let pinned = wp
            .resolve_dispatch_for_execution(exec, "gone", None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            pinned.language, "python",
            "an empty build is a Python package"
        );
        let pin = wp.get_execution_pin(exec, "gone").await.unwrap().unwrap();
        assert_eq!(pin.package_id.0, v1);
        assert_eq!(pin.version, "1.0.0");

        wp.delete_package_metadata_by_id(v1).await.unwrap();
        let _v2 = store_built(&dal, "gone", "2.0.0", "digest-gone-2", Vec::new()).await;
        let err = wp
            .resolve_dispatch_for_execution(exec, "gone", None)
            .await
            .expect_err("the execution must not move to version 2");
        assert!(
            format!("{err}").contains("pinned to version 1.0.0"),
            "{err}"
        );
    }

    /// CLOACINA-T-1098: a delete of a version that an unfinished execution is
    /// pinned to is refused and removes nothing. Once the execution has
    /// finished, the delete removes the row, its `package_artifacts` rows
    /// (library per target, vendored Python sets) and its `package_providers`
    /// rows, and an agent fetch by any of its digests finds nothing.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_is_refused_while_pinned_and_then_removes_artifacts() {
        use crate::database::schema::unified::workflow_executions;
        use crate::models::workflow_execution::NewWorkflowExecution;

        let dal = unique_dal().await;
        let wp = dal.workflow_packages();
        let id = store_built(&dal, "t1098", "1.0.0", "digest-primary", vec![9]).await;
        wp.upsert_artifact(
            "t1098",
            "1.0.0",
            None,
            "aarch64-unknown-linux-gnu",
            "digest-lib",
            vec![1],
        )
        .await
        .unwrap();
        wp.replace_python_vendor_artifacts(
            "t1098",
            "1.0.0",
            vec![(
                "x86_64-linux".to_string(),
                "digest-vendor".to_string(),
                vec![2],
            )],
        )
        .await
        .unwrap();
        wp.upsert_provider(
            "t1098",
            "1.0.0",
            None,
            "prov",
            "0.1.0",
            "digest-provider",
            vec![3],
            "wasm",
            None,
        )
        .await
        .unwrap();

        // A running execution whose first task pinned this version.
        let exec = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "t1098".into(),
                workflow_version: "1.0".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap()
            .id;
        let pinned = wp
            .resolve_dispatch_for_execution(exec, "t1098", None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(pinned.package_id, id);

        let err = wp
            .delete_package_metadata("t1098", "1.0.0")
            .await
            .expect_err("a pinned version must not be deleted");
        match err {
            RegistryError::PackageInUse { active_count, .. } => assert_eq!(active_count, 1),
            other => panic!("expected PackageInUse, got {other:?}"),
        }
        // The by-id path is refused the same way.
        assert!(matches!(
            wp.delete_package_metadata_by_id(id).await,
            Err(RegistryError::PackageInUse { .. })
        ));
        // Nothing was removed.
        assert!(wp.get_package_metadata_by_id(id).await.unwrap().is_some());
        assert_eq!(
            wp.get_artifact_triples_for_package("t1098", "1.0.0")
                .await
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            wp.get_providers_for_package("t1098", "1.0.0", None)
                .await
                .unwrap()
                .len(),
            1
        );

        // The execution finishes: the delete goes through.
        let done = exec;
        crate::interact_on_backend!(dal, |conn| {
            diesel::update(workflow_executions::table.find(done))
                .set(workflow_executions::status.eq("Completed"))
                .execute(conn)
        })
        .unwrap();
        wp.delete_package_metadata_by_id(id).await.unwrap();

        assert!(wp.get_package_metadata_by_id(id).await.unwrap().is_none());
        assert!(
            wp.get_artifact_triples_for_package("t1098", "1.0.0")
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            wp.get_providers_for_package("t1098", "1.0.0", None)
                .await
                .unwrap()
                .is_empty()
        );
        // An agent fetch by the digest of the deleted version finds nothing.
        for digest in ["digest-primary", "digest-lib", "digest-vendor"] {
            assert!(
                wp.get_compiled_data_by_content_hash(digest)
                    .await
                    .unwrap()
                    .is_none(),
                "{digest} is still served"
            );
        }
        assert!(
            wp.get_python_vendor_artifact("t1098", "1.0.0", "x86_64-linux")
                .await
                .unwrap()
                .is_none()
        );
    }

    // CLOACINA-T-1130's Postgres race test (a pin cannot land between the
    // in-use check and the delete) needs a live Postgres: it is in
    // tests/integration/dal/workflow_packages.rs.

    /// The per-target lookups are for one version: the artifact of another
    /// version of the same package and triple is never returned.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_artifact_digest_for_target_is_version_scoped() {
        let dal = unique_dal().await;
        let wp = dal.workflow_packages();
        let triple = "aarch64-unknown-linux-gnu";

        // Version 1 has builds for two triples, version 2 for one. The row of
        // version 1 is stored first, so a lookup with no version returned it.
        wp.upsert_artifact("multi", "1.0.0", None, triple, "arm-v1", vec![1])
            .await
            .unwrap();
        wp.upsert_artifact(
            "multi",
            "1.0.0",
            None,
            "riscv64gc-unknown-linux-gnu",
            "rv-v1",
            vec![3],
        )
        .await
        .unwrap();
        wp.upsert_artifact("multi", "2.0.0", None, triple, "arm-v2", vec![2])
            .await
            .unwrap();

        assert_eq!(
            wp.get_artifact_digest_for_target("multi", "2.0.0", triple)
                .await
                .unwrap()
                .as_deref(),
            Some("arm-v2")
        );
        assert_eq!(
            wp.get_artifact_digest_for_target("multi", "1.0.0", triple)
                .await
                .unwrap()
                .as_deref(),
            Some("arm-v1")
        );
        // A version with no build for the triple: none, not another version's.
        assert!(
            wp.get_artifact_digest_for_target("multi", "3.0.0", triple)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            wp.get_artifact_digest_for_target("multi", "2.0.0", "riscv64gc-unknown-linux-gnu")
                .await
                .unwrap()
                .is_none()
        );

        let mut v1 = wp
            .get_artifact_triples_for_package("multi", "1.0.0")
            .await
            .unwrap();
        v1.sort();
        assert_eq!(
            v1,
            vec![
                triple.to_string(),
                "riscv64gc-unknown-linux-gnu".to_string()
            ]
        );
        assert_eq!(
            wp.get_artifact_triples_for_package("multi", "2.0.0")
                .await
                .unwrap(),
            vec![triple.to_string()]
        );
    }
}
