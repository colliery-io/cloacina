# Correctness Review

## Summary

Cloacina exhibits strong correctness fundamentals: transactional state transitions, clear error type hierarchies, well-structured retry logic, and comprehensive unit tests for core data structures. The most significant correctness risks lie in the double state-update path between DefaultDispatcher and ThreadTaskExecutor (which can produce redundant or conflicting database writes), silent swallowing of dependency-loading failures in the executor, and several `unsafe impl Send/Sync` declarations on Python/FFI wrappers that rely on convention rather than compile-time enforcement.

## Test Coverage Assessment

### What Is Tested
- **Context operations**: insert, update, serialization roundtrip, DB conversion (unit tests in `context.rs`)
- **Merge logic**: 12 tests covering primitive, array, object, nested, null, and bool merging (`thread_task_executor.rs`)
- **Trigger rules**: serialization/deserialization roundtrip for all TriggerRule and TriggerCondition variants, including JSON literal parsing (22 tests in `trigger_rules.rs`)
- **Context condition evaluation**: all 8 ValueOperator variants tested against multiple data types (20+ tests in `context_manager.rs`)
- **Graph algorithms**: cycle detection, topological sort, parallel group detection, serialization roundtrip, root/leaf detection (7 tests in `graph.rs`)
- **Executor construction**: capacity, metrics, clone behavior, runtime isolation (10+ tests in `thread_task_executor.rs`)
- **Transient error classification**: timeout, network, unavailable, connection pool, permanent errors (7 tests)
- **Dispatcher**: routing resolution, executor registration, metrics, execution result types (15 tests in `default.rs`)
- **Work distribution**: poll intervals, shutdown signaling for SQLite distributor (2 async tests)
- **Cron evaluation**: timezone-aware scheduling, DST handling, range queries, validation (13 tests in `cron_evaluator.rs`)
- **Integration tests**: 26 test modules covering DAL, scheduler, executor, workflow, signing, packaging, computation graphs, error paths, and registry operations

### What Is NOT Tested
- **Dispatcher-Executor double state update path**: no test verifies that DefaultDispatcher's `handle_result()` and ThreadTaskExecutor's `complete_task_transaction()` don't conflict
- **Concurrent task claiming under contention**: no test exercises the `FOR UPDATE SKIP LOCKED` path with multiple simultaneous claimants
- **Stale claim sweeper integration**: only config tests exist; no test verifies the actual sweep-and-reset cycle with a running executor
- **Recovery manager end-to-end**: RecoveryManager has no unit tests; only called transitively from integration tests
- **Pipeline completion race conditions**: no test for the scheduler marking a pipeline complete while an executor is still writing task results
- **Heartbeat claim-lost behavior**: no test verifies that a running task properly stops when its heartbeat reports ClaimLost
- **Python task GIL safety**: the `unsafe impl Send/Sync` on PythonTaskWrapper is tested only via integration; no focused concurrency tests
- **Computation graph supervisor restart logic**: the crash recovery/restart loop in `ReactiveScheduler` has no unit or integration tests

### Test Quality
- Tests generally verify correct behavior, not just "runs without panic"
- Tests are deterministic (no randomized inputs, no timing dependencies except work_distributor tests)
- Integration tests use `#[serial]` for isolation, which is correct but limits parallelism
- The `#[cfg(feature = "sqlite")]` gating on executor unit tests means they only run with the sqlite feature enabled

## Key Risk Areas

1. **Double state update in dispatcher path** (HIGH) -- the executor and dispatcher both write task completion status
2. **Silent dependency loading failures** (HIGH) -- context build swallows errors via `if let Ok(...)`, potentially running tasks with incomplete data
3. **Unsafe Send/Sync implementations** (MEDIUM) -- several FFI/Python wrappers; correctness depends on runtime discipline
4. **Pipeline completion timing** (MEDIUM) -- scheduler can mark pipeline complete before all executor writes commit
5. **Stale claim sweeper startup race** (LOW) -- grace period mitigates but doesn't eliminate false positives

