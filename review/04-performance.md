# Performance Review

## Summary

Cloacina's performance characteristics are generally appropriate for an embedded workflow orchestration library. The architecture demonstrates intentional optimization of hot paths -- batch queries for dependency checking, outbox-based task claiming with `FOR UPDATE SKIP LOCKED`, connection pooling, and semaphore-based concurrency control. The most significant performance concerns are: an N+1 query pattern in pipeline completion that executes per-task metadata lookups in a loop, unbounded `get_ready_for_retry` queries that scan the full `task_executions` table, and a `cron_max_catchup_executions` default of `usize::MAX` that could cause runaway execution storms after scheduler downtime.

## Workload Assessment

Cloacina is designed for embedded, in-process workflow orchestration with the following expected workload profile:

- **Workflows**: Tens to low hundreds of concurrent pipeline executions
- **Tasks per workflow**: Typically 2-50 tasks in a DAG
- **Scheduler frequency**: Default poll interval of 100ms
- **Concurrency**: Default 4 concurrent task executions per runner instance
- **Database**: PostgreSQL for production (pooled, max 10 connections default), SQLite for development (single connection)
- **Computation graphs**: Continuous reactive processing with event-driven accumulators and reactors

The system is not designed for massive scale (10k+ concurrent pipelines), but should handle moderate workloads efficiently. Performance findings are assessed against this expected workload.

## Hot Path Analysis

### 1. Scheduler Loop (highest frequency -- every 100ms)
**Path**: `SchedulerLoop::run()` -> `process_active_pipelines()` -> `process_pipelines_batch()` -> `dispatch_ready_tasks()`

This is the main execution loop that drives the entire system. Key operations per tick:
- Query for active pipeline executions
- Batch-load pending tasks across all pipelines (single query -- good)
- For each pipeline's pending tasks, check dependencies and trigger rules
- Check pipeline completion
- Dispatch ready tasks

**Assessment**: Well-optimized. The batch loading of pending tasks (`get_pending_tasks_batch`) avoids N+1 at the pipeline level. Dependency checking uses batch status queries (`get_task_statuses_batch`). The circuit breaker with exponential backoff protects against sustained errors.

### 2. Task Claiming (per ready task dispatch)
**Path**: `claim_ready_task()` via outbox table

**Assessment**: Well-designed. PostgreSQL uses `FOR UPDATE SKIP LOCKED` with a CTE that atomically deletes from outbox and updates task status. SQLite uses `IMMEDIATE` transactions for serialized access. Both are appropriate for their backends.

### 3. Task Execution (per task)
**Path**: `ThreadTaskExecutor::execute()` -> `build_task_context()` -> task function -> `complete_task_transaction()`

**Assessment**: Semaphore-based concurrency control is appropriate. Context building uses batch dependency metadata loading. The SlotToken pattern allows deferred tasks to release their concurrency slot, which is a thoughtful optimization.

### 4. Computation Graph Reactor (continuous)
**Path**: `Reactor::run()` -- receives boundaries from accumulators, evaluates firing criteria, executes graph function

**Assessment**: Uses bounded `mpsc` channels from accumulators. Health state machine provides degradation detection. Backpressure exists via channel capacity.

## Findings

## PERF-001: N+1 query pattern in pipeline final context resolution
**Severity**: Major
**Location**: `crates/cloacina/src/execution_planner/scheduler_loop.rs`, `update_pipeline_final_context()`, lines 359-416
**Confidence**: High

### Description
When a pipeline completes, `update_pipeline_final_context()` iterates over all tasks and calls `self.dal.task_execution_metadata().get_by_pipeline_and_task()` individually for each completed/skipped task. For a workflow with N tasks, this results in N individual database queries to find the final context.

### Evidence
```rust
for task in all_tasks {
    if task.status == "Completed" || task.status == "Skipped" {
        if let Some(completed_at) = task.completed_at {
            if let Ok(task_metadata) = self
                .dal
                .task_execution_metadata()
                .get_by_pipeline_and_task(pipeline_execution_id, &task_namespace)
                .await
            {
                // ...per-task DB lookup
            }
        }
    }
}
```

This is called from `complete_pipeline()` which already loads all tasks via `get_all_tasks_for_pipeline()`. The metadata could be batch-loaded in a single query.

### Suggested Resolution
Add a `get_metadata_batch_by_pipeline(pipeline_execution_id)` query to the `task_execution_metadata` DAL that loads all metadata for a pipeline in a single query. Filter and find the latest context in memory. The data is already small (one row per task), so loading all at once is safe.

---

## PERF-002: Unbounded `get_ready_for_retry` scans full table
**Severity**: Major
**Location**: `crates/cloacina/src/dal/unified/task_execution/claiming.rs`, `get_ready_for_retry_postgres()` / `get_ready_for_retry_sqlite()`, lines 777-828
**Confidence**: High

