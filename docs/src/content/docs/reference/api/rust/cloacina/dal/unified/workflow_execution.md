---
title: "cloacina::dal::unified::workflow_execution"
description: "Unified Workflow Execution DAL with compile-time backend selection"
editUrl: false
sidebar:
  order: 93
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Unified Workflow Execution DAL with compile-time backend selection

All state transitions are transactional: the status update and execution event
are written atomically. If either fails, both are rolled back.

## Structs

### <span id="struct.ExecutionListFilter"></span><code style="font-size: 0.85em">struct ExecutionListFilter</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Default`

Filter for `WorkflowExecutionDAL::list_filtered`. CLOACI-T-0594 / API-02: closes the silent-filter-drop bug where the REST route's `--status` / `--workflow_name` query params were discarded.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `status` | `Option<String>` | Filter by status (e.g. `Pending`, `Running`, `Completed`, `Failed`). `None` means no status filter. |
| `workflow_name` | `Option<String>` | Filter by exact workflow name. `None` means no name filter. |
| `limit` | `i64` | SQL `LIMIT`. Caller is responsible for bounding (route validates `limit <= 1000`); the DAL trusts whatever is passed. |
| `offset` | `i64` | SQL `OFFSET`. |

### <span id="struct.WorkflowExecutionDAL"></span><code style="font-size: 0.85em">struct WorkflowExecutionDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for workflow execution operations with compile-time backend selection.

#### Methods

##### <span id="WorkflowExecutionDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="WorkflowExecutionDAL.create"></span><code>create</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn create(
    &self,
    new_execution: NewWorkflowExecution,
) -> Result<WorkflowExecutionRecord, ValidationError>
```

Creates a new workflow execution record in the database.

This operation is transactional: the workflow record and execution event
are written atomically.

<details>
<summary>Source</summary>

```rust
    pub async fn create(
        &self,
        new_execution: NewWorkflowExecution,
    ) -> Result<WorkflowExecutionRecord, ValidationError> {
        use diesel::connection::Connection;

        let execution: UnifiedWorkflowExecution = crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let id = UniversalUuid::new_v4();
                let now = UniversalTimestamp::now();

                let unified_new = NewUnifiedWorkflowExecution {
                    id,
                    workflow_name: new_execution.workflow_name,
                    workflow_version: new_execution.workflow_version,
                    status: new_execution.status,
                    context_id: new_execution.context_id,
                    started_at: now,
                    created_at: now,
                    updated_at: now,
                };

                // Insert workflow record
                diesel::insert_into(workflow_executions::table)
                    .values(&unified_new)
                    .execute(conn)?;

                // Retrieve the created record
                let execution: UnifiedWorkflowExecution =
                    workflow_executions::table.find(id).first(conn)?;

                // Insert execution event for workflow start
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: execution.id,
                    task_execution_id: None,
                    event_type: ExecutionEventType::WorkflowStarted.as_str().to_string(),
                    event_data: None,
                    worker_id: None,
                    created_at: now,
                    request_id: None,
                    runner_id: None,
                    tenant_id: None,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(execution)
            })
        })?;

        Ok(execution.into())
    }
```

</details>

##### <span id="WorkflowExecutionDAL.get_by_id"></span><code>get_​by_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_by_id(
    &self,
    id: UniversalUuid,
) -> Result<WorkflowExecutionRecord, ValidationError>
```

<details>
<summary>Source</summary>

```rust
    pub async fn get_by_id(
        &self,
        id: UniversalUuid,
    ) -> Result<WorkflowExecutionRecord, ValidationError> {
        let execution: UnifiedWorkflowExecution = crate::interact_on_backend!(self.dal, |conn| {
            workflow_executions::table.find(id).first(conn)
        })?;

        Ok(execution.into())
    }
```

</details>

