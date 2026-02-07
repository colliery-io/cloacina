---
id: outbox-based-task-claiming-in
level: task
title: "Outbox-based task claiming in TaskExecutionDAL"
short_code: "CLOACI-T-0082"
created_at: 2026-02-03T20:16:47.115852+00:00
updated_at: 2026-02-06T00:20:31.728319+00:00
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

# Outbox-based task claiming in TaskExecutionDAL

## Parent Initiative

[[CLOACI-I-0022]] - Execution Events and Outbox-Based Task Distribution

## Objective

Modify `TaskExecutionDAL::claim()` to read from the outbox table instead of polling `task_executions.status = 'Ready'`. Delete outbox row atomically with claim.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `claim()` reads from `task_outbox` instead of `task_executions`
- [x] Outbox row deleted in same transaction as task update
- [x] `FOR UPDATE SKIP LOCKED` semantics preserved
- [x] Both Postgres and SQLite implementations
- [x] Emits `TaskClaimed` event

## Implementation Notes

### Postgres Implementation

```sql
WITH claimed AS (
    DELETE FROM task_outbox
    WHERE id = (
        SELECT id FROM task_outbox
        ORDER BY created_at
        FOR UPDATE SKIP LOCKED
        LIMIT 1
    )
    RETURNING task_execution_id
)
UPDATE task_executions
SET status = 'Running', started_at = NOW(), worker_id = $1
FROM claimed
WHERE task_executions.id = claimed.task_execution_id
RETURNING task_executions.*
```

### SQLite Implementation

SQLite doesn't support `FOR UPDATE SKIP LOCKED`, so use `BEGIN IMMEDIATE`:

```rust
// Begin immediate transaction for exclusive lock
let mut tx = pool.begin_immediate().await?;

// Select oldest unclaimed
let outbox_row = sqlx::query("SELECT id, task_execution_id FROM task_outbox ORDER BY created_at LIMIT 1")
    .fetch_optional(&mut *tx).await?;

if let Some(row) = outbox_row {
    // Delete outbox row
    sqlx::query("DELETE FROM task_outbox WHERE id = ?")
        .bind(row.id).execute(&mut *tx).await?;

    // Update task
    let task = sqlx::query_as("UPDATE task_executions SET status = 'Running' ... RETURNING *")
        .execute(&mut *tx).await?;

    tx.commit().await?;
    return Ok(Some(task));
}
```

### Dependencies

- Requires CLOACI-T-0079 (schema migrations)
- Requires CLOACI-T-0081 (outbox population)

## Status Updates

### Session 1 - Outbox-Based Claiming Implementation

**Completed:**
1. Modified `claim_ready_task_postgres()`:
   - Now uses CTE with DELETE FROM task_outbox to claim tasks
   - Uses FOR UPDATE SKIP LOCKED on outbox entries (not task_executions)
   - Atomically deletes outbox row and updates task to Running
   - Emits TaskClaimed event in same transaction

2. Modified `claim_ready_task_sqlite()`:
   - Uses IMMEDIATE transaction for write lock
   - Selects from task_outbox (filtered by created_at <= NOW())
   - Deletes outbox entries, loads task details, updates to Running
   - Emits TaskClaimed event in same transaction

3. Modified `schedule_retry_postgres()` and `schedule_retry_sqlite()`:
   - Now inserts outbox entry with created_at = retry_at
   - This ensures tasks aren't claimed until retry time passes

4. Both implementations filter outbox by `created_at <= NOW()`:
   - Respects retry delays for scheduled retries
   - Immediate tasks (from mark_ready) have created_at = now

**Files Modified:**
- `crates/cloacina/src/dal/unified/task_execution/claiming.rs`

**Verification:**
- `cargo check --features sqlite` passes
- `cargo check --features postgres` passes
- All 287 unit tests pass
- Integration tests for task claiming pass (3/3)

**Test Fixes:**
- Updated `test_concurrent_task_claiming_no_duplicates` to use "NotStarted" status and call `mark_ready()`
- Updated `test_claimed_tasks_marked_running` similarly
- Added outbox count assertion to verify outbox is populated before workers claim

**Additional Fix:**
- Fixed `test_defer_until_with_downstream_dependency` test bug: task dependencies now use proper namespace format via new `SimpleTask::with_workflow()` helper

**Final Result:** All tests pass (287 unit + 203 integration)