### Description
The `get_ready_for_retry()` query loads ALL tasks with `status = 'Ready'` where `retry_at` is null or in the past. This query has no `LIMIT` clause and scans the entire `task_executions` table (filtered by status index). In a system with many pending retries or after a recovery event, this could return an unbounded result set.

### Evidence
```rust
task_executions::table
    .filter(task_executions::status.eq("Ready"))
    .filter(
        task_executions::retry_at
            .is_null()
            .or(task_executions::retry_at.le(now)),
    )
    .load(conn)
```

This query is called on every scheduler tick (100ms default) by `dispatch_ready_tasks()`. While the `task_executions_status_idx` partial index on `status = 'Running'` exists, there is no corresponding index for `status = 'Ready'` combined with `retry_at`.

### Suggested Resolution
1. Add a `LIMIT` parameter to match the executor's available capacity (e.g., `semaphore.available_permits()`), so only dispatchable tasks are loaded.
2. Consider adding a composite index: `CREATE INDEX idx_task_executions_ready_retry ON task_executions(status, retry_at) WHERE status = 'Ready'`.

---

## PERF-003: Trigger condition evaluation issues individual queries per condition
**Severity**: Minor
**Location**: `crates/cloacina/src/execution_planner/state_manager.rs`, `evaluate_condition()`, lines 245-321
**Confidence**: Medium

### Description
When trigger rules contain multiple `TaskSuccess`/`TaskFailed`/`TaskSkipped` conditions, each condition makes an individual `get_task_status()` call. While dependency checking was upgraded to use `get_task_statuses_batch()`, the trigger condition evaluation path was not.

### Evidence
```rust
TriggerCondition::TaskSuccess { task_name } => {
    let status = self
        .dal
        .task_execution()
        .get_task_status(task_execution.pipeline_execution_id, task_name)
        .await?;
    // ...
}
TriggerCondition::TaskFailed { task_name } => {
    let status = self
        .dal
        .task_execution()
        .get_task_status(task_execution.pipeline_execution_id, task_name)
        .await?;
    // ...
}
```

For the common `TriggerRule::Always` case, no queries are made. The issue only manifests with complex trigger rules containing multiple task-status conditions.

### Suggested Resolution
Pre-collect all task names referenced in trigger conditions, batch-fetch their statuses with `get_task_statuses_batch()`, then evaluate conditions against the in-memory map. This follows the same pattern already used in `check_task_dependencies()`.

---

## PERF-004: Default `cron_max_catchup_executions` is `usize::MAX`
**Severity**: Major
**Location**: `crates/cloacina/src/runner/default_runner/config.rs`, line 271
**Confidence**: High

### Description
The default value for `cron_max_catchup_executions` is `usize::MAX`, which means that after extended scheduler downtime, the system will attempt to execute every missed cron invocation without limit. A cron job scheduled to run every minute that was down for a day would attempt to execute 1,440 backlogged invocations simultaneously.

### Evidence
```rust
cron_max_catchup_executions: usize::MAX,
```

While the `SchedulerConfig` (cron_trigger_scheduler.rs) has a default of 100, the `DefaultRunnerConfig` builder overrides this to `usize::MAX`, meaning users who rely on the default runner get the unbounded behavior.

### Suggested Resolution
Change the default to a reasonable bounded value (e.g., 10 or 100) that prevents execution storms while still allowing catchup for brief outages. The `SchedulerConfig` default of 100 is already more reasonable and these should be aligned.

---

## PERF-005: SQLite connection pool hardcoded to size 1
**Severity**: Minor
**Location**: `crates/cloacina/src/database/connection/mod.rs`, lines 239, 293
**Confidence**: High

### Description
The SQLite connection pool is hardcoded to size 1, ignoring the `max_size` parameter passed by the caller. While SQLite in WAL mode supports concurrent reads with a single writer, the pool size of 1 means all database operations are serialized through a single connection, including reads.

### Evidence
```rust
BackendType::Sqlite => {
    let sqlite_pool_size = 1;
    let pool = SqlitePool::builder(manager)
        .max_size(sqlite_pool_size)
        .build()
```

The caller passes `max_size` (default 10) which is silently ignored. With WAL mode enabled (set in `run_migrations`), a small pool (e.g., 2-4) could allow concurrent reads while maintaining write serialization.

### Suggested Resolution
This is arguably intentional for SQLite's concurrency model but should be documented. If concurrent reads are desired (e.g., scheduler reading while executor writes), consider allowing a small pool (2-4) with WAL mode. At minimum, log a warning when `max_size > 1` is requested but 1 is used, so users understand why their configuration is overridden.

---

## PERF-006: Duplicate context loading in StateManager and ContextManager
**Severity**: Minor
**Location**: `crates/cloacina/src/execution_planner/state_manager.rs` lines 96-145 and `crates/cloacina/src/execution_planner/context_manager.rs` lines 47-144
**Confidence**: Medium

