---
title: "Continuous Reactive Scheduling"
description: "Data-driven DAG execution where a persistent graph reacts to data changes"
weight: 15
reviewer: "dstorey"
review_date: "2026-03-15"
---

## Overview

Cloacina supports two scheduling modes:

- **Event-driven** (existing): Triggers fire one-shot workflow executions. DAGs run to completion and stop.
- **Continuous** (new): A persistent reactive graph watches data sources. When data changes, affected tasks re-execute automatically.

Both modes coexist in the same runner. The continuous scheduler is analogous to `make` — instead of rebuilding files based on timestamps, it re-executes tasks based on upstream data source changes, using boundary-based change detection and configurable coalescing policies.

## Architecture

The `ContinuousScheduler` is a top-level component, peer to `TriggerScheduler`, feeding work into the existing `TaskScheduler` / `Dispatcher` / `Executor` pipeline:

```
                    Runner
  ┌────────────────┐  ┌──────────────────────┐
  │TriggerScheduler│  │ContinuousScheduler   │
  │(event-driven)  │  │(reactive graph)      │
  └───────┬────────┘  └──────────┬───────────┘
          │                      │
          ▼                      ▼
  ┌──────────────────────────────────────┐
  │          TaskScheduler               │
  │  (dependency resolution, state)      │
  └─────────────────┬────────────────────┘
                    ▼
  ┌──────────────────────────────────────┐
  │      Dispatcher / Executors          │
  └──────────────────────────────────────┘
```

## Data Flow

The reactive loop:

1. **Detector workflows** poll external data sources via `DataConnection.connect()`
2. Detectors produce `DetectorOutput` — boundaries describing what data changed
3. Detectors optionally write `__last_known_state` to their output context for crash recovery
4. The `ContinuousScheduler` observes detector completions via the `ExecutionLedger`
5. Boundaries are persisted to the WAL (write-ahead log) and routed to per-edge `SignalAccumulator`s
6. When a `TriggerPolicy` fires, the accumulator drains and coalesces boundaries
7. The coalesced boundary context is merged and the continuous task executes
8. On drain: consumer watermarks are persisted, edge drain cursors advanced, detector state committed when all consumers catch up
9. Task completion is recorded in the `ExecutionLedger`
10. `LedgerTrigger` observers wake up and fire derived data source detectors (feedback loop)

## Key Concepts

### ComputationBoundary

A serializable message describing what slice of data a signal covers. Five kinds:

| Kind | Coalescing | Use Case |
|------|-----------|----------|
| `TimeRange` | min(starts)..max(ends) | Airflow-style time intervals |
| `OffsetRange` | min(starts)..max(ends) | Kafka partition offsets |
| `Cursor` | Latest wins | Opaque resume tokens |
| `FullState` | Latest wins | Entire dataset changes (version hash) |
| `Custom` | Latest wins (schema-validated via `jsonschema` crate) | Domain-specific boundaries |

Boundaries are advisory — the framework carries them, tasks scope their own work. Mixed boundary kinds refuse to coalesce (logged as warning, returns None).

### DataSource and DataConnection

A `DataSource` is a named handle to an external dataset with:
- A `DataConnection` trait implementation for connecting to the system
- A detector workflow reference for change detection
- Lineage metadata

The framework provides `PostgresConnection` (with pool config), `KafkaConnection`, and `S3Connection`. Tasks access data sources via `DataSourceMap`, which provides typed connection handles.

### SignalAccumulator

Per-edge stateful component that buffers boundaries and decides when to fire:

- `receive(boundary)` — buffer incoming boundaries, returns `ReceiveResult` (Accepted or AcceptedWithDrop for backpressure)
- `is_ready()` — check if the trigger policy says "fire now"
- `try_drain()` — atomically check readiness and drain under a single lock (prevents TOCTOU races)
- `drain()` — coalesce buffered boundaries into a partial context fragment
- `metrics()` — O(1) cached metrics (buffered count, oldest/newest emitted_at, max lag)

Two implementations:
- `SimpleAccumulator` — fires based on `TriggerPolicy` alone, no watermark awareness
- `WindowedAccumulator` — adds watermark-gated readiness in `WaitForWatermark` mode

Both have configurable `max_buffer_size` (default 10,000) with drop-oldest eviction on overflow.

### TriggerPolicy

Controls when an accumulator fires. Framework provides:

| Policy | Fires when |
|--------|-----------|
| `Immediate` | Buffer non-empty |
| `BoundaryCount(n)` | N boundaries buffered |
| `WallClockWindow(d)` | Duration since last drain |
| `WallClockDebounce(d)` | No new boundaries for duration (silence = burst over) |
| `AnyPolicy(vec![...])` | Any sub-policy matches (OR combinator) |
| `AllPolicy(vec![...])` | All sub-policies match (AND combinator) |

Composites propagate `mark_drained()` to all sub-policies, so timing-based policies reset correctly in nested configurations.

### ExecutionLedger

Bounded in-memory log recording all graph activity with configurable eviction (default 100K events). Uses `VecDeque` with `base_offset` tracking so cursor-based scanning works across evictions. Emits `Notify` on every append for event-driven observers.

Events: `TaskCompleted`, `TaskFailed`, `BoundaryEmitted`, `AccumulatorDrained`.

### Graph Assembly and Validation

