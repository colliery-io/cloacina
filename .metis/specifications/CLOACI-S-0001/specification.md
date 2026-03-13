---
id: continuous-reactive-scheduling
level: specification
title: "Continuous Reactive Scheduling - Data-Driven DAG Execution"
short_code: "CLOACI-S-0001"
created_at: 2026-03-10T13:51:40.473786+00:00
updated_at: 2026-03-10T13:51:40.473786+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Continuous Reactive Scheduling - Data-Driven DAG Execution

## Overview

Cloacina currently supports event-driven workflow execution: triggers fire, DAGs run, results are stored. This specification defines a second scheduling mode — **continuous reactive scheduling** — where a persistent graph of compute tasks reacts automatically to data changes, re-executing only the affected subgraph.

The model is analogous to `make` but applied to a live, continuously-running computation graph. Rather than rebuilding files based on timestamps, Cloacina would re-execute tasks based on upstream data source changes, using callback-based change detection and configurable coalescing policies.

**Positioning**: This targets teams currently using Flink, Spark Streaming, or similar stream processors purely for micro-batch aggregation windows. Those teams have accepted minute-scale latency but pay the full operational cost of a distributed stream processing cluster. Continuous scheduling provides the same reactive semantics with Cloacina's existing task execution model — no new runtime, no JVM, no state backends.

### Dual-Phase Model

Cloacina becomes a dual-phase orchestrator:

- **Event-driven** (existing): Triggers fire one-shot workflow executions. DAGs run to completion and stop.
- **Continuous** (new): A persistent reactive graph watches data sources. When data changes, affected tasks re-execute automatically. The graph is always "live."

Both modes coexist in the same runner, same tenant, same graph. Tasks in the continuous graph can fire one-shot event-driven DAGs via "exit edges," and output data sources provide re-entrant paths back into the continuous graph.

## Component Specifications

Detailed design for each component lives in its own specification document:

| Spec | Component | Summary |
|------|-----------|---------|
| CLOACI-S-0002 | **ComputationBoundary** | Data slice description — struct+enum, coalescing rules, custom schema enforcement, backpressure observability |
| CLOACI-S-0003 | **DataSource** | External dataset handle — DataConnection trait, ConnectionDescriptor, lineage, framework-provided impls |
| CLOACI-S-0004 | **Detector Workflows** | Change detection contract — DetectorOutput, detector-as-workflow pattern, triggering modes |
| CLOACI-S-0005 | **SignalAccumulator + TriggerPolicy** | Buffering and firing — accumulator trait, trigger policy trait, Any/All composition, presets |
| CLOACI-S-0006 | **Watermark System** | Data completeness — BoundaryLedger, consumer watermarks, late arrival policy, two-watermark model |
| CLOACI-S-0007 | **ExecutionLedger + LedgerTrigger** | Operational backbone — append-only ledger, LedgerTrigger for derived data sources, cursor semantics |
| CLOACI-S-0008 | **Graph Topology + ContinuousScheduler** | Scheduling structure — DataSourceGraph, GraphEdge, JoinMode, run loop, context as data plane |

## Architecture

The `ContinuousScheduler` is a new top-level component, peer to `TriggerScheduler`, feeding work into the existing `TaskScheduler` → `Dispatcher` → `Executor` pipeline.

```
┌─────────────────────────────────────────────┐
│                  Runner                      │
│  ┌───────────────┐  ┌────────────────────┐  │
│  │TriggerScheduler│  │ContinuousScheduler │  │
│  │ (event-driven) │  │ (reactive/make)    │  │
│  └───────┬────────┘  └────────┬───────────┘  │
│          │                    │               │
│          ▼                    ▼               │
│  ┌────────────────────────────────────┐      │
│  │         TaskScheduler              │      │
│  │   (dependency resolution, state)   │      │
│  └──────────────┬─────────────────────┘      │
│                 ▼                             │
│  ┌────────────────────────────────────┐      │
│  │     Dispatcher / Executors         │      │
│  └────────────────────────────────────┘      │
└─────────────────────────────────────────────┘
```

### C4 Context — System Boundary

