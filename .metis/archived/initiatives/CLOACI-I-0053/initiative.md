---
id: continuous-scheduling-for-reactive
level: initiative
title: "Continuous Scheduling for Reactive Strategy Workloads"
short_code: "CLOACI-I-0053"
created_at: 2026-03-26T05:35:56.310027+00:00
updated_at: 2026-03-26T05:35:56.310027+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: continuous-scheduling-for-reactive
---

# Continuous Scheduling for Reactive Strategy Workloads

## Context

Cloacina's continuous scheduling system provides a reactive graph engine for data-driven DAG execution. Analysis of real-world workloads in the low-latency trading and automated decision-making space reveals that the system is architecturally close to what these consumers need, but has specific gaps that would prevent adoption today.

### Target workload pattern

The workloads this initiative targets are **not** traditional data pipelines. They follow a state-materialization + decision-actor pattern:

1. **Multiple independent event streams** (e.g., Kafka consumers for market data, pricing feeds, order lifecycle events) running as long-lived processes
2. **State materializers** that project each stream into shared in-memory state (e.g., latest orderbook per market, latest pricing per instrument, current positions, active orders)
3. **A decision actor** that reads the full materialized state snapshot on demand and produces actions (e.g., quoting decisions, risk adjustments, order submissions)

The key insight is that independent streams must be **correlated at decision time**. Each materializer processes its own stream; the decision actor needs the latest value from every stream when it fires. Cloacina's existing accumulator + JoinMode model already handles this — accumulators hold the latest boundary per edge, and JoinMode controls when the downstream task fires. Task checkpoints handle state that grows across executions (e.g., cumulative position tracking).

### What cloacina's continuous scheduling already provides

From the archive branch (`archive/main-pre-reset`), the continuous scheduling system has a complete implementation (~5000 LOC, 17 files) across four prior initiatives:

- **I-0023** (`bbc6e0a`): Core reactive scheduling — `DataSourceGraph` with edges, `JoinMode` (Any/All), `LateArrivalPolicy`; `ComputationBoundary` types (TimeRange, OffsetRange, Cursor, FullState, Custom); `SignalAccumulator` with `TriggerPolicy` (Immediate, WallClock, composites); `ExecutionLedger` for audit
- **I-0024** (`8a6bf67`): `Watermark` tracking for data completeness and late arrival handling; `LedgerTrigger` for derived data sources; `WindowedAccumulator` with watermark gating
- **I-0025** (`ea1e50d`): `DetectorStateStore` for committed/latest checkpoint tracking; boundary WAL persistence (`pending_boundaries`, `edge_drain_cursors` tables); crash recovery via `restore_from_persisted_state()`
- **I-0030** (`ef0bdcd`): `ContinuousScheduler` wired into `DefaultRunner` background services

This code is **not on main**. It must be re-applied and reconciled with post-reset changes (unified DAL, new trigger system, ManifestV2, reconciler infrastructure).

### What's missing for reactive strategy workloads

The gaps fall into three categories: execution semantics, packaging/deployment, and developer experience.

**Execution semantics:**
- No event-driven scheduler — the archive scheduler polls the ExecutionLedger on a fixed 10ms interval. This timer-based polling is the latency floor for the entire system. For reactive workloads, the scheduler must wake immediately on events (via `tokio::select!` on channels and `Notify`), not on the next poll tick. Additionally, push sources need a direct channel path into the scheduler, bypassing detectors entirely for events that are already boundaries.
- No LatestValue accumulator policy — Simple and Windowed accumulators exist, but there's no latest-wins mode that retains only the most recent boundary per edge.
- No conditional propagation — a task cannot signal "state updated but no downstream action needed." Every task completion triggers downstream edges unconditionally.
- No per-task execution mode — all tasks run on the async runtime. Tasks wrapping blocking I/O (database queries, synchronous SDK calls) can starve the scheduler.

**Packaging & deployment:**
- No ManifestV2 fields for `data_sources`, `detectors`, or `continuous_tasks` — continuous scheduling is only usable through the in-process library API.
- No reconciler support for auto-registering continuous components from packages.
- No daemon/server support for continuous scheduling from packaged workflows.

**Developer experience:**
- Python bindings don't expose channel-based event pushing.
- No reference implementation demonstrating the multi-stream correlation pattern end-to-end.
- No performance baselines for consumers evaluating cloacina for low-latency workloads.