`assemble_graph()` validates and constructs the `DataSourceGraph`:
- Validates all referenced data sources exist
- Creates `GraphEdge` for each task x triggering source pair
- Performs cycle detection via Kahn's algorithm on data-flow dependencies
- Multiple data sources may share the same detector workflow (fan-out from one producer)

## Defining Continuous Tasks

Use the `#[continuous_task]` macro:

```rust
use cloacina::continuous_task;

#[continuous_task(
    id = "aggregate_hourly",
    sources = ["raw_events"],
    referenced = ["config_table"],
)]
async fn aggregate_hourly(
    ctx: &mut Context<Value>,
) -> Result<(), TaskError> {
    let boundary = ctx.get("__boundary").unwrap();
    // Query within the boundary range, write aggregated results
    Ok(())
}
```

- `sources` — data sources that trigger execution (create graph edges with accumulators)
- `referenced` — data sources available but not triggering (no accumulator, no edge)

## Context Keys

The accumulator injects these keys into the task's context on drain:

| Key | Type | Description |
|-----|------|-------------|
| `__boundary` | JSON | The coalesced boundary (kind + metadata) |
| `__signals_coalesced` | integer | Number of raw signals merged into this boundary |
| `__accumulator_lag_ms` | integer | Max ingestion lag across buffered boundaries |
| `__last_known_state` | JSON | Detector's persisted state (available if detector wrote it) |

## Watermarks and Late Arrival

The system uses a **two-watermark model**:

- **Source watermarks** (on `BoundaryLedger`): per data source, "nothing earlier will arrive" — asserted by the detector via `DetectorOutput::WatermarkAdvance`
- **Consumer watermarks** (on each accumulator): "I've processed up to here" — updated on each drain, persisted to DB

Watermarks enforce monotonicity: advancing backward or switching boundary kinds is rejected. The `WindowedAccumulator` in `WaitForWatermark` mode blocks until the source watermark covers the pending coalesced boundary.

When a boundary arrives behind a consumer watermark, the per-edge `LateArrivalPolicy` determines behavior:
- `Discard` — drop the late boundary
- `AccumulateForward` — buffer for next cycle (default)
- `Retrigger` — re-submit for re-execution

## Derived Data Sources

`LedgerTrigger` watches the `ExecutionLedger` for task completions and fires detector workflows for derived data sources — completing the reactive feedback loop without explicit wiring. Supports `Any` (fire on any watched task) and `All` (fire when all watched tasks completed) match modes.

The trigger subscribes to ledger `Notify` events for near-instant wake-up (5-second polling fallback for missed notifications).

## Crash Recovery

With a DAL configured, the scheduler provides full crash recovery via three persistence layers:

### Boundary WAL (Write-Ahead Log)

Per-source ordered log of boundaries with per-edge drain cursors (Kafka consumer group model):

- **On boundary routing**: 1 INSERT per source per boundary (O(1) regardless of fan-out)
- **On edge drain**: advance that edge's cursor
- **Cleanup**: delete boundaries where all edge cursors have advanced past them
- **On restart**: re-inject unconsumed boundaries into each edge's accumulator

### Consumer Watermarks

Per-edge watermarks persisted on drain (existing `accumulator_state` table). Restored on startup to enable correct late-arrival detection.

### Detector State

Per-source committed checkpoint with latest/committed split:

- Detectors write `__last_known_state` to their output context
- The scheduler tracks the latest state in memory
- On drain, the current latest is recorded per-edge
- The committed state advances only when ALL consumers for a source have drained (slowest consumer gates the commit)
- On restart, detectors read committed state to resume from the last fully-processed point

### Startup Sequence

```rust
scheduler.init_drain_cursors().await;         // 1. Init cursor tracking
scheduler.restore_pending_boundaries().await;  // 2. Re-inject un-consumed boundaries
scheduler.restore_from_persisted_state().await; // 3. Restore consumer watermarks
scheduler.restore_detector_state().await;      // 4. Load detector checkpoints
```

Order matters: boundaries must be in buffers before watermarks are set, otherwise restored boundaries would be classified as "late" and potentially discarded.

## Production Safety

### Task Execution Timeout

Configurable per-task timeout (default 5 minutes). Timed-out tasks are recorded as `TaskFailed` in the ledger and the scheduler continues processing.

### Backpressure

Accumulators have configurable `max_buffer_size` (default 10,000). When full, the oldest boundary is dropped and `ReceiveResult::AcceptedWithDrop` is returned with a warning log.

### Bounded Memory

- Ledger: configurable `max_events` (default 100K) with VecDeque eviction
- Accumulators: `max_buffer_size` per edge
- Fired tasks: `max_fired_tasks` (default 10K) trimmed per poll cycle

### Non-Poisoning Locks

All locks use `parking_lot::Mutex`/`RwLock` — a panic in user-supplied task code does not poison locks or cascade-crash the scheduler.

### Observability

All error paths log at `warn!` or `error!` level:
- Boundary validation rejections
- Persistence failures
- Watermark advance rejections
- Detector output deserialization failures

## Configuration

```rust
ContinuousSchedulerConfig {
    poll_interval: Duration::from_millis(100),    // How often to poll the ledger
    max_fired_tasks: 10_000,                       // Retain up to 10K fired task records
    task_timeout: Some(Duration::from_secs(300)),  // 5 minute task timeout
}
```

## Limitations

- **Postgres only** for persistence — continuous scheduling requires DB for crash recovery (SQLite supported for testing)
- **No Python support** — Rust-first, Python continuous tasks planned in CLOACI-I-0026
