---
title: "cloacina::dal::unified::workflow_packages"
description: "Unified Workflow Packages DAL with runtime backend selection"
editUrl: false
sidebar:
  order: 94
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Unified Workflow Packages DAL with runtime backend selection

This module provides CRUD operations for WorkflowPackage entities that work with
both PostgreSQL and SQLite backends, selecting the appropriate implementation
at runtime based on the database connection type.

## Structs

### <span id="struct.WorkflowPackagesDAL"></span><code style="font-size: 0.85em">struct WorkflowPackagesDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for workflow package operations with runtime backend selection.

#### Methods

##### <span id="WorkflowPackagesDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

Creates a new WorkflowPackagesDAL instance.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="WorkflowPackagesDAL.store_package_metadata"></span><code>store_​package_​metadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn store_package_metadata(
    &self,
    registry_id: &str,
    package_metadata: &PackageMetadata,
    storage_type: crate::models::workflow_packages::StorageType,
    tenant_id: Option<&str>,
) -> Result<Uuid, RegistryError>
```

Store package metadata in the database.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.get_package_metadata"></span><code>get_​package_​metadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_package_metadata(
    &self,
    package_name: &str,
    version: &str,
) -> Result<Option<(String, PackageMetadata)>, RegistryError>
```

Retrieve package metadata from the database.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.get_package_metadata_by_id"></span><code>get_​package_​metadata_​by_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_package_metadata_by_id(
    &self,
    package_id: Uuid,
) -> Result<Option<(String, PackageMetadata)>, RegistryError>
```

Retrieve package metadata by UUID from the database.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.list_all_packages"></span><code>list_​all_​packages</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_all_packages(
    &self,
) -> Result<Vec<WorkflowPackage>, RegistryError>
```

List all packages in the registry.

<details>
<summary>Source</summary>

```rust
    pub async fn list_all_packages(&self) -> Result<Vec<WorkflowPackage>, RegistryError> {
        let results: Vec<UnifiedWorkflowPackage> = crate::interact_on_backend!(self.dal, |conn| {
            workflow_packages::table.load::<UnifiedWorkflowPackage>(conn)
        })
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(results.into_iter().map(Into::into).collect())
    }
```

</details>

##### <span id="WorkflowPackagesDAL.delete_package_metadata"></span><code>delete_​package_​metadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn delete_package_metadata(
    &self,
    package_name: &str,
    version: &str,
) -> Result<(), RegistryError>
```

Delete package metadata from the database.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.delete_package_metadata_by_id"></span><code>delete_​package_​metadata_​by_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn delete_package_metadata_by_id(
    &self,
    package_id: Uuid,
) -> Result<(), RegistryError>
```

Delete package metadata by UUID from the database.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.get_compiled_data_by_content_hash"></span><code>get_​compiled_​data_​by_​content_​hash</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_compiled_data_by_content_hash(
    &self,
    content_hash: &str,
) -> Result<Option<Vec<u8>>, RegistryError>
```

Look up a compiled package by its `content_hash` digest and return the raw cdylib bytes. Used by `GET /v1/agent/artifact/{digest}` so DB-less agents can fetch the artifact referenced in a work packet.

Returns `Ok(None)` if no successful build matches the digest. The
partial index `idx_wfp_content_hash_success` (postgres migration 022)
keeps this lookup cheap on the hot path.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.get_artifact_digest_for_target"></span><code>get_​artifact_​digest_​for_​target</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_artifact_digest_for_target(
    &self,
    package_name: &str,
    target_triple: &str,
) -> Result<Option<String>, RegistryError>
```

