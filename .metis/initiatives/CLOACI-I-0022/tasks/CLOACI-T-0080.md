---
id: executionevent-model-and-dal-with
level: task
title: "ExecutionEvent model and DAL with event emission at state transitions"
short_code: "CLOACI-T-0080"
created_at: 2026-02-03T20:16:45.089628+00:00
updated_at: 2026-02-04T13:37:10.567233+00:00
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

# ExecutionEvent model and DAL with event emission at state transitions

## Parent Initiative

[[CLOACI-I-0022]] - Execution Events and Outbox-Based Task Distribution

## Objective

Create the `ExecutionEvent` model, DAL, and integrate event emission at all task/pipeline state transition points.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ExecutionEvent` and `NewExecutionEvent` model structs
- [ ] `ExecutionEventDAL` with `create()` and query methods
- [ ] Event emission integrated at all DAL state transition points
- [ ] Event types enum covering full task/pipeline lifecycle
- [ ] Query methods: `list_by_pipeline()`, `list_by_task()`, `list_by_type()`

## Implementation Notes

### Event Types

```rust
pub enum ExecutionEventType {
    // Task lifecycle
    TaskCreated,
    TaskMarkedReady,
    TaskClaimed,
    TaskStarted,
    TaskDeferred,
    TaskResumed,
    TaskCompleted,
    TaskFailed,
    TaskRetryScheduled,
    TaskSkipped,
    TaskAbandoned,

    // Pipeline lifecycle
    PipelineStarted,
    PipelineCompleted,
    PipelineFailed,
    PipelinePaused,
    PipelineResumed,
}
```

### Integration Points

Emit events in existing DAL methods:
- `TaskExecutionDAL::mark_ready()` → `TaskMarkedReady`
- `TaskExecutionDAL::claim()` → `TaskClaimed`
- `TaskExecutionDAL::mark_completed()` → `TaskCompleted`
- `TaskExecutionDAL::mark_failed()` → `TaskFailed`
- `TaskExecutionDAL::schedule_retry()` → `TaskRetryScheduled`
- `PipelineExecutionDAL::mark_completed()` → `PipelineCompleted`
- etc.

### Dependencies

- Requires CLOACI-T-0079 (schema migrations)

## Progress

### Session 2 (continued from compaction)
- Read existing patterns: recovery_event.rs model, unified/models.rs, unified/recovery_event.rs DAL, schema.rs
- Schema migrations already created in T-0079 (postgres/012, sqlite/011)
- Completed implementation of:
  1. models/execution_event.rs - domain model with ExecutionEvent, NewExecutionEvent, ExecutionEventType enum
  2. database/schema.rs - added execution_events and task_outbox to all three schema modules
  3. dal/unified/models.rs - added UnifiedExecutionEvent, NewUnifiedExecutionEvent, UnifiedTaskOutbox, NewUnifiedTaskOutbox
  4. dal/unified/execution_event.rs - ExecutionEventDAL with create(), list_by_pipeline(), list_by_task(), list_by_type(), delete_older_than()
  5. dal/unified/mod.rs - registered ExecutionEventDAL and accessor method
- Build check passed with cargo check
- All 287 unit tests pass
- Completed event emission integration at all state transition points:
  - TaskExecutionDAL: mark_completed, mark_failed, mark_ready, mark_skipped, mark_abandoned, set_sub_status, reset_retry_state, schedule_retry, claim_ready_task, create
  - PipelineExecutionDAL: create, mark_completed, mark_failed, pause, resume
- Build passes with cargo check
- All 287 unit tests pass

## Implementation Complete

All acceptance criteria met:
- [x] `ExecutionEvent` and `NewExecutionEvent` model structs
- [x] `ExecutionEventDAL` with `create()` and query methods
- [x] Event emission integrated at all DAL state transition points
- [x] Event types enum covering full task/pipeline lifecycle
- [x] Query methods: `list_by_pipeline()`, `list_by_task()`, `list_by_type()`

### Session 3 - Transactional Guarantees Refactor

**Critical requirement from user**: Event emission must be transactional with state changes - not best-effort.

Original implementation used a pattern where events were emitted after the state change with failure logged as warning:
```rust
// OLD PATTERN (broken guarantees)
self.update_status(...).await?;
if let Err(e) = self.dal.execution_event().create(...).await {
    tracing::warn!(...);  // Silent failure - unacceptable
}
```

Refactored all DAL methods to use `conn.transaction()` wrapping both the state update and event insertion:
```rust
// NEW PATTERN (atomic guarantees)
conn.transaction::<_, diesel::result::Error, _>(|conn| {
    // 1. Update task/pipeline status
    diesel::update(...).execute(conn)?;
    // 2. Insert execution event (same transaction)
    diesel::insert_into(execution_events::table).values(&event).execute(conn)?;
    Ok(())
})
```

**Files refactored:**
- `task_execution/state.rs` - mark_completed, mark_failed, mark_ready, mark_skipped, mark_abandoned, set_sub_status, reset_retry_state
- `task_execution/claiming.rs` - schedule_retry, claim_ready_task
- `task_execution/crud.rs` - create (TaskCreated)
- `pipeline_execution.rs` - create, mark_completed, mark_failed, pause, resume

**Result:** All 287 unit tests pass. Event emission is now atomic with state changes - if either fails, both are rolled back.

## Status Updates

- 2026-02-04: Refactored all event emission to be transactional (not best-effort)