## Goals

- Re-apply and integrate the archived continuous scheduling implementation (~5000 LOC) into main, reconciled with current infrastructure
- Extend the scheduler with push-based channel triggers, LatestValue accumulators, conditional propagation (NoFire), and per-task execution modes
- Extend ManifestV2 to declare data sources, detector workflows, and continuous tasks; reconciler auto-registers on package load
- Daemon and server modes support continuous scheduling from packages
- Validate with a concrete reference implementation modeled on a multi-stream decision engine consuming independent data feeds
- Establish performance baselines and benchmarks for low-latency continuous scheduling

## Non-Goals

- Backtest/replay system (important but separate initiative)
- Distributed/horizontal scaling of continuous scheduling across multiple processes — the event-driven scheduler is inherently single-process stateful (in-memory accumulators, ExecutionLedger, push channels). Cloacina's existing horizontal scaling (pipeline claiming, heartbeats) applies to cron/trigger workflows running alongside continuous scheduling, but the continuous scheduler itself runs on one process. Distributed continuous scheduling is future work requiring shared state or partitioned graphs.
- Changing the core scheduling algorithm — the reactive graph, accumulator, and watermark model are validated; this initiative extends them

## Use Cases

### UC-1: Multi-Stream State Materialization

- **Actor**: Strategy developer
- **Scenario**: Multiple data sources (e.g., market data, pricing feeds, order lifecycle events) each have a detector workflow watching for new messages. Each detector emits boundaries into per-edge accumulators. Materializer tasks consume boundaries and produce derived state. A downstream decision task depends on all sources via JoinMode and receives the latest accumulated value from each when it fires.
- **Expected Outcome**: Accumulators always hold the latest data from each stream. The decision task's context contains a correlated snapshot. Accumulator and checkpoint state is durable across restarts.

### UC-2: Decision Engine with Multi-Source Read

- **Actor**: Strategy decision engine (continuous task)
- **Scenario**: A decision task depends on multiple upstream sources via the DataSourceGraph. When any source accumulator becomes ready (JoinMode::Any) or all do (JoinMode::All), the scheduler drains the latest boundaries into the task's execution context. The task reads the correlated snapshot, runs a parameterized model, and emits actions.
- **Expected Outcome**: The decision task receives a consistent set of latest values from all inputs via its context. It can also restore cumulative state from checkpoints. Output flows to a downstream task or is written to an outbox.

### UC-3: Push-Based Event Ingestion

- **Actor**: External system (Kafka, WebSocket, API callback)
- **Scenario**: An external event arrives and must trigger a workflow execution immediately — no polling delay. The event carries a context payload (the message data).
- **Expected Outcome**: Workflow fires within single-digit milliseconds of event arrival. No poll interval overhead.

### UC-4: Conditional Propagation

- **Actor**: Any continuous task
- **Scenario**: A state materializer receives a message but determines nothing meaningful changed (e.g., orderbook update with identical bids/asks, or a fair price within epsilon of the previous value). It should signal "no action needed" without triggering downstream tasks.
- **Expected Outcome**: Downstream tasks (decision engine) do not fire. No wasted computation. The signal is explicit in the task's return type, not implicit.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### Layer 1: Foundation (Re-apply + Integrate)

- [ ] Core continuous scheduling module re-applied to main from archive (`archive/main-pre-reset`)
- [ ] `DataSourceGraph`, `ComputationBoundary` (5 kinds), `SignalAccumulator`, `TriggerPolicy`, `Watermark`, `LateArrivalPolicy`, `ExecutionLedger` all compile and pass unit tests on main
- [ ] `ContinuousScheduler` reactive loop works with current `DefaultRunner`
- [ ] `DetectorStateStore` persistence layer reconciled with unified DAL
- [ ] Boundary WAL (`pending_boundaries`, `edge_drain_cursors`) migrations applied
- [ ] Crash recovery (`restore_from_persisted_state()`) works against Postgres
- [ ] Archived integration and e2e tests re-applied and passing
- [ ] All existing (non-continuous) tests continue to pass

### Layer 2: Packaging & Deployment

