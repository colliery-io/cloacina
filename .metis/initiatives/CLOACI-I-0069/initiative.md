---
id: continuous-scheduling-for-reactive
level: initiative
title: "Continuous Scheduling for Reactive Strategy Workloads"
short_code: "CLOACI-I-0069"
created_at: 2026-04-04T12:00:14.401564+00:00
updated_at: 2026-04-04T12:00:14.401564+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: continuous-scheduling-for-reactive
---

# Continuous Scheduling for Reactive Strategy Workloads Initiative

## Context

Cloacina needs a reactive scheduling engine for workloads that don't fit the cron/trigger model. The target workloads follow a **state-materialization + decision-actor pattern**:

1. **Multiple independent event streams** (market data, pricing feeds, order lifecycle events, sensor readings) running as long-lived processes
2. **State materializers** that project each stream into derived values (latest orderbook, current positions, cumulative exposure)
3. **A decision actor** that reads the full materialized state snapshot on demand and produces actions (quoting decisions, risk adjustments, control signals)

The key insight: independent streams must be **correlated at decision time**. Each materializer processes its own stream; the decision actor needs the latest value from every stream when it fires. This is not a traditional data pipeline — it's a reactive computation graph where data flows through edges, accumulates in per-edge buffers, and fires downstream tasks when readiness conditions are met.

This initiative is a **design meta-initiative**. Its deliverable is a fully specified architecture with component specifications, validated through discussion. Implementation will be decomposed into separate, focused initiatives once the design is settled.

### What exists today

Cloacina's current scheduling infrastructure handles cron schedules and polled triggers. There is no continuous reactive scheduling on main.

An archived implementation exists on `archive/main-pre-reset` (~5000 LOC, 17 files) covering a reactive graph engine with boundaries, accumulators, watermarks, persistence, and crash recovery. This is **prior art and reference material** — it validates that the approach works, but the archived design has known limitations (timer-based polling as the latency floor, no push sources, no conditional propagation, no execution mode control) that make it insufficient for the target workloads. The new design should be informed by what we learned, not constrained by what was built.

## Goals & Non-Goals

**Goals:**
- Design an event-driven reactive computation engine as a **standalone component** — architecturally independent from the existing cron/trigger scheduler, with its own event loop, persistence model, and dispatch semantics
- Define the data model: how events enter the system (Kafka topics, detector handoff, push channels), how boundaries describe materialized state, how accumulators aggregate upstream data
- Define the accumulator trait: backend-agnostic accumulators that read from a source (Kafka topic, Postgres, push channel) and checkpoint to persistent storage. Built-in implementations cover simple aggregation (LatestValue, RunningAggregate). Complex aggregation is handled by whatever writes to the Kafka topic upstream — the accumulator doesn't know or care what produces the data
- Define the execution model: compiled computation graph as a single async function with topology-declared routing via Rust enums and `Option<T>` for branch-level conditional propagation. Nodes are pure async functions. Execution concerns (blocking I/O) handled by `#[node(blocking)]` wrapping
- Define the reactor: long-lived process that wires accumulators to compiled computation graphs via reaction criteria (when_any/when_all), with manual force-fire channel for testing and ops- Define the packaging and deployment model: how continuous scheduling components are declared in manifests and registered by the reconciler
- Validate the design with a concrete reference implementation specification
- Decompose into implementation initiatives once design is settled

**Non-Goals:**
- Implementation — this initiative produces specifications and a decomposition plan, not code
- Complex stream processing (windowed aggregation, watermark-based completeness, multi-partition exactly-once) — not Cloacina's problem. If someone needs complex aggregation, they run whatever upstream process they want and write the result to a Kafka topic. Cloacina consumes it through the same accumulator trait as everything else
- Backtest/replay system
- Changing Cloacina's existing cron/trigger scheduling — the continuous scheduler is additive and architecturally separate

## Use Cases

### UC-1: Multi-Stream State Materialization

