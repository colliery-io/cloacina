---
title: "cloacina::dal::unified::execution_event"
description: "Unified Execution Event DAL with runtime backend selection"
editUrl: false
sidebar:
  order: 73
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Unified Execution Event DAL with runtime backend selection

This module provides CRUD operations for ExecutionEvent entities that work with
both PostgreSQL and SQLite backends, selecting the appropriate implementation
at runtime based on the database connection type.

Execution events form an append-only audit trail of all task and workflow
state transitions for debugging, compliance, and replay capability.

## Structs

### <span id="struct.ExecutionEventDAL"></span><code style="font-size: 0.85em">struct ExecutionEventDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for execution event operations with runtime backend selection.

This DAL provides methods for creating and querying execution events,
which track all state transitions for tasks and workflow executions.

#### Methods

##### <span id="ExecutionEventDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

Creates a new ExecutionEventDAL instance.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="ExecutionEventDAL.create"></span><code>create</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn create(
    &self,
    new_event: NewExecutionEvent,
) -> Result<ExecutionEvent, ValidationError>
```

Creates a new execution event record.

Events are append-only and never updated after creation. Each event
receives a monotonically increasing sequence number for ordering.

<details>
<summary>Source</summary>

```rust
    pub async fn create(
        &self,
        new_event: NewExecutionEvent,
    ) -> Result<ExecutionEvent, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_event).await,
            self.create_sqlite(new_event).await
        )
    }
```

</details>

##### <span id="ExecutionEventDAL.list_by_workflow"></span><code>list_​by_​workflow</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_by_workflow(
    &self,
    workflow_execution_id: UniversalUuid,
) -> Result<Vec<ExecutionEvent>, ValidationError>
```

Gets all execution events for a specific workflow execution, ordered by sequence.

<details>
<summary>Source</summary>

```rust
    pub async fn list_by_workflow(
        &self,
        workflow_execution_id: UniversalUuid,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let results: Vec<UnifiedExecutionEvent> = crate::interact_on_backend!(self.dal, |conn| {
            execution_events::table
                .filter(execution_events::workflow_execution_id.eq(workflow_execution_id))
                .order(execution_events::sequence_num.asc())
                .load(conn)
        })?;

        Ok(results.into_iter().map(Into::into).collect())
    }
```

</details>

##### <span id="ExecutionEventDAL.delete_older_than"></span><code>delete_​older_​than</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn delete_older_than(
    &self,
    cutoff: UniversalTimestamp,
) -> Result<usize, ValidationError>
```

Deletes execution events older than the specified timestamp.

Used for retention policy enforcement to prevent unbounded table growth.
Returns the number of deleted events.

<details>
<summary>Source</summary>

```rust
    pub async fn delete_older_than(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<usize, ValidationError> {
        let deleted: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(execution_events::table.filter(execution_events::created_at.lt(cutoff)))
                .execute(conn)
        })?;

        Ok(deleted)
    }
```

</details>

##### <span id="ExecutionEventDAL.count_older_than"></span><code>count_​older_​than</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn count_older_than(
    &self,
    cutoff: UniversalTimestamp,
) -> Result<i64, ValidationError>
```

Counts execution events older than the specified timestamp.

Used for dry-run mode to preview how many events would be deleted.

<details>
<summary>Source</summary>

```rust
    pub async fn count_older_than(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<i64, ValidationError> {
        let count: i64 = crate::interact_on_backend!(self.dal, |conn| {
            execution_events::table
                .filter(execution_events::created_at.lt(cutoff))
                .count()
                .get_result(conn)
        })?;

        Ok(count)
    }
```

</details>
