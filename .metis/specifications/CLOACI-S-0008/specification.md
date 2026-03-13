---
id: graph-topology-datasourcegraph-and
level: specification
title: "Graph Topology - DataSourceGraph and Scheduling Structure"
short_code: "CLOACI-S-0008"
created_at: 2026-03-10T18:18:26.540197+00:00
updated_at: 2026-03-10T18:18:26.540197+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Graph Topology - DataSourceGraph and Scheduling Structure

*Component specification for CLOACI-S-0001 (Continuous Reactive Scheduling).*

## Overview

The graph topology defines the structure of the continuous reactive graph — how data sources, accumulators, and tasks are wired together. The `ContinuousScheduler` is the concrete orchestrator that runs the reactive loop over this graph.

## Single Graph Per Tenant

There is exactly one continuous graph per tenant. Disjoint subgraphs may exist due to no shared nodes, but they are part of the same logical graph. Cross-tenant data flows are handled via push events to infrastructure in another tenant — never linked in the graph directly (security and operational isolation).

## Edge Types

Three kinds of edges in the dual-phase model:

- **Continuous edges** (DataSource → Task → DataSource): Always live, reactive. The core of the continuous graph.
- **Exit edges** (Task → event-driven DAG): Task completion fires a traditional one-shot workflow execution. The continuous graph does not wait for it.
- **Entry edges** (external → DataSource): Change callbacks/channels that feed the graph from outside.

## Graph Structure Types

```rust
struct DataSourceGraph {
    data_sources: HashMap<String, DataSource>,
    tasks: HashMap<String, ContinuousTaskConfig>,
    edges: Vec<GraphEdge>,
}

struct GraphEdge {
    source: String,                               // data source name
    task: String,                                  // task name
    accumulator: Box<dyn SignalAccumulator>,
    late_arrival_policy: LateArrivalPolicy,
}

struct ContinuousTaskConfig {
    triggered_edges: Vec<usize>,                  // indices into edges vec
    referenced_sources: Vec<String>,              // data sources available but not triggering
    join_mode: JoinMode,                          // how to combine accumulator readiness
}

enum JoinMode {
    Any,    // fire when any accumulator is ready
    All,    // fire when all accumulators are ready
}
```

The `GraphEdge` is the unit of wiring — it binds a data source (CLOACI-S-0003) to a task through an accumulator (CLOACI-S-0005). Per-edge configuration (accumulator, late arrival policy) lives on the edge.

Referenced data sources (available to the task but not triggering execution) have no accumulator and no edge — they're listed in `ContinuousTaskConfig::referenced_sources` and injected into the task's `DataSourceMap` at execution time.

## Boundary Propagation

Boundaries propagate through the graph via accumulators, not automatically. The accumulator's `drain()` method decides whether and how to pass boundary information downstream via context.

The scheduler does not need to understand boundary semantics — it calls `coalesce()` and passes the result to the accumulator. The `contains()` method on `ComputationBoundary` (CLOACI-S-0002) enables skip-detection for redundant executions.

## ContinuousScheduler

A concrete struct, not a trait. Users customize behavior through the components it orchestrates (custom accumulators, custom trigger policies, custom detectors), not by replacing the scheduler itself. It receives a fully assembled graph at construction time — something else (registration, a builder, the macro system) assembles the graph.

```rust
struct ContinuousScheduler {
    graph: DataSourceGraph,
    execution_ledger: ExecutionLedger,
    boundary_ledger: BoundaryLedger,
    exit_edges: HashMap<String, Vec<String>>,    // task name → workflow names
    task_scheduler: Arc<TaskScheduler>,           // existing, shared
}
```

### Task Completion Integration

The `ContinuousScheduler` registers a completion callback when submitting work to the `TaskScheduler`. On completion, the callback writes to the execution ledger (CLOACI-S-0007). `LedgerTrigger` instances observe the ledger and may fire detector workflows, continuing the reactive chain.

### Run Loop

The scheduler runs as a `tokio::select!` loop, same pattern as the existing `TriggerScheduler`:

```
run loop:
  1. Observe execution_ledger for detector workflow completions
     (detector workflows are scheduled independently via existing cron/trigger system)
  2. For each completed detector workflow, extract DetectorOutput from output context
  3. For each DetectorOutput:
      a. WatermarkAdvance → update boundary_ledger
      b. Change → for each accumulator on this data source:
         - Check boundary against consumer_watermark()
         - If late: apply edge's LateArrivalPolicy (discard/forward/retrigger/side-channel)
         - If not late: accumulator.receive(boundary)
  4. For each task, check readiness per join_mode:
      - JoinMode::Any → any accumulator is_ready()
      - JoinMode::All → all accumulators is_ready()
      When ready:
      a. partial_ctx = drain ready accumulators
      b. ContextManager merges partial_ctx with upstream task output context
      c. Inject referenced data sources into DataSourceMap
      d. Submit to TaskScheduler/Dispatcher for execution
  5. On any task/workflow completion (via callback):
      a. Write LedgerEvent::TaskCompleted to execution_ledger
      b. Detector workflow completions are picked up on next iteration (step 1)
      c. For each exit edge: fire one-shot workflow via existing pipeline system
      d. Loop back to step 1
  6. On task/workflow failure (via callback):
      a. Write LedgerEvent::TaskFailed to execution_ledger
      b. Detector workflow failures are observable — scheduling continues for other sources
```

## Failure and Backpressure

When a task fails, upstream change signals continue to accumulate in the accumulator buffer. On recovery, coalesced boundaries ensure no data gaps — accumulated signals merge into a single boundary covering the full missed window. The task runs once with the coalesced boundary rather than replaying each individual signal.

## Context as Universal Data Plane

The existing `Context<serde_json::Value>` is the transport for all data between components. Boundaries, metadata, and task outputs all flow through context, persisted via the existing database-backed context system.

The accumulator produces a partial context fragment via `drain()`. The `ContextManager` at the execution layer merges this with upstream task output context (loaded via DAL). No new storage mechanisms are needed.

**Namespace convention for context keys:**
- `__boundary`, `__source`, `__signals_coalesced` — system keys (prefix `__`)
- Unprefixed — user task output keys
- `__acc_*` — accumulator-injected keys

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| ContinuousScheduler is a concrete struct, not trait | Users customize via components (accumulators, trigger policies, detectors), not by replacing the scheduler. |
| GraphEdge is the unit of wiring | Per-edge config (accumulator, late arrival policy) is the natural granularity. |
| Single graph per tenant | Cross-tenant via push events only. Security and operational isolation. |
| Exit edges fire one-shot workflows | Continuous graph doesn't wait for them. Clean boundary between continuous and event-driven. |
| Context as universal data plane | No new storage mechanisms. Existing ContextManager handles merging and persistence. |

## Resolved Design Questions

- **Accumulator persistence**: Hybrid model — in-memory with persist-on-drain. See S-0005 for details.
- **Backpressure signaling**: Observable only (accumulator lag metrics), no active backpressure. Accumulators continue buffering; operators respond to growing lag.
- **Graph hot-reload**: No. Graph changes require restart. Persisted consumer watermarks resume automatically; orphaned state is pruned via admin command. See S-0001 decisions log.
- **Metrics and observability**: Deferred to implementation. `AccumulatorMetrics` (S-0005) provides per-edge observability. System-level metrics TBD.
