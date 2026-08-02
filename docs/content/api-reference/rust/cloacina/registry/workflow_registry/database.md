# cloacina::registry::workflow_registry::database <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Database operations for workflow registry metadata storage.

## Structs

### `cloacina::registry::workflow_registry::database::InspectedPackage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Result of inspecting a package — full metadata plus the raw build state.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `metadata` | `WorkflowMetadata` |  |
| `build_status` | `String` |  |
| `build_error` | `Option < String >` |  |



### `cloacina::registry::workflow_registry::database::BuildQueueStats`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `serde :: Serialize`

Snapshot of the build queue for the compiler's status endpoint.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `pending` | `u64` |  |
| `building` | `u64` |  |
| `last_success_at` | `Option < chrono :: DateTime < chrono :: Utc > >` |  |
| `last_failure_at` | `Option < chrono :: DateTime < chrono :: Utc > >` |  |
| `heartbeat_at` | `Option < chrono :: DateTime < chrono :: Utc > >` |  |



### `cloacina::registry::workflow_registry::database::ReconcilerStats`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `serde :: Serialize`

Package-availability snapshot for the reconciler tile (CLOACI-T-0718 / absorbs T-0717): how many packages built successfully and are available to load, how many failed, and when the most recent successful build landed. Counts the active (non-superseded) rows. Independent of the registry's package loader — powers the server's ops-metrics publisher.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `built` | `u64` |  |
| `failed` | `u64` |  |
| `last_built_at` | `Option < chrono :: DateTime < chrono :: Utc > >` |  |



### `cloacina::registry::workflow_registry::database::ClaimedBuild`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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

### `cloacina::registry::workflow_registry::database::build_task_graph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


```rust
fn build_task_graph (package_metadata : & crate :: registry :: loader :: package_loader :: PackageMetadata ,) -> Vec < WorkflowTaskNode >
```

Build the task dependency graph (one node per task, with its upstream dependencies) from the persisted package metadata's task list, so the API and UI can render the full DAG. (CLOACI-T-0663)

<details>
<summary>Source</summary>

```rust
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
```

</details>



### `cloacina::registry::workflow_registry::database::build_queue_stats`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn build_queue_stats (database : & crate :: database :: Database ,) -> Result < BuildQueueStats , RegistryError >
```

Build-queue telemetry over a raw [`Database`] handle, independent of the registry's package loader. Powers both the compiler's `/v1/status` (via [`WorkflowRegistryImpl::build_queue_stats`]) and the server's operator compiler-status endpoint (CLOACI-I-0124), which has a `Database` but no reason to construct a full registry (and its FFI loader) per request.

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



### `cloacina::registry::workflow_registry::database::reconciler_stats`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn reconciler_stats (database : & crate :: database :: Database ,) -> Result < ReconcilerStats , RegistryError >
```

Compute [`ReconcilerStats`] over a raw [`Database`] handle.

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
