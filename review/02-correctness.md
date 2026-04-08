# Correctness Review

## Summary

Cloacina demonstrates strong correctness foundations in its core paths: DAL state transitions are fully transactional (status + event atomically), the outbox-based task claiming uses `FOR UPDATE SKIP LOCKED` on Postgres and `IMMEDIATE` transactions on SQLite to prevent double-claim, and the dependency graph cycle detection and topological sorting are correctly implemented with petgraph. The primary correctness gaps are concentrated in three areas: (1) the pipeline always marks "Completed" regardless of whether tasks failed, (2) the `complete_task_transaction` in the executor is not actually atomic -- context save and status update are separate database operations, and (3) the `SchedulerLoop::run()` has no shutdown mechanism and will loop forever. There are also several error-handling patterns that silently swallow failures or produce misleading error variants.

## Test Coverage Assessment

**What is tested well:**
- Unit tests for all core data structures: `DependencyGraph`, `WorkflowGraph`, `DirtyFlags`, `TriggerRule`/`TriggerCondition` serialization roundtrips, `ValueOperator` evaluation, `CronEvaluator` timezone handling, Ed25519 signing/verification, API key generation, context merge logic, `PipelineStatus` parsing, and transient error classification.
- Integration tests exist for DAL operations, executor behavior, scheduler state transitions, signing workflows, packaging, registry loading, and the computation graph reactor.
- The test suite is substantial (77 files with `#[cfg(test)]` modules, plus 26+ integration test files).

**What is not tested or under-tested:**
- Pipeline completion status determination -- no test verifies that a pipeline with failed tasks is marked as "Failed" (because it cannot be; see COR-01).
- The `complete_task_transaction` non-atomicity -- no test exercises a failure between `save_task_context` and `mark_task_completed`.
- `SchedulerLoop::run()` -- no test verifies shutdown behavior (because it has none).
- Concurrent claiming -- the SQLite claiming path lacks a concurrent stress test to verify the IMMEDIATE transaction prevents double-claims in practice.
- `WorkflowGraph::add_task` -- no test for adding a task with a duplicate ID (it silently overwrites the `task_index` entry).
- `find_parallel_groups` sorting -- the sort is by group size (ascending), not by depth level. No test verifies execution order semantics.
- Error path coverage for `merge_dependency_contexts` -- errors from individual dependency context loads are silently swallowed with `if let Ok(...)`.
- `TriggerRule::All` / `TriggerRule::Any` / `TriggerRule::None` with empty conditions -- the behavioral semantics (All-empty = true, Any-empty = false, None-empty = true) are tested for serialization but not for evaluation.

## Key Risk Areas

1. **Pipeline status always "Completed"** -- the most impactful correctness gap. Pipelines with failed tasks are indistinguishable from fully successful ones.
2. **Non-atomic task completion** -- a crash between context save and status mark leaves a task in an inconsistent state (context persisted but task still "Running").
3. **SchedulerLoop has no shutdown** -- it runs `loop { ... }` without a shutdown channel, unlike the `UnifiedScheduler` and `StaleClaimSweeper` which both have `watch::Receiver<bool>` shutdown support.

## Findings

### COR-01: Pipeline marked "Completed" even when tasks failed (Critical)

**File:** `crates/cloacina/src/task_scheduler/scheduler_loop.rs` lines 175-179, `crates/cloacina/src/dal/unified/pipeline_execution.rs` lines 361-414

**Evidence:** `check_pipeline_completion()` returns `true` when all tasks are in terminal states (`Completed`, `Failed`, or `Skipped`). The `complete_pipeline()` method then unconditionally calls `self.dal.pipeline_execution().mark_completed(execution.id)`, which hardcodes the status string to `"Completed"` and emits a `PipelineCompleted` event.

A pipeline where every task failed will be marked `status = "Completed"`. Users calling `get_execution_status()` / `get_execution_result()` will see `PipelineStatus::Completed` and may incorrectly assume success. The `complete_pipeline` method does log the failed count, but the database state and API response are wrong.

**Impact:** Any consumer relying on pipeline status to detect failures (API clients, cron-triggered retry logic, monitoring) will miss failures entirely.

