---
title: "cloacina::registry::workflow_registry::database"
description: "Database operations for workflow registry metadata storage."
editUrl: false
sidebar:
  order: 173
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Database operations for workflow registry metadata storage.

## Structs

### <span id="struct.InspectedPackage"></span><code style="font-size: 0.85em">struct InspectedPackage</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Result of inspecting a package — full metadata plus the raw build state.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `metadata` | `WorkflowMetadata` |  |
| `build_status` | `String` |  |
| `build_error` | `Option<String>` |  |

### <span id="struct.BuildQueueStats"></span><code style="font-size: 0.85em">struct BuildQueueStats</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `serde::Serialize`

Snapshot of the build queue for the compiler's status endpoint.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `pending` | `u64` |  |
| `building` | `u64` |  |
| `last_success_at` | `Option<chrono::DateTime<chrono::Utc>>` |  |
| `last_failure_at` | `Option<chrono::DateTime<chrono::Utc>>` |  |
| `heartbeat_at` | `Option<chrono::DateTime<chrono::Utc>>` |  |

### <span id="struct.ReconcilerStats"></span><code style="font-size: 0.85em">struct ReconcilerStats</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `serde::Serialize`

Package-availability snapshot for the reconciler tile (CLOACI-T-0718 / absorbs T-0717): how many packages built successfully and are available to load, how many failed, and when the most recent successful build landed. Counts the active (non-superseded) rows. Independent of the registry's package loader — powers the server's ops-metrics publisher.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `built` | `u64` |  |
| `failed` | `u64` |  |
| `last_built_at` | `Option<chrono::DateTime<chrono::Utc>>` |  |

### <span id="struct.ClaimedBuild"></span><code style="font-size: 0.85em">struct ClaimedBuild</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

A build row claimed by the compiler. Everything the compiler needs to locate the source and write back results.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `Uuid` |  |
| `registry_id` | `Uuid` |  |
| `package_name` | `String` |  |
| `version` | `String` |  |
| `metadata` | `String` |  |

## Functions

### <span id="fn.build_queue_stats"></span><code style="font-size: 0.85em">fn build_​queue_​stats</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn build_queue_stats(
    database: &crate::database::Database,
) -> Result<BuildQueueStats, RegistryError>
```

Build-queue telemetry over a raw [`Database`](../../../../../rust/cloacina/database/connection/#struct.Database) handle, independent of the registry's package loader. Powers both the compiler's `/v1/status` (via `WorkflowRegistryImpl::build_queue_stats`) and the server's operator compiler-status endpoint (CLOACI-I-0124), which has a `Database` but no reason to construct a full registry (and its FFI loader) per request.

<details>
<summary>Source</summary>

```rust
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
            last_success_at: last_success.and_then(|r| r.compiled_at.map(|t| t.0)),
            last_failure_at: last_failure.map(|r| r.updated_at.0),
            heartbeat_at: heartbeat_row.and_then(|r| r.build_claimed_at.map(|t| t.0)),
        })
    })
    .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))
}
```

</details>

### <span id="fn.reconciler_stats"></span><code style="font-size: 0.85em">fn reconciler_​stats</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn reconciler_stats(
    database: &crate::database::Database,
) -> Result<ReconcilerStats, RegistryError>
```

Compute [`ReconcilerStats`](../../../../../rust/cloacina/registry/workflow_registry/database/#struct.ReconcilerStats) over a raw [`Database`](../../../../../rust/cloacina/database/connection/#struct.Database) handle.

<details>
<summary>Source</summary>

```rust
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
```

</details>