## Findings

## [COR-001]: Double State Update on Task Completion
**Severity**: Major
**Location**: `crates/cloacina/src/executor/thread_task_executor.rs:906-920`, `crates/cloacina/src/dispatcher/default.rs:92-103`
**Confidence**: High

### Description
When a task completes successfully via the dispatcher path, the state is updated **twice**:
1. `ThreadTaskExecutor::execute()` calls `self.complete_task_transaction()` (line 909), which calls `mark_completed()` on the task_execution record.
2. Control returns to `DefaultDispatcher::handle_result()` (line 96), which calls `self.dal.task_execution().mark_completed()` **again**.

The second `mark_completed` call loads the task record (which is already "Completed"), re-updates it to "Completed" with a new timestamp, and inserts a duplicate `TaskCompleted` execution event. This produces:
- Duplicate execution events in the audit trail
- Misleading `updated_at` timestamps (second write overwrites the first)
- An extra DB transaction per task completion

For failed tasks, the same pattern applies but with additional risk: if `ThreadTaskExecutor` schedules a retry but `DefaultDispatcher::handle_result()` then calls `mark_failed()`, it could overwrite the retry status.

### Evidence
In `thread_task_executor.rs`, the `TaskExecutor::execute()` impl:
```rust
Ok(result_context) => {
    match self.complete_task_transaction(&claimed_task, result_context).await {
        Ok(_) => {
            // ... returns ExecutionResult::success
        }
```

In `default.rs`, `handle_result()`:
```rust
ExecutionStatus::Completed => {
    self.dal.task_execution().mark_completed(event.task_execution_id).await?;
```

Both call `mark_completed` for the same `task_execution_id`.

### Suggested Resolution
Remove the state-transition calls from `DefaultDispatcher::handle_result()` for `Completed` and `Failed` statuses, since the executor already handles these. The dispatcher should only log the result. Alternatively, have the executor return a result that indicates whether the state was already persisted.

---

## [COR-002]: Silent Swallowing of Dependency Context Loading Failures
**Severity**: Major
**Location**: `crates/cloacina/src/executor/thread_task_executor.rs:248-293`
**Confidence**: High

### Description
When building context for task execution, the `build_task_context` method uses `if let Ok(...)` guards to silently swallow failures when loading dependency metadata and parsing dependency contexts. If the dependency context fails to load (database error, connection issue, corrupt data), the task runs with an **empty or partial context** rather than failing with a clear error.

Specifically:
- Line 249: `if let Ok(dep_metadata_with_contexts) = ...` -- a database failure results in running with no dependency data
- Line 265: `if let Ok(dep_context) = Context::<serde_json::Value>::from_json(json_str)` -- malformed JSON silently produces no context
- Lines 214-238: Root task pipeline context loading also uses `if let Ok(...)` guards

A task that depends on data from upstream tasks could silently execute with empty context, producing incorrect results or failing with a confusing error unrelated to the actual cause.

### Evidence
```rust
if let Ok(dep_metadata_with_contexts) = self
    .dal
    .task_execution_metadata()
    .get_dependency_metadata_with_contexts(...)
    .await
{
    // ... process contexts
} else {
    debug!("Failed to load dependency metadata for dependencies: {:?}", dependencies);
}
```

The `else` branch only logs at `debug` level, and the function continues to return `Ok(context)` with whatever partial data was loaded.

### Suggested Resolution
Return an error when dependency context loading fails for non-root tasks (tasks with dependencies). At minimum, elevate the logging to `warn` or `error` level. Consider making the behavior configurable: fail-fast vs. best-effort.

---

