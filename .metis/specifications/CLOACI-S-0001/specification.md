---
id: continuous-reactive-scheduling
level: specification
title: "Continuous Reactive Scheduling — Core Architecture"
short_code: "CLOACI-S-0001"
created_at: 2026-04-04T11:45:19.834595+00:00
updated_at: 2026-04-04T11:45:19.834595+00:00
parent: CLOACI-I-0053
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Continuous Reactive Scheduling — Core Architecture

## Overview

The Continuous Reactive Scheduling system is Cloacina's engine for data-driven, event-reactive DAG execution. Unlike Cloacina's cron and trigger-based scheduling (which fires workflows on a time schedule or polling condition), continuous scheduling maintains a persistent reactive graph where data changes propagate through edges, accumulate in per-edge buffers, and fire downstream tasks when readiness conditions are met.

The system serves workloads that follow a **state-materialization + decision-actor pattern**: multiple independent event streams run as long-lived processes, state materializers project each stream into derived values, and a decision actor reads the correlated snapshot from all streams when it fires. The key architectural insight is that independent streams are correlated at decision time through accumulators and JoinMode, not through shared mutable state.

### Key components

- **DataSourceGraph** — the reactive DAG defining data sources, tasks, and edges between them
- **DataSource** — describes an external data provider (table, stream, file) with connection and detector metadata
- **Detector workflows** — regular Cloacina workflows that watch data sources and emit `DetectorOutput` (boundaries describing what changed)
- **ComputationBoundary** — describes a slice of data that changed (see CLOACI-S-0002)
- **SignalAccumulator** — per-edge buffer that collects boundaries and decides when to fire (see CLOACI-S-0002)
- **ContinuousScheduler** — the reactive loop that routes boundaries, checks readiness, and fires tasks
- **ExecutionLedger** — append-only event log recording all scheduling activity
- **Watermarks** — track data completeness per source for late arrival handling

### Relationship to other specs

- **CLOACI-S-0002** (ComputationBoundary & Accumulators) — defines boundary types, coalescing rules, accumulator implementations, trigger policies, and persistence
- **CLOACI-S-0003** (Continuous Task Execution Model) — defines how tasks execute (NoFire, execution modes, checkpoints, context injection)

## System Context

### Actors

- **Detector workflow**: A regular Cloacina workflow registered as a data source's detector. Runs on a schedule or channel trigger, inspects the data source for changes, emits `DetectorOutput` containing `ComputationBoundary` values describing what changed.
- **Continuous task**: A task registered in the `DataSourceGraph` that consumes boundaries from one or more edges. Receives a correlated snapshot of latest values from its input accumulators. May produce output that flows to downstream edges.
- **External event producer**: A system (Kafka consumer, WebSocket handler, API callback) that pushes boundaries directly into the scheduler's push channel. No detector involved — the producer constructs the `ComputationBoundary` and sends it via `mpsc::Sender`. This is the lowest-latency ingestion path (sub-millisecond).
- **Package reconciler**: On package load, reads ManifestV2 `data_sources`, `detectors`, and `continuous_tasks` declarations and registers them into the DataSourceGraph and scheduler.

### External Systems

- **Persistence layer (DAL)**: Stores detector state checkpoints, pending boundary WAL, edge drain cursors, and accumulator state. Must support both SQLite (daemon mode) and Postgres (server mode) via the unified DAL.
- **DefaultRunner**: The Cloacina execution engine. The ContinuousScheduler runs as a background service within the DefaultRunner, alongside the unified Scheduler for cron/trigger workflows.

### Boundaries

**Inside scope**: DataSourceGraph construction and validation, boundary routing from detectors to accumulators, readiness checking and task firing, the scheduler reactive loop (both polling and channel-based), crash recovery and state restoration, ExecutionLedger event recording.

**Outside scope**: What detectors actually do (they are user-defined workflows), what continuous tasks compute (they are user-defined), the persistence layer implementation (owned by the DAL), package loading and manifest parsing (owned by the reconciler).

## Data Source Graph

### Structure

The `DataSourceGraph` is a directed acyclic graph with three node types:

