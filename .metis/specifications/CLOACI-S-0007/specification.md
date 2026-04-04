---
id: integration-points
level: specification
title: "Integration Points"
short_code: "CLOACI-S-0007"
created_at: 2026-04-04T17:23:19.020445+00:00
updated_at: 2026-04-04T17:23:19.020445+00:00
parent: CLOACI-I-0069
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Integration Points

## Overview

This spec defines how the computation graph system (accumulators, reactors, compiled graphs) connects to existing Cloacina infrastructure and external systems. Four integration points:

1. **API server registration & authZ** — WebSocket endpoints for accumulators and reactors, per-endpoint authorization
2. **Detector handoff** — unified scheduler to accumulator bridge via WebSocket
3. **Python bindings** — PyO3 wrapping for Python computation graphs
4. **Server lifecycle** — spawning, managing, and shutting down continuous processes within the API server (Postgres-only, loaded via reconciler)

All writes to accumulators (external producers, detectors, nodes within graphs, cross-reactor) go through the API server WebSocket. One write path, one auth model, one place to audit. This simplifies the system and ensures it works unchanged when workflows are distributed across systems in the future.

### Relationship to other specs

- **CLOACI-S-0004** (Accumulator Trait) — defines accumulator registration, health states, socket receiver
- **CLOACI-S-0005** (Reactor) — defines reactor registration, manual channel, health states
- **CLOACI-S-0006** (Computation Graph Macro) — defines the compiled function that the reactor calls
- **CLOACI-I-0049** (API Server) — the existing HTTP/WebSocket server that hosts accumulator and reactor endpoints, extended with WebSocket support and per-endpoint authZ

## 1. API Server Registration

Accumulators and reactors register as named WebSocket endpoints on the API server. The API server handles auth and proxies messages to the registered process. This is the external-facing interface for the computation graph system.

### Registration Protocol

On startup, each accumulator and reactor opens an internal channel to the API server and sends a registration message:

```rust
enum RegistrationMessage {
    RegisterAccumulator(AccumulatorRegistration),
    RegisterReactor(ReactorRegistration),
    Deregister { name: String },
    HealthUpdate { name: String, health: HealthState },
}
```

The API server maintains a registry of active accumulators and reactors. External clients connect via WebSocket to named endpoints:

```
ws://api-server/v1/accumulator/{name}     // push events to accumulator
ws://api-server/v1/reactor/{name}          // manual operations on reactor
ws://api-server/v1/reactor/{name}/state    // inspect reactor cache
```

### Accumulator Endpoints

External producers connect to `ws://api-server/v1/accumulator/{name}` and push serialized events. The API server:

1. Authenticates the WebSocket connection (PAK auth, same as HTTP endpoints)
2. Looks up all accumulators registered under `{name}`
3. Broadcasts each incoming message to all matching accumulators

Multiple accumulators with the same name = broadcast. This handles the fan-out case where multiple reactors consume the same source.

Wire format: the same dual-format as internal boundaries — bincode in release, JSON in debug. The producer must match the build profile of the server, or use JSON (always accepted as fallback).

### Authorization

Each accumulator and reactor has its own auth policy. Authorization is per-endpoint, not per-type:

```rust
struct AccumulatorAuthPolicy {
    name: String,                    // accumulator name
    allowed_producers: Vec<String>,  // PAK key IDs authorized to push to this accumulator
}

struct ReactorAuthPolicy {
    name: String,                    // reactor name
    allowed_operators: Vec<String>,  // PAK key IDs authorized for manual operations
    allowed_operations: Vec<ReactorOp>, // which operations each key can perform
}

enum ReactorOp { ForceFire, FireWith, GetState, Pause, Resume, GetHealth }
```

A producer connecting to accumulator "alpha" must present a PAK key authorized specifically for "alpha." A different key may access "beta" but not "alpha." Same for reactor operations — force-firing "market_maker" requires specific authorization for that reactor and that operation.

Auth policies are declared in the computation graph's package metadata and enforced by the API server on WebSocket connection. This is the same PAK auth model used for HTTP endpoints, extended to WebSocket connections.

All writes go through this auth layer — external producers, detectors, nodes within graphs writing to other accumulators, cross-reactor communication. One path, one auth model.

### Reactor Endpoints

Operators connect to `ws://api-server/v1/reactor/{name}` and send commands:

```rust
// Client -> Server
enum ReactorCommand {
    ForceFire,
    FireWith { cache: HashMap<String, Vec<u8>> },
    GetState,
    Pause,
    Resume,
    GetHealth,
}

// Server -> Client
enum ReactorResponse {
    Fired { result: String },
    State { cache: HashMap<String, String> },  // JSON for readability
    Health { state: ReactorHealth },
    Error { message: String },
}
```

The API server proxies commands to the reactor's manual channel and returns responses. In debug mode, state and cache are returned as JSON for human readability.

### Health Reporting

Accumulators and reactors push health updates to the API server via the registration channel. The API server exposes aggregate health via REST:

```
GET /v1/health/accumulators     // list all accumulators + health states
GET /v1/health/reactors         // list all reactors + health states
GET /v1/health/reactors/{name}  // single reactor health + its accumulators
```

This integrates with Cloacina's existing `/health` and `/ready` endpoints — the server reports unhealthy if any reactor is in a non-live state.

## 2. Detector Handoff

A detector is a regular Cloacina workflow running on the unified scheduler (cron or trigger-based). When it completes, it may produce data that should feed into an accumulator for a computation graph.

The handoff uses the same WebSocket path as any other producer. The detector writes to an accumulator via the API server's WebSocket endpoint. No special infrastructure — detectors are just another authorized producer.

```rust
// In a detector workflow task
#[task]
async fn detect_table_changes(ctx: Context) -> Result<Context> {
    let changes = query_for_changes(&ctx).await?;
    if !changes.is_empty() {
        let boundary = TableChanges { rows: changes };
        // Write to accumulator via API server WebSocket — same path as any external producer
        ctx.ws_client()
            .send("table_watcher", &serialize(&boundary)?)
            .await?;
    }
    Ok(ctx)
}
```

The unified scheduler doesn't know about accumulators or reactors. It runs the detector workflow like any other workflow. The detector's task writes to the accumulator via the API server WebSocket as a side effect. Same auth, same path, same audit trail as external producers.

### When to use detectors vs stream accumulators

| Approach | Use case |
|----------|----------|
| Stream accumulator | Source has a stream protocol (Kafka, Redpanda). Accumulator connects directly. |
| Polling accumulator | Source can be queried on a timer (database, API). Accumulator polls directly. |
| Detector + passthrough accumulator | Source requires complex detection logic (diff tables, check file hashes, run queries with business logic). The detector workflow handles the complexity; the accumulator just receives the result. |

Detectors are for when the "what changed?" logic is too complex for a simple accumulator poll function.

## 3. Python Bindings

Computation graphs can be authored in Python using PyO3 bindings. The Python experience mirrors the Rust experience: accumulators are decorated functions, the graph topology is declared, nodes are async functions.

### Python Accumulators

```python
@stream_accumulator(type="kafka", topic="market.orderbook")
def alpha(event: OrderBookUpdate) -> AlphaData:
    return AlphaData(top_high=event.best_ask, top_low=event.best_bid)

@passthrough_accumulator
def beta(event: PricingUpdate) -> BetaData:
    return BetaData(estimate=event.mid_price)

@stream_accumulator(type="kafka", topic="fills", state=float)
def gamma(event: FillEvent, exposure: float) -> tuple[ExposureData, float]:
    if event.side == "buy":
        exposure += event.qty
    else:
        exposure -= event.qty
    return ExposureData(exposure=exposure), exposure
```

Python accumulators always run in `spawn_blocking` (GIL constraint). The decorator generates the same `Accumulator` trait implementation as the Rust macro, wrapping the Python function.

### Python Computation Graphs

```python
@computation_graph(
    react=when_any("alpha", "beta", "gamma"),
    graph={
        "decision_engine": {"inputs": ["alpha", "beta", "gamma"], "routes": {
            "Signal": "output_handler",
            "NoAction": "audit_logger",
        }},
    }
)
class MarketMaker:
    async def decision_engine(self, alpha, beta, gamma):
        output = compute(alpha, beta, gamma)
        if output.confidence > 0.8:
            return ("Signal", Signal(...))
        else:
            return ("NoAction", NoActionReason(...))

    async def output_handler(self, signal):
        await publish(signal)
        return OutputConfirmation(...)

    async def audit_logger(self, reason):
        logger.info(f"no action: {reason}")
        return AuditRecord(...)
```

