---
id: core-continuous-scheduling-minimum
level: initiative
title: "Core Continuous Scheduling — Minimum Viable Reactive Graph"
short_code: "CLOACI-I-0023"
created_at: 2026-03-13T02:44:38.132491+00:00
updated_at: 2026-03-16T01:08:21.245061+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: core-continuous-scheduling-minimum
---

# Core Continuous Scheduling — Minimum Viable Reactive Graph

## Context

Cloacina supports event-driven workflow execution: triggers fire, DAGs run, results are stored. CLOACI-S-0001 specifies a second scheduling mode — continuous reactive scheduling — where a persistent graph of compute tasks reacts automatically to data changes, re-executing only the affected subgraph.

This initiative delivers the minimum viable end-to-end path: a developer can define data sources, write detector workflows, wire up continuous tasks, and have the graph react to external data changes. Sophisticated features (watermarks, late arrival handling, derived data sources via LedgerTrigger) are deferred to CLOACI-I-0024.

**Specification**: CLOACI-S-0001 and sub-specs S-0002 through S-0008.

## Goals & Non-Goals

**Goals:**
- Implement all foundation types: `ComputationBoundary`, `BoundaryKind`, `DataSource`, `DataConnection`, `DataSourceMap`, `DetectorOutput`
- Implement `SignalAccumulator` trait and `SimpleAccumulator` preset
- Implement `TriggerPolicy` trait with `Immediate` and `WallClockWindow` policies
- Implement `ExecutionLedger` (in-memory append-only log)
- Implement `DataSourceGraph` and `ContinuousScheduler` run loop
- Implement `#[continuous_task]` proc macro with `DataSourceMap` parameter injection
- Integrate `ContinuousScheduler` with existing `TaskScheduler` / `Dispatcher` / `Executor` pipeline
- Support cron-triggered detector workflows (using existing `CronScheduler`)
- Deliver a working end-to-end example: external data source → detector → accumulator → continuous task

**Non-Goals:**
- Watermark system and `BoundaryLedger` (CLOACI-I-0024)
- `WindowedAccumulator` with watermark awareness (CLOACI-I-0024)
- Late arrival detection and `LateArrivalPolicy` (CLOACI-I-0024)
- `LedgerTrigger` and derived data sources (CLOACI-I-0024)
- Accumulator persistence / persist-on-drain (CLOACI-I-0025)
- Custom boundary schema registration and validation (CLOACI-I-0025)
- `Any`/`All` composition for `TriggerPolicy` (CLOACI-I-0025)
- Framework-provided `DataConnection` impls beyond a basic Postgres one (CLOACI-I-0025)
- Python/Cloaca support (CLOACI-I-0026)
- SQLite backend (Postgres only — see S-0001 decisions log)

## Architecture

### Overview

The `ContinuousScheduler` is a new top-level component, peer to `TriggerScheduler`, feeding work into the existing `TaskScheduler` → `Dispatcher` → `Executor` pipeline.

```
┌─────────────────────────────────────────────────┐
│                    Runner                        │
│  ┌────────────────┐  ┌──────────────────────┐   │
│  │TriggerScheduler│  │ContinuousScheduler   │   │
│  │(event-driven)  │  │(reactive graph)      │   │
│  └───────┬────────┘  └──────────┬───────────┘   │
│          │                      │                │
│          ▼                      ▼                │
│  ┌──────────────────────────────────────┐       │
│  │          TaskScheduler               │       │
│  │  (dependency resolution, state)      │       │
│  └─────────────────┬────────────────────┘       │
│                    ▼                             │
│  ┌──────────────────────────────────────┐       │
│  │      Dispatcher / Executors          │       │
│  └──────────────────────────────────────┘       │
└─────────────────────────────────────────────────┘
```

### Data Flow (This Initiative)

