# Performance Review

## Summary

The system demonstrates thoughtful performance design in its critical paths: batch database queries replace N+1 patterns in the scheduler loop, the computation graph pipeline uses channel-based backpressure with configurable capacity, and the PostgreSQL claiming path uses `FOR UPDATE SKIP LOCKED` for contention-free concurrent claiming. However, there are several areas where resource usage could be tightened -- most notably individual-query trigger condition evaluation, reactor state persistence on every execution, and a completion-time polling loop in `wait_for_completion_with_timeout` that could be replaced with a notification-based approach.

## Workload Assessment

Based on code and docs, the system serves two deployment profiles:

- **Embedded/daemon mode (SQLite)**: Small-to-medium workflow loads (tens to low hundreds of concurrent pipelines). Single-process. The SQLite pool is capped at 1 connection, so throughput is inherently serialized at the DB layer.
- **Server mode (PostgreSQL)**: Multi-tenant, potentially high throughput. Many concurrent pipelines, multiple workers, continuous computation graph streams. This is where performance matters most.

The computation graph subsystem adds a continuous-processing dimension: accumulators consume event streams and reactors fire graph functions. Throughput here depends on boundary arrival rate and graph execution time.

Expected hot paths (frequency order):
1. Accumulator event processing (continuous, per-event)
2. Reactor boundary evaluation and graph execution (continuous, per-boundary)
3. Task scheduler loop: poll active pipelines, evaluate readiness, dispatch (periodic, ~500ms)
4. Task claiming and execution (per-task)
5. Trigger polling (periodic, per-trigger)
6. Reconciler (periodic, ~30s)

## Hot Path Analysis

### Computation Graph Pipeline

The accumulator-to-reactor pipeline is well-structured with `mpsc` channels providing natural backpressure. The merge channel capacity defaults to 1024 events, which is reasonable. The reactor uses a `tokio::select!` loop that processes signals sequentially, avoiding concurrent mutation of the cache.

**Good**: Snapshot isolation for the InputCache means the graph function receives an immutable clone while new boundaries continue to arrive. The receiver and executor are decoupled via a strategy signal channel.

**Concern**: The reactor persists state to the DAL after every successful graph execution (see PERF-01). For high-throughput graphs this adds write latency to the hot path.

### Scheduler Loop

The scheduler loop shows evidence of intentional batching optimization:
- `get_pending_tasks_batch` loads pending tasks across all active pipelines in a single query
- `get_task_statuses_batch` checks dependency statuses in a single query
- Tasks are grouped by pipeline ID in memory (`HashMap<UniversalUuid, Vec<TaskExecution>>`)

**Good**: Batch loading avoids the classic N+1 anti-pattern for the main dependency-checking path.

**Concern**: Individual trigger condition evaluation still issues per-condition database queries (see PERF-02).

### Task Claiming (PostgreSQL)

Uses `FOR UPDATE SKIP LOCKED` in a CTE that atomically deletes from the outbox and updates task status. This is the gold-standard pattern for concurrent work claiming in PostgreSQL -- no contention, no lost updates, no busy-waiting.

### Task Claiming (SQLite)

Uses `IMMEDIATE` transactions for write-lock-at-start semantics. Given the pool size of 1, this is effectively single-threaded, which is correct for SQLite.

## Findings

### PERF-01: Reactor persists state on every graph execution (Minor)

**Location**: `crates/cloacina/src/computation_graph/reactor.rs`, lines 586-591 (Latest mode), lines 616-621 (Sequential mode)

**Description**: After every successful graph execution, `persist_reactor_state` serializes the full cache and dirty flags to JSON and writes them to the database. For high-frequency graphs (e.g., sub-second boundaries from a Kafka stream), this means a database write on every execution.

**Impact**: Adds write latency to the reactor hot path. On SQLite this serializes with all other DB operations through the single connection. On PostgreSQL the impact is a consumed connection from the pool per persist.

**Observation**: The persistence is best-effort (errors are logged, not propagated), which suggests it is intended for crash recovery rather than correctness. A write-behind or periodic persistence strategy (e.g., persist every N executions or every T seconds) would reduce I/O without meaningfully impacting recovery guarantees.

**Severity**: Minor -- the persistence is async and best-effort, so it does not block graph execution. It will only matter at high throughput.

---

### PERF-02: Trigger condition evaluation issues per-condition database queries (Minor)

**Location**: `crates/cloacina/src/task_scheduler/state_manager.rs`, lines 250-320

**Description**: When evaluating trigger rules, each `TriggerCondition` variant (`TaskSuccess`, `TaskFailed`, `TaskSkipped`) issues an individual `get_task_status()` call to the database. If a trigger rule has `All { conditions: [TaskSuccess("a"), TaskSuccess("b"), TaskSuccess("c")] }`, this generates 3 separate database queries.