1. **DataSource nodes** — represent external data providers. Each has a name, `DataConnection` (how to reach the data), a detector workflow reference, and metadata (description, owner, tags).
2. **Task nodes** — `ContinuousTaskRegistration` entries with a task ID, a list of triggering source edges (drive task firing), and a list of referenced source edges (read-only side lookups).
3. **Edges** — `GraphEdge` connecting a source to a task, with `JoinMode` and `LateArrivalPolicy` per edge.

### Graph Assembly and Validation

`DataSourceGraph::assemble_graph()` validates:
- No duplicate data source names
- No duplicate task registrations
- All edge references resolve to existing sources and tasks
- **No cycles** — validated via Kahn's algorithm (topological sort). A cycle means a task's output feeds back into its own input chain, which would cause infinite firing.
- At least one data source and one task exist

### JoinMode

Controls when a multi-input task fires relative to its input edges:

| Mode | Semantics | Use case |
|------|-----------|----------|
| `Any` | Fire when **any** input accumulator is ready | Decision engine that should react to any new data |
| `All` | Fire when **all** input accumulators are ready | Task that needs a complete snapshot before proceeding |

JoinMode is set per edge but evaluated per task — the scheduler collects readiness across all edges for a task and applies the task's join semantics.

### LateArrivalPolicy

Controls what happens when a boundary arrives with a timestamp/offset behind the consumer watermark for that edge:

| Policy | Behavior |
|--------|----------|
| `Discard` | Drop the boundary silently |
| `AccumulateForward` | Buffer the boundary for the next accumulation cycle (default) |
| `Retrigger` | Re-fire the task with the late boundary |
| `RouteToSideChannel` | Send to a separate dead-letter accumulator for manual inspection |

## Scheduler Reactive Loop

### Event-Driven Architecture

The `ContinuousScheduler` is fully event-driven. It does **not** poll on a timer for its primary work loop. Instead, it uses `tokio::select!` to wake immediately when any of three event sources fire:

```rust
loop {
    tokio::select! {
        // Push sources: external events arrive as boundaries directly
        boundary = push_channel.recv()      => route_boundary(boundary),

        // Polled sources: detectors complete and notify via the ledger
        _        = ledger_notify.notified() => drain_ledger_events(),

        // Housekeeping: watermark advancement, metrics flush, stale task sweep
        _        = heartbeat.tick()         => housekeeping(),

        // Graceful shutdown
        _        = shutdown.recv()          => break,
    }
}
```

This eliminates the poll interval as a latency floor. The scheduler wakes within microseconds of an event arriving, not on the next 10ms tick.

### Three Ingestion Paths

**1. Push sources (direct channel)**

For sources where the external producer already knows what changed and sends the boundary directly. No detector workflow involved — the event IS the boundary.

```
External event → tokio::mpsc channel → scheduler select! wakes
    → route_boundary() → per-edge accumulators → check readiness → fire tasks
```

Latency: sub-millisecond from event push to accumulator routing. This is the hot path for reactive strategy workloads (market data feeds, execution confirmations, pricing updates).

The channel sender is exposed to:
- Rust code via `mpsc::Sender<ComputationBoundary>` obtained from the scheduler
- Python code via PyO3 bindings wrapping the sender

**2. Polled sources (detector → ledger → notify)**

For sources where a detector workflow must inspect the data source to discover what changed (e.g., polling a database table, checking an S3 prefix).

```
Detector workflow completes → TaskCompleted written to ExecutionLedger
    → ExecutionLedger fires tokio::sync::Notify → scheduler select! wakes
    → drain_ledger_events() → extract DetectorOutput → route boundaries to accumulators
```

Latency: microseconds from detector completion to scheduler wakeup (Notify is immediate). The detector's own execution time dominates, not the scheduler. This replaces the archived design where the scheduler polled the ledger on a timer — the Notify mechanism (already used by LedgerTrigger) now drives the scheduler directly.

**3. Task completion (downstream propagation)**

When a continuous task completes with `ContinuousTaskResult::Fire`, its output must propagate to downstream edges. Task completions also write to the ExecutionLedger and trigger the same Notify, so the scheduler wakes immediately to route downstream boundaries.

```
Continuous task completes (Fire) → TaskCompleted to ExecutionLedger
    → Notify → scheduler wakes → extract output boundaries → route to downstream accumulators
```

