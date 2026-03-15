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
3. The `ContinuousScheduler` observes detector completions via the `ExecutionLedger`
4. Boundaries are routed to per-edge `SignalAccumulator`s
5. When a `TriggerPolicy` fires, the accumulator drains and coalesces boundaries
6. The coalesced boundary context is merged and submitted to the `TaskScheduler`
7. The continuous task executes with boundary context
8. Task completion is recorded in the `ExecutionLedger`

## Key Concepts

### ComputationBoundary

A serializable message describing what slice of data a signal covers. Five kinds:

| Kind | Coalescing | Use Case |
|------|-----------|----------|
| `TimeRange` | min(starts)..max(ends) | Airflow-style time intervals |
| `OffsetRange` | min(starts)..max(ends) | Kafka partition offsets |
| `Cursor` | Latest wins | Opaque resume tokens |
| `FullState` | Latest wins | Entire dataset changes (version hash) |
| `Custom` | User-defined | Domain-specific boundaries |

Boundaries are advisory — the framework carries them, tasks scope their own work.

### DataSource and DataConnection

A `DataSource` is a named handle to an external dataset with:
- A `DataConnection` trait implementation for connecting to the system
- A detector workflow reference for change detection
- Lineage metadata

Tasks access data sources via `DataSourceMap`, which provides typed connection handles.

### SignalAccumulator

Per-edge stateful component that buffers boundaries and decides when to fire:

- `receive(boundary)` — buffer incoming boundaries
- `is_ready()` — check if the trigger policy says "fire now"
- `drain()` — coalesce buffered boundaries into a partial context fragment

The `SimpleAccumulator` preset fires based on an injected `TriggerPolicy` without watermark awareness.

### TriggerPolicy

Controls when an accumulator fires. Framework provides:

- `Immediate` — fire on every boundary
- `WallClockWindow` — fire after wall clock duration since last drain

Custom policies implement the `TriggerPolicy` trait.

### ExecutionLedger

In-memory append-only log recording all graph activity. The scheduler writes to it; observers scan from cursors. Events include `TaskCompleted`, `TaskFailed`, `BoundaryEmitted`, and `AccumulatorDrained`.

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

## Graph Assembly

The graph emerges implicitly from declarations:
1. `DataSource` registrations (name, connection, detector workflow)
2. `#[continuous_task]` declarations (which sources trigger, which are referenced)

At startup, `assemble_graph()` creates `GraphEdge`s for each task × triggering source pair, validates references, and constructs the `DataSourceGraph`.

## Configuration

Enable continuous scheduling via `DefaultRunnerConfig`:

```rust
let config = DefaultRunnerConfig::builder()
    .enable_continuous_scheduling(true)
    .continuous_poll_interval(Duration::from_millis(100))
    .build();
```

## Context Keys

The accumulator injects these keys into the task's context on drain:

| Key | Type | Description |
|-----|------|-------------|
| `__boundary` | JSON | The coalesced boundary (kind + metadata) |
| `__signals_coalesced` | integer | Number of raw signals merged into this boundary |
| `__accumulator_lag_ms` | integer | Max ingestion lag across buffered boundaries |

## Watermarks and Late Arrival

The system uses a **two-watermark model**:

- **Source watermarks** (on `BoundaryLedger`): per data source, "nothing earlier will arrive" — a user assertion from the detector
- **Consumer watermarks** (on each accumulator): "I've processed up to here" — updated on each drain

The `WindowedAccumulator` checks source watermarks before firing:

```rust
// WaitForWatermark mode: blocks until source confirms data completeness
let acc = WindowedAccumulator::new(
    Box::new(WallClockWindow::new(Duration::from_secs(3600))),
    WatermarkMode::WaitForWatermark,
    boundary_ledger.clone(),
    "raw_events".into(),
);
```

When a boundary arrives behind a consumer watermark, the **per-edge `LateArrivalPolicy`** determines the behavior: `Discard`, `AccumulateForward`, `Retrigger`, or `RouteToSideChannel`.

## Derived Data Sources

`LedgerTrigger` watches the `ExecutionLedger` for task completions and fires detector workflows for derived data sources — completing the reactive feedback loop without explicit wiring.

## Limitations

- **Postgres only** — continuous scheduling requires LISTEN/NOTIFY capabilities
- **No accumulator persistence** — in-memory only, lost on restart (deferred to I-0025)
- **No TriggerPolicy composition** — Any/All combinators for policies (deferred to I-0025)
- **No Python support** — Rust-first, Python continuous tasks in I-0026
