---
id: eliminate-n-1-query-pattern-in
level: initiative
title: "Eliminate N+1 Query Pattern in Task Execution DAL"
short_code: "CLOACI-I-0006"
created_at: 2025-11-29T02:40:07.223787+00:00
updated_at: 2025-12-06T03:31:16.051177+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
strategy_id: NULL
initiative_id: eliminate-n-1-query-pattern-in
---

# Eliminate N+1 Query Pattern in Task Execution DAL Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

The task execution DAL in `cloacina/src/dal/postgres_dal/task_execution.rs` (lines 137-173) exhibits N+1 query patterns where every task state transition requires two database queries:

```rust
pub async fn mark_ready(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
    let task = conn.interact(move |conn| {  // QUERY 1: SELECT
        task_executions::table.find(task_id.0).first::<TaskExecution>(conn)
    }).await??;

    conn.interact(move |conn| {  // QUERY 2: UPDATE
        diesel::update(task_executions::table.find(task_id_clone.0))
            .set((task_executions::status.eq("Ready"), ...))
            .execute(conn)
    }).await??;

    tracing::debug!("Task state change: {} -> Ready ...", task.status);
}
```

This pattern repeats in `mark_skipped`, `mark_running`, `mark_completed`, `mark_failed`, etc.

**Impact:** With 1000 tasks in a workflow, this results in 2000+ database queries. The scheduler loop iterates over pending tasks frequently, making this a critical hot path.

## Goals & Non-Goals

**Goals:**
- Reduce database queries per state transition from 2 to 1
- Implement batched state updates where possible
- Add query performance metrics/logging

**Non-Goals:**
- Changing the task state machine logic
- Caching task state in memory (could introduce consistency issues)

## Detailed Design

### Use RETURNING Clause for Single-Query Updates

Replace SELECT + UPDATE pattern with UPDATE ... RETURNING:

```rust
pub async fn mark_ready(&self, task_id: UniversalUuid) -> Result<TaskExecution, ValidationError> {
    let conn = self.pool.get().await?;

    conn.interact(move |conn| {
        diesel::update(task_executions::table.find(task_id.0))
            .set((
                task_executions::status.eq("Ready"),
                task_executions::updated_at.eq(diesel::dsl::now),
            ))
            .returning(TaskExecution::as_returning())
            .get_result(conn)
    }).await?
    .map_err(|e| ValidationError::DatabaseError(e.to_string()))
}
```

### Batch State Updates

For scheduler tick processing multiple tasks:

```rust
pub async fn mark_tasks_ready(&self, task_ids: &[UniversalUuid]) -> Result<Vec<TaskExecution>, ValidationError> {
    let conn = self.pool.get().await?;
    let ids: Vec<Uuid> = task_ids.iter().map(|t| t.0).collect();

    conn.interact(move |conn| {
        diesel::update(task_executions::table)
            .filter(task_executions::id.eq_any(&ids))
            .set((
                task_executions::status.eq("Ready"),
                task_executions::updated_at.eq(diesel::dsl::now),
            ))
            .returning(TaskExecution::as_returning())
            .get_results(conn)
    }).await?
    .map_err(|e| ValidationError::DatabaseError(e.to_string()))
}
```

### Add Query Metrics

```rust
use std::time::Instant;

pub async fn mark_ready(&self, task_id: UniversalUuid) -> Result<TaskExecution, ValidationError> {
    let start = Instant::now();
    let result = /* ... */;

    tracing::debug!(
        task_id = %task_id,
        duration_ms = start.elapsed().as_millis(),
        "Task state transition to Ready"
    );

    metrics::histogram!("dal.task_execution.mark_ready.duration_ms", start.elapsed().as_millis() as f64);

    result
}
```

## Affected Methods

All these methods need refactoring:
- `mark_ready()`
- `mark_running()`
- `mark_completed()`
- `mark_failed()`
- `mark_skipped()`
- `mark_cancelled()`

## Testing Strategy

- Benchmark before/after query counts
- Load test with 1000+ task workflows
- Verify RETURNING data matches expected state
- Test batch updates with varying sizes

## Alternatives Considered

1. **In-memory caching** - Risk of stale data, consistency issues
2. **Optimistic locking** - Adds complexity, doesn't reduce query count
3. **Event sourcing** - Major architectural change, out of scope

## Implementation Plan

1. Add RETURNING support to individual mark_* methods
2. Create batch versions of frequently-called methods
3. Update scheduler to use batch methods where possible
4. Add query metrics/logging
5. Benchmark and validate improvements