**Fix:** After `check_pipeline_completion` returns true, inspect the task statuses. If any task is `Failed`, mark the pipeline as `"Failed"` with an appropriate error message. The DAL already has an `update_status` method that accepts an arbitrary status string.

---

### COR-02: `complete_task_transaction` is not actually atomic (Major)

**File:** `crates/cloacina/src/executor/thread_task_executor.rs` lines 498-511

**Evidence:** Despite its name, `complete_task_transaction` performs two separate async operations:
```rust
async fn complete_task_transaction(...) -> Result<(), ExecutorError> {
    self.save_task_context(claimed_task, context).await?;
    self.mark_task_completed(claimed_task.task_execution_id).await?;
    Ok(())
}
```

Each of these acquires a separate database connection from the pool and runs its own transaction. If the process crashes or the connection pool fails between the two calls, the context will be saved (with a task_execution_metadata record pointing to it) but the task will remain in `Running` status. The stale claim sweeper will eventually reset it to `Ready`, causing it to re-execute and potentially produce duplicate side effects.

**Impact:** Data inconsistency on crash; potential duplicate task execution.

**Fix:** Combine both operations into a single database transaction. The DAL already supports this pattern (see `schedule_retry` which writes multiple tables atomically).

---

### COR-03: `SchedulerLoop::run()` has no shutdown mechanism (Major)

**File:** `crates/cloacina/src/task_scheduler/scheduler_loop.rs` lines 82-97

**Evidence:** The `run()` method is an infinite `loop {}` with only `interval.tick().await` and `process_active_pipelines()`. Compare with `StaleClaimSweeper::run()` (which uses `tokio::select!` with `self.shutdown_rx.changed()`) and `UnifiedScheduler::run()` (which also uses `shutdown.changed()`). The `SchedulerLoop` struct does not even have a shutdown channel field.

**Impact:** Once started, this loop can only be stopped by dropping the tokio runtime or aborting the task handle. This prevents graceful shutdown and can leave in-flight database operations incomplete.

**Fix:** Add a `watch::Receiver<bool>` shutdown channel to `SchedulerLoop` (matching the pattern used elsewhere) and integrate it via `tokio::select!` in the loop.

---

### COR-04: Error conversion maps Database/ConnectionPool errors to `KeyNotFound` (Major)

**File:** `crates/cloacina/src/error.rs` lines 354-378

**Evidence:** The `From<ContextError> for TaskError` implementation converts `ContextError::Database(e)` to `ContextError::KeyNotFound(format!("Database error: {}", e))` and `ContextError::ConnectionPool(msg)` to `ContextError::KeyNotFound(format!("Connection pool error: {}", msg))`. This means a database connectivity failure will surface to the user as a "Key not found" error with a misleading message like `"Key not found: Database error: connection refused"`.

**Impact:** Misleading error messages make debugging difficult. Retry logic that checks error types may incorrectly treat infrastructure failures as application-level key-not-found errors (non-retryable).

**Fix:** Either add `Database` and `ConnectionPool` variants to `cloacina_workflow::ContextError`, or use `TaskError::ExecutionFailed` instead of `TaskError::ContextError` for these infrastructure errors.

---

### COR-05: `WorkflowGraph::add_task` silently overwrites duplicate task IDs (Minor)

**File:** `crates/cloacina/src/graph.rs` lines 91-95

**Evidence:** `add_task` inserts into both `self.graph` (always creates a new node) and `self.task_index` (HashMap insert that overwrites on collision). If called with two `TaskNode`s having the same `id`, the `task_index` will point to the second node but the first node will still exist in the graph as an orphan, invisible to ID-based lookups but counted in iteration.

Note: The `Workflow::add_task` method in `workflow/mod.rs` does check for duplicates and returns an error, so this is only exploitable through direct `WorkflowGraph` API usage. The `WorkflowGraph` is mainly used for serializable metadata, not the core execution path.

**Impact:** Orphaned graph nodes that are invisible to ID-based operations but visible in iteration-based operations. Low impact since `Workflow::add_task` guards against this.

**Fix:** Return a `Result` from `WorkflowGraph::add_task` and reject duplicates, or document that duplicate IDs cause the old node to be orphaned.

---

### COR-06: `find_parallel_groups` sorts by group size, not depth level (Minor)

**File:** `crates/cloacina/src/graph.rs` lines 251-262