```
                  ┌─────────────┐
                  │  Developer  │
                  │             │
                  │  Defines    │
                  │  graph,     │
                  │  tasks,     │
                  │  detectors  │
                  └──────┬──────┘
                         │
                         │  registers graph
                         │
                         ▼
┌──────────────────────────────────────────────────────────┐
│                                                          │
│                   Cloacina Runner                         │
│                                                          │
│   ┌────────────────────┐    ┌─────────────────────────┐  │
│   │  Event-Driven      │◄───│  Continuous Reactive     │  │
│   │  Scheduling        │    │  Scheduling              │  │
│   │  (existing)        │    │  (new)                   │  │
│   └────────────────────┘    └──────────┬──────────────┘  │
│              ▲                         │                  │
│              │  exit edges             │  detector        │
│              │  fire one-shot DAGs     │  workflows       │
│              └─────────────────────────┘  poll via        │
│                                          DataConnection   │
│                                                │         │
└────────────────────────────────────────────────┼─────────┘
                                                 │
                         polls via               │
                         .connect()              │
                                                 ▼
                  ┌──────────────────────────────────┐
                  │       External Data Sources       │
                  │                                   │
                  │  Postgres, Kafka, S3, APIs, etc.  │
                  │                                   │
                  └──────────────────────────────────┘
```

### C4 Component — Continuous Scheduling Internals