- **Actor**: Strategy developer
- **Scenario**: Multiple data sources each produce events at different rates. Each source has an accumulator (user-defined function) that materializes raw events into typed boundaries. A computation graph consumes these accumulators with reaction criteria (`when_any` or `when_all`) and receives the latest accumulated value from each source when it fires.
- **Expected Outcome**: Accumulators hold the latest data from each stream. The graph's entry node receives the full cache — a correlated snapshot of all inputs. Accumulator state is durable across restarts.

### UC-2: Decision Engine with Multi-Source Read

- **Actor**: Strategy decision engine (computation graph node)
- **Scenario**: A computation graph depends on multiple accumulators. When reaction criteria are met (any accumulator has new data, or all do), the reactor fires the graph with the full input cache. The decision node reads the correlated snapshot, runs a parameterized model, and emits to an output port.
- **Expected Outcome**: Consistent set of latest values from all inputs. State restored from accumulators (written by previous execution's collector or mid-graph writes). Output flows to downstream nodes via output ports.

### UC-3: Push-Based Event Ingestion

- **Actor**: External system (Kafka consumer, WebSocket handler, API callback)
- **Scenario**: An external event arrives and must enter the computation graph immediately — no polling delay. The event is already a typed boundary (the producer knows what changed). It pushes to a named accumulator endpoint on the API server.
- **Expected Outcome**: Sub-millisecond from push to accumulator. No timer-based latency floor.

### UC-4: Conditional Branch Propagation

- **Actor**: Any intermediate computation graph node
- **Scenario**: An intermediate node receives input but determines a downstream branch should not continue. It returns `None` from `Option<T>`, which prevents downstream nodes on that branch from executing.
- **Expected Outcome**: Downstream nodes on that branch do not fire. No wasted computation. Standard Rust `Option` — no framework types.

## Architecture

### Process Model

The **Reactive Scheduler** manages computation graph processes — the reactive counterpart to the unified scheduler. Both run within the API server process (Postgres-only), loaded via the reconciler.

```
API Server (Postgres)
  ├── Unified Scheduler (cron + triggers, database-backed, horizontally scalable)
  ├── Reactive Scheduler (computation graphs, event-driven, per-graph processes)
  │     alpha_accumulator.run()  ──→
  │     beta_accumulator.run()   ──→  Reactor.run()  →  graph.execute(cache)
  │     gamma_accumulator.run()  ──→       ↑
  │                                   manual_channel (via WS)
  ├── WebSocket Layer (auth, accumulator/reactor endpoints)
  └── Shared: DAL (Postgres), tokio runtime, Reconciler
```

- **Accumulators** — long-lived processes, one per source. Each consumes from its backend (Kafka topic, socket, Postgres, detector channel), maintains its own state, and pushes typed boundaries to the reactor.
- **Reactor** — long-lived process, one per computation graph. Receives boundaries from all its accumulators, maintains the input cache and dirty flags, evaluates reaction criteria, and calls `graph.execute(cache)` when criteria are met. Persists cache to DAL for fast recovery.
- **Compiled computation graph** — a single async function generated by the `#[computation_graph]` macro. Not a process — just a function the reactor calls.

The reactor also exposes a **manual channel** for force-firing:
- Force-fire with current cached state (manual retry)
- Force-fire with injected state values (testing, debugging, ops recovery)
- Trivially simple — just push to the channel

Both accumulators and reactors **register as named endpoints** on the API server. External producers push data to accumulators through the API server's WebSocket layer (which handles auth). Operators interact with reactors through the same layer (force-fire, inject state, status queries). The API server proxies to the process — accumulators and reactors don't handle auth themselves.

A detector running on the unified scheduler writes to an accumulator via the API server WebSocket when it completes — same path as any external producer, same auth. That's the only coupling between the two systems.

### Accumulator Architecture

Accumulators are **trait-based** and **backend-agnostic**. An accumulator reads from a WAL-like source and checkpoints its state to persistent storage. The trait doesn't care what the WAL is:

| Backend | Source | Checkpoint | Use case |
|---------|--------|-----------|----------|
| **Socket/channel** | Direct push (TCP, Unix socket, mpsc channel) | None — stateless passthrough | Lowest latency. External system pushes pre-materialized boundaries directly. |
| **Stream broker** | Topic/stream (Kafka, Redpanda, Iggy, etc.) | Consumer offsets | Durable push sources. Broker handles replay and offset management. Kafka-compatible protocol first. |
| **Postgres** | Query-based change detection | Checkpoint table | Polled sources — detector queries, emits boundaries. |
| **Cloacina internal** | Boundary log in DAL | Checkpoint table | Detector-sourced boundaries where no external WAL exists. |

Built-in accumulator implementations, simplest to most stateful:

- **Passthrough**: Zero state. Receives a boundary on a socket/channel, forwards it into the graph immediately. Used when the upstream already did all materialization/aggregation — Cloacina is just the execution engine. Any external system that can push to a socket can feed the graph.
- **LatestValue**: State = 1 value. Replaces previous boundary on receive, forwards the latest.
- **RunningAggregate**: State = 1 scalar + last offset. Applies a function (sum, count, min, max) to each incoming boundary, forwards the running result.
- **Stream consumer**: State = consumer offset. Reads boundaries from a topic/stream, tracks position. The broker handles durability and replay. Kafka first, but the interface should support any compatible broker (Redpanda, Iggy, etc.).

Complex aggregation is not Cloacina's concern. Whatever upstream process handles it pushes the result to a passthrough accumulator (direct socket) or writes to a Kafka topic. Cloacina consumes either through the same trait.

### Execution Model

A computation graph is a **compiled async function**. The `#[computation_graph]` macro resolves the node dependency graph at compile time and generates a single async function that executes the entire graph as one unit. There is no runtime graph traversal, no per-node dispatching, no routing between steps. The reactor calls one function; the function runs to completion.

```rust
// The graph topology is declared once — the single source of truth
#[computation_graph(
    react = when_any(alpha, beta, gamma),
    graph = {
        decision_engine(alpha, beta, gamma) => {
            Signal -> risk_check,
            NoAction -> audit_logger,
        },
        risk_check(gamma) => {
            Approved -> output_handler,
            Blocked -> alert_handler,
        },
    }
)]
mod market_maker {
    // Nodes are just async functions — no routing annotations
    // Entry node always fires — reactor already decided to execute
    async fn decision_engine(
        alpha: Option<&AlphaData>,
        beta: Option<&BetaData>,
        gamma: Option<&ExposureData>,
    ) -> DecisionOutcome {
        let output = compute(alpha, beta, gamma);
        if output.confidence > 0.8 {
            DecisionOutcome::Signal(output.into())
        } else {
            DecisionOutcome::NoAction(output.reason())
        }
    }

    async fn risk_check(signal: &Signal, gamma: Option<&ExposureData>) -> RiskCheck {
        if gamma.map(|g| g.exposure > 1000.0).unwrap_or(false) {
            RiskCheck::Blocked(RiskAlert { ... })
        } else {
            RiskCheck::Approved(signal.clone())
        }
    }

    async fn output_handler(signal: &Signal) -> OutputConfirmation {
        publish(signal).await;
        OutputConfirmation { ... }
    }

    async fn alert_handler(alert: &RiskAlert) -> AlertRecord {
        notify_ops(alert).await;
        AlertRecord { ... }
    }

    async fn audit_logger(reason: &NoActionReason) -> AuditRecord {
        log::info!("no action: {}", reason.reason);
        AuditRecord { ... }
    }
}

// What the macro compiles it into: one async function
async fn market_maker_compiled(cache: &InputCache) -> GraphResult {
    // Entry node always fires — reactor already decided to execute
    match decision_engine(cache.alpha, cache.beta, cache.gamma).await {
        DecisionOutcome::Signal(signal) => {
            match risk_check(&signal, cache.gamma).await {
                RiskCheck::Approved(approved) => output_handler(&approved).await,
                RiskCheck::Blocked(alert) => alert_handler(&alert).await,
            }
        }
        DecisionOutcome::NoAction(reason) => {
            audit_logger(&reason).await
        }
    }
}
```

**Graph topology is declared once** in the `#[computation_graph]` macro header. Nodes are pure async functions with no routing annotations. The topology declaration is the single source of truth — you read it once to understand the full graph.

**Routing uses Rust enums.** Nodes return enums for conditional paths (`Option<T>` for conditional propagation, custom enums for multi-path routing). The topology declaration maps enum variants to downstream nodes. The macro generates nested match arms.

**Compile-time validation:**
- Every node in the module must appear in the graph declaration (no orphans)
- Every node referenced in the graph must exist in the module (no dangling references)
- Every enum variant returned by a routing node must be wired to a downstream node (no unhandled variants)
- Return types match input types across edges (type safety)

**Terminal nodes** are the leaves of the graph — nodes with no dependents. The macro identifies them at compile time. When all terminal nodes complete, the graph is done. Their outputs are collected into the `GraphResult`, which the reactor uses to persist cache, flush batch accumulators, and emit audit events via Cloacina's existing infrastructure.

**Nodes are pure async functions.** They receive typed inputs and return typed outputs. If a node needs to communicate with an external system (write to another reactor's accumulator, publish to Kafka, call an API), it makes a normal async call via the API server WebSocket. One write path for all external communication — same auth, same audit. No framework handles, no context injection. The macro doesn't need to know about external calls.

The reactor has no special persistence logic beyond its own cache. State persistence is accumulator writes made by nodes via standard async I/O through the WebSocket layer.

The macro handles at compile time:
- **Topological sort** from the graph declaration → execution order
- **Enum variant routing** → nested match arms in the generated code
- **Fan-out** → multiple downstream calls from one enum variant or output
- **Fan-in** → node parameters assembled from multiple upstream results
- **Type checking** → return types match input types across edges, enum variants match graph wiring
- **Completeness validation** → every node in module appears in graph, every graph reference exists in module, every enum variant is wired
- **Terminal node detection** → identifies leaf nodes, collects their outputs into GraphResult

The reactor's job is trivial:
1. Wait for accumulator emission
2. Update the input cache
3. Check dirty flags (reaction criteria)
4. Call the compiled graph function
5. Persist cache to DAL, signal batch accumulators to flush, emit audit events

No runtime graph state. No checkpoint management. No pending sets. The complexity is in the macro, not the reactor.

### Routing Model

Routing is declared in the graph topology, not in nodes. Nodes are pure async functions that return Rust types — enums for multi-path routing, `Option<T>` for conditional propagation. The topology maps enum variants to downstream nodes.

- Nodes return enums or `Option<T>` — standard Rust types, no framework return types
- The graph declaration maps each enum variant to a downstream node
- `None` from `Option<T>` = conditional propagation (downstream doesn't fire)
- The macro generates nested match arms from the topology declaration
- The reactor never sees routing — it's compiled away

Testing is straightforward: give a node specific inputs, assert on the returned enum variant and its payload. Each node is a pure async function with no framework coupling.

### Recovery Model

The graph execution is the unit of recovery, not individual node firings. No internal execution tracking — the only question is: "did we finish processing this set of inputs?"

**Execution flow:**
1. Inputs arrive (boundaries from accumulators)
2. Compiled graph function executes (all nodes, as one async call)
3. Terminal nodes complete — any node can write to accumulators during execution (state persistence, external output, cross-reactor communication)
4. Graph returns `GraphResult` with terminal node outputs
5. **Persist**: reactor snapshots cache to DAL, signals batch accumulators to flush, emits audit events via Cloacina's existing infrastructure

On crash: the reactor restores its cache from the DAL and is immediately operational with the last known state. Accumulators restart independently and progressively freshen the cache. Any node that writes to external systems (accumulators, Kafka, APIs) must be idempotent in case the graph re-executes after a crash.

| Source type | Replay mechanism | Completion marker |
|-------------|-----------------|-------------------|
| Stream broker (Kafka, Redpanda, etc.) | Resume from last committed offset | Offset commit after graph completes |
| Socket/passthrough | Upstream re-sends (or lost — accepted tradeoff for zero-state) | None |
| Detector | Replay from boundary log | Boundary marked processed after graph completes |

### Reaction Criteria

A computation graph is a function with a cache. Accumulators update cache entries. Reaction criteria decide when to call the function.

The reactor maintains per-source state:
- **Cache**: `HashMap<SourceName, Option<Boundary>>` — last seen value per source
- **Dirty flags**: `HashMap<SourceName, bool>` — has this source emitted since the last graph execution?

Two reaction criteria:

- **`when_any`**: Fire if any dirty flag is set. The graph receives the full cache (fresh value for the triggering source, last-seen values for everything else). At least one input changed on every firing.
- **`when_all`**: Fire if all dirty flags are set. The graph receives the full cache with every value fresh. Naturally handles initialization — all flags start unset, so the graph doesn't fire until every source has emitted at least once.

After firing, all dirty flags are cleared. The cache values persist (they're the "last seen" for the next firing).

This is the entire coordination state: one boolean per source. No timestamps, no sequence coordination, no complex streaming logic. Anything more complex belongs upstream in the accumulator or in an external stream processor.

### Input Strategy

The reactor receives boundaries continuously from accumulators while it may be blocked on a graph execution. The **input strategy** controls how the reactor handles incoming data between executions:

- **`latest`** (default): One slot per source, overwritten on each update. The reactor always fires with the freshest value. Intermediate values are collapsed — if 10 orderbook updates arrive during one graph execution, only the 10th matters. Correct for reactive workloads where stale intermediate states have no value.
- **`sequential`**: Boundaries are preserved in order, one graph execution per boundary. No collapsing. Correct for workloads where every event must be processed (audit trails, compliance logging, event sourcing).

This is NOT concurrency. Concurrency in Cloacina means how many workflow executions can run simultaneously (claiming, slots, parallel runners). Input strategy is about how the reactor receives and retains data between executions.

## Ontology

All new terms are distinct from existing Cloacina concepts — no namespace collisions.

| New (continuous) | Existing (cron/trigger) | Distinction |
|---|---|---|
| **ComputationGraph** | Workflow | Compiled async function from topology declaration vs runtime-walked DAG |
| **Node** | Task | Pure async function in a graph vs step in a workflow. No framework return types — uses standard Rust enums and Option. |
| **Reactor** | Trigger | Long-lived event-driven process vs poll-based check |
| **Reaction criteria** | Trigger rules | when_any/when_all on dirty flags vs cron/poll conditions |
| **Reactive Scheduler** | Unified Scheduler | Spawns/supervises computation graph processes vs manages cron/trigger workflows |
| **Accumulator** | — | New concept (no existing equivalent) |
| **Boundary** | — | Standard Rust struct with serde derives, serialized by accumulator for transport (no existing equivalent) |
| **Input strategy** | Concurrency | latest/sequential (how reactor handles data) vs slots (how many run at once) |

## Developer Experience

### Accumulator definition

Four accumulator macros, one per class. Each is explicit about what kind of accumulator it creates:

```rust
// Stream consumer — reads from a broker topic
#[stream_accumulator(type = "kafka", topic = "market.orderbook")]
fn alpha(event: OrderBookUpdate) -> AlphaData {
    AlphaData { top_high: event.best_ask, top_low: event.best_bid }
}

// Passthrough — socket-only, no event loop
#[passthrough_accumulator]
fn beta(event: PricingUpdate) -> BetaData {
    BetaData { estimate: event.mid_price }
}

// Stateful stream consumer — checkpointed running aggregate
#[stream_accumulator(type = "kafka", topic = "fills", state = f64)]
fn gamma(event: FillEvent, exposure: &mut f64) -> ExposureData {
    match event.side {
        Side::Buy  => *exposure += event.qty,
        Side::Sell => *exposure -= event.qty,
    }
    ExposureData { exposure: *exposure }
}

// Polling — timer-based query of external system
#[polling_accumulator(interval = "5s")]
async fn config(ctx: &PollingContext) -> Option<ConfigData> {
    let row = ctx.db.query("SELECT ...").await.ok()?;
    Some(ConfigData { ... })
}
```

### Computation graph definition

The graph is a Rust module. Topology is declared once in the macro header — the single source of truth. Nodes are pure async functions with no framework annotations or return types:

```rust
#[computation_graph(
    react = when_any(alpha, beta, gamma),
    graph = {
        decision_engine(alpha, beta, gamma) => {
            Signal -> risk_check,
            NoAction -> audit_logger,
        },
        risk_check(gamma) => {
            Approved -> output_handler,
            Blocked -> alert_handler,
        },
    }
)]
mod market_maker {
    // Routing node — returns an enum, topology maps variants to downstream
    // Graph always fires when reactor calls it — no Option wrapper on entry nodes
    async fn decision_engine(
        alpha: Option<&AlphaData>,
        beta: Option<&BetaData>,
        gamma: Option<&ExposureData>,
    ) -> DecisionOutcome {
        let output = compute(alpha, beta, gamma);
        if output.confidence > 0.8 {
            DecisionOutcome::Signal(output.into())
        } else {
            DecisionOutcome::NoAction(output.reason())
        }
    }

    // Second routing point
    async fn risk_check(signal: &Signal, gamma: Option<&ExposureData>) -> RiskCheck {
        if gamma.map(|g| g.exposure > 1000.0).unwrap_or(false) {
            RiskCheck::Blocked(RiskAlert { ... })
        } else {
            RiskCheck::Approved(signal.clone())
        }
    }

    // Terminal nodes — no further downstream, outputs collected into GraphResult
    async fn output_handler(signal: &Signal) -> OutputConfirmation {
        publish(signal).await;
        OutputConfirmation { ... }
    }

    async fn alert_handler(alert: &RiskAlert) -> AlertRecord {
        notify_ops(alert).await;
        AlertRecord { ... }
    }

    async fn audit_logger(reason: &NoActionReason) -> AuditRecord {
        log::info!("no action: {}", reason.reason);
        AuditRecord { ... }
    }
}
```

### Packaging strategy

Embedded first (wired in code), packaged deployment second. Same pattern Cloacina followed with workflows — `#[workflow]` macro first, manifest/reconciler abstraction layered on once the primitives are stable. The packaging shape will emerge from implementation. Uses fidius for Rust packaging and PyO3 executor for Python — the reconciler gets a callable async function through the same interface regardless of language.

## Components to Design

Each component gets its own specification document during discovery/design. The specs must be detailed enough that implementation initiatives can be scoped directly from them.

### 1. Computation Graph & `#[computation_graph]` Macro

The computation graph is a Rust module with a topology declaration in the macro header and pure async functions as nodes. The macro resolves topology at compile time, validates completeness and type safety, and generates a single async function with nested match arms.

**Key design questions:**
- How does the macro parse the `graph = { ... }` topology declaration? What's the syntax for linear chains vs routing vs fan-in?
- How does it validate enum variant coverage — that every variant of every routing enum is wired?
- How does `#[node(blocking)]` interact with the compiled function? Does the macro detect it and wrap in `spawn_blocking`?
- Can a graph contain sub-graphs (nested modules), or is it always a flat set of functions?
- How do Python nodes work within a computation graph? PyO3 decorators generating the same topology?

### 2. Accumulator Trait & Built-in Implementations

The trait-based abstraction for reading from a WAL source, aggregating, and checkpointing. The trait assumes it has a WAL to read from and a place to checkpoint.

**Key design questions:**
- What's the trait surface? `poll() -> Option<Boundary>`? `receive(raw_event) -> Option<Boundary>`? Does the accumulator pull from its WAL or get events pushed to it?
- How does Kafka offset tracking integrate? If the accumulator is a Kafka consumer, offset commits = checkpoints. Does the trait abstract over this or expose it?
- For RunningAggregate: the aggregation function (sum, count, etc.) needs to be user-defined or selected from built-ins. What's the interface?
- How does the reactor know an accumulator has new data? The accumulator pushes to the reactor's channel — but what's the channel type? mpsc? broadcast (for multiple reactors)?
- What's the lifecycle? Accumulator starts → registers on API server → connects to source → catches up from last checkpoint → enters steady state → pushes boundaries to reactor
- API registration: accumulators register as named endpoints on the API server for external producers to push through (with auth). What's the registration protocol? WebSocket? gRPC?

### 3. Reactor

Long-lived process, one per computation graph. Receives boundaries from accumulators, maintains cache + dirty flags, evaluates reaction criteria, calls the compiled graph, commits on completion. Exposes a manual channel for force-fire.

```rust
// Reactor::run()
loop {
    select! {
        boundary = accumulator_channel.recv() => {
            cache.update(boundary.source, boundary);
            dirty.set(boundary.source, true);
            if reaction_criteria.met(&dirty) {
                let result = graph.execute(&cache).await;
                advance_offsets(&result);
                dirty.clear_all();
            }
        }
        injected = manual_channel.recv() => {
            // Force-fire with injected or current state
            let result = graph.execute(&injected.unwrap_or(&cache)).await;
            advance_offsets(&result);
        }
        _ = shutdown.recv() => break,
    }
}
```

**Key design questions:**
- Accumulators are always independent — each reactor owns its own accumulator instances. Fan-out for multiple reactors consuming the same source is handled at the registration layer: broker-backed accumulators use consumer groups, push/socket-backed accumulators register by name on the API server which broadcasts to all accumulators registered with that name. The accumulator itself has no broadcast logic.
- Input strategy: `latest` (one slot per source, overwrite) vs `sequential` (preserve every boundary). How does this change the reactor's internal loop? Does `sequential` need backpressure to the accumulators?
- How does the reactor interact with DefaultRunner lifecycle (start, shutdown, crash)?
- API registration: the reactor registers itself as a named endpoint on the API server (like accumulators do) for manual operations. What's the registration protocol? What operations are exposed (force-fire, inject state, status, pause/resume)?
- Manual channel: what's the API? `reactor.force_fire()`, `reactor.fire_with(state)`? Are these exposed via the API server WebSocket?

### 4. Persistence & Recovery

Covered by S-0004 (accumulator checkpoints, health states) and S-0005 (reactor cache DAL, startup health gate). The cross-cutting recovery sequence:

1. Reactor loads cache from DAL (instant)
2. Reactor spawns accumulators, each restores from own checkpoint
3. Accumulators connect to sources, signal healthy
4. All healthy → reactor goes live, starts evaluating reaction criteria

**Remaining design questions:**
- Detector-sourced accumulators: need a small boundary log in the DAL. How small? Just the latest boundary per source?
- State accumulators: persist VecDeque to DAL on every write. Recovery = load from DAL, emit to reactor.
- Idempotency: nodes making external writes (HTTP, Kafka) must be idempotent in case the graph re-executes after crash.
- Target MTR?

### 5. Integration Points

How accumulators, reactors, and computation graphs connect to the rest of Cloacina and external systems.

**Key design questions:**
- Detector handoff: a detector runs on the unified scheduler, completes, and pushes a boundary into an accumulator. What's the channel? How does the unified scheduler know to route to an accumulator?
- Stream consumer lifecycle: who owns the Kafka/Redpanda consumer? The accumulator? A shared consumer pool? How are consumer groups managed?
- Push channel API: for Rust callers pushing boundaries directly into an accumulator. What does the API look like?
- Python bindings: how does a Python process push boundaries? PyO3 wrapper around the accumulator's push channel?
- Output: how do nodes emit results to external systems (Kafka producer, outbox table, API call)?

### 6. Packaging & Deployment

ManifestV2 extensions for computation graph components (accumulators, reactor, graph). Reconciler spawns long-lived accumulator and reactor processes on package load. Daemon and server mode support.

**Key design questions:**
- How does the manifest declare a Kafka-sourced accumulator? Topic, consumer group, accumulator type, checkpoint config?
- How does the manifest declare a detector-sourced input? References the detector workflow (running on the unified scheduler) and the boundary format?
- Can a single package contain both workflows (unified scheduler) and computation graphs (reactor)?
- How does the reconciler know to spawn accumulators + reactor vs register with the unified scheduler?
- What's the lifecycle when a package is hot-reloaded? Do accumulators and reactors restart?

### 7. Reference Implementation

Multi-stream decision engine exercising all capabilities. Three sources (Kafka-sourced with different update frequencies), decision engine with parameterized model, output task. Deliberately abstract domain language.

**Key design questions:**
- Should this be a demo (examples directory) or an integration test (or both)?
- The materializers are now accumulators (LatestValue for alpha/beta, RunningAggregate for gamma) — not Cloacina tasks. The Cloacina graph is: 3 accumulator outputs → decision engine → output task. Is this the right split, or do some materializers still live as Cloacina tasks?
- What's the mock data generation strategy? Kafka test fixtures? In-process mock producers?
- How do we instrument end-to-end latency without perturbing the measurement?

## Alternatives Considered

### Build a dedicated reactive stream-processing framework

Rejected. Reactive graph frameworks (node-graph with data flowing through links) sound natural but break down for these workloads: independent streams need to be correlated through shared state at decision time, not joined through streaming edges. The graph topology becomes decorative. Cloacina's model — accumulators feed a cached input snapshot, reaction criteria fire a compiled computation graph — is the right abstraction.

### Build a separate "strategy engine" outside cloacina

Rejected. The scheduling infrastructure (graph, persistence, crash recovery, packaging, deployment modes) already exists or is being built in Cloacina. A separate engine duplicates all of this. Better to extend what we have.

### Own all accumulation internally

Rejected. Simple accumulation (LatestValue, RunningAggregate) is trivial and belongs in Cloacina — users shouldn't need external infrastructure to track a running sum. But complex aggregation (windowed, watermarked, multi-partition exactly-once) is not our problem to solve. The accumulator trait reads from a source (Kafka topic, Postgres, push channel). Whatever writes to that source is upstream and invisible. No special integration, no escape hatch — just a Kafka topic that something else writes to.

### Embed the continuous scheduler in the existing unified scheduler

Rejected. The unified scheduler is designed for cron/trigger workloads with database polling, claim semantics, heartbeats, and horizontal scaling. These are irrelevant to (and actively harmful for) continuous reactive scheduling. Bolting continuous scheduling onto that infrastructure means inheriting its latency characteristics and dispatch model. A standalone component with its own event loop — just cache, dirty flags, and a compiled workflow call — is radically simpler and faster.

### Re-apply the archived implementation and extend it

Rejected as primary framing. The archive validates the approach but was designed around timer-based ledger polling with a runtime graph walker. The new architecture is fundamentally different: long-lived accumulator and reactor processes, compile-time graph resolution into a single async function, no runtime scheduler. The archive is reference material, not a starting point.

## Implementation Plan

This initiative stays in **discovery** while we work through the component designs. The workflow is:

1. **Discovery**: Walk through each component (1-7 above), discuss design questions, make decisions
2. **Specifications**: Write detailed specs for each component as we settle the design
3. **Decomposition**: Break the implementation into focused initiatives, each scoped by one or more specs:
   - `#[computation_graph]` macro (topology declaration, compile-time resolution, enum routing, type validation, completeness checks, blocking node wrapping)
   - Reactor (receiver/strategy/executor, cache, dirty flags, reaction criteria, batch flush, health states, DAL persistence, manual channel)
   - Accumulator trait + six classes + stream backend factory + health states + DAL persistence
   - Integration points (detector handoff, push channel, API server registration, Python bindings)
   - Packaging & deployment (manifest, reconciler, daemon/server)
   - Reference implementation & benchmarks
4. **Transition**: Move this meta-initiative to completed; child initiatives carry the implementation

## Prior Art

An archived implementation on `archive/main-pre-reset` (~5000 LOC, 17 files) covers:
- `DataSourceGraph`, `ComputationBoundary` (5 kinds), `SignalAccumulator` (Simple + Windowed), `TriggerPolicy`, `Watermark`, `LateArrivalPolicy`, `ExecutionLedger`
- `ContinuousScheduler` with timer-based polling, crash recovery, persistence (boundary WAL, detector state, edge drain cursors)
- `LedgerTrigger` for derived data sources
- 480+ tests including e2e crash recovery against Postgres

Key commits: `bbc6e0a` (core), `8a6bf67` (watermarks), `ea1e50d` (persistence), `ef0bdcd` (DefaultRunner wiring).

Known limitations that motivate the redesign: timer-based polling (10ms latency floor), no push sources, no conditional propagation, no execution mode control, no packaging support.
