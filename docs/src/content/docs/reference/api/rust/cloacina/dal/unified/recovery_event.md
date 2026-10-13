---
title: "cloacina::dal::unified::recovery_event"
description: "Unified Recovery Event DAL with runtime backend selection"
editUrl: false
sidebar:
  order: 82
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Unified Recovery Event DAL with runtime backend selection

This module provides CRUD operations for RecoveryEvent entities that work with
both PostgreSQL and SQLite backends, selecting the appropriate implementation
at runtime based on the database connection type.

## Structs

### <span id="struct.RecoveryEventDAL"></span><code style="font-size: 0.85em">struct RecoveryEventDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for recovery event operations with runtime backend selection.

This DAL provides methods for creating and querying recovery events,
which track recovery operations performed on tasks and workflow executions.

#### Methods

##### <span id="RecoveryEventDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

Creates a new RecoveryEventDAL instance.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="RecoveryEventDAL.create"></span><code>create</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn create(
    &self,
    new_event: NewRecoveryEvent,
) -> Result<RecoveryEvent, ValidationError>
```

Creates a new recovery event record.

<details>
<summary>Source</summary>

```rust
    pub async fn create(
        &self,
        new_event: NewRecoveryEvent,
    ) -> Result<RecoveryEvent, ValidationError> {
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedRecoveryEvent {
            id,
            workflow_execution_id: new_event.workflow_execution_id,
            task_execution_id: new_event.task_execution_id,
            recovery_type: new_event.recovery_type,
            recovered_at: now,
            details: new_event.details,
            created_at: now,
            updated_at: now,
        };

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(recovery_events::table)
                .values(&new_unified)
                .execute(conn)
        })?;

        let result: UnifiedRecoveryEvent = crate::interact_on_backend!(self.dal, |conn| {
            recovery_events::table.find(id).first(conn)
        })?;

        Ok(result.into())
    }
```

</details>