### Description
Both `StateManager::check_task_dependencies()` and `ContextManager::load_context_for_task()` independently fetch the pipeline execution record and the workflow from the runtime. When trigger rules include `ContextValue` conditions, `evaluate_condition()` creates a new `ContextManager` and calls `load_context_for_task()`, which re-fetches the pipeline and workflow that `StateManager` already looked up.

### Evidence
In `state_manager.rs`:
```rust
let pipeline = self.dal.workflow_execution().get_by_id(task_execution.pipeline_execution_id).await?;
let workflow = self.runtime.get_workflow(&pipeline.pipeline_name);
```

In `context_manager.rs` (called from `evaluate_condition`):
```rust
let pipeline = self.dal.workflow_execution().get_by_id(task_execution.pipeline_execution_id).await?;
let workflow = self.runtime.get_workflow(&pipeline.pipeline_name);
```

### Suggested Resolution
Pass the already-fetched pipeline and workflow through to the context manager to avoid the redundant lookups. Alternatively, introduce a lightweight per-tick cache for pipeline records since the same pipeline is looked up repeatedly for each pending task within it.

---

## PERF-007: Execution events table grows without automatic retention
**Severity**: Observation
**Location**: `crates/cloacina/src/database/migrations/postgres/012_create_execution_events_and_outbox/up.sql`; `crates/cloacinactl/src/main.rs` (cleanup-events command)
**Confidence**: Medium

### Description
The `execution_events` table is append-only and grows with every task state transition (claimed, running, completed, failed, retry-scheduled). For active systems, this table will grow unboundedly. While `cloacinactl admin cleanup-events` exists, there is no automatic retention policy. The events table has indexes on `created_at DESC` and `pipeline_execution_id`, but without periodic cleanup, these indexes will grow large.

### Evidence
Each task execution creates multiple events (claimed, completed/failed). For a workflow with 10 tasks that each retry once, a single pipeline execution generates approximately 40 events. At 100 pipeline executions per day, the table grows by 4,000 rows daily. The `idx_execution_events_created` index facilitates cleanup but cleanup must be run manually.

### Suggested Resolution
Consider adding an optional auto-retention policy to the `DefaultRunnerConfig` (e.g., `event_retention_days: Option<u32>`) that spawns a periodic cleanup background task. Alternatively, document the importance of running `cleanup-events` in production deployment guides. For PostgreSQL, table partitioning by time range could also help.

---

## PERF-008: Dispatcher executes tasks synchronously in dispatch loop
**Severity**: Observation
**Location**: `crates/cloacina/src/dispatcher/default.rs`, `dispatch()` method, lines 143-170
**Confidence**: Medium

### Description
The `DefaultDispatcher::dispatch()` method calls `executor.execute(event.clone()).await` and then `self.handle_result(&event, result).await` synchronously. This means the `dispatch_ready_tasks()` loop in the scheduler dispatches tasks one at a time, waiting for each to be claimed before dispatching the next. While the executor uses a semaphore for concurrent execution, the dispatch serialization means there is a latency ceiling on how fast ready tasks can be handed off.

### Evidence
```rust
async fn dispatch(&self, event: TaskReadyEvent) -> Result<(), DispatchError> {
    // ...
    let result = executor.execute(event.clone()).await?;
    self.handle_result(&event, result).await?;
    Ok(())
}
```

And the calling loop:
```rust
for task in ready_tasks {
    // ...
    if let Err(e) = dispatcher.dispatch(event).await {
        warn!("Failed to dispatch ready task");
    }
}
```

### Suggested Resolution
For the current workload (4 concurrent tasks, typical workflows with a few tasks), this is unlikely to be a bottleneck. If scaling to higher concurrency is needed, dispatching could be parallelized with `tokio::spawn` or `FuturesUnordered`. The capacity check (`has_capacity()`) already provides the gating mechanism. This is noted as an observation, not a problem at current scale.

---

## PERF-009: Context merge uses `contains()` for array deduplication
**Severity**: Observation
**Location**: `crates/cloacina/src/executor/thread_task_executor.rs`, `merge_context_values()`, lines 317-352
**Confidence**: Low

### Description
When merging arrays from multiple dependency contexts, `merge_context_values()` uses `merged.contains(item)` to check for duplicates before inserting. For `serde_json::Value`, `contains()` is O(n) per check, making the deduplication O(n*m) where n and m are the array lengths.

### Evidence
```rust
(Value::Array(existing_arr), Value::Array(new_arr)) => {
    let mut merged = existing_arr.clone();
    for item in new_arr {
        if !merged.contains(item) {
            merged.push(item.clone());
        }
    }
    Value::Array(merged)
}
```

### Suggested Resolution
For typical workflow contexts, arrays are small (a few elements), so this is unlikely to be a practical concern. If large arrays are anticipated, a `HashSet`-based approach or preserving insertion order with `IndexSet` would reduce complexity to O(n+m). However, `serde_json::Value` does not implement `Hash`, so this would require a wrapper. Given expected workloads, this is not worth the complexity.
