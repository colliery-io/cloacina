# cloacina::dal::unified::execution_event <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified Execution Event DAL with runtime backend selection

This module provides CRUD operations for ExecutionEvent entities that work with
both PostgreSQL and SQLite backends, selecting the appropriate implementation
at runtime based on the database connection type.
Execution events form an append-only audit trail of all task and workflow
state transitions for debugging, compliance, and replay capability.

## Structs

### `cloacina::dal::unified::execution_event::ExecutionEventDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Data access layer for execution event operations with runtime backend selection.

This DAL provides methods for creating and querying execution events,
which track all state transitions for tasks and workflow executions.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



## Functions

### `cloacina::dal::unified::execution_event::build_event_outbox_row`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn build_event_outbox_row (event_id : UniversalUuid , new_event : & NewExecutionEvent , now : UniversalTimestamp ,) -> Result < NewUnifiedDeliveryOutbox , ValidationError >
```

Substrate (CLOACI-I-0115 / T-0629): build the `delivery_outbox` row that gets inserted in the same transaction as the event, so a subscribed CLI / future SDK receives the event over the substrate WS. Recipient is keyed by `workflow_execution_id` (convention: `exec_events:<uuid>`). Payload is JSON of the fields the consumer displays — sequence_num is intentionally omitted (substrate row id provides arrival ordering).

<details>
<summary>Source</summary>

```rust
fn build_event_outbox_row(
    event_id: UniversalUuid,
    new_event: &NewExecutionEvent,
    now: UniversalTimestamp,
) -> Result<NewUnifiedDeliveryOutbox, ValidationError> {
    let payload = serde_json::json!({
        "id": event_id.0.to_string(),
        "workflow_execution_id": new_event.workflow_execution_id.0.to_string(),
        "task_execution_id": new_event.task_execution_id.map(|u| u.0.to_string()),
        "event_type": new_event.event_type.as_str(),
        "event_data": new_event.event_data.as_deref(),
        "created_at": now.0.to_rfc3339(),
    });
    let bytes = serde_json::to_vec(&payload).map_err(|e| {
        ValidationError::ConnectionPool(format!("execution_event outbox payload: {}", e))
    })?;
    Ok(NewUnifiedDeliveryOutbox {
        recipient: format!("exec_events:{}", new_event.workflow_execution_id.0),
        kind: "execution_event".to_string(),
        tenant_id: new_event.tenant_id.clone(),
        payload: UniversalBinary::from(bytes),
        delivery_state: "pending".to_string(),
        delivery_attempts: 0,
        created_at: now,
    })
}
```

</details>