**Contrast**: The dependency checking path correctly uses `get_task_statuses_batch` to load all statuses in a single query. The trigger condition path does not use this batch approach.

**Impact**: With small condition lists (typical: 1-3 conditions), this is negligible. It would matter if trigger rules routinely had many conditions, which does not appear to be a common pattern.

**Severity**: Minor -- the inconsistency with the batched dependency checking is worth aligning, but the typical condition count keeps the real impact low.

---

### PERF-03: Pipeline completion context lookup is per-task (Minor)

**Location**: `crates/cloacina/src/task_scheduler/scheduler_loop.rs`, lines 264-319 (`update_pipeline_final_context`)

**Description**: When a pipeline completes, the code iterates all tasks and for each completed/skipped task, issues a `get_by_pipeline_and_task` query to `task_execution_metadata` to find the one with the latest context. This is O(n) database queries where n is the number of completed/skipped tasks in the pipeline.

**Impact**: This code only runs once per pipeline completion, not in the hot loop. Typical workflows have a small number of tasks (5-20). The impact is proportional to pipeline size and only incurred at completion.

**Severity**: Minor -- pipeline completion is a cold path. A batch query loading all metadata for the pipeline would be cleaner but the practical impact is small.

---

### PERF-04: wait_for_completion polls with 500ms sleep (Observation)

**Location**: `crates/cloacina/src/executor/pipeline_executor.rs`, lines 239-269

**Description**: `wait_for_completion_with_timeout` polls execution status in a loop with a hardcoded 500ms sleep between checks. Each iteration issues a database query to check if the pipeline has reached a terminal state.

**Impact**: This introduces up to 500ms of unnecessary latency between actual completion and the caller receiving the result. For short-running pipelines (< 1s), this is significant relative to execution time.

**Alternative**: A `watch` channel or `tokio::sync::Notify` from the scheduler loop when a pipeline transitions to a terminal state would provide immediate notification without polling.

**Severity**: Observation -- this is the synchronous API convenience wrapper. The async API (`execute_async`) does not have this problem. Most production usage likely uses the async path.

---

### PERF-05: SQLite pool size hardcoded to 1 (Observation)

**Location**: `crates/cloacina/src/database/connection/mod.rs`, line 239

**Description**: The SQLite connection pool size is hardcoded to 1 regardless of the `max_size` parameter passed by the caller. The `max_size` parameter is silently ignored for SQLite.

**Rationale**: SQLite in WAL mode supports concurrent reads but only one writer. A pool size of 1 serializes all access, which is the safest approach for avoiding `SQLITE_BUSY` errors.

**Trade-off**: With WAL mode enabled (which the code correctly configures), multiple reader connections could improve read-heavy workloads. The scheduler loop reads (active pipelines, pending tasks, completion checks) while the executor writes (status updates, event inserts). Multiple read connections could reduce contention.

**Severity**: Observation -- the current approach is safe and simple. For daemon mode with modest workloads, the serialization overhead is unlikely to be the bottleneck since task execution time dominates.

---

### PERF-06: Reactor RwLock contention between receiver and executor (Observation)

**Location**: `crates/cloacina/src/computation_graph/reactor.rs`, lines 515-650

**Description**: The reactor's receiver task writes to the cache (`cache_recv.write().await`) and dirty flags (`dirty_recv.write().await`) on every boundary arrival. The executor task reads from both (`cache_exec.read().await`, `dirty_exec.read().await`) and then writes to dirty flags (`dirty_exec.write().await`) to clear them. Both use `tokio::sync::RwLock`.

**Analysis**: Under high boundary arrival rates, the write lock from the receiver could momentarily block the executor's read lock (and vice versa). However, `tokio::sync::RwLock` is fair and the critical sections are short (HashMap insert/lookup). The strategy signal channel ensures the executor only wakes when there is something to do.

**Severity**: Observation -- the current design is correct and the contention window is small. If profiling reveals this as a bottleneck, a lock-free approach (e.g., `arc-swap` for the cache snapshot) could eliminate the contention entirely, but this would be premature optimization without evidence.

---

### PERF-07: Sequential reactor mode drains queue under a single write lock per item (Observation)

**Location**: `crates/cloacina/src/computation_graph/reactor.rs`, lines 609-635

**Description**: In `InputStrategy::Sequential` mode, the reactor drains the sequential queue by repeatedly acquiring a write lock (`seq_queue_exec.write().await.pop_front()`) to pop one item, then acquiring another write lock to update the cache, then executing the graph, then persisting state. Each queue item involves multiple lock acquisitions and a full persist cycle.

**Impact**: For burst arrivals where many items queue up, this is O(n) lock acquisitions and O(n) persist operations. The persist-after-every-execution pattern (PERF-01) compounds here.