##### <span id="WorkflowExecutionDAL.get_active_executions"></span><code>get_​active_​executions</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_active_executions(
    &self,
) -> Result<Vec<WorkflowExecutionRecord>, ValidationError>
```

<details>
<summary>Source</summary>

```rust
    pub async fn get_active_executions(
        &self,
    ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError> {
        let executions: Vec<UnifiedWorkflowExecution> =
            crate::interact_on_backend!(self.dal, |conn| {
                workflow_executions::table
                    .filter(workflow_executions::status.eq_any(vec!["Pending", "Running"]))
                    .load(conn)
            })?;

        Ok(executions.into_iter().map(Into::into).collect())
    }
```

</details>

##### <span id="WorkflowExecutionDAL.update_status"></span><code>update_​status</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn update_status(
    &self,
    id: UniversalUuid,
    status: &str,
) -> Result<(), ValidationError>
```

<details>
<summary>Source</summary>

```rust
    pub async fn update_status(
        &self,
        id: UniversalUuid,
        status: &str,
    ) -> Result<(), ValidationError> {
        let status = status.to_string();
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(workflow_executions::table.find(id))
                .set((
                    workflow_executions::status.eq(status),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="WorkflowExecutionDAL.set_trigger_origin"></span><code>set_​trigger_​origin</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn set_trigger_origin(
    &self,
    id: UniversalUuid,
    origin: &str,
) -> Result<(), ValidationError>
```

Set a workflow execution's trigger origin (CLOACI-T-0776). Called right after creation by the REST execute endpoint to mark an operator run as `"manual"`. Best-effort metadata — failures are non-fatal to the run.

<details>
<summary>Source</summary>

```rust
    pub async fn set_trigger_origin(
        &self,
        id: UniversalUuid,
        origin: &str,
    ) -> Result<(), ValidationError> {
        let origin = origin.to_string();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(workflow_executions::table.find(id))
                .set(workflow_executions::trigger_origin.eq(origin))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="WorkflowExecutionDAL.mark_completed"></span><code>mark_​completed</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn mark_completed(
    &self) -> Result<(,
    id: UniversalUuid) -> Result<(,
) -> Result<(), ValidationError>
```

Marks a workflow execution as completed.

This operation is transactional: the status update and execution event
are written atomically.

<details>
<summary>Source</summary>

```rust
    pub async fn mark_completed(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Only transition from Running to Completed — prevents duplicate
                // WorkflowCompleted events when two scheduler ticks race.
                let rows = diesel::update(
                    workflow_executions::table
                        .find(id)
                        .filter(workflow_executions::status.ne_all(vec!["Completed", "Failed"])),
                )
                .set((
                    workflow_executions::status.eq("Completed"),
                    workflow_executions::completed_at.eq(Some(now)),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)?;

                // Only insert event if we actually transitioned
                if rows > 0 {
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        workflow_execution_id: id,
                        task_execution_id: None,
                        event_type: ExecutionEventType::WorkflowCompleted.as_str().to_string(),
                        event_data: None,
                        worker_id: None,
                        created_at: now,
                        request_id: None,
                        runner_id: None,
                        tenant_id: None,
                    };
                    diesel::insert_into(execution_events::table)
                        .values(&event)
                        .execute(conn)?;
                }

                Ok(())
            })
        })?;

        Ok(())
    }
```

</details>

##### <span id="WorkflowExecutionDAL.get_last_version"></span><code>get_​last_​version</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_last_version(
    &self,
    workflow_name: &str,
) -> Result<Option<String>, ValidationError>
```

<details>
<summary>Source</summary>

```rust
    pub async fn get_last_version(
        &self,
        workflow_name: &str,
    ) -> Result<Option<String>, ValidationError> {
        let workflow_name = workflow_name.to_string();
        let version: Option<String> = crate::interact_on_backend!(self.dal, |conn| {
            workflow_executions::table
                .filter(workflow_executions::workflow_name.eq(workflow_name))
                .order(workflow_executions::started_at.desc())
                .select(workflow_executions::workflow_version)
                .first(conn)
                .optional()
        })?;

        Ok(version)
    }
```

</details>

##### <span id="WorkflowExecutionDAL.mark_failed"></span><code>mark_​failed</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn mark_failed(
    &self,
    id: UniversalUuid,
    reason: &str,
) -> Result<(), ValidationError>
```

Marks a workflow execution as failed with an error reason.

This operation is transactional: the status update and execution event
are written atomically.

<details>
<summary>Source</summary>

```rust
    pub async fn mark_failed(
        &self,
        id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let reason = reason.to_string();
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Only transition from Running to Failed — prevents duplicate
                // WorkflowFailed events when two scheduler ticks race.
                let rows = diesel::update(
                    workflow_executions::table
                        .find(id)
                        .filter(workflow_executions::status.ne_all(vec!["Completed", "Failed"])),
                )
                .set((
                    workflow_executions::status.eq("Failed"),
                    workflow_executions::completed_at.eq(Some(now)),
                    workflow_executions::error_details.eq(&reason),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)?;

                // Only insert event if we actually transitioned
                if rows > 0 {
                    let event_data = serde_json::json!({ "reason": reason }).to_string();
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        workflow_execution_id: id,
                        task_execution_id: None,
                        event_type: ExecutionEventType::WorkflowFailed.as_str().to_string(),
                        event_data: Some(event_data),
                        worker_id: None,
                        created_at: now,
                        request_id: None,
                        runner_id: None,
                        tenant_id: None,
                    };
                    diesel::insert_into(execution_events::table)
                        .values(&event)
                        .execute(conn)?;
                }

                Ok(())
            })
        })?;

        Ok(())
    }
```

</details>