Python nodes return `(variant_name, value)` tuples for routing — the decorator maps variant names to downstream nodes. All Python nodes run in `spawn_blocking` due to the GIL.

The topology declaration in Python uses a dict instead of Rust's macro syntax, but maps to the same compiled structure. The PyO3 executor wraps the class into a callable async function that the reactor can call identically to a Rust-compiled graph.

One language per package — either Rust or Python. Mixed Rust/Python compilation within a single package is not supported. The reactor doesn't care about language — it calls a compiled async function — but packaging and compilation toolchains are separate.

## 4. Reactive Scheduler & Server Lifecycle

The **Reactive Scheduler** is the computation graph coordinator — the reactive counterpart to the unified scheduler. It spawns and supervises accumulators + reactors from packages loaded via the reconciler.

```
Reconciler → package loaded → routes to:
  ├── Unified Scheduler (if package contains workflows/triggers)
  └── Reactive Scheduler (if package contains computation graphs)
```

Both schedulers run within the same API server process, share the same DAL (Postgres) and tokio runtime.

Computation graphs are a **server-mode feature** (Postgres-only). They are loaded via the reconciler when packages are uploaded, not via daemon filesystem watching.

### Startup

When a package containing a computation graph is loaded via the reconciler:

1. Reconciler routes the package to the Reactive Scheduler
2. Reactive Scheduler spawns accumulator tasks — each connects to its source, registers as WebSocket endpoint
3. Reactive Scheduler spawns reactor task — loads cache from DAL (Postgres), waits for accumulators to go healthy
4. Reactor enters warming state, then live when all accumulators healthy
5. Health reported via `/v1/health/reactors/{name}`

### Shutdown

On graceful shutdown (SIGTERM, `cloacinactl stop`) or package removal:

1. Reactive Scheduler sends shutdown signal to reactor + accumulator tasks
2. Reactor stops accepting new boundaries, finishes current execution if in progress
3. Reactor persists final cache snapshot to DAL
4. Accumulators disconnect from sources, persist final checkpoints
5. Accumulators and reactor deregister from API server WebSocket registry
6. Tasks complete

### Package Update

When a package is updated (new version uploaded via reconciler):

1. Reconciler notifies the Reactive Scheduler of package update
2. Existing reactor + accumulators receive shutdown signal
3. Wait for current execution to complete
4. Shut down old tasks
5. Spawn new tasks from updated code
6. New reactor loads cache from DAL (state from the old instance)
7. New accumulators restore from checkpoints
8. Resume operation

The DAL (Postgres) provides continuity across package updates — the new reactor instance picks up where the old one left off.

### Supervision

The Reactive Scheduler supervises accumulator and reactor tasks:

- If an accumulator task panics: restart it. The accumulator restores from checkpoint. The reactor enters degraded state until the accumulator reconnects and goes healthy.
- If a reactor task panics: restart it. The reactor restores cache from DAL. Accumulators continue running independently — they push boundaries that queue until the reactor is back.
- If the API server process crashes: standard process restart (k8s restarts the pod). All tasks restart from their persisted state in Postgres.

## Constraints

### Technical Constraints

- All writes to accumulators go through the API server WebSocket — one write path, one auth model, one audit trail. No in-process shortcuts. This ensures the system works unchanged when workflows are distributed across processes or systems.
- Per-endpoint authZ adds overhead per WebSocket connection (PAK key lookup, policy check). This is a one-time cost on connection, not per-message. Negligible for long-lived WebSocket connections.
- Python nodes add `spawn_blocking` overhead on every call. For latency-sensitive computation graphs, Rust is preferred. Python is suitable for prototype/development and for graphs where individual node latency is dominated by I/O (database queries, API calls), not computation.
- One language per package. Mixed Rust/Python not supported.
- Package update creates a brief gap between old tasks shutting down and new tasks starting. During this gap, push-based events to passthrough accumulators are lost (no buffer). Stream-backed accumulators resume from their broker offset so no events are lost.
- Computation graphs are server-mode only (Postgres). Not available in daemon mode (SQLite).
- The detector handoff via WebSocket is fire-and-forget from the detector's perspective. If the accumulator is down, the WebSocket send will fail. The detector should handle this (log, retry, or accept the loss).