CLOACI-T-0780 (multi-arch): the content-hash of the per-target artifact for `(package, target_triple)`, or `None` if there's no build for that target. Dispatch prefers this over the primary build so each agent gets its arch.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.get_artifact_data_for_target"></span><code>get_​artifact_​data_​for_​target</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_artifact_data_for_target(
    &self,
    package_name: &str,
    version: &str,
    target_triple: &str,
) -> Result<Option<Vec<u8>>, RegistryError>
```

CLOACI-T-0905 (multi-arch, version-scoped): the cdylib bytes of the per-target artifact for `(package, version, target_triple)`, or `None` when no build exists for that target. Unlike `get_artifact_digest_for_target` (which serves fleet dispatch and is version-agnostic), the reconciler loads a SPECIFIC package version, so the version must participate in selection — otherwise an old version's artifact could satisfy a new version's load.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.get_artifact_triples_for_package"></span><code>get_​artifact_​triples_​for_​package</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_artifact_triples_for_package(
    &self,
    package_name: &str,
) -> Result<Vec<String>, RegistryError>
```

CLOACI-T-0780: the set of target triples this package has a per-target artifact for. The fleet uses this (∪ the host primary) to only dispatch a package to an agent whose arch it actually has a cdylib for — so an agent is never handed a WorkPacket it would fail-closed refuse.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.get_artifact_data_by_content_hash"></span><code>get_​artifact_​data_​by_​content_​hash</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_artifact_data_by_content_hash(
    &self,
    content_hash: &str,
) -> Result<Option<Vec<u8>>, RegistryError>
```

CLOACI-T-0780: cdylib bytes of a per-target artifact by content-hash — the fallback `get_compiled_data_by_content_hash` uses so the agent artifact endpoint can serve per-arch builds (not just the primary).

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.upsert_artifact"></span><code>upsert_​artifact</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn upsert_artifact(
    &self,
    package_name: &str,
    version: &str,
    tenant_id: Option<&str>,
    target_triple: &str,
    content_hash: &str,
    compiled_data: Vec<u8>,
) -> Result<(), RegistryError>
```

CLOACI-T-0780: store (replace) the per-target artifact for `(package, version, tenant, target_triple)`. Called by a target-scoped compiler. Delete-then-insert keeps it idempotent across rebuilds.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.upsert_provider"></span><code>upsert_​provider</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn upsert_provider(
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
) -> Result<(), RegistryError>
```

CLOACI-T-0836: upsert one bundled constructor provider for a consumer package. `provider_data` is the provider's packed `.cloacina` archive (arch-neutral WASM) that the reconciler unpacks into a `providers/` tree at load. Delete+insert in a transaction (mirrors `Self::upsert_artifact`) so a rebuild replaces the previous bundle for the same (package, provider).

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.get_providers_for_package"></span><code>get_​providers_​for_​package</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_providers_for_package(
    &self,
    package_name: &str,
    version: &str,
    tenant_id: Option<&str>,
) -> Result<Vec<crate::dal::unified::models::PackageProvider>, RegistryError>
```

CLOACI-T-0836: every bundled provider row for a consumer package (the set the reconciler unpacks into `providers/` before resolving `constructor!` nodes). Empty for packages that use no constructors.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.find_packages_missing_target_provider"></span><code>find_​packages_​missing_​target_​provider</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn find_packages_missing_target_provider(
    &self,
    target_triple: &str,
    tenant_id: Option<&str>,
    name_filter: Option<&str>,
) -> Result<
    Vec<(crate::database::universal_types::UniversalUuid, String, String)>,
    RegistryError,
>
```

CLOACI-T-0908 (producer): active (success, non-superseded) packages whose PRIMARY rows include a NATIVE provider but which lack a per-arch provider row for `target_triple` — the provider twin of the artifact scan below and `find_packages_missing_target_artifact`. The per-target compiler unions this with the artifact scan; rebuilding the package (from retained source, on the target arch) re-bundles its native providers and stores triple-keyed rows. WASM providers are arch-neutral and never appear here.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.find_packages_missing_target_artifact"></span><code>find_​packages_​missing_​target_​artifact</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn find_packages_missing_target_artifact(
    &self,
    target_triple: &str,
    tenant_id: Option<&str>,
    name_filter: Option<&str>,
) -> Result<
    Vec<(crate::database::universal_types::UniversalUuid, String, String)>,
    RegistryError,
>
```

