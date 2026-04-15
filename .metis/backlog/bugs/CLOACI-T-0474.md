---
id: fix-double-state-update-path
level: task
title: "Fix double state-update path between dispatcher and executor"
short_code: "CLOACI-T-0474"
created_at: 2026-04-11T13:33:09.011353+00:00
updated_at: 2026-04-11T15:52:23.670629+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Fix double state-update path between dispatcher and executor

## Objective

Eliminate duplicate state transitions where both `DefaultDispatcher::handle_result()` and `ThreadTaskExecutor::complete_task_transaction()` call `mark_completed()` for the same task. Fix the pipeline completion race condition.

## Review Finding References

COR-001, COR-004 (from architecture review `review/10-recommendations.md` REC-002)

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P0 - Critical (blocks users/revenue)

### Impact Assessment
- **Affected Users**: All users running workflows — duplicate execution events silently corrupt audit trails
- **Expected vs Actual**: One `TaskCompleted` event per task completion. Actual: two events emitted, with potentially conflicting timestamps and state.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `DefaultDispatcher::handle_result()` no longer calls `mark_completed()` or `mark_failed()` — dispatcher role is routing and logging only
- [ ] `check_pipeline_completion` and `update_pipeline_final_context` run within a single database transaction
- [ ] Pipeline completion uses `SELECT ... FOR UPDATE` on the pipeline execution record to prevent concurrent completion
- [ ] Integration test: execute a 2-task workflow, assert exactly 2 completion events (one per task), not 4
- [ ] No regression in existing integration tests

## Implementation Notes

### Technical Approach

1. **Remove dispatcher state transitions**: In `DefaultDispatcher::handle_result()`, change the `Completed` and `Failed` branches to only log the result (keep existing `tracing::info!` calls) without calling DAL state-transition methods. The executor already persists state in `complete_task_transaction()`.

2. **Transactional pipeline completion**: Wrap `check_pipeline_completion` query and `update_pipeline_final_context` writes in a single transaction. Use `SELECT ... FOR UPDATE` on the pipeline execution record to prevent concurrent completion when multiple tasks finish simultaneously.

3. **Verification test**: Execute a simple 2-task workflow, then query `execution_events` and assert exactly 2 completion events.

### Key Files
- `crates/cloacina/src/dispatcher/default.rs` — `handle_result()` with duplicate state calls
- `crates/cloacina/src/executor/thread_task_executor.rs` — `complete_task_transaction()` (authoritative path)
- `crates/cloacina/src/state_manager.rs` — `check_pipeline_completion`, `update_pipeline_final_context`

### Dependencies
None. Should be done before REC-007 (pipeline→workflow rename) and REC-008 (metrics) to avoid conflicts and clarify state ownership.

## Status Updates

**2026-04-11**: Implementation complete, pending compile check + tests.
- Removed `mark_completed()`/`mark_failed()` from `DefaultDispatcher::handle_result()`
- Added status guard in `scheduler_loop.rs::complete_pipeline()` — skips if pipeline already completed/failed
- Added `WHERE status = 'Running'` filter to pipeline DAL `mark_completed`/`mark_failed` (Postgres + SQLite) — events only inserted on actual transition
- Files: `dispatcher/default.rs`, `execution_planner/scheduler_loop.rs`, `dal/unified/pipeline_execution.rs`
