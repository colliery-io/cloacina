---
id: taskoutbox-model-and-dal-with
level: task
title: "TaskOutbox model and DAL with outbox population in mark_ready()"
short_code: "CLOACI-T-0081"
created_at: 2026-02-03T20:16:45.995592+00:00
updated_at: 2026-02-04T14:53:16.794648+00:00
parent: CLOACI-I-0022
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0022
---

# TaskOutbox model and DAL with outbox population in mark_ready()

## Parent Initiative

[[CLOACI-I-0022]] - Execution Events and Outbox-Based Task Distribution

## Objective

Create the `TaskOutbox` model and DAL, and modify `mark_ready()` to populate the outbox within the same transaction.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `TaskOutbox` and `NewTaskOutbox` model structs
- [x] `TaskOutboxDAL` with `create()` and `delete()` methods
- [x] `TaskExecutionDAL::mark_ready()` modified to insert outbox row in same transaction
- [x] Outbox insert is atomic with status update (single transaction)

## Implementation Notes

### Model

```rust
pub struct TaskOutbox {
    pub id: i64,
    pub task_execution_id: Uuid,
    pub created_at: DateTime<Utc>,
}

pub struct NewTaskOutbox {
    pub task_execution_id: Uuid,
}
```

### Modified mark_ready()

```rust
async fn mark_ready(&self, task_id: Uuid) -> Result<()> {
    let mut tx = self.pool.begin().await?;

    // Update task status (existing)
    sqlx::query("UPDATE task_executions SET status = 'Ready' WHERE id = $1")
        .bind(task_id).execute(&mut *tx).await?;

    // Insert to outbox (new)
    sqlx::query("INSERT INTO task_outbox (task_execution_id) VALUES ($1)")
        .bind(task_id).execute(&mut *tx).await?;

    // Emit event (from T-0080)
    self.emit_event(&mut tx, task_id, EventType::TaskMarkedReady).await?;

    tx.commit().await
}
```

### Dependencies

- Requires CLOACI-T-0079 (schema migrations)

## Status Updates

### Session 1 - 2026-02-04

**Completed implementation:**

1. **Created domain model** (`models/task_outbox.rs`)
   - `TaskOutbox` struct with id, task_execution_id, created_at
   - `NewTaskOutbox` struct for creation
   - Documentation explaining transient nature of outbox

2. **Created TaskOutboxDAL** (`dal/unified/task_outbox.rs`)
   - `create()` - Insert new outbox entry
   - `delete_by_task()` - Remove entry when task is claimed
   - `list_pending()` - List pending entries (for polling)
   - `count_pending()` - Count pending entries (for monitoring)
   - `delete_older_than()` - Cleanup stale entries
   - Both Postgres and SQLite implementations

3. **Registered new components**
   - Added `task_outbox` module to `models/mod.rs`
   - Added `task_outbox` module to `dal/unified/mod.rs`
   - Added `TaskOutboxDAL` re-export
   - Added `task_outbox()` accessor method to DAL

4. **Modified `mark_ready()` in `state.rs`**
   - Added import for `NewUnifiedTaskOutbox` and `task_outbox` schema
   - Added outbox insertion within the same transaction as status update and event emission
   - All three operations (status update, event, outbox) are atomic

**Verification:**
- `cargo check --features sqlite` passes
- `cargo check --features postgres` passes
- All 287 unit tests pass

**Acceptance criteria status:**
- [x] `TaskOutbox` and `NewTaskOutbox` model structs
- [x] `TaskOutboxDAL` with `create()` and `delete()` methods
- [x] `TaskExecutionDAL::mark_ready()` modified to insert outbox row in same transaction
- [x] Outbox insert is atomic with status update (single transaction)
