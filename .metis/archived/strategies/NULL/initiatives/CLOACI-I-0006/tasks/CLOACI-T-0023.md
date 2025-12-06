---
id: refactor-create-and-reset-task-for
level: task
title: "Refactor create and reset_task_for_recovery to use RETURNING"
short_code: "CLOACI-T-0023"
created_at: 2025-12-06T02:46:35.422111+00:00
updated_at: 2025-12-06T02:46:35.422111+00:00
parent: CLOACI-I-0006
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0006
---

# Refactor create and reset_task_for_recovery to use RETURNING

## Parent Initiative

[[CLOACI-I-0006]]

## Objective

Eliminate N+1 query patterns in `create_postgres`, `create_sqlite`, and `reset_task_for_recovery_sqlite` methods by using RETURNING clause for inserts and eliminating unnecessary pre-SELECT queries.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `create_postgres` uses `.get_result()` with RETURNING instead of INSERT + SELECT
- [ ] `create_sqlite` uses `.get_result()` with RETURNING instead of INSERT + SELECT
- [ ] `reset_task_for_recovery_sqlite` uses SQL expression `recovery_attempts + 1` instead of SELECT + UPDATE
- [ ] All existing tests pass
- [ ] No functional changes to return values or behavior

## Implementation Notes

### Technical Approach

#### create_postgres / create_sqlite (lines 89-174)

**Current Pattern (2 queries):**
```rust
// Query 1: INSERT
diesel::insert_into(task_executions::table)
    .values(&new_unified_task)
    .execute(conn)?;

// Query 2: SELECT to get the inserted row
let task: UnifiedTaskExecution = task_executions::table.find(id).first(conn)?;
```

**Target Pattern (1 query with RETURNING):**
```rust
// Single query: INSERT with RETURNING
let task: UnifiedTaskExecution = diesel::insert_into(task_executions::table)
    .values(&new_unified_task)
    .get_result(conn)?;
```

#### reset_task_for_recovery_sqlite (lines 1263-1302)

**Current Pattern (2 queries):**
```rust
// Query 1: SELECT to get current recovery_attempts
let current_recovery: i32 = task_executions::table
    .find(task_id)
    .select(task_executions::recovery_attempts)
    .first(conn)?;

// Query 2: UPDATE with current_recovery + 1
diesel::update(task_executions::table.find(task_id))
    .set(task_executions::recovery_attempts.eq(current_recovery + 1))
    .execute(conn)?;
```

**Target Pattern (1 query - same as postgres version):**
```rust
// Single query: UPDATE using SQL expression
diesel::update(task_executions::table.find(task_id))
    .set(task_executions::recovery_attempts.eq(task_executions::recovery_attempts + 1))
    .execute(conn)?;
```

Note: The PostgreSQL version already uses this pattern correctly (lines 1234-1261).

### Dependencies

- Depends on T-0022 being complete first (mark_ready/mark_skipped refactoring)

### Risk Considerations

- Low risk: These are straightforward Diesel API changes
- The RETURNING clause is supported by both PostgreSQL and SQLite
- SQLite column arithmetic (`col + 1`) is standard SQL and works correctly

## Status Updates

*To be added during implementation*