**Severity**: Observation -- Sequential mode is an explicit choice for ordered processing, so per-item execution is by design. The repeated lock acquisitions could be batched (pop multiple items under one lock), but this would change the semantics.

---

### PERF-08: Auth key cache uses `tokio::sync::Mutex` (Observation)

**Location**: `crates/cloacinactl/src/server/auth.rs`, lines 57-59

**Description**: The LRU key cache wraps `LruCache` in a `tokio::sync::Mutex`. Every auth check (on every HTTP request) must acquire this mutex to check the cache, even for cache hits.

**Analysis**: With the 256-entry / 30s TTL configuration, this is fine for moderate request rates. The LRU lookup is O(1) and the critical section is short. At very high request rates (thousands of concurrent requests), the mutex could become contended. A sharded approach (e.g., `DashMap` with TTL, or multiple independent LRU shards) would reduce contention.

**Severity**: Observation -- the current approach is appropriate for the expected workload of a workflow orchestration API, which is not a high-QPS service like a CDN edge.

---

### PERF-09: Batch execution event inserts are per-task in a loop (Observation)

**Location**: `crates/cloacina/src/dal/unified/task_execution/claiming.rs`, lines 281-295 (PostgreSQL), lines 375-393 (SQLite)

**Description**: When claiming tasks, execution events are inserted one-at-a-time in a for loop inside a transaction. With PostgreSQL, a batch `INSERT INTO ... VALUES (...), (...), (...)` would be a single round-trip instead of N.

**Impact**: The claim limit is typically small (e.g., 10), so this is N small inserts within an already-open transaction. The overhead is minimal compared to the CTE that does the actual claiming work.

**Severity**: Observation -- the current approach is within a transaction so there is no extra round-trip overhead. A batch insert would be marginally faster but the difference is negligible for typical claim sizes.

---

### PERF-10: Computation graph snapshot clones entire cache (Observation)

**Location**: `crates/cloacina-computation-graph/src/lib.rs`, lines 149-153 (`InputCache::snapshot`)

**Description**: `snapshot()` clones the entire `HashMap<SourceName, Vec<u8>>`. For graphs with many sources or large boundary payloads, this involves allocating and copying all the byte vectors.

**Analysis**: The clone is necessary for snapshot isolation -- the graph function needs a stable view while the receiver continues updating the cache. The `Arc<RwLock<InputCache>>` design means the read lock is held only long enough to clone, not for the duration of graph execution.

**Alternative**: An `Arc`-based cache where entries are `Arc<Vec<u8>>` would make the clone nearly free (just Arc reference count bumps). This would matter for large payloads.

**Severity**: Observation -- for typical boundary sizes (kilobytes), the clone cost is negligible. This would only matter for unusually large boundaries (megabytes per source).

---

### PERF-11: Dependency checking reconstructs workflow from global registry on every check (Observation)

**Location**: `crates/cloacina/src/task_scheduler/state_manager.rs`, lines 91-108

**Description**: In `check_task_dependencies`, the state manager looks up the pipeline name, then calls the workflow constructor from the global registry to get a fresh `Workflow` instance, then calls `get_dependencies()` on it. This happens for every pending task on every scheduler tick.

**Analysis**: The global registry uses `parking_lot::RwLock` (non-async, fast) and the constructor presumably just builds a struct. The `get_dependencies` call walks the petgraph DAG, which is O(degree) per task. Since the workflow definition is static for the lifetime of a pipeline, caching the workflow instance per pipeline would avoid repeated construction.

**Severity**: Observation -- the construction cost is likely trivial (struct allocation + petgraph setup), and caching adds complexity. Only notable if profiling shows this path is hot.

## Positive Patterns Worth Noting

- **Batch pending task loading**: `get_pending_tasks_batch` correctly avoids N+1 at the highest-frequency point in the scheduler
- **FOR UPDATE SKIP LOCKED**: The PostgreSQL claiming path is textbook-correct for concurrent work queues
- **Semaphore-based concurrency limiting**: `ThreadTaskExecutor` uses a `Semaphore` to bound concurrent task execution, providing natural backpressure
- **Channel-based backpressure**: The computation graph pipeline uses bounded `mpsc` channels (configurable capacity) throughout, preventing unbounded memory growth
- **Explicit cleanup command**: `cloacinactl admin cleanup-events` provides manual purging of the append-only execution events table, preventing unbounded growth
- **LISTEN/NOTIFY with poll fallback**: The PostgreSQL work distributor correctly uses notification for latency-sensitive wakeup with a 30s poll fallback for reliability
- **MissedTickBehavior::Skip**: Both the sweeper and scheduler correctly configure tick behavior to skip missed ticks rather than burst
- **Profile-aware serialization**: Debug=JSON / Release=bincode for computation graph boundaries is a pragmatic choice that favors debuggability in development without sacrificing release performance