- [ ] ManifestV2 supports `data_sources`, `detectors`, `continuous_tasks` declarations
- [ ] Package reconciler registers continuous scheduling components (data sources, detectors, accumulators, graph edges) on package load
- [ ] A `.cloacina` package can declare a detector that watches a data source and continuous tasks that consume its boundaries
- [ ] Daemon mode runs continuous scheduling from packaged workflows
- [ ] Server mode runs continuous scheduling from packaged workflows
- [ ] Package load → detector fires → boundaries accumulate → continuous tasks execute (end-to-end through packaging)

### Layer 3: Reactive Workload Extensions

#### Event-Driven Scheduler + Push Sources

- [ ] The `ContinuousScheduler` main loop is fully event-driven via `tokio::select!` — no timer-based polling for the primary work path
- [ ] Three wakeup sources coexist in the select loop: push channel (direct boundaries), ledger notify (detector/task completions), and heartbeat (housekeeping only)
- [ ] Push sources send `ComputationBoundary` directly into the scheduler via `tokio::mpsc` channel — no detector workflow involved for push-based ingestion
- [ ] Polled sources use detectors that write to the `ExecutionLedger`, which fires `tokio::sync::Notify` to wake the scheduler immediately (replacing the archived timer-based ledger polling)
- [ ] Push source event-to-accumulator latency is sub-millisecond (bounded by tokio task scheduling, no timer floors) — establish this as a benchmark
- [ ] Python bindings expose the `mpsc::Sender` so Python processes can push boundaries into Rust-executed continuous tasks
- [ ] Push channel capacity is bounded with backpressure (senders block when full)

#### Accumulator-Based State Correlation

- [ ] Accumulators support a `LatestValue` trigger policy that retains only the most recent boundary per edge (latest-wins semantics for "give me the current state" patterns)
- [ ] When a multi-input task fires (via JoinMode::Any or JoinMode::All), its `Context` contains the latest drained value from each input edge — the task does not need to reach outside the execution context
- [ ] Accumulator state (including latest-value buffers) is persisted via the existing DAL and restored on scheduler restart
- [ ] Task checkpoints (`Task::checkpoint()`) can persist cross-execution cumulative state (e.g., position tracking that grows across hundreds of events) and are restored on restart
- [ ] A multi-input task receiving boundaries from 3+ independent sources sees a consistent set of latest values at drain time (no partial updates from concurrent accumulator writes during drain)

#### Conditional Propagation (NoFire Semantic)

- [ ] A continuous task can return a result indicating "state updated but no downstream action needed"
- [ ] When a task returns NoFire, downstream tasks in the `DataSourceGraph` are not triggered for that execution
- [ ] When a task returns normally, downstream tasks fire per existing accumulator/JoinMode logic
- [ ] The NoFire decision is recorded in the `ExecutionLedger` for observability

#### Per-Task Execution Mode

- [ ] Continuous tasks can declare an execution mode: `Inline` (run on scheduler tokio runtime), `SpawnBlocking` (offload to blocking thread pool), `Dedicated` (dedicated OS thread)
- [ ] `SpawnBlocking` mode is suitable for tasks wrapping blocking I/O (database queries, synchronous SDK calls) without starving the async runtime
- [ ] Execution mode is declared in the task definition (macro attribute or manifest field), not at runtime

### Layer 4: Reference Implementation & Benchmarks

#### Reference Implementation: Multi-Stream Decision Engine

The reference implementation exercises all capabilities above with a concrete computation graph. Source names and domain language are deliberately abstract.

**Data Sources** — Three independent streams, each with a detector and LatestValue accumulator:

| Source | Detector | Boundary payload | Update frequency |
|--------|----------|-----------------|-----------------|
| **source_alpha** | Watches mock stream A | `{ "top_high": f64, "top_low": f64, "levels": Vec<(f64, i64)> }` | ~10-50ms |
| **source_beta** | Watches mock stream B | `{ "estimate": f64, "published_at": i64 }` | ~100-500ms |
| **source_gamma** | Watches mock stream C (execution confirmations) | `{ "side": "up"\|"down", "filled_qty": f64, "fill_price": f64, "fee": f64 }` | Sporadic (event-driven) |

**Materializer Tasks:**