All three paths converge at the same `route_boundary()` function — from that point forward, accumulator routing, readiness checking, and task firing are identical.

### Scheduler Wakeup Cycle

On each wakeup (whether from push channel, ledger notify, or heartbeat):

1. **Receive events** — drain the push channel (all pending boundaries); scan ExecutionLedger for new events since last cursor (detector completions, task completions)
2. **Extract boundaries** — from push channel events directly; from `DetectorOutput` in completed detector contexts; from `ContinuousTaskResult::Fire` output in completed task contexts
3. **Route boundaries** — for each boundary, find the matching data source, then find all edges from that source. For each edge:
   a. Check consumer watermark — apply `LateArrivalPolicy` if boundary is behind watermark
   b. Send boundary to the edge's `SignalAccumulator`
4. **Check readiness** — for each task with at least one ready accumulator:
   a. Evaluate `JoinMode` across all input edges
   b. If ready: atomically drain all ready accumulators, inject coalesced boundaries into task context
5. **Fire tasks** — submit ready tasks to the executor (see CLOACI-S-0003 for execution modes)
6. **Record results** — write `TaskCompleted`, `TaskFailed`, `AccumulatorDrained`, or `BoundaryEmitted` events to the ExecutionLedger (which may Notify for downstream propagation)
7. **Handle NoFire** — if a task returns NoFire, do NOT route its output to downstream edges (see CLOACI-S-0003). Record `TaskCompletedNoFire` in ledger.
8. **Advance watermarks** — update consumer watermarks based on drained boundaries

### Heartbeat (Housekeeping Timer)

The heartbeat is the only timer in the loop. It does NOT drive the primary work path. It fires at a configurable interval (default 1s) for:

- **Watermark advancement** — advance source watermarks based on committed detector state
- **Metrics flush** — emit accumulator metrics, lag measurements, throughput counters
- **Stale task sweep** — detect tasks that have exceeded `task_timeout` and cancel them
- **Persistence flush** — batch-write pending boundary WAL entries and drain cursor updates (if batched writes are enabled for throughput)

If no housekeeping is needed, the heartbeat tick is a no-op.

### Concurrency Model

- Each push source has its own `mpsc::Sender` — multiple producers can push boundaries concurrently into the scheduler's channel
- The scheduler's main loop is single-threaded (one `tokio::select!` loop) — this simplifies accumulator state management and avoids locking on the hot path
- Task execution is dispatched to the executor (which may spawn blocking threads per CLOACI-S-0003) — the scheduler does not block on task completion
- Task completion events flow back through the ExecutionLedger + Notify, waking the scheduler immediately for downstream propagation
- The push channel uses `mpsc::Receiver` (not `broadcast`) — the scheduler is the single consumer. Channel capacity is bounded to provide backpressure when the scheduler falls behind.

### Configuration

`ContinuousSchedulerConfig`:
- `heartbeat_interval: Duration` — housekeeping timer interval (default 1s). Does NOT affect event-driven wakeups.
- `push_channel_capacity: usize` — bounded capacity for the push event channel (default 10,000). Senders block when full (backpressure).
- `max_fired_tasks: usize` — bounded buffer for fired task records in the ExecutionLedger (default 100,000). FIFO eviction.
- `task_timeout: Duration` — maximum time a continuous task may execute before being cancelled.
- `batch_persistence: bool` — if true, WAL writes are batched on heartbeat tick instead of written per-boundary (trades durability for throughput). Default false.

## Execution Ledger

The `ExecutionLedger` is an in-memory append-only event log shared between the scheduler and the rest of the system. It records four event types:

| Event | Fields | Produced by |
|-------|--------|-------------|
| `TaskCompleted` | task_id, completed_at, context | Executor (after detector or continuous task completes) |
| `TaskFailed` | task_id, error | Executor (on task failure) |
| `BoundaryEmitted` | source_name, boundary | Scheduler (when boundary is routed) |
| `AccumulatorDrained` | task_id, coalesced_boundaries | Scheduler (when accumulator is drained for task firing) |