**Evidence:** `find_parallel_groups()` computes depth-based groups correctly, then sorts the result by `group.len()` (ascending). This means a depth-0 group with 1 task sorts before a depth-1 group with 3 tasks, but a depth-2 group with 1 task would also sort before the depth-1 group. The execution order is by group size, not by dependency depth.

The test at line 455 only checks that there are 3 groups, not their order.

**Impact:** If a consumer iterates `find_parallel_groups()` expecting execution order (depth 0 first, then depth 1, etc.), tasks could be scheduled out of dependency order. However, the actual execution engine uses `topological_sort()` and the scheduler's dependency checking, not `find_parallel_groups()`, so this does not affect production execution.

**Fix:** Sort by depth level: `result.sort_by_key(|_| groups.keys().min())` or use a `BTreeMap` keyed by depth.

---

### COR-07: Stale claim sweeper log over-reports released claims (Minor)

**File:** `crates/cloacina/src/task_scheduler/stale_claim_sweeper.rs` lines 183-186

**Evidence:** The final log message says `"{} claims released"` using `stale_claims.len()`, but the loop above uses `continue` on errors (lines 155, 168). If some claims fail to release, the count will overstate the number actually released.

**Impact:** Misleading operational log messages. Low severity but can complicate incident investigation.

**Fix:** Track a `released_count` counter and increment only on success.

---

### COR-08: `PipelineStatus::from_str` silently defaults invalid strings to `Failed` (Observation)

**File:** `crates/cloacina/src/executor/pipeline_executor.rs` lines 508-518

**Evidence:** The case-sensitive `from_str` method returns `PipelineStatus::Failed` for any unrecognized string (including lowercase variants like `"running"` or `"completed"`). This is tested and intentional, but the silent defaulting could mask data corruption or serialization bugs by treating them as failures rather than surfacing them.

**Impact:** Low -- the method is `pub(crate)` and the test explicitly documents the behavior. However, a `Result`-returning API would be safer for detecting corrupt data.

---

### COR-09: Sequential reactor queue persistence timing gap (Observation)

**File:** `crates/cloacina/src/computation_graph/reactor.rs` lines 603-635

**Evidence:** In `InputStrategy::Sequential` mode, the reactor persists the queue before draining (line 604, comment: "Persist queue BEFORE draining so crash mid-drain doesn't lose items"). However, after each successful graph execution within the drain loop, it persists again (line 618). If the process crashes between `pop_front()` (line 610) and the successful persist at line 618, the popped item is lost -- it was removed from the in-memory queue but the updated queue was not yet persisted.

**Impact:** In a crash scenario during sequential processing, one boundary could be lost. The comment acknowledges this design but the pre-drain persist does not fully protect against it. The impact depends on exactly-once delivery requirements.

---

### COR-10: `TriggerRule::All` with empty conditions evaluates to `true` (Observation)

**File:** `crates/cloacina/src/task_scheduler/state_manager.rs` lines 163-186

**Evidence:** When `TriggerRule::All { conditions: vec![] }` is evaluated, the for-loop body never executes, so the method returns `Ok(true)`. Similarly, `TriggerRule::Any { conditions: vec![] }` returns `Ok(false)` and `TriggerRule::None { conditions: vec![] }` returns `Ok(true)`. These follow standard logical conventions (vacuous truth for All, false for Any, true for None), which is correct. However, this behavior is not documented or tested at the evaluation level (only serialization roundtrip tests exist for empty conditions).

**Impact:** None if intentional (standard logic), but a test should document the contract.

---

### COR-11: Context merge silently swallows dependency load errors (Observation)

**File:** `crates/cloacina/src/executor/thread_task_executor.rs` lines 230-274, `crates/cloacina/src/task_scheduler/context_manager.rs` lines 155-189

**Evidence:** Both `build_task_context` in the executor and `merge_dependency_contexts` in the context manager use `if let Ok(...)` patterns to silently skip dependency contexts that fail to load. A task with 3 dependencies where one dependency's context fails to load from the database will proceed with a partial context (only 2 of 3 dependencies' data). The `debug!` log mentions the failure but no error is propagated to the caller.

**Impact:** Tasks may execute with incomplete input data, leading to incorrect results that are difficult to diagnose since the partial context load is only visible at debug log level.
