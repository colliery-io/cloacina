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

use super::models::{NewUnifiedWorkflowPackage, UnifiedWorkflowPackage};
use super::DAL;
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

    /// Delete package metadata from the database.
    pub async fn delete_package_metadata(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        let package_name = package_name.to_string();
        let version = version.to_string();

        crate::interact_on_backend!(self.dal, |conn| {
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

    /// Delete package metadata by UUID from the database.
    pub async fn delete_package_metadata_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        let id = UniversalUuid(package_id);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(workflow_packages::table.filter(workflow_packages::id.eq(id)))
                .execute(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(())
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
    /// `(package, target_triple)`, or `None` if there's no build for that target.
    /// Dispatch prefers this over the primary build so each agent gets its arch.
    pub async fn get_artifact_digest_for_target(
        &self,
        package_name: &str,
        target_triple: &str,
    ) -> Result<Option<String>, RegistryError> {
        use crate::database::schema::unified::package_artifacts;
        let (pn, tt) = (package_name.to_string(), target_triple.to_string());
        crate::interact_on_backend!(self.dal, |conn| {
            package_artifacts::table
                .filter(package_artifacts::package_name.eq(pn))
                .filter(package_artifacts::target_triple.eq(tt))
                .select(package_artifacts::content_hash)
                .first::<String>(conn)
                .optional()
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
    }

    /// CLOACI-T-0905 (multi-arch, version-scoped): the cdylib bytes of the
    /// per-target artifact for `(package, version, target_triple)`, or `None` when
    /// no build exists for that target. Unlike
    /// [`get_artifact_digest_for_target`](Self::get_artifact_digest_for_target)
    /// (which serves fleet dispatch and is version-agnostic), the reconciler loads
    /// a SPECIFIC package version, so the version must participate in selection —
    /// otherwise an old version's artifact could satisfy a new version's load.
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

    /// CLOACI-T-0780: the set of target triples this package has a per-target
    /// artifact for. The fleet uses this (∪ the host primary) to only dispatch a
    /// package to an agent whose arch it actually has a cdylib for — so an agent
    /// is never handed a WorkPacket it would fail-closed refuse.
    pub async fn get_artifact_triples_for_package(
        &self,
        package_name: &str,
    ) -> Result<Vec<String>, RegistryError> {
        use crate::database::schema::unified::package_artifacts;
        let pn = package_name.to_string();
        crate::interact_on_backend!(self.dal, |conn| {
            package_artifacts::table
                .filter(package_artifacts::package_name.eq(pn))
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
        };
        let (pn, ver, prov) = (
            package_name.to_string(),
            version.to_string(),
            provider_name.to_string(),
        );
        let tid = tenant_id.map(|s| s.to_string());
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let mut del = diesel::delete(package_providers::table)
                    .filter(package_providers::package_name.eq(&pn))
                    .filter(package_providers::version.eq(&ver))
                    .filter(package_providers::provider_name.eq(&prov))
                    .into_boxed();
                del = match &tid {
                    Some(t) => del.filter(package_providers::tenant_id.eq(t.clone())),
                    None => del.filter(package_providers::tenant_id.is_null()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
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
}