Events are assigned monotonic offsets. Consumers (like `LedgerTrigger`) scan `events_since(cursor)` to read new events without re-reading old ones. The ledger has a bounded buffer (`max_fired_tasks`) with FIFO eviction — the oldest events are dropped when capacity is reached.

### LedgerTrigger

A derived trigger that implements the `Trigger` trait by scanning the ExecutionLedger for specific task completion events. Supports two match modes:

- `Any` — fire when any of the specified tasks complete
- `All` — fire when all specified tasks have completed (since last drain)

Uses `tokio::sync::Notify` for event-driven wakeup with a 5-second fallback poll to handle missed notifications.

## Persistence and Crash Recovery

### Persisted State

| Table | Purpose | Write path |
|-------|---------|------------|
| `detector_state` | Latest + committed checkpoint per data source | Written on detector completion; committed after downstream processing |
| `pending_boundaries` | Boundary WAL — all emitted boundaries in order | Written when boundary is routed to accumulators |
| `edge_drain_cursors` | Per-edge offset tracking (Kafka consumer group model) | Written when accumulator is drained |

All persistence goes through the unified DAL, supporting both SQLite and Postgres.

### Crash Recovery

On startup, `ContinuousScheduler::restore_from_persisted_state()`:

1. Load `detector_state` — restore latest checkpoint per source
2. Load `pending_boundaries` — replay any boundaries emitted but not yet drained
3. Load `edge_drain_cursors` — restore consumer watermarks per edge
4. Rebuild in-memory accumulator state from pending boundaries after each edge's drain cursor
5. Resume the reactive loop from the restored state

This ensures exactly-once processing semantics for boundary routing — boundaries written to the WAL but not yet drained are replayed, boundaries already drained (cursor advanced) are skipped.

## Packaging & Deployment

### ManifestV2 Extensions

The ManifestV2 schema gains three new top-level fields:

```
data_sources:
  - name: "source_alpha"
    connection_type: "channel"  # or "postgres", "kafka", etc.
    detector_workflow: "detect_alpha"
    config: { ... }

detectors:
  - name: "detect_alpha"
    workflow: "detect_alpha_workflow"
    trigger_mode: "channel"  # or "poll"
    poll_interval: "10ms"  # only for poll mode

continuous_tasks:
  - name: "alpha_materializer"
    function: "materialize_alpha"
    sources: ["source_alpha"]
    join_mode: "any"
    execution_mode: "inline"  # or "spawn_blocking", "dedicated"
    late_arrival_policy: "accumulate_forward"
```

### Reconciler Integration

When the package reconciler loads a `.cloacina` package containing continuous scheduling declarations:

1. Parse `data_sources`, `detectors`, `continuous_tasks` from manifest
2. Register `DataSource` entries in the `DataSourceGraph`
3. Register detector workflows with the unified Scheduler (poll-based) or create channel triggers
4. Register `ContinuousTaskRegistration` entries with edges, JoinMode, and LateArrivalPolicy
5. Call `DataSourceGraph::assemble_graph()` to validate the complete graph
6. Start the `ContinuousScheduler` as a background service in the DefaultRunner

This works in both daemon mode (SQLite, local filesystem) and server mode (Postgres, multi-tenant).

## Constraints

### Technical Constraints

- The scheduler main loop must remain single-threaded to avoid locking accumulators. Task execution is dispatched to the executor.
- Push source events must route from `mpsc::send()` to accumulator within sub-millisecond. The event-driven `select!` loop eliminates timer-based latency floors — the remaining latency is channel wake + boundary routing, bounded by tokio task scheduling.
- The continuous scheduler is inherently **single-process stateful** — in-memory accumulators, the ExecutionLedger, and the push channel are all local to one process. Horizontal scaling of continuous scheduling across multiple processes is explicitly out of scope (see I-0053 Non-Goals). Cloacina's existing horizontal scaling (pipeline claiming, heartbeats) applies to cron/trigger workflows running alongside continuous scheduling in the same DefaultRunner, but not to the continuous scheduler itself.
- The boundary WAL must use O(1) writes on the hot path regardless of graph fan-out — write once, route to N edges via cursor math.
- Crash recovery must be safe against partial writes — the WAL + cursor model ensures boundaries are either fully processed or fully replayed.