```
External Data Source
       │
       │ CronTrigger fires detector workflow
       ▼
Detector Workflow (existing task execution)
       │
       │ writes DetectorOutput to output context
       ▼
ContinuousScheduler observes completion via ExecutionLedger
       │
       │ routes ComputationBoundary to accumulator
       ▼
SignalAccumulator.receive(boundary)
       │
       │ TriggerPolicy.should_fire() → true
       ▼
accumulator.drain() → partial Context
       │
       │ merged with upstream context by ContextManager
       ▼
TaskScheduler → Dispatcher → Executor
       │
       │ continuous task runs with DataSourceMap + boundary context
       ▼
LedgerEvent::TaskCompleted written to ExecutionLedger
```

### Key Integration Points

| New Component | Integrates With | How |
|---|---|---|
| `ContinuousScheduler` | `TaskScheduler` | Submits work via `schedule_workflow_execution()` |
| `ContinuousScheduler` | `TriggerScheduler` | Detector workflows are scheduled by existing cron triggers |
| `ContinuousScheduler` | `ExecutionLedger` | Observes detector completions, writes task completion events |
| `#[continuous_task]` | `Dispatcher`/`TaskExecutor` | New execution path that injects `DataSourceMap` |
| `DataSourceMap` | `ContextManager` | Accumulator `drain()` output merged via existing context pipeline |

## Detailed Design

### New Types (S-0002, S-0003, S-0004)

All types as specified in the sub-specifications:
- `ComputationBoundary` { kind: `BoundaryKind`, metadata, emitted_at } with coalescing rules per variant
- `BoundaryKind` enum: `TimeRange`, `OffsetRange`, `Cursor`, `FullState`, `Custom`
- `DataSource` { name, connection, detector_workflow, lineage }
- `DataConnection` trait: `connect() → Box<dyn Any>`, `descriptor()`, `system_metadata()`
- `DataSourceMap` with typed `connection<T>()` helper returning `GraphError::ConnectionTypeMismatch` on mismatch
- `DetectorOutput` enum: `Change`, `WatermarkAdvance`, `Both`
- `ConnectionDescriptor` { system_type, location }
- `BufferedBoundary` { boundary, received_at }

### SignalAccumulator & TriggerPolicy (S-0005, partial)

Traits as specified. This initiative delivers:
- `SignalAccumulator` trait: `receive()`, `is_ready()`, `drain()`, `metrics()`, `consumer_watermark()`
- `SimpleAccumulator` — no watermark awareness, fires based on trigger policy alone
- `TriggerPolicy` trait: `should_fire(&[BufferedBoundary]) -> bool`
- `Immediate` policy — fires on every boundary
- `WallClockWindow` policy — fires after wall clock duration since last drain

`WindowedAccumulator` (watermark-aware) deferred to CLOACI-I-0024.

### ExecutionLedger (S-0007, partial)

In-memory append-only log as specified:
- `LedgerEvent` enum: `TaskCompleted`, `TaskFailed`, `BoundaryEmitted`, `AccumulatorDrained`
- Cursor-based scanning for observers
- No `LedgerTrigger` in this initiative (deferred to CLOACI-I-0024)

### ContinuousScheduler & Graph (S-0008)

- `DataSourceGraph` { data_sources, tasks, edges } assembled from task/data source registrations
- `GraphEdge` { source, task, accumulator } — `late_arrival_policy` field present but only `AccumulateForward` supported in this initiative
- `ContinuousTaskConfig` { triggered_edges, referenced_sources, join_mode } — `JoinMode::Any` only in this initiative
- `ContinuousScheduler` run loop steps 1-5 from S-0008, simplified (no watermark checks, no late arrival routing)
- Exit edges (continuous task → one-shot event-driven DAG) supported

### `#[continuous_task]` Macro

New proc macro, separate from `#[task]`:

```rust
#[continuous_task(
    id = "aggregate_hourly",
    sources = ["raw_events"],           // triggering data sources
    referenced = ["config_table"],      // available but not triggering
)]
async fn aggregate_hourly(
    ctx: &mut Context<Value>,
    inputs: &DataSourceMap,
) -> Result<(), TaskError> {
    let pool = inputs.connection::<PgPool>("raw_events")?;
    let boundary = ctx.get("__boundary")?;
    // ... query within boundary, write results ...
}
```

The macro generates registration code that the framework uses to assemble the `DataSourceGraph` at startup. The `sources` attribute defines triggering edges; `referenced` defines non-triggering data source access.

### Graph Assembly

The graph is not explicitly built by the user. It emerges from:
1. `DataSource` registrations (name, connection, detector workflow)
2. `#[continuous_task]` declarations (which sources trigger, which are referenced)

At startup, the framework:
1. Collects all registered data sources and continuous tasks
2. Creates `GraphEdge` for each task × triggering source pair
3. Validates: no cycles, all referenced sources exist, all detector workflows exist
4. Constructs `DataSourceGraph` and passes to `ContinuousScheduler`

## Alternatives Considered

- **Extend `#[task]` macro instead of new `#[continuous_task]`**: Rejected — continuous tasks have fundamentally different semantics (persistent graph, DataSourceMap injection, no explicit dependencies). An explicit separate macro prevents confusion.
- **Builder API for graph construction**: Rejected — the graph is implicit in the task and data source declarations. "I watch these sources" + "I emit here" is the natural mental model. No separate wiring step needed.
- **Include watermarks in this initiative**: Rejected — watermarks add significant complexity (BoundaryLedger, WindowedAccumulator, late arrival). The core reactive loop works without them for many use cases. Better to ship a working MVP first.

## Implementation Plan

### Phase 1: Foundation Types
- [ ] `ComputationBoundary`, `BoundaryKind`, coalescing logic
- [ ] `DataSource`, `DataConnection` trait, `ConnectionDescriptor`
- [ ] `DataSourceMap` with typed `connection<T>()` helper
- [ ] `DetectorOutput` enum
- [ ] `BufferedBoundary`
- [ ] Basic `PostgresConnection` implementation
- [ ] Unit tests for all types and coalescing rules

### Phase 2: Accumulation Layer
- [ ] `SignalAccumulator` trait
- [ ] `TriggerPolicy` trait
- [ ] `SimpleAccumulator` implementation
- [ ] `Immediate` and `WallClockWindow` policy implementations
- [ ] `AccumulatorMetrics`
- [ ] Unit tests for accumulator behavior and policy firing

### Phase 3: ExecutionLedger
- [ ] `ExecutionLedger` struct with append-only `Vec<LedgerEvent>`
- [ ] `LedgerEvent` enum
- [ ] Cursor-based `events_since()` scanning
- [ ] Unit tests

### Phase 4: `#[continuous_task]` Macro
- [ ] Proc macro implementation with `sources` and `referenced` attributes
- [ ] Registration code generation for graph assembly
- [ ] `DataSourceMap` injection into task execution path
- [ ] Extend `Dispatcher`/`TaskExecutor` to handle continuous task execution
- [ ] Macro validation tests

### Phase 5: ContinuousScheduler & Graph Assembly
- [ ] `DataSourceGraph` and `GraphEdge` structs
- [ ] `ContinuousTaskConfig` and `JoinMode`
- [ ] Graph assembly from registered data sources and continuous tasks
- [ ] Graph validation (cycles, missing references)
- [ ] `ContinuousScheduler` struct and run loop
- [ ] Integration with `TaskScheduler` for work submission
- [ ] Completion callback → `ExecutionLedger` write
- [ ] Exit edge support (continuous → event-driven)

### Phase 6: Integration & Example
- [ ] Wire `ContinuousScheduler` into `Runner` alongside `TriggerScheduler`
- [ ] End-to-end integration test: Postgres data source → cron detector → accumulator → continuous task
- [ ] Example project demonstrating continuous scheduling
- [ ] Documentation for continuous scheduling concepts and usage
