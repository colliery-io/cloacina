---
title: "cloacina::dal::unified::task_execution_metadata"
description: "Unified Task Execution Metadata DAL with runtime backend selection"
editUrl: false
sidebar:
  order: 92
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Unified Task Execution Metadata DAL with runtime backend selection

This module provides CRUD operations for TaskExecutionMetadata entities that work with
both PostgreSQL and SQLite backends, selecting the appropriate implementation
at runtime based on the database connection type.

## Structs

### <span id="struct.TaskExecutionMetadataDAL"></span><code style="font-size: 0.85em">struct TaskExecutionMetadataDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for task execution metadata operations with runtime backend selection.

#### Methods

##### <span id="TaskExecutionMetadataDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

Creates a new TaskExecutionMetadataDAL instance.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="TaskExecutionMetadataDAL.create"></span><code>create</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn create(
    &self,
    new_metadata: NewTaskExecutionMetadata,
) -> Result<TaskExecutionMetadata, ValidationError>
```

Creates a new task execution metadata record.

<details>
<summary>Source</summary>

```rust
    pub async fn create(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedTaskExecutionMetadata {
            id,
            task_execution_id: new_metadata.task_execution_id,
            workflow_execution_id: new_metadata.workflow_execution_id,
            task_name: new_metadata.task_name,
            context_id: new_metadata.context_id,
            created_at: now,
            updated_at: now,
        };

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(task_execution_metadata::table)
                .values(&new_unified)
                .execute(conn)
        })?;

        let result: UnifiedTaskExecutionMetadata = crate::interact_on_backend!(self.dal, |conn| {
            task_execution_metadata::table.find(id).first(conn)
        })?;

        Ok(result.into())
    }
```

</details>

##### <span id="TaskExecutionMetadataDAL.get_by_workflow_and_task"></span><code>get_​by_​workflow_​and_​task</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_by_workflow_and_task(
    &self,
    workflow_id: UniversalUuid,
    task_namespace: &TaskNamespace,
) -> Result<TaskExecutionMetadata, ValidationError>
```

Retrieves task execution metadata for a specific workflow and task.

<details>
<summary>Source</summary>

```rust
    pub async fn get_by_workflow_and_task(
        &self,
        workflow_id: UniversalUuid,
        task_namespace: &TaskNamespace,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let task_name = task_namespace.to_string();
        let result: UnifiedTaskExecutionMetadata = crate::interact_on_backend!(self.dal, |conn| {
            task_execution_metadata::table
                .filter(task_execution_metadata::workflow_execution_id.eq(workflow_id))
                .filter(task_execution_metadata::task_name.eq(task_name))
                .first(conn)
        })?;

        Ok(result.into())
    }
```

</details>

##### <span id="TaskExecutionMetadataDAL.get_by_task_execution"></span><code>get_​by_​task_​execution</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_by_task_execution(
    &self,
    task_execution_id: UniversalUuid,
) -> Result<TaskExecutionMetadata, ValidationError>
```

Retrieves task execution metadata by task execution ID.

<details>
<summary>Source</summary>

```rust
    pub async fn get_by_task_execution(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let result: UnifiedTaskExecutionMetadata = crate::interact_on_backend!(self.dal, |conn| {
            task_execution_metadata::table
                .filter(task_execution_metadata::task_execution_id.eq(task_execution_id))
                .first(conn)
        })?;

        Ok(result.into())
    }
```

</details>

##### <span id="TaskExecutionMetadataDAL.update_context_id"></span><code>update_​context_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn update_context_id(
    &self,
    task_execution_id: UniversalUuid,
    context_id: Option<UniversalUuid>,
) -> Result<(), ValidationError>
```

Updates the context ID for a specific task execution.

<details>
<summary>Source</summary>

```rust
    pub async fn update_context_id(
        &self,
        task_execution_id: UniversalUuid,
        context_id: Option<UniversalUuid>,
    ) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(task_execution_metadata::table)
                .filter(task_execution_metadata::task_execution_id.eq(task_execution_id))
                .set((
                    task_execution_metadata::context_id.eq(context_id),
                    task_execution_metadata::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="TaskExecutionMetadataDAL.upsert_task_execution_metadata"></span><code>upsert_​task_​execution_​metadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn upsert_task_execution_metadata(
    &self,
    new_metadata: NewTaskExecutionMetadata,
) -> Result<TaskExecutionMetadata, ValidationError>
```

Creates or updates task execution metadata.

<details>
<summary>Source</summary>

```rust
    pub async fn upsert_task_execution_metadata(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.upsert_task_execution_metadata_postgres(new_metadata)
                .await,
            self.upsert_task_execution_metadata_sqlite(new_metadata)
                .await
        )
    }
```

</details>

##### <span id="TaskExecutionMetadataDAL.get_dependency_metadata"></span><code>get_​dependency_​metadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_dependency_metadata(
    &self,
    workflow_id: UniversalUuid,
    dependency_task_names: &[String],
) -> Result<Vec<TaskExecutionMetadata>, ValidationError>
```

Retrieves metadata for multiple dependency tasks within a workflow execution.

<details>
<summary>Source</summary>

```rust
    pub async fn get_dependency_metadata(
        &self,
        workflow_id: UniversalUuid,
        dependency_task_names: &[String],
    ) -> Result<Vec<TaskExecutionMetadata>, ValidationError> {
        let dependency_task_names = dependency_task_names.to_vec();
        let results: Vec<UnifiedTaskExecutionMetadata> =
            crate::interact_on_backend!(self.dal, |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::workflow_execution_id.eq(workflow_id))
                    .filter(task_execution_metadata::task_name.eq_any(dependency_task_names))
                    .load(conn)
            })?;

        Ok(results.into_iter().map(Into::into).collect())
    }
```

</details>

##### <span id="TaskExecutionMetadataDAL.get_dependency_metadata_with_contexts"></span><code>get_​dependency_​metadata_​with_​contexts</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_dependency_metadata_with_contexts(
    &self,
    workflow_id: UniversalUuid,
    dependency_task_namespaces: &[TaskNamespace],
) -> Result<Vec<(TaskExecutionMetadata, Option<String>)>, ValidationError>
```

Retrieves metadata and context data for multiple dependency tasks in a single query.

<details>
<summary>Source</summary>

```rust
    pub async fn get_dependency_metadata_with_contexts(
        &self,
        workflow_id: UniversalUuid,
        dependency_task_namespaces: &[TaskNamespace],
    ) -> Result<Vec<(TaskExecutionMetadata, Option<String>)>, ValidationError> {
        if dependency_task_namespaces.is_empty() {
            return Ok(Vec::new());
        }

        let dependency_task_names: Vec<String> = dependency_task_namespaces
            .iter()
            .map(|ns| ns.to_string())
            .collect();

        let results: Vec<(UnifiedTaskExecutionMetadata, Option<String>)> =
            crate::interact_on_backend!(self.dal, |conn| {
                task_execution_metadata::table
                    .left_join(
                        contexts::table
                            .on(task_execution_metadata::context_id.eq(contexts::id.nullable())),
                    )
                    .filter(task_execution_metadata::workflow_execution_id.eq(workflow_id))
                    .filter(task_execution_metadata::task_name.eq_any(dependency_task_names))
                    .select((
                        task_execution_metadata::all_columns,
                        contexts::value.nullable(),
                    ))
                    .load::<(UnifiedTaskExecutionMetadata, Option<String>)>(conn)
            })?;

        Ok(results.into_iter().map(|(m, c)| (m.into(), c)).collect())
    }
```

</details>