```
                  ┌──────────────────────────────────┐
                  │       External Data Sources       │
                  │  Postgres, Kafka, S3, APIs, etc.  │
                  └──────────────────▲───────────────┘
                                     │
                                     │ detector workflows poll
                                     │ via DataConnection.connect()
                                     │
┌────────────────────────────────────┼────────────────────────────────────────────┐
│                         Cloacina Runner                                          │
│                                    │                                             │
│  ┌─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┼─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐   │
│    Detection Layer                 │                                         │   │
│  │                                 │                                         │   │
│     ┌──────────────────────────────┼─────┐    ┌────────────────────────────┐  │   │
│  │  │  TriggerScheduler (existing) │     │    │  Detector Workflows        │ ││  │
│     │                              │     │    │  (S-0004)                  │  │   │
│  │  │  Fires detector workflows:   │     ├───►│                            │ ││  │
│     │                              │     │    │  Regular Cloacina workflows│  │   │
│  │  │  - CronTrigger (interval)    │     │  ┌─│  that poll external data   │ ││  │
│     │  - LedgerTrigger (S-0007) ◄──┼─────┼──┘ │  via DataConnection and    │  │   │
│  │  │                              │     │    │  produce DetectorOutput    │ ││  │
│     └──────────────────────────────┘     │    │  (Change / Watermark /    │  │   │
│  │                                       │    │   Both)                   │ ││  │
│                                          │    └─────────────┬──────────────┘  │   │
│  └─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┼─ ─ ─ ─ ─ ─ ─ ─┼─ ─ ─ ─ ─ ─ ┘   │
│                                          │                 │                    │
│                                          │                 │ writes             │
│                                          │                 │ DetectorOutput     │
│                                          │                 ▼                    │
│  ┌───────────────────────────────────────┼─────────────────────────────────┐   │
│  │ ContinuousScheduler (S-0008)          │                                 │   │
│  │                                       │                                 │   │
│  │  ┌────────────────────────┐    ┌──────┴──────────────────┐              │   │
│  │  │  BoundaryLedger        │    │  ExecutionLedger         │              │   │
│  │  │  (S-0006)              │    │  (S-0007)                │              │   │
│  │  │                        │    │                          │              │   │
│  │  │  Source watermarks     │    │  Append-only log:        │              │   │
│  │  │  per data source       │    │  - TaskCompleted         │──────┐       │   │
│  │  │                        │    │  - TaskFailed            │      │       │   │
│  │  └───────────┬────────────┘    │  - BoundaryEmitted       │      │       │   │
│  │              │                 │  - AccumulatorDrained     │      │       │   │
│  │              │ covers()?       └──────────────────────────┘      │       │   │
│  │              │                                                   │ reads │   │
│  │              ▼                                                   │ events│   │
│  │  ┌───────────────────────────────────────────────────────┐      │       │   │
│  │  │  Per-Edge Components                                   │      │       │   │
│  │  │                                                        │      │       │   │
│  │  │  ┌─────────────────┐  ┌──────────────┐  ┌──────────┐ │      │       │   │
│  │  │  │ SignalAccumulator│  │ TriggerPolicy │  │ LateArr. │ │      │       │   │
│  │  │  │ (S-0005)        │  │ (S-0005)      │  │ Policy   │ │      │       │   │
│  │  │  │                 │  │               │  │ (S-0006) │ │      │       │   │
│  │  │  │ receive()       │  │ should_fire() │  │          │ │      │       │   │
│  │  │  │ is_ready() ────►│  │ Any/All       │  │ Discard  │ │      │       │   │
│  │  │  │ drain()         │  │ composable    │  │ Forward  │ │      │       │   │
│  │  │  │ consumer_wm()   │  │               │  │ Retrigger│ │      │       │   │
│  │  │  └────────┬────────┘  └──────────────┘  │ SideChan │ │      │       │   │
│  │  │           │                              └──────────┘ │      │       │   │
│  │  └───────────┼───────────────────────────────────────────┘      │       │   │
│  │              │                                                   │       │   │
│  │              │ drain() → partial context                         │       │   │
│  │              ▼                                                   │       │   │
│  │  ┌────────────────────────────────────────┐                     │       │   │
│  │  │  DataSourceGraph (S-0008)              │                     │       │   │
│  │  │                                        │                     │       │   │
│  │  │  DataSources ──► GraphEdges ──► Tasks  │                     │       │   │
│  │  │       │              │           │     │                     │       │   │
│  │  │  connection      accumulator   join    │                     │       │   │
│  │  │  descriptor      late_arrival  mode    │                     │       │   │
│  │  │  detector_wf     policy        Any/All │                     │       │   │
│  │  └────────────────────────────────────────┘                     │       │   │
│  │                                                                  │       │   │
│  └──────────────────────────────┬───────────────────────────────────┘       │   │
│                                 │                                   ▲       │   │
│                                 │ submit task                       │LedgerTrigger
│                                 │ + exit edges                      │reads  │   │
│                                 ▼                                   │ledger │   │
│  ┌─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┼─ ─ ─ ┘   │
│    Existing Execution Infrastructure                                │          │
│  │                                                                  │         ││
│     ┌─────────────────────────────────────┐                         │          │
│  │  │  ContextManager                     │                         │         ││
│     │  merges partial ctx + upstream ctx   │                         │          │
│  │  └──────────────────┬──────────────────┘                         │         ││
│                        ▼                                            │          │
│  │  ┌─────────────────────────────────────┐                         │         ││
│     │  TaskScheduler                      │                         │          │
│  │  │  dependency resolution, state mgmt  │                         │         ││
│     └──────────────────┬──────────────────┘                         │          │
│  │                     ▼                                            │         ││
│     ┌─────────────────────────────────────┐   completion callback   │          │
│  │  │  Dispatcher / Executors             ├─────────────────────────┘         ││
│     │  task execution, routing            │                                    │
│  │  └─────────────────────────────────────┘                                  ││
│  └─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘ │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Reactive Feedback Loop

The core "make" reactor — the cycle that keeps the graph alive:

```
  ┌──────────────────────────────────────────────────────────────────┐
  │                                                                  │
  │   Task completes                                                 │
  │       │                                                          │
  │       ▼                                                          │
  │   LedgerEvent::TaskCompleted ──► ExecutionLedger                 │
  │                                       │                          │
  │                                       ▼                          │
  │                                  LedgerTrigger fires             │
  │                                       │                          │
  │                                       ▼                          │
  │                              Detector workflow runs              │
  │                                       │                          │
  │                                       ▼                          │
  │                              DetectorOutput emitted              │
  │                              (Change + Watermark)                │
  │                                       │                          │
  │                          ┌────────────┴────────────┐             │
  │                          ▼                         ▼             │
  │                   BoundaryLedger            Accumulator           │
  │                   (watermark advance)       .receive(boundary)   │
  │                                                    │             │
  │                                                    ▼             │
  │                                            TriggerPolicy         │
  │                                            should_fire()?        │
  │                                                    │             │
  │                                              ┌─────┴─────┐      │
  │                                              │ yes        │ no   │
  │                                              ▼            ▼      │
  │                                        drain() ──►  wait for     │
  │                                           │        more signals  │
  │                                           ▼                      │
  │                                     Submit task ─────────────────┘
  │                                                                  │
  └──────────────────────────────────────────────────────────────────┘