##### <span id="WorkflowExecutionDAL.cancel"></span><code>cancel</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn cancel(&self, id: UniversalUuid) -> Result<(), ValidationError>
```

<details>
<summary>Source</summary>

```rust
    pub async fn cancel(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(workflow_executions::table.find(id))
                .set((
                    workflow_executions::status.eq("Cancelled"),
                    workflow_executions::completed_at.eq(Some(now)),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="WorkflowExecutionDAL.pause"></span><code>pause</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn pause(
    &self,
    id: UniversalUuid,
    reason: Option<&str>,
) -> Result<(), ValidationError>
```

Pauses a running workflow execution.

Sets the workflow status to 'Paused', records the pause timestamp,
and optionally stores a reason for the pause.

This operation is transactional: the status update and execution event
are written atomically.

<details>
<summary>Source</summary>

```rust
    pub async fn pause(
        &self,
        id: UniversalUuid,
        reason: Option<&str>,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let reason = reason.map(|r| r.to_string());
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Update workflow status
                diesel::update(workflow_executions::table.find(id))
                    .set((
                        workflow_executions::status.eq("Paused"),
                        workflow_executions::paused_at.eq(Some(now)),
                        workflow_executions::pause_reason.eq(&reason),
                        workflow_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with pause reason
                let event_data = reason.map(|r| serde_json::json!({ "reason": r }).to_string());
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: id,
                    task_execution_id: None,
                    event_type: ExecutionEventType::WorkflowPaused.as_str().to_string(),
                    event_data,
                    worker_id: None,
                    created_at: now,
                    request_id: None,
                    runner_id: None,
                    tenant_id: None,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })?;

        Ok(())
    }
```

</details>

##### <span id="WorkflowExecutionDAL.resume"></span><code>resume</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn resume(&self, id: UniversalUuid) -> Result<(), ValidationError>
```

Resumes a paused workflow execution.

Sets the workflow status back to 'Running' and clears the pause metadata.

This operation is transactional: the status update and execution event
are written atomically.

<details>
<summary>Source</summary>

```rust
    pub async fn resume(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Update workflow status
                diesel::update(workflow_executions::table.find(id))
                    .set((
                        workflow_executions::status.eq("Running"),
                        workflow_executions::paused_at.eq(None::<UniversalTimestamp>),
                        workflow_executions::pause_reason.eq(None::<String>),
                        workflow_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: id,
                    task_execution_id: None,
                    event_type: ExecutionEventType::WorkflowResumed.as_str().to_string(),
                    event_data: None,
                    worker_id: None,
                    created_at: now,
                    request_id: None,
                    runner_id: None,
                    tenant_id: None,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })?;

        Ok(())
    }
```

</details>

##### <span id="WorkflowExecutionDAL.update_final_context"></span><code>update_​final_​context</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn update_final_context(
    &self,
    id: UniversalUuid,
    final_context_id: UniversalUuid,
) -> Result<(), ValidationError>
```

<details>
<summary>Source</summary>

```rust
    pub async fn update_final_context(
        &self,
        id: UniversalUuid,
        final_context_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(workflow_executions::table.find(id))
                .set((
                    workflow_executions::context_id.eq(Some(final_context_id)),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="WorkflowExecutionDAL.list_recent"></span><code>list_​recent</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_recent(
    &self,
    limit: i64,
) -> Result<Vec<WorkflowExecutionRecord>, ValidationError>
```

<details>
<summary>Source</summary>

```rust
    pub async fn list_recent(
        &self,
        limit: i64,
    ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError> {
        let executions: Vec<UnifiedWorkflowExecution> =
            crate::interact_on_backend!(self.dal, |conn| {
                workflow_executions::table
                    .order(workflow_executions::started_at.desc())
                    .limit(limit)
                    .load(conn)
            })?;

        Ok(executions.into_iter().map(Into::into).collect())
    }
```

</details>

##### <span id="WorkflowExecutionDAL.list_filtered"></span><code>list_​filtered</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_filtered(
    &self,
    filter: ExecutionListFilter,
) -> Result<Vec<WorkflowExecutionRecord>, ValidationError>
```

CLOACI-T-0594 / API-02: filtered list endpoint. Supports `status` (e.g. "Failed", "Running") and `workflow_name` filters with bounded pagination. Replaces the old route-level `get_active_executions()` call that silently ignored the query string.

<details>
<summary>Source</summary>

```rust
    pub async fn list_filtered(
        &self,
        filter: ExecutionListFilter,
    ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError> {
        let executions: Vec<UnifiedWorkflowExecution> =
            crate::interact_on_backend!(self.dal, |conn| {
                let mut query = workflow_executions::table.into_boxed();
                if let Some(ref status) = filter.status {
                    query = query.filter(workflow_executions::status.eq(status.clone()));
                }
                if let Some(ref name) = filter.workflow_name {
                    query = query.filter(workflow_executions::workflow_name.eq(name.clone()));
                }
                query
                    .order(workflow_executions::started_at.desc())
                    .limit(filter.limit)
                    .offset(filter.offset)
                    .load(conn)
            })?;

        Ok(executions.into_iter().map(Into::into).collect())
    }
```

</details>
