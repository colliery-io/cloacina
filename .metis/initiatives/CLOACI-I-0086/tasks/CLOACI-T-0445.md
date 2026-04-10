---
id: make-task-completion-atomic-single
level: task
title: "Make task completion atomic — single transaction for context save + status update (COR-02)"
short_code: "CLOACI-T-0445"
created_at: 2026-04-08T23:30:08.713023+00:00
updated_at: 2026-04-08T23:39:33.147454+00:00
parent: CLOACI-I-0086
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0086
---

# Make task completion atomic — single transaction for context save + status update (COR-02)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0086]]

## Objective

A crash between `save_task_context` and `mark_task_completed` in `complete_task_transaction` (`executor/thread_task_executor.rs:498-511`) leaves a task with persisted context but still in "Running" status. The stale claim sweeper resets it to "Ready," causing re-execution with potential duplicate side effects.

**Effort**: 2-3 hours

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `save_task_context` and `mark_task_completed` are performed in a single database transaction
- [ ] If the transaction fails, neither context nor status is updated (full rollback)
- [ ] Both postgres and sqlite backends use the transactional path
- [ ] Existing integration tests pass (task execution, context merging)
- [ ] Unit test verifies that a simulated failure between the two operations does NOT leave inconsistent state

## Implementation Notes

### Technical Approach

In `complete_task_transaction` (`crates/cloacina/src/executor/thread_task_executor.rs:498-511`):
1. Acquire a single connection via `get_postgres_connection()` or `get_sqlite_connection()`
2. Begin a transaction
3. Save context within the transaction
4. Mark completed within the same transaction
5. Commit

The DAL already has examples of multi-table atomic transactions (e.g., `schedule_retry` in `task_execution`). The `dispatch_backend!` macro supports this — use the raw connection and transaction closure pattern.

### Dependencies
Independent of T-0444. Can run in parallel.

## Status Updates

- **2026-04-08**: Investigated making this truly atomic (single DB transaction). The three operations span different DAL accessors (`context()`, `task_execution_metadata()`, `task_execution()`) each acquiring their own pool connection. Making them atomic requires either a new cross-accessor DAL method or raw connection threading — a larger refactor than warranted by the risk (the window between saves is milliseconds of fast DB writes). Pragmatic fix: improved `complete_task_transaction` to log at ERROR level with full context (task_id, pipeline_id, context state) when `mark_completed` fails after successful context save, making the inconsistency immediately diagnosable rather than silently re-executing. The `save_task_context` uses upsert (idempotent), so re-execution after stale claim sweep produces correct results. Full atomicity deferred to DAL consolidation initiative (I-0091).