## [COR-003]: Unsafe Send/Sync on Python Wrappers Without Compile-Time Enforcement
**Severity**: Major
**Location**: `crates/cloacina/src/python/task.rs:129-130`, `crates/cloacina/src/python/trigger.rs:163-164`, `crates/cloacina/src/python/computation_graph.rs:491-492`, `crates/cloacina/src/python/bindings/trigger.rs:130-131`
**Confidence**: Medium

### Description
Four struct types holding `PyObject` fields are marked `unsafe impl Send` and `unsafe impl Sync`. The safety argument in comments is that "ALL access to PyObject fields goes through Python::with_gil()." This is a convention-based invariant, not a compile-time guarantee. Any future code change that accesses the `PyObject` without acquiring the GIL could introduce undefined behavior (data races on Python's reference counting).

`PythonTaskWrapper` holds three `PyObject` fields (`python_function`, `on_success_callback`, `on_failure_callback`). The execute method correctly acquires the GIL, but the `code_fingerprint()`, `trigger_rules()`, and `checkpoint()` methods don't access PyObject -- meaning any future refactor that adds PyObject access to these methods could silently break the safety invariant.

### Evidence
```rust
// SAFETY: PythonTaskWrapper holds PyObject fields which are not Send/Sync.
// This is safe because ALL access to PyObject fields goes through Python::with_gil()
unsafe impl Send for PythonTaskWrapper {}
unsafe impl Sync for PythonTaskWrapper {}
```

There are 4 separate structs with this pattern, each with their own safety argument. The `LoadedWorkflowPlugin` and `LoadedGraphPlugin` types (FFI wrappers) use a similar pattern with `unsafe impl Send/Sync`, relying on `Mutex` serialization.

### Suggested Resolution
Consider wrapping PyObject in a newtype that enforces GIL acquisition at the type level (e.g., `GilProtected<PyObject>`). Short of that, add `#[deny(unsafe_code)]` annotations to test modules that exercise these paths, and add targeted concurrent stress tests that run Python tasks from multiple threads.

---

## [COR-004]: Pipeline Completion Check Races With Executor Writes
**Severity**: Major
**Location**: `crates/cloacina/src/execution_planner/scheduler_loop.rs:249-259`
**Confidence**: Medium

### Description
In `process_pipelines_batch`, after processing task readiness updates, the scheduler immediately calls `check_pipeline_completion` for each active pipeline. This check queries the database for all task statuses and determines if the pipeline is done.

However, the executor runs concurrently. The scheduler could observe a task as "Completed" (because `mark_completed` committed) but not yet have the final context written (because `save_task_context` in `complete_task_transaction` runs before `mark_completed`). This is handled correctly.

The more concerning race is the reverse: the scheduler polls task statuses, sees all tasks as Completed/Failed/Skipped, and calls `complete_pipeline`. Meanwhile, the executor is still running `complete_task_transaction` for the last task -- and the final context update (`update_pipeline_final_context`) races with the executor's context save. Since `update_pipeline_final_context` iterates all tasks and picks the one with the latest `completed_at`, a task that completes *during* this scan might be missed.

### Evidence
```rust
// Check if pipeline is complete
if self.dal.task_execution().check_pipeline_completion(execution.id).await? {
    self.complete_pipeline(execution).await?;
}
```

`complete_pipeline` then calls `update_pipeline_final_context`, which reads all tasks:
```rust
for task in all_tasks {
    if task.status == "Completed" || task.status == "Skipped" {
        if let Some(completed_at) = task.completed_at {
            // ... find latest context
```

If the last task's context is written between the status check and the context scan, the pipeline's final context points to a stale task.

### Suggested Resolution
Make `check_pipeline_completion` and `update_pipeline_final_context` run within a single database transaction, or add a brief settling delay before finalizing the pipeline context. An alternative is to have the executor, not the scheduler, trigger pipeline completion when it completes the last task.

---

## [COR-005]: Heartbeat ClaimLost Does Not Cancel Running Task
**Severity**: Minor
**Location**: `crates/cloacina/src/executor/thread_task_executor.rs:767-793`
**Confidence**: High

### Description
When the heartbeat background task detects `HeartbeatResult::ClaimLost`, it logs a warning and breaks out of the heartbeat loop. However, the actual task execution continues running to completion. The running task is not cancelled -- it will finish, attempt to save its context, and call `complete_task_transaction`. By that point, another runner may have already claimed and started executing the same task.

This means:
1. Two runners can execute the same task concurrently
2. Both may try to save context, with the second overwriting the first
3. Both may call `mark_completed`, producing duplicate events

The `release_runner_claim` at line 967 will release a claim that may no longer belong to this runner.

### Evidence
```rust
Ok(crate::dal::unified::task_execution::HeartbeatResult::ClaimLost) => {
    tracing::warn!(task_id = %task_id, "Heartbeat failed - claim lost to another runner");
    break;  // Stops heartbeat but NOT the task execution
}
```

The `execute` method continues to `let result = match execution_result { ... }` regardless of whether the heartbeat loop has exited.

### Suggested Resolution
Provide a cancellation token (e.g., `tokio::sync::watch` or `CancellationToken`) that the heartbeat task can trigger when it detects ClaimLost. The executor should check this token before saving results, and ideally pass it into the task execution for cooperative cancellation.

---

## [COR-006]: Consecutive Error Counter Can Never Reset Under Sustained Errors
**Severity**: Minor
**Location**: `crates/cloacina/src/execution_planner/scheduler_loop.rs:132-170`
**Confidence**: High

### Description
The `consecutive_errors` counter in `SchedulerLoop` is a `u32` that increments on every error. While the backoff exponent is capped at `min(errors, 8)`, the counter itself continues incrementing. After 4,294,967,295 errors, it will overflow and wrap to 0.

More practically, the rate-limited logging at line 151 (`if self.consecutive_errors % 10 == 0`) means that after the circuit opens at error 5, warnings are only emitted every 10th error. At high error rates, this could mean hundreds of errors pass silently between log messages.

The reset logic at line 134 only fires after a successful iteration, so under sustained database outages, the counter grows unboundedly.

### Evidence
```rust
self.consecutive_errors += 1;  // u32, no overflow check
// ...
if self.consecutive_errors % 10 == 0 {
    warn!("...");
}
```

### Suggested Resolution
Either use `saturating_add` or cap the counter at a maximum value. Consider emitting a periodic warning regardless of counter value (e.g., every 30 seconds during sustained failure) rather than every 10th error, since error rates may vary.

---

## [COR-007]: Context Merge Strategy Differs Between Executor and ContextManager
**Severity**: Minor
**Location**: `crates/cloacina/src/executor/thread_task_executor.rs:317-352` vs `crates/cloacina/src/execution_planner/context_manager.rs:147-198`
**Confidence**: High

### Description
The executor and the context manager use different merge strategies when combining contexts from multiple dependencies:

- **ThreadTaskExecutor** (`merge_context_values`): Uses smart merging -- arrays are concatenated with deduplication, objects are merged recursively, primitives use latest-wins.
- **ContextManager** (`merge_dependency_contexts`): Uses simple overwrite -- `context.update(key, value)` replaces the entire value with the latest dependency's value, regardless of type.

This means the same workflow can produce different merged contexts depending on whether the context is loaded by the executor (for task execution) or by the scheduler (for trigger rule evaluation). A trigger rule that checks a context value after merging could see a different result than the actual task receives.

### Evidence
In `thread_task_executor.rs`:
```rust
let merged_value = Self::merge_context_values(existing_value, value);
let _ = context.update(key, merged_value);
```

In `context_manager.rs`:
```rust
merged_context.update(key.clone(), value.clone())
```

No smart merging (array concatenation, object recursion) in the ContextManager path.

### Suggested Resolution
Extract the merge strategy into a shared utility function and use it in both locations. The executor's smart merge logic is more correct, so the ContextManager should adopt it.

---

## [COR-008]: Stale Claim Sweeper Counts Releases Even When Some Fail
**Severity**: Observation
**Location**: `crates/cloacina/src/execution_planner/stale_claim_sweeper.rs:148-186`
**Confidence**: High

### Description
The sweep summary log at line 183 reports the total number of stale claims found, not the number successfully released. If some claims fail to release (lines 159-162) or fail to reset to Ready (lines 167-171), the log still reports all claims as released.

### Evidence
```rust
info!("Stale claim sweep complete: {} claims released", stale_claims.len());
```

This uses `stale_claims.len()` rather than counting successful releases.

### Suggested Resolution
Track successful releases in a counter and report that in the summary log.

---

## [COR-009]: PostgreSQL Distributor Cannot Be Created Via `create_work_distributor`
**Severity**: Observation
**Location**: `crates/cloacina/src/dispatcher/work_distributor.rs:333-341`
**Confidence**: High

### Description
The `create_work_distributor` factory function for PostgreSQL always returns an error:
```rust
Err("PostgreSQL distributor requires database URL. Use PostgresDistributor::new() directly.".into())
```

This means the factory function only works for SQLite, making the abstraction incomplete. Any code using `create_work_distributor` will fail at runtime when connected to PostgreSQL.

### Evidence
The comment in the code acknowledges the limitation: "Extract the database URL - this is tricky since we don't store it."

### Suggested Resolution
Either store the database URL in the `Database` struct and use it here, or remove the factory function and require callers to construct the distributor directly.

---

## [COR-010]: `TriggerRule::All` With Empty Conditions Vacuously Returns True
**Severity**: Observation
**Location**: `crates/cloacina/src/execution_planner/state_manager.rs:163-188`
**Confidence**: High

### Description
When a `TriggerRule::All` has an empty conditions list, the evaluation loop runs zero iterations and returns `Ok(true)`. Similarly, `TriggerRule::None` with empty conditions returns `Ok(true)`, and `TriggerRule::Any` with empty conditions returns `Ok(false)`.

These are mathematically correct (vacuous truth for All/None, identity for Any), but may be surprising to users who construct an `All` rule with no conditions expecting it to block execution. The test at line 286 (`trigger_rule_all_empty_conditions`) confirms the serialization roundtrip but does not test the evaluation behavior.

### Evidence
The `All` evaluation in `state_manager.rs`:
```rust
TriggerRule::All { conditions } => {
    for (i, condition) in conditions.iter().enumerate() {
        // ...
        if !condition_result { return Ok(false); }
    }
    Ok(true)  // Vacuously true if conditions is empty
}
```

### Suggested Resolution
Document the vacuous truth behavior explicitly. Consider adding a validation warning or error when trigger rules with empty conditions are created, if this represents a likely user error.

---

## [COR-011]: `clone()` on ThreadTaskExecutor Snapshots Atomic Counters
**Severity**: Observation
**Location**: `crates/cloacina/src/executor/thread_task_executor.rs:700-715`
**Confidence**: High

### Description
The `Clone` implementation for `ThreadTaskExecutor` creates new `AtomicU64` values initialized from the current values of the original's counters. This means the clone's `total_executed` and `total_failed` start from a snapshot of the original's counts, but then diverge. The `metrics()` method returns per-clone counts, not aggregate counts, which could be misleading if clones are used as workers sharing a semaphore.

### Evidence
```rust
total_executed: AtomicU64::new(self.total_executed.load(Ordering::SeqCst)),
total_failed: AtomicU64::new(self.total_failed.load(Ordering::SeqCst)),
```

The comment says "Shared semaphore -- clones coordinate on the same concurrency limit," but the metrics are not shared.

### Suggested Resolution
Either share the atomics via `Arc<AtomicU64>` (so aggregate metrics are consistent) or document that metrics are per-clone.
