---
id: refactor-mark-ready-and-mark
level: task
title: "Refactor mark_ready and mark_skipped to eliminate N+1 queries"
short_code: "CLOACI-T-0022"
created_at: 2025-12-06T02:46:35.266515+00:00
updated_at: 2025-12-06T02:46:35.266515+00:00
parent: CLOACI-I-0006
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0006
---

# Refactor mark_ready and mark_skipped to eliminate N+1 queries

## Parent Initiative

[[CLOACI-I-0006]]

## Objective

Eliminate the N+1 query pattern in `mark_ready` and `mark_skipped` methods by removing the pre-SELECT query used only for logging. The current pattern fetches task info before updating, resulting in 2 queries per state transition.

## Acceptance Criteria

- [ ] `mark_ready_postgres` reduced from 2 queries to 1
- [ ] `mark_ready_sqlite` reduced from 2 queries to 1
- [ ] `mark_skipped_postgres` reduced from 2 queries to 1
- [ ] `mark_skipped_sqlite` reduced from 2 queries to 1
- [ ] Logging still includes relevant task info (task_id instead of task_name)
- [ ] All existing tests pass
- [ ] cargo check passes for both backends

## Implementation Notes

### Current Pattern (N+1)
```rust
// Query 1: SELECT for logging
let task = task_executions::table.find(task_id).first(conn)?;
let task_status = task.status.clone();

// Query 2: UPDATE
diesel::update(task_executions::table.find(task_id))
    .set((status.eq("Ready"), ...))
    .execute(conn)?;

tracing::debug!("Task state change: {} -> Ready", task_status);
```

### Target Pattern (Single Query)
```rust
// Single UPDATE - use task_id for logging instead of fetching name
diesel::update(task_executions::table.find(task_id))
    .set((status.eq("Ready"), ...))
    .execute(conn)?;

tracing::debug!("Task {} marked as Ready", task_id);
```

### Affected Methods
- `mark_ready_postgres` (lines 837-873)
- `mark_ready_sqlite` (lines 876-913)
- `mark_skipped_postgres` (lines 927-971)
- `mark_skipped_sqlite` (lines 974-1018)

### Trade-offs
- Logging will show task_id instead of task_name (acceptable for debug logs)
- If task_name is needed, caller can provide it or we add an optional parameter

## Status Updates

*To be added during implementation*