- **alpha_materializer**: Receives source_alpha boundaries. Extracts `top_high` and `top_low` from the latest boundary. Passes through to decision engine via accumulator (no transformation needed — LatestValue policy retains the most recent).
- **beta_materializer**: Receives source_beta boundaries. Passes `estimate` through. Returns NoFire if `abs(new_estimate - previous_estimate) < 0.001` (conditional propagation — skip if nothing meaningful changed).
- **gamma_materializer**: Receives source_gamma boundaries. Updates a **cumulative exposure** counter via `Task::checkpoint()`: `exposure += filled_qty` if side is "up", `exposure -= filled_qty` if side is "down". Emits the new exposure value. This validates cross-execution checkpoint persistence.

**Decision Engine Task** — Multi-input task with edges from all three materializers (JoinMode::Any — fires when any input has new data). On each execution:

1. **Read correlated snapshot** from context: latest `top_high`, `top_low`, `estimate`, and `exposure` from accumulator drain + checkpoint
2. **Guard**: if `estimate` is None (no beta data yet), return NoFire
3. **Compute** (parameterized model with config values `k1`, `k2`, `k3`, `floor_param`, `cap_param`):

```
tau = sqrt(max(0, time_remaining) / config.total_duration)
spread = config.k1 + (config.k2 * max(config.floor_param, tau) / 2)

shift_magnitude = max(
    config.floor_param * config.k2,
    abs(exposure) * config.k2 * tau
)
shift_sign = -sign(exposure)  // positive exposure → shift down, negative → shift up

scaled_shift = shift_magnitude * config.k3
capped_shift = min(scaled_shift, config.cap_param * tau)

center = clamp(estimate + shift_sign * capped_shift, 0.05, 0.95)
output_high = clamp(center + spread, 0.05, 0.95)
output_low  = clamp(center - spread, 0.05, 0.95)
```

4. **Reconcile** against previous output (restored from checkpoint): if `output_high` and `output_low` are unchanged from last execution, return NoFire
5. **Emit** `{ "output_high": f64, "output_low": f64, "center": f64 }` to downstream output task

**Output Task** — Receives decision actions. Logs them. In a real deployment this would write to a Kafka producer or outbox table.

#### Reference Implementation Acceptance Criteria