CLOACI-T-0780 (producer): active (success, non-superseded) packages in this tenant scope that LACK a per-target artifact for `target_triple` — the set a `--build-target` compiler scan-and-fills. `name_filter` limits the scan to a single package (keeps the emulated demo build cheap). Returns (id, name, version) so the caller can `execute_build(id)` then `upsert_artifact`.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.get_active_content_hash_for_package"></span><code>get_​active_​content_​hash_​for_​package</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_active_content_hash_for_package(
    &self,
    package_name: &str,
    tenant_id: Option<&str>,
) -> Result<Option<String>, RegistryError>
```

Resolve the **active** content-hash (digest) for a package name within a tenant scope — the `FleetExecutor` (CLOACI-T-0633) uses this to build a work packet's `ArtifactRef` from the task's package.

"Active" = `build_status = 'success'` AND `NOT superseded`. Both filters
are load-bearing: dropping `superseded` would route a stale cdylib to
agents after a re-upload; dropping the status filter would point at an
unbuilt/failed row. Returns `Ok(None)` if no active row matches.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.get_active_dispatch_for_package"></span><code>get_​active_​dispatch_​for_​package</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_active_dispatch_for_package(
    &self,
    package_name: &str,
    tenant_id: Option<&str>,
) -> Result<Option<(String, String)>, RegistryError>
```

`(content_hash, language)` for the **active** build of a package, where `language` is `"python"` when the build produced no cdylib (`compiled_data` empty — the compiler short-circuits pure-Python packages to an empty artifact) and `"rust"` otherwise. The `FleetExecutor` stamps `language` into the `WorkPacket` so the agent loads the package the right way (dlopen vs PyO3 import). Same active-row filter as `get_active_content_hash_for_package`.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.get_package_archive_by_content_hash"></span><code>get_​package_​archive_​by_​content_​hash</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_package_archive_by_content_hash(
    &self,
    content_hash: &str,
) -> Result<Option<Vec<u8>>, RegistryError>
```

Source `.cloacina` archive bytes for the package whose active build has `content_hash == digest`. The registry (`workflow_registry.data`) holds the uploaded archive — for a Python package this is the importable `workflow/` + `vendor/` tree (`compiled_data` is empty). Served by `GET /v1/agent/source/{digest}` so a DB-less agent can fetch + import a Python package (CLOACI-T-0716).

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="WorkflowPackagesDAL.get_package_name_version_by_content_hash"></span><code>get_​package_​name_​version_​by_​content_​hash</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_package_name_version_by_content_hash(
    &self,
    content_hash: &str,
) -> Result<Option<(String, String)>, RegistryError>
```

Resolve a successfully-built package's `(package_name, version)` from its content hash. CLOACI-T-0838: agents identify packages by artifact digest (content-addressed), while `package_providers` is keyed by name+version — this is the bridge the `/v1/agent/providers/{digest}` route uses.

<details>
<summary>Source</summary>

```rust
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
```

</details>

## Functions

### <span id="fn.select_provider_rows_for_target"></span><code style="font-size: 0.85em">fn select_​provider_​rows_​for_​target</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn select_provider_rows_for_target(
    rows: Vec<crate::dal::unified::models::PackageProvider>,
    target_triple: &str,
) -> Vec<crate::dal::unified::models::PackageProvider>
```

CLOACI-T-0908 (consumer): pick the provider rows THIS host should stage from the full row set of a package — for each `provider_name`, prefer the row whose `target_triple` matches `target_triple` exactly (a per-arch NATIVE build), else fall back to the primary (`target_triple` NULL) row. Rows for OTHER triples are never returned. Pure function so the reconciler, the agent route, and tests share one selection semantic.

<details>
<summary>Source</summary>

```rust
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
```

</details>
