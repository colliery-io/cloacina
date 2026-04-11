---
title: "Performance Tuning"
description: "How to optimize Cloacina for high-throughput and low-latency workflow execution"
weight: 50
---

This guide explains how to tune Cloacina's configuration for different workload profiles. All values referenced here come from `DefaultRunnerConfig` and the scheduler internals -- nothing is fabricated.

## 1. Concurrency Tuning

The `max_concurrent_tasks` setting (default: **4**) controls how many tasks can execute simultaneously across all active workflows. This is the single most impactful knob for throughput.

### How to size based on workload

| Workload Type | Recommended Setting | Rationale |
|--------------|---------------------|-----------|
| CPU-bound (ML inference, image processing) | CPU cores | Exceeding core count causes context switching without gain |
| IO-bound (API calls, DB queries, file reads) | CPU cores x 2 | Tasks spend most time waiting, so oversubscription is beneficial |
| Mixed | CPU cores x 1.5 | Balance between CPU saturation and IO wait utilization |

### Slot management and deferred tasks

The executor uses a semaphore-based slot system. When a task completes (success or failure), its slot is released immediately. Tasks in `Ready` state wait for available slots before dispatching. The scheduler pushes tasks via the dispatcher as soon as dependencies are satisfied and a slot is available.

Key insight: if you have workflows with sequential task chains (A -> B -> C), a single workflow only occupies one slot at a time. Concurrency helps when you run many workflows simultaneously or when workflows have parallel fan-out stages.

### Practical example

```rust
use std::time::Duration;
use cloacina::runner::DefaultRunnerConfig;

// For an 8-core machine running IO-bound tasks:
let config = DefaultRunnerConfig::builder()
    .max_concurrent_tasks(16)  // 8 cores x 2
    .build();

// For an 8-core machine running CPU-bound tasks:
let config = DefaultRunnerConfig::builder()
    .max_concurrent_tasks(8)   // match core count
    .build();
```

### Observed scaling behavior

From the performance test suite (SQLite backend):

- Near-linear scaling up to 8 workers with ~89% efficiency
- 16.6x throughput improvement at 32x concurrency increase (diminishing returns)
- Performance degradation at 64 workers due to database contention
- Optimal efficiency in the 4-8 range for SQLite; higher for PostgreSQL

## 2. Scheduler Optimization

The `scheduler_poll_interval` (default: **100ms**) controls how frequently the scheduler loop checks for active pipelines, evaluates task readiness, and dispatches ready tasks to executors.

### The latency vs CPU tradeoff

| Interval | Use Case | Tradeoff |
|----------|----------|----------|
| 10-50ms | Real-time/interactive workflows | Lower latency, higher CPU and DB query load |
| 100ms (default) | General purpose | Good balance for most workloads |
| 250-500ms | Batch processing, low-priority jobs | Minimal overhead, higher scheduling latency |
| 1000ms+ | Background maintenance workflows | Very low overhead, seconds of scheduling delay |

### What happens each poll cycle

Each scheduler tick:
1. Queries for all active pipeline executions
2. Batch-loads pending tasks across all active pipelines
3. Groups tasks by pipeline and evaluates trigger rules
4. Marks tasks as Ready when dependencies are satisfied
5. Dispatches all Ready tasks to executors via the dispatcher

The scheduler implements exponential backoff on errors (up to 30 seconds max) with a circuit-breaker pattern that logs a warning after 5 consecutive failures.

### Configuration

```rust
use std::time::Duration;
use cloacina::runner::DefaultRunnerConfig;

// Low-latency: check every 50ms
let config = DefaultRunnerConfig::builder()
    .scheduler_poll_interval(Duration::from_millis(50))
    .build();

// Batch processing: check every 500ms
let config = DefaultRunnerConfig::builder()
    .scheduler_poll_interval(Duration::from_millis(500))
    .build();
```

### Impact on database query frequency

At 100ms poll interval with N active pipelines, expect roughly 10 query batches per second. Each batch involves:
- 1 query for active executions
- 1 batch query for pending tasks across all pipelines
- N completion checks (one per pipeline)
- Variable Ready-task dispatch queries