- [ ] All five tasks (3 materializers + decision engine + output) run as continuous tasks in a single long-lived process
- [ ] source_alpha and source_beta use push sources (direct `mpsc` channel into scheduler, sub-millisecond to accumulator)
- [ ] source_gamma uses push source with sporadic events (validates that the system doesn't depend on regular cadence)
- [ ] beta_materializer demonstrates NoFire when estimate unchanged (downstream decision engine does not execute)
- [ ] gamma_materializer persists cumulative exposure via checkpoint; after scheduler restart, exposure is restored correctly
- [ ] Decision engine receives correlated snapshot from all three accumulators on each firing
- [ ] Decision engine guards on missing beta data (NoFire until first estimate arrives)
- [ ] Decision engine reconciles against previous output checkpoint (NoFire when output unchanged)
- [ ] The full computation (snapshot → math → reconcile → emit) completes correctly with the formula above
- [ ] End-to-end latency instrumented: time from source_alpha event push to output task receiving the decision
- [ ] The example runs for 60+ seconds under sustained load (alpha at 10ms, beta at 200ms, gamma sporadic) without memory growth or accumulator backup

#### Performance Baselines

- [ ] Establish benchmark for: event-to-task-fire latency (push source `mpsc::send()` → task `execute()` called)
- [ ] Establish benchmark for: accumulator drain latency (task receives latest values from N input edges)
- [ ] Establish benchmark for: end-to-end latency (event arrives → decision task completes)
- [ ] All benchmarks run as part of the soak test infrastructure (CLOACI-I-0054)
- [ ] Publish baseline numbers as concrete targets for consumers evaluating cloacina for low-latency workloads

## Prior Art

### Archive branch: `archive/main-pre-reset`

The complete continuous scheduling module (17 files, ~5000 LOC):

| Component | File(s) | LOC | Status |
|-----------|---------|-----|--------|
| ComputationBoundary (5 kinds) | `continuous/boundary.rs` | ~514 | Ready to re-apply |
| DataSource + DataConnection | `continuous/datasource.rs` | ~341 | Ready to re-apply |
| DetectorOutput | `continuous/detector.rs` | ~144 | Ready to re-apply |
| DataSourceGraph + JoinMode | `continuous/graph.rs` | ~420 | Ready to re-apply |
| SignalAccumulator (Simple + Windowed) | `continuous/accumulator.rs` | ~292 | Ready; extend with LatestValue |
| TriggerPolicy (Immediate, WallClock, composites) | `continuous/trigger_policy.rs` | ~146 | Ready to re-apply |
| Watermark tracking | `continuous/watermark.rs` | ~329 | Ready to re-apply |
| ExecutionLedger | `continuous/ledger.rs` | ~248 | Ready to re-apply |
| LedgerTrigger | `continuous/ledger_trigger.rs` | ~309 | Ready to re-apply |
| ContinuousScheduler | `continuous/scheduler.rs` | ~416 | Ready; extend with channel triggers + NoFire |
| DetectorStateStore + persistence | `continuous/detector_state_store.rs`, `continuous/accumulator_persistence.rs`, `continuous/state_management.rs` | ~288+ | Reconcile with unified DAL |
| DAL extensions | `dal/unified/detector_state_dal.rs`, `dal/unified/pending_boundary_dal.rs` | ~718 | Reconcile with unified DAL |
| Connections (Postgres, Kafka, S3) | `continuous/connections/` | ~144+ | Re-apply Postgres; Kafka/S3 deferred |

Testing on archive: 439+ unit tests, 41 integration tests, 6 e2e crash recovery tests against Postgres.

### Archive branch: `archive/cloacina-server-week1`

Continuous soak tests (`e2c8f0b`, `74e3038`, `39f3ef8`) — sustained load, batched boundaries, multi-source scenarios.

### Current main branch

- Trigger system (polling-based trait, registry, config, manifest support) — channel trigger extends this
- ManifestV2 with `TriggerDefinition` — needs new fields for continuous scheduling
- `cloacina-testing` crate has placeholder `BoundaryEmitter` and `ComputationBoundary` — to be replaced by real implementations

## Relationship to Other Initiatives

- **CLOACI-I-0054** (Soak Tests & Performance Benchmarks): The performance baselines from this initiative feed into I-0054's benchmark infrastructure.
- **CLOACI-I-0049** (API Server): The channel trigger must work in both daemon and server deployment modes.
- **CLOACI-I-0061** (Daemon Infrastructure): The reference implementation should run in daemon mode for local development.

## Alternatives Considered

### Build a dedicated reactive stream-processing framework

Rejected. Reactive graph frameworks (node-graph with data flowing through links) sound like a natural fit for these workloads, but real-world usage shows the pattern breaks down: independent streams need to be correlated through shared state, not joined through edges. Nodes become state materializers that don't propagate data downstream. The graph topology becomes decorative. Building a new framework around this abstraction would encode the mismatch rather than solving it.

### Build a separate "strategy engine" outside cloacina

Rejected. The continuous scheduling system already has 80% of the required infrastructure (reactive graph, accumulators, watermarks, persistence, crash recovery). Adding push-based triggers and validating the accumulator/checkpoint model for these workloads is less work than building a new system, and avoids maintaining two orchestration platforms.

## Implementation Notes

### Sequencing

The four layers are roughly sequential but with parallelism opportunities:

1. **Layer 1** (foundation) must come first — everything depends on the core being on main
2. **Layer 2** (packaging) and **Layer 3** (reactive extensions) can proceed in parallel once the foundation is stable — they operate at different layers (manifest/reconciler vs. scheduler internals)
3. **Layer 4** (reference implementation) requires all three prior layers

### Key integration points to watch

- The archived `ContinuousScheduler` uses `ExecutionLedger` as its event bus. The channel trigger bypasses this — it pushes directly into the scheduler's event loop via `tokio::select!`. Both paths must coexist.
- The archived accumulator persistence uses raw SQL. It must be reconciled with the unified DAL that now exists on main.
- The `#[continuous_task]` proc macro from the archive must be reconciled with current macro infrastructure on main.
- The `cloacina-testing` placeholder types (`BoundaryEmitter`, `ComputationBoundary`) should be replaced by the real implementations once Layer 1 lands.

### Validation strategy

The most important validation is that the existing accumulator + checkpoint model handles the multi-stream correlation pattern without requiring a new shared state abstraction. The reference implementation is the proof point — if a multi-input decision task can receive correlated latest values from 3+ independent streams via accumulator drain, and maintain cumulative state across executions via checkpoints, the architecture works. If it can't, that's when we revisit whether a dedicated state store is needed.
