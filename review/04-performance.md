# Performance Review

## Summary

Cloacina is structured for correctness and operability first; performance is a secondary concern reflected in real but limited spots (batch DAL queries on the scheduler hot path, content-hash artifact reuse in packaging, push-based dispatch over polling for the post-T-0509 surface). Within that frame the system has several **production-relevant** performance liabilities: the scheduler loop polls Postgres every 100 ms with at least four un-paginated `SELECT` queries against `workflow_executions` and `task_executions`; the per-tenant Postgres pool is hardcoded to **2 connections** (`crates/cloacina-server/src/lib.rs:80`), which is an active throughput cliff for any tenant under non-trivial concurrent load; SQLite is hardcoded to **pool size 1** (`crates/cloacina/src/database/connection/mod.rs:239`), meaning every `interact()` serializes globally; and several DAL/scheduler paths exhibit clear **N+1 patterns** (`update_execution_final_context`, `merge_dependency_contexts`, `StaleClaimSweeper::sweep`, server's `list_executions`).

Beyond DAL, the **reactor's in-memory hot path** uses `tokio::sync::RwLock` around a `HashMap`-shaped `InputCache` and `DirtyFlags`, plus another `RwLock<VecDeque>` for the sequential queue, plus a `RwLock` per accumulator in `EndpointRegistry`. Every WS accumulator message takes a *write* lock on the registry to send (because pruning of closed channels is interleaved with sending), serializing all accumulator pushes globally. The reactor persists state via `serde_json::to_vec` of cache + dirty flags after **every firing** — JSON is the wire format for an internal blob that has no consumer outside the reactor itself; bincode would be faster. Each batch accumulator persists the full buffer to the DB on **every event** (not periodically), which is an O(n²) write pattern for buffer-sized n.

The **public `runner.execute()` entry point polls the database every 500 ms** waiting for terminal status (`crates/cloacina/src/runner/default_runner/workflow_executor_impl.rs:97`) — this is the worst possible synchronization primitive for an in-process workflow framework and adds floor latency equal to half the poll interval to every synchronous workflow. **`CronEvaluator::new` re-parses the cron expression and timezone string on every cron schedule that fires** (`crates/cloacina/src/cron_trigger_scheduler.rs:399, 437`); no cache. **`runtime.get_task` and `get_workflow` invoke the constructor closure on every call** with no cache, so every dispatched task pays for a fresh allocation of the `Arc<dyn Task>`. The Python task wrapper acquires the GIL **at least 5 times per task execution** (clone_ref of function/on_success/on_failure, then again inside spawn_blocking, then for each post-CG hook) — every acquisition is a serialization point against any other Python task in the process.

Most of these are addressable without architectural change. The biggest "needs measurement" items are the scheduler poll interval (currently a fixed 100 ms; an event-driven path against `task_outbox` would eliminate all four poll queries on the idle path), and the connection pool sizes (right values depend on workload).

## Workload Assessment

The system targets **medium-throughput, persistence-first workflow orchestration**: tasks measured in seconds-to-minutes, workflow DAGs with up to maybe a few hundred tasks, single-digit-thousand workflow executions per hour per runner. The default `max_concurrent_tasks = 4` and `db_pool_size = 10` (`crates/cloacina/src/runner/default_runner/config.rs:271-275`) reinforces this — it is not configured for heavy parallelism out of the box.

The **fast-throughput** path is the **computation graph reactor**: in-process execution after fire, in-memory channels, Kafka-backed accumulators. This is where `examples/performance/computation-graph` benchmarks land, and it's the only place where allocation/lock-free design would meaningfully matter. The CG runtime is also the area where the design has the highest concentration of `serde_json` per fire (every persist) and tokio-RwLock contention.

The **server** is designed for **multi-tenant API exposure**, not raw RPS — body limit is 100 MB, per-tenant pools are 2 connections, the auth cache is 256 entries. It's a plausible internal service (10s of tenants, dozens of QPS), not a public-internet API gateway.

The **packaging/compiler service** is **batch-throughput**, claim-based, latency-tolerant — polling `workflow_packages` at 2 s default. Individual `cargo build` invocations dominate any per-row overhead. Performance there is governed by `CARGO_TARGET_DIR` reuse and content-hash dedup, both of which are present.

`SchedulerConfig` defaults (cron 30s poll, trigger 1s tick) are conservative — fine.

## Hot Path Analysis

### HP-1: Scheduler tick (`SchedulerLoop::process_active_executions`)

`crates/cloacina/src/execution_planner/scheduler_loop.rs:159-187`. Runs every `scheduler_poll_interval` (default 100 ms, `crates/cloacina/src/runner/default_runner/config.rs:272`).

Per tick, on the active path:
1. `dal.workflow_execution().get_active_executions()` — `SELECT * FROM workflow_executions WHERE status IN ('Pending','Running')` with **no LIMIT**, **no pagination** (`crates/cloacina/src/dal/unified/workflow_execution.rs:259-261`).
2. `dal.task_execution().get_pending_tasks_batch(execution_ids)` — `SELECT * FROM task_executions WHERE workflow_execution_id IN (...) AND status IN ('NotStarted','Pending')`. Good — this is batched.
3. For each active workflow: `state_manager.update_workflow_task_readiness(...)` → `check_task_dependencies` does a `runtime.get_workflow(name)` (allocates a workflow), then `dal.task_execution().get_task_statuses_batch(...)` (one batch query per pending task), then `mark_ready` or `mark_skipped` (transactional, one DB roundtrip each, both write to `execution_events` too).
4. For each active workflow: `dal.task_execution().check_workflow_completion(execution.id)` — separate `SELECT count(*)` query per workflow.
5. If complete, `dal.workflow_execution().get_by_id(...)` → `dal.task_execution().get_all_tasks_for_workflow(...)` → then for **every** terminal task, `dal.task_execution_metadata().get_by_workflow_and_task(...)` (an N+1 in `update_execution_final_context`, `crates/cloacina/src/execution_planner/scheduler_loop.rs:394-421`).
6. `dispatch_ready_tasks` — `dal.task_execution().get_ready_for_retry()` — un-paginated `SELECT * FROM task_executions WHERE status='Ready' AND retry_at IS NULL OR retry_at <= NOW()`. Then a separate `dispatcher.dispatch(event).await` per row.

On the idle path (no active executions) it's still query #6 every tick.

That's at minimum **2 queries per tick** when idle and **4 + (3·active) + (1·complete) + (N·complete_tasks)** when busy. At 100 ms poll, that's 20 idle queries per second per runner, times multi-tenancy. Postgres can handle this, but it's pure overhead — `task_outbox` exists for exactly the push-based path that would obsolete `get_ready_for_retry`, but the polling fallback runs alongside.

### HP-2: Task dispatch (`ThreadTaskExecutor::execute`)

`crates/cloacina/src/executor/thread_task_executor.rs:677-1002`. Runs once per dispatched task.

Critical-section walk:
- `dal.task_execution().claim_for_runner(...)` → 1 query (atomic UPDATE).
- `tokio::spawn` for heartbeat — emits `dal.task_execution().heartbeat(...)` every `heartbeat_interval` (10 s default), **separate connection per heartbeat per task**.
- `semaphore.acquire_owned()` — fine.
- `runtime.get_task(&namespace)` — invokes the constructor closure, allocating a fresh `Arc<dyn Task>` (`crates/cloacina/src/runtime.rs:190`). Hot-path allocation; see PERF-08.
- `build_task_context(...)` — for tasks **without** dependencies, 2 queries (`get_by_id` + `context().read`). For tasks **with** dependencies, 1 batched query via `get_dependency_metadata_with_contexts`. Good.
- Several `Python::with_gil` calls (Python tasks only) — see PERF-15.
- Task body — user code.
- `complete_task_transaction(...)` — `mark_completed` (1 transaction with execution_event insert) + `save_task_context` (creates new context row + upserts task_execution_metadata = 2 queries).
- `release_runner_claim` — 1 update.

Floor: roughly **4-7 DB round trips per task** (excluding heartbeats during execution). Each transaction inserts one row in `execution_events` (line 99-110 of state.rs). For a workflow of N tasks this is ~5N DB transactions; at 100ms scheduler tick + 4 concurrent tasks default the system is DB-bound long before it's CPU-bound.

The `keys: Vec<_> = context.data().keys().collect()` allocation at `thread_task_executor.rs:462` happens unconditionally on every task completion, regardless of `info` log filter (the macro is lazy but the `let` is not).

### HP-3: Reactor firing loop

`crates/cloacina/src/computation_graph/reactor.rs:351-666`. One reactor per CG. Fires on every accumulator boundary in `WhenAny` mode, on every "all sources dirty" event in `WhenAll` mode.

Per fire:
- `cache.read().await.snapshot()` — clones the `InputCache` (`HashMap<SourceName, Vec<u8>>`) inside an async RwLock read.
- `dirty.write().await.clear_all()` — write lock.
- `(graph)(snapshot).await` — runs the user graph.
- On success: `persist_reactor_state(...)` — `cache.read().await`, `dirty.read().await`, optionally `seq_queue.read().await`, then **`serde_json::to_vec(&entries_raw())`** of the cache, `serde_json::to_vec(&dirty.flags)`, `serde_json::to_vec(&queue)`. **JSON ser/de of an internal-only blob** — bincode would be faster and smaller (`reactor.rs:686, 694, 707`).
- `dal.checkpoint().save_reactor_state(...)` — 1 DB upsert.
- `for sender in batch_flush: sender.try_send(())` — fan-out to batch accumulators.

So every WhenAny fire pays a JSON serialization of the entire input cache, plus a Postgres write. For accumulators producing thousands of events/sec into a WhenAny reactor, this is the dominant cost.

### HP-4: Accumulator boundary forwarding (server WS)

`crates/cloacina-server/src/routes/ws.rs:213-249` calls `EndpointRegistry::send_to_accumulator` for each binary message.

`send_to_accumulator` (`crates/cloacina/src/computation_graph/registry.rs:349-393`) acquires a **write** lock on the registry's `tokio::sync::RwLock<RegistryInner>` for **every** message — even though the only writes done are `senders.remove(i)` for closed channels, which is rare. Sending uses `try_send` and clones `bytes` per recipient. A multi-producer accumulator workload (Kafka feeding many WS sources) thus serializes globally on this lock.

Inside the accumulator runtime there are then **3 hops** between mpsc channels per event: socket_rx → event_tx → process → output (`crates/cloacina/src/computation_graph/accumulator.rs:373-441`). Each hop is a tokio mpsc send, each a heap allocation.

### HP-5: Cron + trigger scheduler tick

`crates/cloacina/src/cron_trigger_scheduler.rs:184-218`. Ticks at `trigger_base_poll_interval` (default 1 s).

Per tick:
- Every 30s (default cron interval): `dal.schedule().get_due_cron_schedules(now)` — un-paginated SELECT.
- Every tick: `dal.schedule().get_enabled_triggers()` — un-paginated SELECT of *all* enabled triggers, regardless of poll interval (the per-trigger filter happens *after* the query, in Rust).
- For each due cron: `CronEvaluator::new(cron_expr, tz)` is called **twice** — once in `calculate_execution_times` and once in `calculate_next_run`. The croner crate parses the expression each time (`crates/cloacina-workflow/src/cron_evaluator.rs:131-134`). PERF-09.

The full cron pipeline per fire is ~6 DB queries (claim, audit-create, executor.execute → context().create → workflow_executions+task_executions transaction → schedule_executions update). Acceptable for cron cadence; would be a bottleneck for sub-second triggers.

### HP-6: Auth middleware (per HTTP request)

`crates/cloacina-server/src/routes/auth.rs:175-194`. Gates every `/v1/*` request.

- `extract_bearer_token` — header parse. Cheap.
- `validate_token` → `hash_api_key(token)` — SHA-256 of the bearer string.
- `key_cache.get(&hash)` — **`tokio::sync::Mutex` lock acquisition** even for a cache hit (`crates/cloacina-server/src/routes/auth.rs:81-91`). LRU's `get` requires a write lock anyway (it updates recency), but the *outer* primitive is `Mutex` rather than `RwLock` — even a hit serializes all auth checks across the process. PERF-12.
- On miss: `dal.api_keys().validate_hash(&hash).await` — DB query + cache insert (still under the same Mutex).

Cache TTL is 30 s. Capacity 256. A workload with >256 distinct keys hitting the server inside 30 s effectively defeats the cache.

## Findings

### PERF-01: Per-tenant Postgres pool hardcoded to 2 connections

**Severity**: Major
**Location**: `crates/cloacina-server/src/lib.rs:77-82`
**Confidence**: High

#### Description

`TenantDatabaseCache::resolve` creates a per-tenant `Database` with `max_size = 2`. Every API request scoped to a tenant (workflow upload, list, execute, list executions, get execution events, list/get triggers) goes through this pool. With **2 connections per tenant**, four concurrent requests for any given tenant queue on connection acquisition. `deadpool` waits with no configured timeout, so under contention the requests hang until a connection frees.

The runner's main pool (used for cross-tenant operations and the scheduler loop) is also constructed inside `DefaultRunner` and uses `db_pool_size` (default 10) — that's the public-schema pool, not the tenant pool.

#### Evidence

```rust
// crates/cloacina-server/src/lib.rs:77-82
let db = Database::try_new_with_schema(
    &self.database_url,
    "cloacina",
    2, // small pool per tenant
    Some(tenant_id),
)?;
```

The comment "small pool per tenant" reads as a deliberate decision; given the server runs **per-tenant scheduler loops** (the runner is shared but uses its own pool, and tenant-scoped read paths use the tenant pool) — 2 is not enough for any non-trivial RPS.

#### Suggested Resolution

Make this configurable via the server CLI / config (e.g., `--tenant-pool-size`, default 8 or 10). Production workloads will need to tune this to their tenant count × per-tenant RPS profile. Worth measuring with a soak test that varies tenant count.

---

### PERF-02: SQLite pool fixed at size 1, defeats default `db_pool_size = 10`

**Severity**: Major
**Location**: `crates/cloacina/src/database/connection/mod.rs:239-245, 293`
**Confidence**: High

#### Description

`Database::try_new_with_schema` overrides the caller's `max_size` to a hardcoded `let sqlite_pool_size = 1;` for SQLite. The `db_pool_size` field on `DefaultRunnerConfig` is silently ignored on SQLite. Combined with the `MissedTickBehavior::Skip` 100ms scheduler poll, every async DAL call is a global mutex on the one connection. For SQLite this is consistent with the database's writer-singleton model — but **WAL mode allows concurrent readers**, which is why the comment in `run_migrations` references it. With pool size 1, those concurrent readers can't happen.

This is also why the daemon mode ships with task_outbox-based polling: a single SQLite connection means every executor and the scheduler serialize on the pool.

#### Evidence

```rust
// crates/cloacina/src/database/connection/mod.rs:236-245
BackendType::Sqlite => {
    let connection_url = Self::build_sqlite_url(connection_string);
    let manager = SqliteManager::new(connection_url, SqliteRuntime::Tokio1);
    let sqlite_pool_size = 1;
    let pool = SqlitePool::builder(manager)
        .max_size(sqlite_pool_size)
        .build()
```

```rust
// crates/cloacina/src/database/connection/mod.rs:597-608
let conn = pool.get().await?;
// Ensure SQLite pragmas are set on every checkout — pragmas are per-connection
// and may be lost if the pool recycles the connection.
conn.interact(|conn| {
    use diesel::prelude::*;
    let _ = diesel::sql_query("PRAGMA journal_mode=WAL;").execute(conn);
    let _ = diesel::sql_query("PRAGMA busy_timeout=30000;").execute(conn);
})
```

`get_sqlite_connection` runs **PRAGMA journal_mode=WAL; PRAGMA busy_timeout=30000;** on every checkout. WAL is a database-wide setting (file metadata), it does not need to be set on every connection — once set it sticks. busy_timeout is per-connection but doesn't need to be re-set if the pool keeps the connection alive. With pool size 1, this runs on every single DAL call — two extra round-trips per query.

#### Suggested Resolution

- Allow SQLite pool size > 1 for read-heavy workloads (WAL mode supports it). The writer-singleton issue can be handled by deadpool's serialization.
- Move PRAGMA setup into a connection initializer (deadpool `Manager::create` hook) instead of every `get_sqlite_connection` checkout. Confirm with a benchmark.

---

### PERF-03: `runner.execute()` polls DB every 500 ms instead of using a notification

**Severity**: Major
**Location**: `crates/cloacina/src/runner/default_runner/workflow_executor_impl.rs:73-100`
**Confidence**: High

#### Description

The synchronous `WorkflowExecutor::execute` waits for completion via:

```rust
loop {
    // ... timeout check ...
    let execution = dal.workflow_execution().get_by_id(...).await?;
    match execution.status.as_str() {
        "Completed" | "Failed" => return self.build_workflow_result(execution_id).await,
        _ => tokio::time::sleep(Duration::from_millis(500)).await,
    }
}
```

500 ms median latency floor on every synchronous workflow regardless of how fast the workflow ran, plus 1 DB query per poll (which is `SELECT * FROM workflow_executions WHERE id=?`). Same pattern in `execute_with_callback` (`workflow_executor_impl.rs:172-174`).

The same pattern in the in-tree performance benchmark (`examples/performance/simple/src/main.rs`) means measured throughput is rate-limited by this poll loop, not by actual workflow processing.

#### Evidence

`workflow_executor_impl.rs:97`: `tokio::time::sleep(Duration::from_millis(500)).await;`

#### Suggested Resolution

The scheduler already writes every state transition into `execution_events` and updates `workflow_executions.status`. A `tokio::sync::Notify` (or `watch` channel) keyed by execution_id, signaled by the scheduler loop's `complete_execution` path, would let `execute()` await a notification with a fallback poll for resilience. For Postgres, a `LISTEN/NOTIFY` channel keyed on workflow execution status changes is even more elegant. **Need a benchmark first** to confirm this is the user-visible bottleneck for the synchronous path.

---

### PERF-04: N+1 DAL queries in workflow finalization

**Severity**: Major
**Location**: `crates/cloacina/src/execution_planner/scheduler_loop.rs:384-438`; `crates/cloacina/src/execution_planner/context_manager.rs:147-198`; `crates/cloacina/src/execution_planner/stale_claim_sweeper.rs:149-181`
**Confidence**: High

#### Description

Three separate scheduler-hot N+1 patterns:

1. **`update_execution_final_context`** (`scheduler_loop.rs:394-421`): iterates over `all_tasks` (every terminal task in the workflow) and for each, calls `dal.task_execution_metadata().get_by_workflow_and_task(...)`. There's already a batch path for the executor's dependency loading — same shape would work here.

2. **`merge_dependency_contexts`** (`context_manager.rs:155-187`): for each dependency, separate `task_execution_metadata().get_by_workflow_and_task(...)` then `context().read(context_id)`. So a task with N dependencies costs **2N round-trips** to load dependency contexts. The executor's `build_task_context` uses a batched `get_dependency_metadata_with_contexts` for the same data; the scheduler's trigger-rule context evaluation does not.

3. **`StaleClaimSweeper::sweep`** (`stale_claim_sweeper.rs:149-181`): for each stale claim, call `release_runner_claim(claim.task_id)` followed by `mark_ready(claim.task_id)` — both separate transactions. With N stale claims that's **2N transactions** + 2N execution_events inserts. Should be one batched UPDATE plus a single batched insert.

#### Evidence

```rust
// scheduler_loop.rs:394 — N+1 per-task DAL hit
for task in all_tasks {
    if task.status == "Completed" || task.status == "Skipped" {
        if let Some(completed_at) = task.completed_at {
            // ...
            if let Ok(task_metadata) = self
                .dal
                .task_execution_metadata()
                .get_by_workflow_and_task(workflow_execution_id, &task_namespace)
                .await
            { ... }
        }
    }
}
```

```rust
// stale_claim_sweeper.rs:149-181 — 2N transactions per sweep
for claim in &stale_claims {
    self.dal.task_execution().release_runner_claim(claim.task_id).await ...
    self.dal.task_execution().mark_ready(claim.task_id).await ...
}
```

#### Suggested Resolution

Add batched DAL methods: `get_metadata_for_tasks_batch(...)` (already exists via `get_dependency_metadata_with_contexts` — reuse with a status filter), `release_runner_claims_batch(task_ids)`, `mark_ready_batch(task_ids)`. The data is already loaded; just don't call DAL in a loop.

---

### PERF-05: Reactor in-memory cache uses async RwLock for tight read/write hot path

**Severity**: Major
**Location**: `crates/cloacina/src/computation_graph/reactor.rs:200, 247, 283-287, 513`
**Confidence**: Medium (would benefit from measurement)

#### Description

The reactor's `InputCache` (a `HashMap<SourceName, Vec<u8>>`) and `DirtyFlags` are wrapped in `Arc<tokio::sync::RwLock<…>>`. On every boundary:

```rust
// reactor.rs:530-541 (receiver task)
cache_recv.write().await.update(source.clone(), bytes);
dirty_recv.write().await.set(source, true);

// reactor.rs:591-592 (executor)
let snapshot = cache_exec.read().await.snapshot();
dirty_exec.write().await.clear_all();
```

`tokio::sync::RwLock` is **not** equivalent to `parking_lot::RwLock` — it serializes all writes and yields to the executor, which adds task scheduling overhead per boundary. For the reactor's actual access pattern (single producer task on the receiver side, single consumer task on the executor side, with `snapshot()` cloning the entire HashMap before releasing the lock), a `parking_lot::Mutex` or even an actor-pattern (single owning task with a channel for ops) would be substantially cheaper because there's no genuine read-write parallelism — readers and writers all run on dedicated tokio tasks.

The sequential queue (`SeqQueue = Arc<RwLock<VecDeque>>`) in the same file (line 38, 513, 538, 622) has the same shape and the same problem — actually worse, because Sequential mode does `cache_exec.write().await.update(...)` followed by `cache_exec.read().await.snapshot()` for every queued boundary, dropping and re-acquiring the lock between them.

`EndpointRegistry::inner` is also `tokio::sync::RwLock<HashMap>` (registry.rs:193), and `send_to_accumulator` takes a **write** lock (registry.rs:354) for every send — see PERF-06.

#### Evidence

```rust
// reactor.rs:530-541
Some((source, bytes)) = accumulator_rx.recv() => {
    match input_strategy_recv {
        InputStrategy::Latest => {
            cache_recv.write().await.update(source.clone(), bytes);
            dirty_recv.write().await.set(source, true);
        }
        ...
    }
    let _ = strategy_tx_recv.send(StrategySignal::BoundaryReceived).await;
}
```

```rust
// reactor.rs:591-593 (executor side, every fire)
let snapshot = cache_exec.read().await.snapshot();
dirty_exec.write().await.clear_all();
let result = (graph)(snapshot).await;
```

#### Suggested Resolution

- Replace `Arc<tokio::sync::RwLock<InputCache>>` with `Arc<parking_lot::Mutex<InputCache>>` since access is short and never holds the lock across `.await`. Run the existing `examples/performance/computation-graph` bench before/after to confirm.
- Or — more invasive — restructure the reactor into a single owning task that receives `(SourceName, Vec<u8>)` and `StrategySignal` events on a single channel, eliminating the lock entirely.

---

### PERF-06: `EndpointRegistry::send_to_accumulator` takes write lock per message

**Severity**: Major
**Location**: `crates/cloacina/src/computation_graph/registry.rs:349-393`
**Confidence**: High

#### Description

Every WS accumulator message routes through `send_to_accumulator`, which takes a **write** lock on the entire registry's `RwLock<RegistryInner>`. The rationale visible in the code is to enable pruning of closed channels (lines 381-384). But pruning is incidental — it happens once per closed channel. By taking a write lock on every message, all accumulator pushes are serialized through this single lock.

Multi-producer Kafka workloads, or multiple WS clients pushing to the same accumulator concurrently, hit this directly.

#### Evidence

```rust
// registry.rs:349-384
pub async fn send_to_accumulator(
    &self,
    name: &str,
    bytes: Vec<u8>,
) -> Result<usize, RegistryError> {
    let mut inner = self.inner.write().await;  // <-- WRITE lock per message
    let senders = inner
        .accumulators
        .get_mut(name)
        ...

    for (i, sender) in senders.iter().enumerate() {
        match sender.try_send(bytes.clone()) {
            Ok(()) => sent += 1,
            Err(mpsc::error::TrySendError::Closed(_)) => closed.push(i),
            ...
        }
    }
    // Prune closed channels
    for i in closed.into_iter().rev() {
        senders.remove(i);
    }
    ...
}
```

`bytes.clone()` per recipient is expected (each receiver gets its own copy); the issue is the registry-wide write lock.

#### Suggested Resolution

Restructure: store senders behind a per-name `Arc<Mutex<Vec<mpsc::Sender>>>` so the registry-level lock is only taken to add/remove names, not to send. Alternative: use `parking_lot::RwLock` and only acquire write when a closed channel is observed (i.e., a two-pass: read-lock to send, escalate to write-lock only for pruning).

---

### PERF-07: Reactor persists state via JSON serialization on every fire

**Severity**: Major
**Location**: `crates/cloacina/src/computation_graph/reactor.rs:670-726`
**Confidence**: High

#### Description

`persist_reactor_state` (called after every reactor fire) serializes the cache, dirty flags, and sequential queue via `serde_json::to_vec`. The schema spec (CLOACI-S-0011 area) and the wire format for accumulator-to-reactor communication is **bincode**. The data persisted here is internal-only (the reactor's own state), and bincode would be both faster to serialize and smaller on the wire.

Every fire pays a JSON serialization of the full `HashMap<SourceName, Vec<u8>>` cache plus the dirty flags. For a reactor with N accumulators each producing M-byte boundaries, this is O(N·M) JSON encoding per fire.

There's also a `cache.read().await` and `dirty.read().await` and optional `seq_queue.read().await` taken sequentially — three separate async lock acquisitions per persist.

#### Evidence

```rust
// reactor.rs:686-714
let cache_bytes = match serde_json::to_vec(&cache_snapshot.entries_raw()) {
    Ok(b) => b,
    Err(e) => { ... return; }
};
let dirty_bytes = match serde_json::to_vec(&dirty_snapshot.flags) {
    Ok(b) => b,
    ...
};
let seq_bytes = if let Some(q) = seq_queue {
    let queue = q.read().await;
    if queue.is_empty() { None }
    else { match serde_json::to_vec(&*queue) { Ok(b) => Some(b), ... } }
} ...
```

#### Suggested Resolution

Switch to `bincode` (already a workspace dep) for these blobs. Confirm with the existing CG benchmark.

---

### PERF-08: `runtime.get_task` invokes constructor per call (no instance caching)

**Severity**: Major
**Location**: `crates/cloacina/src/runtime.rs:188-191`; called from `crates/cloacina/src/executor/thread_task_executor.rs:802`
**Confidence**: High

#### Description

```rust
// runtime.rs:188-191
pub fn get_task(&self, namespace: &TaskNamespace) -> Option<Arc<dyn Task>> {
    self.inner.tasks.read().get(namespace).map(|ctor| ctor())
}
```

Every dispatched task triggers a `ctor()` — which for macro-emitted tasks calls `Arc::new(...)` (at minimum), and for `DynamicLibraryTask` constructs a fresh struct (`crates/cloacina/src/registry/loader/task_registrar/dynamic_task.rs:115-145`) — even though the heavy `LoadedWorkflowPlugin` is shared via `Arc`. For pure Rust tasks emitted by `#[task]`, this is one `Arc` allocation per dispatch; for FFI tasks, it's one struct allocation. Same shape applies to `get_workflow`, `get_trigger`, `get_computation_graph`, `get_triggerless_graph`, `get_reactor`.

The `state_manager.check_task_dependencies` path also calls `runtime.get_workflow(name)` per pending task per scheduler tick (`state_manager.rs:101`). At 100ms ticks with N pending tasks, that's a `Workflow` constructor invocation N times per tick — and `Workflow` cloning isn't cheap (it carries a `DependencyGraph`, `Vec<TaskNode>`, etc.).

The deeper issue is that the **constructor pattern is by-design** (the comment at runtime.rs:54 explains: "constructor closures over instance handles" so a fresh instance can be produced on demand). For Tasks/Triggers, there's no need for fresh instances per call — they're stateless `Arc<dyn Trait>`. The closures could cache.

#### Evidence

`Runtime::register_task` accepts `F: Fn() -> Arc<dyn Task> + Send + Sync + 'static` — there's no signal that the Fn is supposed to produce different values per call.

`scheduler_loop.rs:217` constructs a `StateManager` per scheduler tick (`StateManager::new(self.dal, self.runtime.clone())`) — that's an Arc clone, but cheap.

In `state_manager.check_task_dependencies` (`state_manager.rs:101`):
```rust
let workflow = match self.runtime.get_workflow(&workflow_execution.workflow_name) {
    Some(wf) => wf,
    ...
};
```
called inside `update_workflow_task_readiness`'s loop over `pending_tasks` — so it's per pending task, **not** per workflow.

#### Suggested Resolution

- For each registry, materialize the constructor's output once into a cache (e.g. `OnceLock<Arc<dyn Task>>` per namespace). The closure is `Fn` not `FnMut`/`FnOnce`, so callers were never promised a fresh value.
- Or, hoist the `runtime.get_workflow(name)` call out of the per-task loop in `state_manager` — fetch once per workflow, reuse across tasks.

---

### PERF-09: `CronEvaluator::new` re-parses the cron expression on every fire

**Severity**: Minor
**Location**: `crates/cloacina/src/cron_trigger_scheduler.rs:399, 437`
**Confidence**: High

#### Description

Each cron fire goes through `calculate_execution_times` → `CronEvaluator::new(cron_expr, tz)` and then `calculate_next_run` → `CronEvaluator::new(cron_expr, tz)` again. The `CronEvaluator` parses the cron string (`croner::Cron::new`) and the timezone string (`Tz::from_str`) every time. For a single schedule firing twice means double work; for many schedules it's double work per schedule per fire.

`CronEvaluator` is `Clone` and contains owned data — it could be cached per `Schedule.id`.

#### Evidence

```rust
// cron_trigger_scheduler.rs:399
let evaluator = CronEvaluator::new(cron_expr, tz).map_err(|e| {
    WorkflowExecutionError::ExecutionFailed { ... }
})?;
// ... a few lines down
// cron_trigger_scheduler.rs:437
let evaluator = CronEvaluator::new(cron_expr, tz).map_err(|e| {
    WorkflowExecutionError::ExecutionFailed { ... }
})?;
```

#### Suggested Resolution

Cache `CronEvaluator` per `(schedule_id, expression, tz)` keyed on `Scheduler.last_poll_times` or a sibling map. Invalidate on schedule update. Cheap, no benchmark needed — the wins are obvious.

---

### PERF-10: Batch accumulator persists buffer to DB on every event

**Severity**: Minor
**Location**: `crates/cloacina/src/computation_graph/accumulator.rs:633-647` (and the `persist_batch_buffer` function)
**Confidence**: High

#### Description

The batch accumulator runtime persists the entire buffer **every time a single event arrives**:

```rust
Some(bytes) = socket_rx.recv() => {
    buffer.push(bytes);
    // Persist buffer snapshot for crash resilience
    persist_batch_buffer(&ctx, &buffer).await;
    ...
}
```

For a buffer of N events, the total persistence cost over the buffer's lifetime is O(N²) bytes written (event 1 persists a 1-element buffer, event 2 persists a 2-element buffer, …, event N persists an N-element buffer). For batch accumulators with `flush_interval` configured (the typical case), the buffer should grow to thousands or millions of events between flushes — that's a huge write amplification.

Batch accumulators are designed to **amortize** the cost of per-event work; persisting the full buffer per event undoes the amortization.

#### Suggested Resolution

Persist the buffer (a) on flush, (b) periodically (every K events or M seconds), or (c) only the *new* event (append-only persistence). The "crash resilience" requirement is satisfied by appending each event individually rather than rewriting the buffer.

---

### PERF-11: `get_active_executions` and `get_ready_for_retry` return un-paginated full table scans

**Severity**: Minor
**Location**: `crates/cloacina/src/dal/unified/workflow_execution.rs:259, 282`; `crates/cloacina/src/dal/unified/task_execution/claiming.rs:786-799, 813-826`
**Confidence**: High

#### Description

`get_active_executions` returns ALL Pending+Running workflow executions; `get_ready_for_retry` returns ALL Ready tasks. There's no LIMIT, no pagination, no priority ordering. Both are called every scheduler tick (default 100 ms). Postgres handles this well via index lookup, but the wire-format payload grows linearly with the active workload, and the scheduler then iterates over the full result set in a single tick — which is bounded by `max_concurrent_tasks` worth of dispatches.

For a system with thousands of concurrent workflows, the Vec<WorkflowExecutionRecord> returned per tick is megabytes of model rows that may not all be processable in one tick anyway.

#### Evidence

```rust
// workflow_execution.rs:257-264
let executions: Vec<UnifiedWorkflowExecution> = conn
    .interact(move |conn| {
        workflow_executions::table
            .filter(workflow_executions::status.eq_any(vec!["Pending", "Running"]))
            .load(conn)  // <-- no .limit()
    })
    ...
```

```rust
// claiming.rs:786-799
let ready_tasks: Vec<UnifiedTaskExecution> = conn
    .interact(move |conn| {
        task_executions::table
            .filter(task_executions::status.eq("Ready"))
            .filter(task_executions::retry_at.is_null().or(task_executions::retry_at.le(now)))
            .load(conn)  // <-- no .limit()
    })
    ...
```

The scheduler dispatches all of them in `dispatch_ready_tasks` (`scheduler_loop.rs:262-278`) inside the same tick — but if `max_concurrent_tasks` is, say, 4 and there are 500 Ready tasks, only 4 will actually start running, while the dispatcher runs through 500 `event.clone()` allocations and `dispatcher.dispatch(event).await` calls.

#### Suggested Resolution

Add `.limit(N)` based on capacity (executor.has_capacity() / available_permits()). Return only what can be dispatched. For Postgres, an `ORDER BY created_at` + `LIMIT N` keeps things fair.

---

### PERF-12: Auth `KeyCache` uses `tokio::sync::Mutex` instead of `RwLock`; capacity 256 too small for multi-tenant

**Severity**: Minor
**Location**: `crates/cloacina-server/src/routes/auth.rs:58-117`
**Confidence**: High

#### Description

The auth cache uses `tokio::sync::Mutex<LruCache<…>>` — every request takes an exclusive lock for **read**. (LRU's `get` does mutate recency, so a `RwLock<LruCache>` doesn't help directly without restructuring; but a sharded cache or a TTL cache that doesn't update on read would.)

Capacity is hardcoded to 256 entries with a 30 s TTL (`auth.rs:77`). A workload with more than 256 distinct API keys (e.g. one key per tenant × dozens of tenants × distinct caller principals) will thrash the cache, causing every request to hit the DB. The DB validate_hash query is indexed (`idx_api_keys_hash`), so it's not catastrophic — but it adds round-trip latency to every request.

#### Evidence

```rust
// auth.rs:58-78
pub struct KeyCache {
    cache: Mutex<LruCache<String, CachedEntry>>,
    ttl: Duration,
}

impl KeyCache {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(capacity).expect(...)),
            ttl,
        }
    }

    pub fn default_cache() -> Self {
        Self::new(256, Duration::from_secs(30))
    }
```

#### Suggested Resolution

- Make capacity and TTL configurable via the server CLI / config. Defaults of 4096 / 60 s would be better for multi-tenant.
- Consider `dashmap::DashMap` (sharded reader-friendly map) with a TTL eviction task — the `LruCache` recency is a nice-to-have, not worth serializing every request.

---

### PERF-13: `info!` macro with eager Vec allocation in executor hot path

**Severity**: Minor
**Location**: `crates/cloacina/src/executor/thread_task_executor.rs:461-466`
**Confidence**: High

#### Description

```rust
let key_count = context.data().len();
let keys: Vec<_> = context.data().keys().collect();  // <-- always allocates
info!(
    "Context saved: {} (workflow: {}, {} keys: {:?}, context_id: {:?})",
    claimed_task.task_name, claimed_task.workflow_execution_id, key_count, keys, context_id
);
```

The `let keys: Vec<_> = context.data().keys().collect();` is unconditional. It runs even if the `info` level filter would suppress the log. For tasks with large contexts, this is a per-task allocation of `Vec<&String>` plus the implicit overhead of calling `info!` (which does check the filter, but only after the let binding executes).

Same concern at `thread_task_executor.rs:299-304` — but that's `debug!` and the collect is *inside* the macro args, which is lazy. So the issue is specifically the `let` extraction at line 462.

The `format!()` strings at lines 788, 806, 826, 930 are only built on the error path, so they're fine.

#### Evidence

(Lines 461-466 above.)

#### Suggested Resolution

Move the `keys: Vec<_>` and `key_count` computations *into* the `info!` macro args:

```rust
info!(
    task = %claimed_task.task_name,
    workflow_id = %claimed_task.workflow_execution_id,
    key_count = context.data().len(),
    keys = ?context.data().keys().collect::<Vec<_>>(),
    "Context saved"
);
```

This way `tracing` only formats them if the layer accepts the event. Same fix is generic — all eager `let s = format!(...)` patterns before `info!`/`debug!`/`warn!` should be inlined.

---

### PERF-14: Heartbeat task spawned per claimed task; per-tick heartbeat is one DB write per active task

**Severity**: Minor
**Location**: `crates/cloacina/src/executor/thread_task_executor.rs:723-758`
**Confidence**: Medium

#### Description

Every claimed task spawns a `tokio::spawn` heartbeat task that runs `dal.task_execution().heartbeat(task_id, runner_id)` on a 10-s interval. With M concurrent tasks this is M Postgres connections held by heartbeats every 10 s plus M `tokio::spawn` allocations per task.

For typical workloads (M ≤ max_concurrent_tasks ≤ 32) this is fine; for heavily-claimed runners (M = 100s), the heartbeat fan-out is significant — and each heartbeat is a separate transaction (`UPDATE task_executions SET heartbeat_at=NOW() WHERE id=? AND claimed_by=?`), with no batching across runner-owned tasks.

#### Evidence

```rust
// thread_task_executor.rs:728-755
Some(tokio::spawn(async move {
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        ticker.tick().await;
        match dal.task_execution().heartbeat(task_id, runner_id).await {
            ...
        }
    }
}))
```

#### Suggested Resolution

For heavy parallelism, a single per-runner heartbeat task that does `UPDATE task_executions SET heartbeat_at=NOW() WHERE claimed_by=$runner_id AND status='Running'` would be one DB transaction per interval regardless of M. Need a benchmark to confirm M needs to be high enough for this to matter.

---

### PERF-15: Python task wrapper acquires GIL 5+ times per task invocation

**Severity**: Minor
**Location**: `crates/cloacina-python/src/task.rs:184-188, 204-206, 298-313, 340-353, 368-389`
**Confidence**: High

#### Description

Each Python task execution acquires the GIL multiple times:

1. `Python::with_gil(|py| self.python_function.clone_ref(py))` — line 184.
2. `Python::with_gil(|py| self.on_success_callback.as_ref().map(|f| f.clone_ref(py)))` — line 185-186.
3. `Python::with_gil(|py| self.on_failure_callback.as_ref().map(|f| f.clone_ref(py)))` — line 187-188.
4. `Python::with_gil(|py| on_failure.as_ref().map(|f| f.clone_ref(py)))` — line 204 (fourth — this is `on_failure_for_body`, which already-cloned was on line 188).
5. `tokio::task::spawn_blocking(|| Python::with_gil(|py| { ... }))` — line 205-206 (inside spawn_blocking — actual user code execution).
6. (For CG-invoking tasks) `Python::with_gil(|py| ... PyContext ...)` — line 298 (after task body returns).
7. `Python::with_gil(|py| -> PyResult<()> { ... })` — line 340 (post_invocation hook).
8. `Python::with_gil(|py| { ... callback ... })` — line 368 (on_success).

The GIL is a process-wide mutex. Each `with_gil` is a lock acquisition. Pre-spawn_blocking acquisitions (1-4) are particularly wasteful: they could be combined into a single `with_gil` block that clones all three callbacks at once.

#### Evidence

(Lines 184-188 and 204 above.)

#### Suggested Resolution

Combine the 3-4 pre-spawn_blocking GIL acquisitions into one block:

```rust
let (function, on_success, on_failure) = Python::with_gil(|py| {
    (
        self.python_function.clone_ref(py),
        self.on_success_callback.as_ref().map(|f| f.clone_ref(py)),
        self.on_failure_callback.as_ref().map(|f| f.clone_ref(py)),
    )
});
```

This reduces 3-4 acquisitions to 1 per task. Net win regardless of workload; cost zero (refactor only).

---

### PERF-16: Reactor uses `tokio::sync::watch` for accumulator health with 100 ms polling startup gate

**Severity**: Minor
**Location**: `crates/cloacina/src/computation_graph/reactor.rs:413-454`
**Confidence**: Medium

#### Description

During reactor startup, the gating loop polls every 100 ms (`reactor.rs:437`) checking each accumulator's health watch:

```rust
'gating: loop {
    for (name, rx) in &self.accumulator_health_rxs {
        let h = rx.borrow().clone();
        ...
    }

    if healthy_set.len() >= all_names.len() { break 'gating; }

    tokio::select! {
        _ = shutdown_gate.changed() => { return; }
        _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => { }
    }
    ...
}
```

`watch::Receiver` supports `changed().await` which waits for state change. The current code polls every 100 ms regardless of whether any health change has happened. For a reactor with N accumulators, this is N `borrow().clone()` calls every 100 ms during the warm-up phase.

The same pattern in the degraded-mode monitor at `reactor.rs:476-503` polls every 1 s. The 1 s cadence is fine for monitoring; the startup polling is more wasteful given that watch can deliver instant notifications.

#### Evidence

(Lines 413-440 above.)

#### Suggested Resolution

Replace the 100ms sleep with `tokio::select!` over each `watch::Receiver::changed()` (using `futures::stream::FuturesUnordered` for the per-accumulator awaits). Cost: small refactor; benefit: reactor warmup lock-step with accumulator readiness, no idle polling.

---

### PERF-17: `Database::get_sqlite_connection` runs PRAGMA on every checkout

**Severity**: Minor
**Location**: `crates/cloacina/src/database/connection/mod.rs:579-608`
**Confidence**: High

#### Description

```rust
pub async fn get_sqlite_connection(&self) -> Result<...> {
    let pool = ...;
    let conn = pool.get().await?;
    // Ensure SQLite pragmas are set on every checkout — pragmas are per-connection
    // and may be lost if the pool recycles the connection.
    conn.interact(|conn| {
        let _ = diesel::sql_query("PRAGMA journal_mode=WAL;").execute(conn);
        let _ = diesel::sql_query("PRAGMA busy_timeout=30000;").execute(conn);
    })
    .await
    .ok();
    Ok(conn)
}
```

Two extra round-trips per DAL call. The comment claims pragmas "may be lost if the pool recycles the connection". `journal_mode` is database-wide (file metadata, persists across connections forever once set). `busy_timeout` is per-connection, but with `deadpool` keeping connections alive, it sticks for the life of the connection — only a fresh connection needs to set it. deadpool has a `Manager::create` hook for this.

With pool size 1 (PERF-02), every single DAL call pays for `journal_mode=WAL` (no-op effectively, but still a round-trip) and `busy_timeout=30000` (per-connection, but the connection is the same one). This is pure overhead.

#### Suggested Resolution

Move PRAGMA setup into a custom `deadpool_diesel::Manager` post-create hook (or use the existing `recycle` hook). Run pragmas once per connection lifetime, not per checkout. Modest but free win.

---

### PERF-18: Server `list_executions` does N+1 by re-fetching per-execution detail

**Severity**: Minor
**Location**: `crates/cloacina/src/runner/default_runner/workflow_executor_impl.rs:341-362`
**Confidence**: High

#### Description

```rust
async fn list_executions(&self) -> Result<Vec<WorkflowExecutionResult>, _> {
    let dal = DAL::new(self.database.clone());
    let executions = dal.workflow_execution().list_recent(100).await?;
    let mut results = Vec::new();
    for execution in executions {
        if let Ok(result) = self.build_workflow_result(execution.id.into()).await {
            results.push(result);
        }
    }
    Ok(results)
}
```

`build_workflow_result` is presumably another DAL call (probably the `get_by_id` shape). So `list_executions` does **101 DB queries** to return 100 rows. Used by the server's `GET /v1/tenants/{tenant_id}/executions` endpoint.

#### Evidence

(Lines above.)

#### Suggested Resolution

If `build_workflow_result` is hydrating from already-loaded fields, project them into the `WorkflowExecutionResult` directly without the extra round-trip. If it needs additional joined data (task counts, etc.), do a single batch query after the list. Standard list-detail collapse.

---

### PERF-19: `RunnerMessage` channel between Python and runtime thread is unbounded

**Severity**: Observation
**Location**: `crates/cloacina-python/src/bindings/runner.rs:24, 162`
**Confidence**: High

#### Description

```rust
use tokio::sync::{mpsc, oneshot};
...
tx: mpsc::UnboundedSender<RuntimeMessage>,
```

The Python → runtime-thread channel is `UnboundedSender`. If the runtime thread stalls (e.g., DB slow), Python callers can enqueue messages without bound, growing memory. Each `RuntimeMessage::Execute` carries a `Context<serde_json::Value>` and a `oneshot::Sender` — non-trivial heap.

Practical risk is low because Python callers usually `.await` the response (oneshot) before sending the next message. But programmatic batch-fire from Python could fill memory.

#### Suggested Resolution

`mpsc::channel(N)` with a sensible bound (e.g., 1024) and treat backpressure as a real signal in the Python wrapper.

---

### PERF-20: Hot-path `format!` allocations in scheduler debug paths

**Severity**: Observation
**Location**: `crates/cloacina/src/execution_planner/state_manager.rs:252-256, 269-272, 286-289`; `crates/cloacina/src/dispatcher/default.rs:86-132`
**Confidence**: High

#### Description

Several `tracing::debug!` and `info!` calls in scheduler/state-manager/dispatcher paths use eagerly-formatted strings or expensive lookups (`task_namespace.to_string()` returning `String`) before the macro check. Cumulatively these are non-trivial when scheduler tick is 100 ms and there are many tasks.

E.g., `state_manager.rs:111` constructs `let task_namespace = TaskNamespace::from_string(&task_execution.task_name)` — a parse — before checking dependencies. The same parse runs in `update_workflow_task_readiness`, `check_task_dependencies`, `evaluate_trigger_rules` (probably), and `update_execution_final_context` (`scheduler_loop.rs:399`) for the same task name on the same tick. Each parse allocates `Vec<String>`s.

#### Evidence

```rust
// state_manager.rs:252-256 — verbose debug log uses {:?} of complex types
tracing::debug!(
    "[DEBUG] Scheduler evaluating TaskSuccess trigger rule: looking up task_name '{}' in workflow execution {}",
    task_name, task_execution.workflow_execution_id
);
```

Multiple `task_namespace.to_string()` and `TaskNamespace::from_string(&task_execution.task_name)` calls per task per tick.

#### Suggested Resolution

- Hoist the `TaskNamespace::from_string` parse to once per task, reuse across calls within `update_workflow_task_readiness`.
- Audit `[DEBUG]`-style logs with `{:?}` of complex types for inlining into the macro args.

These are minor individually but cumulative on the scheduler tick.

---

## Positive Patterns

1. **SQL-derived gauge** for `cloacina_active_workflows` (`scheduler_loop.rs:166-168`) — re-seeded every tick from the row count, avoiding the gauge-drift bug pattern (per CLOACI-T-0534 in the system overview). Correct and self-healing.

2. **Push-based dispatch** post-T-0509: the scheduler emits `TaskReadyEvent`s through a `Dispatcher` that routes to executors, replacing the older polling loop on `task_outbox`. The pattern is right — the remaining DAL polling on the scheduler loop is the residue, and `task_outbox` exists for the eventual LISTEN/NOTIFY path.

3. **Content-hash artifact reuse** in the compiler service (`crates/cloacina/src/registry/workflow_registry/mod.rs:325` `find_success_by_hash`) — fresh upload with matching content hash skips `cargo build` entirely. Prevents redundant compilation work and supports hot-reload deployment.

4. **Atomic DB-row claiming** for cron schedules (`claim_and_update_cron`) and tasks (`claim_for_runner`) — a single `UPDATE ... WHERE claimed_by IS NULL` returning rows-affected. This is the canonical correct pattern for distributed claim-once semantics, and avoids the reservation-race bugs that plague naive read-then-write designs.

5. **Batched DAL queries on the scheduler critical path**: `get_pending_tasks_batch` (`task_execution/queries.rs:91`), `get_task_statuses_batch` (`queries.rs:300`), `get_dependency_metadata_with_contexts` (`task_execution_metadata.rs`) — the right shape, eliminating per-task round-trips. The remaining N+1 patterns (PERF-04) are the exceptions, not the rule.

6. **Bounded reason labels** for `cloacina_tasks_total{reason}` (`thread_task_executor.rs:62-79`, gated `#[cfg(test)]` as a behavior-spec test) — prevents the cardinality explosion that's otherwise easy to accidentally cause with Prometheus labels.

7. **`MissedTickBehavior::Skip`** on the cron scheduler (`cron_trigger_scheduler.rs:183`) and stale-claim sweeper (`stale_claim_sweeper.rs:91`) — right choice for periodic loops; means the system never thunders after a pause.

8. **Per-task semaphore-based concurrency** (`thread_task_executor.rs:153, 761-766`) — clean, well-bounded; integrates with `SlotToken` for deferred-task patterns. The shared semaphore across `Clone` (line 663) is the right choice.