```

See CLOACI-S-0008 for the full `ContinuousScheduler` struct, `DataSourceGraph`, `GraphEdge`, `JoinMode`, run loop, and context data plane details.

## New Types Summary

| Type | Spec | Purpose |
|---|---|---|
| `ComputationBoundary` / `BoundaryKind` | S-0002 | Data slice description (struct + enum) |
| `BufferedBoundary` | S-0002 | Boundary + receipt timestamp (backpressure) |
| `DataSource` / `DataConnection` | S-0003 | External dataset handle with connection trait |
| `DetectorOutput` | S-0004 | Detector emission (change, watermark, both) |
| `SignalAccumulator` (trait) | S-0005 | Per-edge buffer — decides when to fire, produces partial context |
| `TriggerPolicy` (trait) | S-0005 | When to fire — composable via Any/All |
| `BoundaryLedger` | S-0006 | Source watermark tracking |
| `LateArrivalPolicy` | S-0006 | Per-edge policy for late boundaries |
| `ExecutionLedger` / `LedgerEvent` | S-0007 | Append-only graph activity log |
| `DataSourceMap` | S-0003 | Typed accessor for task input connections |
| `LedgerTrigger` / `LedgerMatchMode` | S-0007 | Trigger impl for derived data sources |
| `ContinuousScheduler` | S-0008 | Top-level orchestrator (concrete struct) |
| `DataSourceGraph` / `GraphEdge` | S-0008 | Graph structure: sources → accumulators → tasks |
| `ContinuousTaskConfig` / `JoinMode` | S-0008 | Per-task config: triggered edges, Any/All join |

## Existing Cloacina Infrastructure Leveraged

| Component | Role in Continuous Scheduling |
|---|---|
| `WorkflowGraph` (petgraph) | Graph algorithms: `get_dependents()`, `topological_sort()`, `find_parallel_groups()` |
| `Workflow::subgraph()` | Build minimal re-execution subgraphs from affected nodes |
| `Context<Value>` + `ContextManager` | Universal data transport, database persistence, fan-in merging |
| `TaskScheduler` | Dependency resolution and task state management |
| `Dispatcher` / `TaskExecutor` | Actual task execution, routing, capacity management |
| `WorkDistributor` (LISTEN/NOTIFY) | Reactive work signaling for executors |
| `TriggerRule` / `TriggerCondition` | Conditional execution logic (reused for multi-input join semantics) |
| Content-based versioning | Detect graph structure changes between deployments |

## Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| `connect()` return type | `Box<dyn Any>` with typed `DataSourceMap::connection<T>()` helper | Generic trait can't be stored in heterogeneous graph. Typed helper gives clean ergonomics with clear error on wiring mismatch. See S-0003. |
| Continuous task macro | New `#[continuous_task]` macro, separate from `#[task]` | Explicit distinction. Continuous tasks receive `DataSourceMap` as an additional parameter. Different execution semantics warrant a separate macro. |
| DataSource connections | May need DB-persisted connection configs (like Airflow Connections) | `DataSourceMap` injection needs connection details at runtime. DB storage enables runtime management without code changes. Exact persistence model TBD during implementation. |
| Backend support | Postgres only | Continuous scheduling relies on high-throughput coordination. SQLite lacks LISTEN/NOTIFY and the concurrency characteristics needed. |
| Python/Cloaca support | Yes, but second phase | Rust-first implementation. Python tasks and detectors in the continuous graph are a follow-on initiative after the core is stable. |
| Lifecycle management | Pause/resume at detector level | Operators control the graph by enabling/disabling detector triggers. No graph-level pause/resume — detector control is sufficient granularity. |
| Testing | Simulated boundary emissions as starting point | Broader testing strategy conversation needed, but boundary emission simulation is the minimum viable testing capability. |
| Error handling | Deferred to implementation | Existing retry policy applies. Additional mechanisms (circuit breakers, max accumulator depth) to be evaluated during implementation. |
| Accumulator persistence | Hybrid: in-memory with persist-on-drain | Hot path (receive) stays in-memory. On drain, persist consumer watermark and metadata. On restart, load watermarks; detectors re-poll from their own persisted state. Re-processing window bounded by trigger policy interval; coalescing makes it idempotent. See S-0005. |
| Graph migration | Restart with watermark resume | No diffing or explicit migration. New process loads new graph definition, matches persisted consumer watermarks by edge ID, ignores orphans. Detectors resume from their own persisted `__last_known_state`. Correctness is automatic; orphaned state is harmless. |
| Orphaned state cleanup | Administrative prune command | No auto-deletion of orphaned watermarks/edges. Startup warns about unmatched persisted state. Operators prune via REST API endpoint, CLI, or ctl command. Matches existing pattern of explicit operator control over destructive actions. |