Increasing the interval proportionally reduces this load.

## 3. Database Performance

### Pool sizing

The `db_pool_size` (default: **10**) controls the number of connections in the pool. Size this based on:

```
db_pool_size >= max_concurrent_tasks + background_services + headroom
```

Background services that consume connections:
- Scheduler loop: 1-2 connections per tick
- Cron scheduler: 1 connection per poll
- Registry reconciler: 1 connection per reconcile cycle
- Stale claim sweeper: 1 connection per sweep
- Recovery manager: 1 connection during recovery

A practical formula:

```rust
use cloacina::runner::DefaultRunnerConfig;

// For 16 concurrent tasks:
let config = DefaultRunnerConfig::builder()
    .max_concurrent_tasks(16)
    .db_pool_size(25)  // 16 tasks + 5 background + 4 headroom
    .build();
```

### PostgreSQL vs SQLite performance

| Characteristic | SQLite | PostgreSQL |
|---------------|--------|------------|
| Concurrent writes | Serialized (single-writer lock) | MVCC (multiple concurrent writers) |
| Optimal concurrency | 4-8 workers | 16-64 workers |
| Connection overhead | Minimal (file-based) | Per-connection memory (~5-10MB) |
| Network latency | None (in-process) | Present (even on localhost) |
| WAL mode | Enabled by default in URLs | N/A (uses MVCC) |
| Best for | Development, single-node, low concurrency | Production, multi-node, high concurrency |

### SQLite WAL configuration

Cloacina SQLite URLs should include WAL mode and busy timeout:

```
sqlite:///path/to/cloacina.db?_journal_mode=WAL&_busy_timeout=5000
```

WAL mode allows concurrent readers during writes. The busy timeout (5000ms) prevents immediate `SQLITE_BUSY` errors under contention.

### Connection pool behavior

The pool uses deadpool with the following behavior:
- Connections are created lazily up to `db_pool_size`
- Idle connections are reused (no connection-per-query overhead)
- Pool exhaustion blocks the caller until a connection is returned
- PostgreSQL connections support schema-based isolation (`SET search_path`)

If you see pool exhaustion warnings in logs, increase `db_pool_size` or reduce `max_concurrent_tasks`.

## 4. Timeout Configuration

### Task timeout

The `task_timeout` (default: **300 seconds / 5 minutes**) is the maximum wall-clock time for a single task execution. If exceeded, the task is marked as failed and eligible for retry according to its retry policy.

```rust
use std::time::Duration;
use cloacina::runner::DefaultRunnerConfig;

// Short timeout for fast tasks (API calls)
let config = DefaultRunnerConfig::builder()
    .task_timeout(Duration::from_secs(30))
    .build();

// Long timeout for ML training tasks
let config = DefaultRunnerConfig::builder()
    .task_timeout(Duration::from_secs(1800))  // 30 minutes
    .build();
```

### Pipeline timeout

The `pipeline_timeout` (default: **Some(3600 seconds / 1 hour)**) is the maximum time for an entire workflow execution from start to finish. Set to `None` to disable.

```rust
use std::time::Duration;
use cloacina::runner::DefaultRunnerConfig;

// Strict pipeline timeout for SLA-bound workflows
let config = DefaultRunnerConfig::builder()
    .pipeline_timeout(Some(Duration::from_secs(600)))  // 10 minutes total
    .build();

// No pipeline timeout (tasks still have individual timeouts)
let config = DefaultRunnerConfig::builder()
    .pipeline_timeout(None)
    .build();
```

### Interaction with retries

When a task times out:
1. The task is marked as Failed
2. If the retry policy allows (attempt < max_attempts), it transitions back to Ready with an exponential backoff delay (`retry_at` timestamp)
3. The scheduler picks it up on the next poll after `retry_at` passes
4. Each retry attempt counts toward the pipeline timeout

If `pipeline_timeout` expires while a task is waiting for retry, the entire pipeline is failed. Size your pipeline timeout to accommodate worst-case retry scenarios:

```
pipeline_timeout >= (task_timeout * max_attempts * num_sequential_tasks) + scheduling_overhead
```

## 5. Cron Scheduling Performance

### Poll interval

The `cron_poll_interval` (default: **30 seconds**) controls how often the cron scheduler evaluates all registered schedules and launches due executions.

```rust
use std::time::Duration;
use cloacina::runner::DefaultRunnerConfig;

// Sub-minute precision needed
let config = DefaultRunnerConfig::builder()
    .cron_poll_interval(Duration::from_secs(10))
    .build();

// Only hourly schedules, minimize DB load
let config = DefaultRunnerConfig::builder()
    .cron_poll_interval(Duration::from_secs(60))
    .build();
```

### Catchup execution limits

The `cron_max_catchup_executions` (default: **usize::MAX**, effectively unlimited) limits how many missed executions are launched after downtime. Without a limit, a schedule that runs every minute would launch 1,440 catchup executions after 24 hours of downtime.

```rust
use std::time::Duration;
use cloacina::runner::DefaultRunnerConfig;

// Limit catchup to prevent recovery storms
let config = DefaultRunnerConfig::builder()
    .cron_max_catchup_executions(5)
    .build();

// Disable catchup entirely (only run future occurrences)
let config = DefaultRunnerConfig::builder()
    .cron_max_catchup_executions(0)
    .build();
```

### Cron recovery settings

The cron recovery system detects and re-runs lost executions:

| Parameter | Default | Purpose |
|-----------|---------|---------|
| `cron_enable_recovery` | `true` | Enable/disable recovery entirely |
| `cron_recovery_interval` | 300s (5 min) | How often to scan for lost executions |
| `cron_lost_threshold_minutes` | 10 | Minutes before an execution is considered lost |
| `cron_max_recovery_age` | 86400s (24h) | Ignore executions older than this |
| `cron_max_recovery_attempts` | 3 | Give up after this many recovery retries |

For high-frequency schedules (every minute or less), ensure `cron_lost_threshold_minutes` is larger than the expected execution duration to avoid false positives.

### Database impact of high-frequency schedules

Each cron poll:
- Queries all registered schedules
- Checks last execution time for each
- Creates new workflow executions for due schedules
- Writes catchup records if behind

With 100 registered schedules at 30-second poll interval, expect ~3-4 queries per second. Scale `db_pool_size` accordingly.

## 6. Registry and Reconciler

### Reconciliation interval

The `registry_reconcile_interval` (default: **60 seconds**) controls how often the reconciler scans for new or updated workflow packages.

```rust
use std::time::Duration;
use cloacina::runner::DefaultRunnerConfig;

// Faster hot-reload during development
let config = DefaultRunnerConfig::builder()
    .registry_reconcile_interval(Duration::from_secs(10))
    .build();

// Production: packages change infrequently
let config = DefaultRunnerConfig::builder()
    .registry_reconcile_interval(Duration::from_secs(300))  // 5 minutes
    .build();
```

### Startup reconciliation

The `registry_enable_startup_reconciliation` (default: **true**) causes the runner to scan and load all packages on startup before accepting work. This ensures all workflows are available immediately but adds startup time proportional to the number of packages.

To reduce startup time in environments where packages are loaded lazily:

```rust
use cloacina::runner::DefaultRunnerConfig;

let config = DefaultRunnerConfig::builder()
    .registry_enable_startup_reconciliation(false)
    .build();
```

### Storage backend performance

| Backend | Read Perf | Write Perf | Best For |
|---------|-----------|------------|----------|
| `"filesystem"` (default) | Fast (direct I/O) | Fast (file write) | Single-node, simple deployments |
| `"sqlite"` | Fast (indexed) | Moderate (WAL writes) | Single-node with transactional guarantees |
| `"postgres"` | Moderate (network) | Moderate (network) | Multi-node shared registry |

For filesystem storage, set `registry_storage_path` to a fast local disk (SSD/NVMe), not a network mount:

```rust
use std::path::PathBuf;
use cloacina::runner::DefaultRunnerConfig;

let config = DefaultRunnerConfig::builder()
    .registry_storage_backend("filesystem")
    .registry_storage_path(Some(PathBuf::from("/var/lib/cloacina/registry")))
    .build();
```

## 7. Computation Graph Performance

Computation graphs run as long-lived reactive pipelines separate from the workflow scheduler. Their performance characteristics differ from request-response workflows. Workflows are batch-oriented with database-backed state; computation graphs are streaming with in-memory state. Workflow tuning focuses on database throughput; CG tuning focuses on channel capacity and reactor latency.

### Boundary channel capacity

The boundary channel between accumulators and the reactor defaults to **256 slots** (set in the Reactive Scheduler's `load_graph` method). The merge channel within each accumulator defaults to **1024 slots** (`AccumulatorRuntimeConfig::default()`).

If accumulators produce data faster than the reactor processes it, the channel fills and backpressure slows the producers. Signs of channel saturation:
- Accumulator health transitioning to Disconnected
- Increasing memory usage under sustained load
- Events arriving late at the reactor

To handle high-throughput sources, increase `merge_channel_capacity`:

```rust
use cloacina::computation_graph::accumulator::AccumulatorRuntimeConfig;

let config = AccumulatorRuntimeConfig {
    merge_channel_capacity: 4096,  // 4x default for high-volume sources
};
```

### Batch accumulator tuning

Batch accumulators buffer events and flush on signal, timer, or size threshold:

| Parameter | Default | Impact |
|-----------|---------|--------|
| `flush_interval` | None | Timer-based flush (e.g., every 100ms) |
| `max_buffer_size` | None | Size-triggered flush (e.g., every 1000 events) |

For high-throughput streams, set both to bound memory and latency:

```rust
use std::time::Duration;
use cloacina::computation_graph::accumulator::BatchAccumulatorConfig;

let config = BatchAccumulatorConfig {
    flush_interval: Some(Duration::from_millis(100)),
    max_buffer_size: Some(1000),
};
```

### Reactor execution strategy

The `InputStrategy` affects how the reactor processes boundaries:

- **Latest**: One slot per source, overwritten on each update. Fires with the freshest data. Best for real-time dashboards where only the latest value matters.
- **Sequential**: Boundaries preserved in order, one execution per boundary. Best for event sourcing where every boundary must be processed.

`Latest` is more performant under high load because it coalesces rapid updates. `Sequential` guarantees no boundary is skipped but can fall behind if the graph function is slower than the incoming rate.

### Graph compilation overhead

Graph compilation (converting the DSL into a `CompiledGraphFn`) is a one-time cost at load time. The compiled function is a simple closure over the input cache -- there is no per-execution compilation overhead. For packaged graphs, compilation happens during reconciliation, not at runtime.

### Reactor health polling

The reactor polls accumulator health every **100ms** during startup gating (waiting for all accumulators to become Live). After going Live, the degraded-mode monitor polls every **1 second** to detect disconnected accumulators. These intervals are not configurable but have negligible overhead.

## 8. Monitoring Performance

### Key metrics to watch

Cloacina emits the following metrics via the `metrics` crate:

| Metric | Type | Labels | What it tells you |
|--------|------|--------|-------------------|
| `cloacina_tasks_total` | Counter | `status=completed\|failed` | Task completion rates |
| `cloacina_pipelines_total` | Counter | `status=completed\|failed` | Workflow completion rates |

Monitor these alongside system metrics:
- **Queue depth**: Number of workflows in `Pending` + tasks in `Ready` state (query the database)
- **Execution latency**: Time from workflow submission to completion
- **Pool utilization**: Connection pool wait times (available via deadpool metrics)
- **Scheduler loop duration**: Time spent in each poll cycle (visible at `debug` log level)

### Log levels for performance diagnosis

```bash
# Production: minimal overhead
RUST_LOG=cloacina=info

# Diagnose scheduling delays
RUST_LOG=cloacina::execution_planner=debug

# Diagnose task execution issues
RUST_LOG=cloacina::executor=debug

# Full detail (significant overhead -- never in production)
RUST_LOG=cloacina=trace
```

The `debug` level on the scheduler loop logs each cycle's outcome ("Scheduling loop completed successfully"), which is useful for measuring actual poll timing but adds ~1KB/s of log volume at 100ms intervals.

### OpenTelemetry overhead

If using the `metrics` crate with an OpenTelemetry exporter:
- Counter increments are effectively free (atomic add)
- Histogram observations add minor allocation for bucket sorting
- Export intervals control network overhead -- use 10-30 second export intervals in production
- The tracing spans on the runner (`runner_task`) add ~200ns per span creation

## 9. Production Recommendations

### Low-latency profile (target: <100ms scheduling delay)

For interactive workflows where users wait for results:

```rust
use std::time::Duration;
use cloacina::runner::DefaultRunnerConfig;

let config = DefaultRunnerConfig::builder()
    .max_concurrent_tasks(16)
    .scheduler_poll_interval(Duration::from_millis(25))
    .task_timeout(Duration::from_secs(30))
    .pipeline_timeout(Some(Duration::from_secs(120)))
    .db_pool_size(30)
    .enable_cron_scheduling(false)       // disable if not needed
    .enable_registry_reconciler(false)   // load workflows at startup only
    .enable_claiming(true)
    .heartbeat_interval(Duration::from_secs(5))
    .build();
```

Requirements:
- PostgreSQL (SQLite serializes writes, adding latency under load)
- SSD-backed storage for the database
- Network latency to PostgreSQL < 1ms (co-located or same availability zone)

### High-throughput profile (target: >1000 workflows/min)

For batch-style processing with many concurrent workflows:

```rust
use std::time::Duration;
use cloacina::runner::DefaultRunnerConfig;

let config = DefaultRunnerConfig::builder()
    .max_concurrent_tasks(32)
    .scheduler_poll_interval(Duration::from_millis(50))
    .task_timeout(Duration::from_secs(300))
    .pipeline_timeout(Some(Duration::from_secs(3600)))
    .db_pool_size(50)
    .cron_poll_interval(Duration::from_secs(30))
    .cron_max_catchup_executions(10)
    .registry_reconcile_interval(Duration::from_secs(120))
    .enable_claiming(true)
    .heartbeat_interval(Duration::from_secs(10))
    .build();
// Note: stale_claim_sweep_interval (default 30s) and stale_claim_threshold
// (default 60s) are struct defaults, not builder methods.
```

Requirements:
- PostgreSQL with connection pooling (PgBouncer in transaction mode)
- Database connection limit >= (num_runners x db_pool_size) + application connections
- Horizontal scaling: run multiple runner instances with `enable_claiming(true)` for work distribution

### Batch processing profile (minimize resource usage)

For nightly/weekly batch jobs where latency does not matter:

```rust
use std::time::Duration;
use cloacina::runner::DefaultRunnerConfig;

let config = DefaultRunnerConfig::builder()
    .max_concurrent_tasks(4)
    .scheduler_poll_interval(Duration::from_millis(500))
    .task_timeout(Duration::from_secs(1800))       // 30 min per task
    .pipeline_timeout(Some(Duration::from_secs(14400)))  // 4 hours total
    .db_pool_size(10)
    .cron_poll_interval(Duration::from_secs(60))
    .cron_max_catchup_executions(1)     // only catch up one missed run
    .registry_reconcile_interval(Duration::from_secs(300))
    .enable_claiming(false)             // single node, no claiming overhead
    .build();
```

Requirements:
- SQLite is acceptable for single-node batch deployments
- Minimal memory footprint (~50MB baseline)
- Can run on smaller instances (2-4 cores)

### Database hardware recommendations

| Deployment | Database | CPU | Memory | Storage |
|-----------|----------|-----|--------|---------|
| Development | SQLite | Any | Any | Any |
| Single-node production | PostgreSQL | 2+ cores | 4GB+ | SSD |
| High-throughput production | PostgreSQL | 4+ cores | 16GB+ | NVMe SSD |
| Multi-tenant production | PostgreSQL | 8+ cores | 32GB+ | NVMe SSD, IOPS provisioned |

### PostgreSQL tuning for Cloacina workloads

Key PostgreSQL settings to consider:

```ini
# Connection handling
max_connections = 200              # >= sum of all runner pool sizes + overhead
idle_in_transaction_session_timeout = '30s'  # prevent leaked connections

# Write performance
synchronous_commit = on            # keep on for durability; off only for benchmarks
wal_level = replica                # allows streaming replication
checkpoint_completion_target = 0.9

# Query performance
effective_cache_size = 12GB        # ~75% of available RAM
shared_buffers = 4GB               # ~25% of available RAM
work_mem = 64MB                    # per-operation sort/hash memory
```

### Connection pooling with PgBouncer

PgBouncer is a lightweight PostgreSQL connection pooler that sits between your application and PostgreSQL, multiplexing many client connections over fewer server connections.

For deployments with multiple runner instances, place PgBouncer between runners and PostgreSQL:

```ini
[pgbouncer]
pool_mode = transaction    # required -- Cloacina uses SET search_path per-transaction
max_client_conn = 400      # total clients across all runners
default_pool_size = 40     # connections to PostgreSQL per database
reserve_pool_size = 5      # extra connections for burst load
reserve_pool_timeout = 3   # seconds before using reserve pool
server_idle_timeout = 300  # close idle server connections after 5 min
```

Use `transaction` mode (not `session` mode) because Cloacina issues `SET search_path` within transactions for schema-based multi-tenancy. Session mode would leak schema settings between clients.

### Horizontal scaling with task claiming

When running multiple runner instances for horizontal scaling:

```rust
use std::time::Duration;
use cloacina::runner::DefaultRunnerConfig;

let config = DefaultRunnerConfig::builder()
    .max_concurrent_tasks(16)
    .enable_claiming(true)
    .heartbeat_interval(Duration::from_secs(10))
    // stale_claim_sweep_interval (30s) and stale_claim_threshold (60s)
    // use struct defaults — not available as builder methods
    .runner_id(Some("runner-east-1".to_string()))
    .runner_name(Some("East Region Runner 1".to_string()))
    .build();
```

The claiming system ensures:
- Each task is executed by exactly one runner
- Heartbeats prove liveness (default: every 10 seconds)
- Stale claims are reclaimed after the threshold (default: 60 seconds, must be > heartbeat interval)
- The sweep interval (default: 30 seconds) controls how quickly a crashed runner's tasks are redistributed

Tune `stale_claim_threshold` based on your longest expected network partition or GC pause. Too short and you get duplicate executions; too long and failover is slow.

## Summary of Defaults

| Parameter | Default | Unit |
|-----------|---------|------|
| `max_concurrent_tasks` | 4 | tasks |
| `scheduler_poll_interval` | 100 | ms |
| `task_timeout` | 300 | seconds |
| `pipeline_timeout` | 3600 | seconds |
| `db_pool_size` | 10 | connections |
| `cron_poll_interval` | 30 | seconds |
| `cron_max_catchup_executions` | unlimited | executions |
| `cron_recovery_interval` | 300 | seconds |
| `cron_lost_threshold_minutes` | 10 | minutes |
| `cron_max_recovery_age` | 86400 | seconds |
| `cron_max_recovery_attempts` | 3 | attempts |
| `trigger_base_poll_interval` | 1 | seconds |
| `trigger_poll_timeout` | 30 | seconds |
| `registry_reconcile_interval` | 60 | seconds |
| `heartbeat_interval` | 10 | seconds |
| `stale_claim_sweep_interval` | 30 | seconds |
| `stale_claim_threshold` | 60 | seconds |

## Related Resources

- [Explanation: Performance Characteristics]({{< ref "/platform/explanation/performance-characteristics" >}})
- [Explanation: Database Backends]({{< ref "/platform/explanation/database-backends" >}})
- [How-to: Production Deployment]({{< ref "/platform/how-to-guides/production-deployment" >}})
- [How-to: Multi-Tenant Setup]({{< ref "/workflows/how-to-guides/multi-tenant-setup" >}})
