---
id: accumulator-trait-built-in
level: specification
title: "Accumulator Trait & Built-in Implementations"
short_code: "CLOACI-S-0004"
created_at: 2026-04-04T15:24:04.695947+00:00
updated_at: 2026-04-04T15:24:04.695947+00:00
parent: CLOACI-I-0069
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Accumulator Trait & Built-in Implementations

## Overview

An accumulator is a long-lived process that consumes events from a source, optionally aggregates them, and pushes typed boundaries to a reactor. Accumulators are the ingestion layer for computation graphs — they sit between external data sources and the reactor's input cache.

Every accumulator has two input paths:
- **Event loop** (optional): the accumulator actively pulls from a source (stream broker topic, Postgres query, etc.). If no `run` function is defined, no event loop is started.
- **Receive socket**: a message endpoint proxied through the API server's WebSocket layer. External producers and other accumulators can push directly. Always available.

Both paths feed through the same `process()` function and produce the same typed boundary output (`Self::Output`). This is enforced by the trait — there is one `Event` type in and one `Output` type out, regardless of whether the event came from the event loop or the receive socket. The accumulator pushes boundaries to the reactor via a serialized channel (bincode for speed, small messages).

### Relationship to other specs

- **CLOACI-I-0069** (parent initiative) — defines the overall architecture and how accumulators fit into the process model
- Reactor spec (forthcoming) — defines how the reactor receives boundaries from accumulators and manages the input cache
- Boundary Types spec (forthcoming) — defines `#[derive(Boundary)]` and the typed structs that flow between accumulators and nodes

## Trait Definition

```rust
/// An accumulator is a long-lived process that consumes from a source
/// and pushes typed boundaries to a reactor.
trait Accumulator: Send + 'static {
    /// The raw event type consumed from the source
    type Event: DeserializeOwned + Send;

    /// The typed boundary produced for the reactor
    type Output: Boundary + Serialize + Send;

    /// Process a received event and optionally produce a boundary.
    /// Called for both event-loop events and receive-socket events.
    /// Returns None if the event doesn't produce a boundary (e.g., filtered out).
    fn process(&mut self, event: Self::Event) -> Option<Self::Output>;

    /// Optional: active event loop that pulls from a source and pushes
    /// raw events into the merge channel. The runtime calls process() on
    /// the other end — run() should NOT call process() directly.
    /// If not overridden, the accumulator only receives via its socket.
    async fn run(&mut self, _ctx: &AccumulatorContext, _events: mpsc::Sender<Self::Event>) -> Result<(), AccumulatorError> {
        // Default: no active event loop. Accumulator is socket-only (passthrough).
        std::future::pending().await
    }

    /// Called on startup before run() or first receive.
    /// Use to restore state from last checkpoint.
    async fn init(&mut self, _ctx: &AccumulatorContext) -> Result<(), AccumulatorError> {
        Ok(())
    }
}
```

### AccumulatorContext

Provided to the accumulator by the runtime. Contains handles for pushing boundaries and managing lifecycle:

```rust
struct AccumulatorContext {
    /// Send a boundary to the reactor. Serializes via bincode and sends
    /// over the channel. This is the only way boundaries reach the reactor.
    output: BoundarySender,

    /// Accumulator's name (used for API server registration and logging)
    name: String,

    /// Handle to persist accumulator state (for stateful accumulators).
    /// Wraps the DAL for simple key-value checkpoint storage.
    checkpoint: CheckpointHandle,

    /// Shutdown signal — accumulator should exit run() when this fires
    shutdown: tokio::sync::watch::Receiver<bool>,
}
```

### BoundarySender

The output channel to the reactor. Serializes boundaries before sending — wire format depends on build profile:

- **Release**: bincode (compact binary, fast serialization)
- **Debug**: JSON (human-readable, inspectable in logs and debuggers)

Same pattern as fidius wire format. The `Boundary` derive macro generates both `Serialize` implementations. The sender picks the format at compile time.

```rust
struct BoundarySender {
    inner: mpsc::Sender<(SourceName, Vec<u8>)>,
    source_name: String,
}

impl BoundarySender {
    fn send<T: Boundary + Serialize>(&self, boundary: T) -> Result<(), SendError> {
        #[cfg(debug_assertions)]
        let bytes = serde_json::to_vec(&boundary)?;
        #[cfg(not(debug_assertions))]
        let bytes = bincode::serialize(&boundary)?;

        self.inner.send((self.source_name.clone(), bytes))?;
        Ok(())
    }
}
```

This applies to all boundary serialization — accumulator-to-reactor channels, receive socket messages, and stream broker transport. Debug builds are readable everywhere; release builds are fast everywhere.

The reactor deserializes on the other end into the expected type using the matching format. Type mismatches are caught at compile time by the `#[computation_graph]` macro (which validates that accumulator output types match node input types).

## Receive Socket

Every accumulator has a receive socket — a message endpoint that accepts pushed events. The socket is proxied through the API server's WebSocket layer, which handles authentication and routing.

### Message Contract

Events pushed to the receive socket must conform to the accumulator's `Event` type. The wire format:

```rust
struct AccumulatorMessage {
    /// Serialized event payload (bincode)
    payload: Vec<u8>,
}
```

The accumulator deserializes the payload into `Self::Event` and calls `self.process(event)`. If `process` returns `Some(boundary)`, the boundary is sent to the reactor.

### Registration

On startup, the accumulator registers with the API server:

```rust
struct AccumulatorRegistration {
    name: String,            // unique name, used for routing
    event_schema: Schema,    // schema of Self::Event, for validation/discovery
    output_schema: Schema,   // schema of Self::Output, for downstream validation
}
```

Multiple accumulators can register with the same name (e.g., two reactors consuming the same source). The API server broadcasts incoming messages to all accumulators registered under that name. The accumulator itself has no broadcast logic.

### External Producer Flow

```
External producer → WebSocket to API server (auth) → API server looks up "alpha"
    → broadcasts to all accumulators registered as "alpha"
    → each accumulator: deserialize → process() → send boundary to its reactor
```

## Runtime Process

Each accumulator runs as **three tokio tasks** connected by a merge channel. The event loop and socket receiver run independently and never block each other. All events merge into a single channel for sequential processing.

```
┌─────────────────┐
│  Event loop task │──→ mpsc<Event> ──┐
│  (optional)      │                  │     ┌─────────────────┐
└─────────────────┘                  ├────→│  Processor task  │──→ BoundarySender ──→ Reactor
┌─────────────────┐                  │     │  (calls process) │
│  Socket task     │──→ mpsc<Event> ──┘     └─────────────────┘
│  (always active) │
└─────────────────┘
```

```rust
async fn accumulator_runtime<A: Accumulator>(mut acc: A, ctx: AccumulatorContext, socket: SocketReceiver) {
    // 1. Initialize — restore state from checkpoint
    acc.init(&ctx).await?;

    // 2. Create merge channel — both event loop and socket push raw events here
    let (event_tx, mut event_rx) = mpsc::channel::<A::Event>(1024);

    // 3. Spawn event loop (if accumulator defines one)
    let event_tx_loop = event_tx.clone();
    let loop_handle = tokio::spawn(async move {
        acc.run(&ctx, event_tx_loop).await
    });

    // 4. Spawn socket receiver
    let event_tx_socket = event_tx.clone();
    let socket_handle = tokio::spawn(async move {
        while let Some(msg) = socket.recv().await {
            if let Ok(event) = deserialize::<A::Event>(&msg.payload) {
                event_tx_socket.send(event).await.ok();
            }
        }
    });

    // 5. Processor — reads from merge channel, calls process() sequentially
    //    Single-threaded, no mutex, no contention on &mut self
    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                if let Some(boundary) = acc.process(event) {
                    ctx.output.send(boundary).ok();
                }
            }
            _ = ctx.shutdown.changed() => break,
        }
    }
}
```

**Key properties:**
- `process()` is called sequentially by the processor task — no concurrent access to `&mut self`, no mutex needed
- The event loop and socket receiver run concurrently and independently — a slow broker read doesn't block socket pushes, and vice versa
- The merge channel provides natural ordering (FIFO) and backpressure (bounded capacity)
- The accumulator author only writes `process()` — they never think about concurrency

Note: the `run()` signature changes slightly from the trait definition — it receives the event sender instead of calling `process()` directly:

```rust
async fn run(&mut self, ctx: &AccumulatorContext, events: mpsc::Sender<Self::Event>) -> Result<(), AccumulatorError> {
    let consumer = connect_to_broker(...).await?;
    loop {
        let msg = consumer.recv().await?;
        let event: Self::Event = deserialize(msg)?;
        events.send(event).await?;
    }
}
```

This keeps `run()` as a pure event producer — it doesn't call `process()` or send boundaries. It just feeds raw events into the merge channel.

### Health States

Each accumulator reports its health. The reactor watches accumulator health to gate its own startup (warming → live) and detect degradation.

```rust
enum AccumulatorHealth {
    Starting,       // loading checkpoint from DAL
    Connecting,     // checkpoint loaded, connecting to source. Socket already active.
    Live,           // connected, processing events, pushing boundaries
    Disconnected,   // was live, lost source connection. Socket still active. Retrying.
    SocketOnly,     // passthrough — no source to connect to. Healthy by definition.
}
```

| State | Socket active? | Event loop active? | Reactor considers healthy? |
|---|---|---|---|
| Starting | No | No | No |
| Connecting | Yes | Retrying | No |
| Live | Yes | Yes | Yes |
| Disconnected | Yes | Retrying with backoff | No (reactor goes Degraded) |
| SocketOnly | Yes | N/A | Yes |

Health is reported to the reactor via a `watch` channel and to the API server via registration updates. On sustained disconnection, the accumulator emits metrics/callbacks for alerting.

### Source Unavailability

If the event loop can't connect to its source (broker down, Postgres unreachable), the accumulator enters `Connecting` (on startup) or `Disconnected` (if previously live) state and retries with backoff. During retry, the socket receiver and processor tasks remain active — external producers can still push events and `process()` still runs.

## Accumulator Classes

Each accumulator class has a distinct shape, lifecycle, and configuration. The trait is the same underneath — the classes differ in what `run()` does and what state they manage.

### Passthrough

**Shape**: Socket receiver + process function. No event loop, no state, no factory.

```rust
struct PassthroughAccumulator<F, E, O>
where
    F: Fn(E) -> O,
{
    transform: F,
    _phantom: PhantomData<(E, O)>,
}
```

**Lifecycle**: spawn → register on API server → wait for socket events → process → push to reactor

**Config**: None. Fully determined by the macro at compile time. No factory needed.

### Stream

**Shape**: Stream backend + process function + optional state. Needs a backend trait and factory for pluggable broker implementations.

```rust
struct StreamAccumulator<B, F, E, O, S = ()>
where
    B: StreamBackend,
    F: Fn(E, &mut S) -> O,
{
    backend: B,
    transform: F,
    state: S,
    topic: String,
    group: String,
}
```

**StreamBackend trait** — abstracts over broker implementations:

```rust
trait StreamBackend: Send + 'static {
    /// Connect to the broker and subscribe to the topic
    async fn connect(config: &StreamConfig) -> Result<Self, StreamError>
    where Self: Sized;

    /// Receive the next message. Blocks until available.
    async fn recv(&mut self) -> Result<RawMessage, StreamError>;

    /// Commit the current offset. Called by the accumulator after processing.
    async fn commit(&mut self) -> Result<(), StreamError>;

    /// Get the current uncommitted offset (for recovery tracking)
    fn current_offset(&self) -> Option<Offset>;
}

struct StreamConfig {
    broker_url: String,
    topic: String,
    group: String,
    /// Backend-specific config (auth, TLS, partition assignment, etc.)
    extra: HashMap<String, String>,
}

struct RawMessage {
    payload: Vec<u8>,
    offset: Offset,
    timestamp: Option<i64>,
}
```

**Factory** — backends register themselves at startup:

```rust
type StreamBackendFactory = Box<dyn Fn(StreamConfig) -> BoxFuture<Result<Box<dyn StreamBackend>>>>;

struct StreamBackendRegistry {
    backends: HashMap<String, StreamBackendFactory>,
}

impl StreamBackendRegistry {
    fn register(&mut self, type_name: &str, factory: StreamBackendFactory);
    async fn create(&self, type_name: &str, config: StreamConfig) -> Result<Box<dyn StreamBackend>>;
}

// At startup:
registry.register("kafka", |config| Box::pin(KafkaBackend::connect(config)));
registry.register("redpanda", |config| Box::pin(RedpandaBackend::connect(config)));
registry.register("iggy", |config| Box::pin(IggyBackend::connect(config)));
```

The `#[stream_accumulator(type = "kafka", ...)]` macro generates code that looks up the backend by type from the registry at runtime. Kafka is the first implementation; others plug in via the same factory.

**Lifecycle**: spawn → register on API server → create backend via factory → connect to broker (retry with backoff on failure) → consume messages → process → push to reactor → commit offsets independently

**Config**: `type` (required — backend name from registry), `topic` (required), `group` (optional — auto-generated), `state` (optional — checkpoint type). Backend-specific config via extra key-value pairs or environment variables.

### Polling

**Shape**: Async poll function + interval timer + optional state. No factory needed — the function IS the polling logic.

```rust
struct PollingAccumulator<F, O, S = ()>
where
    F: AsyncFn(&PollingContext, &mut S) -> Option<O>,
{
    poll_fn: F,
    interval: Duration,
    state: S,
}

struct PollingContext {
    /// DAL handle for database queries
    db: DatabaseHandle,
    /// HTTP client for API calls
    http: reqwest::Client,
    /// Accumulator name (for logging)
    name: String,
}
```

**Lifecycle**: spawn → register on API server → init (restore state) → poll on interval → if Some(boundary), push to reactor → also accept socket events between polls

The poll function is user-defined and async — it can query a database, call an API, read a file. `None` return means nothing changed this poll cycle (no boundary pushed).

**Config**: `interval` (required), `state` (optional). No factory.

### Batch

**Shape**: Dormant until flushed. On flush signal from the reactor: drains all available events from its source (and any socket-buffered events), processes the entire batch in one call, emits one boundary. Goes dormant until next flush. This is the only accumulator class that receives a signal from the reactor — all others are fully independent.

```rust
struct BatchAccumulator<B, F, E, O>
where
    B: StreamBackend,  // optional — for source draining
    F: Fn(Vec<E>) -> O,
{
    backend: Option<B>,
    transform: F,
}
```

**On flush**: drain source (read all available messages from broker/socket buffer) → collect into `Vec<E>` → call transform with full batch → emit one boundary. No continuous buffering in memory — the source holds events between flushes.

**Lifecycle**: spawn → register on API server → optionally connect to source → wait (dormant) → reactor signals flush → drain source + socket buffer → process batch → emit boundary → wait (dormant) → repeat

**Config**: `type` (optional — stream backend for active source draining), `topic` (optional). The flush mechanism is "after every reactor execution" (more strategies may be added later).

Used for batch-oriented patterns: "all fills since last decision," "all order updates since last reconciliation." The graph execution defines the batch boundary. The source (Kafka, socket) holds events between flushes.

```rust
#[batch_accumulator(type = "kafka", topic = "fills")]
fn recent_fills(events: Vec<FillEvent>) -> FillSummary {
    // Called with ALL events gathered since last flush
    let total_qty: f64 = events.iter().map(|e| e.qty).sum();
    FillSummary { count: events.len(), total_qty }
}

// Identity batch — just pass through all events as a list
#[batch_accumulator(type = "kafka", topic = "raw_orders")]
fn pending_orders(events: Vec<RawOrder>) -> Vec<RawOrder> {
    events
}

// Graph receives one boundary per flush:
async fn reconciler(
    fills: &FillSummary,            // aggregated batch result
    orders: &Vec<RawOrder>,          // raw batch
) -> NodeResult { ... }
```

The function signature takes `Vec<Event>` (the whole batch), not a single event. It returns one boundary. This is fundamentally different from other accumulators which process event-by-event.

### State

**Shape**: Bounded list (VecDeque) that receives values from the collector (or mid-graph writes), appends to the list, evicts oldest when at capacity, and emits the entire list as its boundary. Persisted to the DAL. No event loop, no external source — only written to by the computation graph itself.

```rust
struct StateAccumulator<T: Boundary> {
    buffer: VecDeque<T>,
    capacity: usize,
}
```

On receive: append to buffer, evict oldest if `len > capacity`, emit the full `VecDeque<T>` as the boundary. On restart: load from DAL, emit to reactor.

**Lifecycle**: spawn → load from DAL → emit current list to reactor → wait for writes from collector/nodes → append, evict, emit, persist → repeat

**Config**: `capacity` controls list size:
- `capacity = 1`: just the last output (reconciliation, skip-if-unchanged)
- `capacity = N`: bounded history window (pattern detection, rate limiting, trend analysis)
- `capacity = -1`: unbounded — list grows without limit. Use with caution.
- No capacity / omitted: no state — the accumulator is effectively a passthrough for graph-internal writes (write-only sink, no history emitted back)

Used for cyclic state patterns — the graph's output feeds back as input on the next execution. The graph sees the full history window and can make decisions based on patterns, trends, reconciliation, rate limiting, etc.

### Custom

**Shape**: Developer implements the `Accumulator` trait directly. Full control over `run()`, `process()`, `init()`.

No macro, no factory. The runtime still wires in the socket receiver and API server registration automatically around the developer's implementation.

**Lifecycle**: Developer-defined via `run()`.

### Summary

| Class | Event loop | Backend trait | Factory | State | Socket | Reactor signal |
|-------|-----------|--------------|---------|-------|--------|---------------|
| Passthrough | No | No | No | No | Yes | No |
| Stream | Broker consumer via `StreamBackend` | Yes | Yes (registry) | Optional (checkpoint) | Yes | No |
| Polling | Timer + async function | No | No | Optional (checkpoint) | Yes | No |
| Batch | Optional (same as Stream) | Optional | Optional | Vec buffer (flushed on signal) | Yes | Yes (flush after execution) |
| State | No | No | No | Bounded VecDeque (DAL) | Yes (for mid-graph writes) | No |
| Custom | Developer-defined | Developer-defined | No | Developer-defined | Yes | Developer-defined |

The stream backend factory is the only factory pattern. It exists because broker implementations are pluggable — Kafka first, others register via the same trait. All other classes are fully determined at compile time.

## Checkpoint Protocol

Accumulators manage their own state independently — the reactor does not signal or coordinate checkpointing.

- **Stateful accumulators** (RunningAggregate, custom) persist their state via `CheckpointHandle` after each `process()` call (or periodically, at the accumulator's discretion).
- **Stream accumulators** manage their own broker offsets. Offset commits are the accumulator's responsibility.
- **State accumulators** persist their VecDeque to the DAL on every write (append + evict + persist).
- **Passthrough accumulators** have no state to checkpoint.

**Recovery**: each accumulator restores from its own last checkpoint independently on startup. Stream accumulators resume from their last committed offset. State accumulators load their VecDeque from the DAL. All accumulators persist their last emitted boundary to the DAL — this allows the reactor to restore its cache from the DAL on restart without waiting for accumulators to reconnect to their sources and re-emit.

The `CheckpointHandle` wraps simple key-value persistence in the DAL:

```rust
struct CheckpointHandle { /* DAL handle, accumulator name as key */ }

impl CheckpointHandle {
    async fn save<T: Serialize>(&self, state: &T) -> Result<()>;
    async fn load<T: DeserializeOwned>(&self) -> Result<Option<T>>;
}
```

## Accumulator Macros

Four distinct macros, one per accumulator class. Each generates an `Accumulator` trait implementation with the appropriate `run()` behavior. No over-abstraction — each macro is explicit about what kind of accumulator it creates.

### `#[passthrough_accumulator]`

Socket-only. No event loop, no state. The simplest accumulator — just a routing endpoint.

```rust
// What the developer writes
#[passthrough_accumulator]
fn beta(event: PricingUpdate) -> BetaData {
    BetaData { estimate: event.mid_price }
}

// What the macro generates
struct BetaAccumulator;
impl Accumulator for BetaAccumulator {
    type Event = PricingUpdate;
    type Output = BetaData;
    fn process(&mut self, event: PricingUpdate) -> Option<BetaData> {
        Some(beta(event))
    }
    // No run() override — socket-only, default pending().await
}
```

### `#[stream_accumulator]`

Reads from a stream broker topic. Macro generates the consumer loop. Optionally stateful.

```rust
// Stateless
#[stream_accumulator(type = "kafka", topic = "market.orderbook")]
fn alpha(event: OrderBookUpdate) -> AlphaData {
    AlphaData { top_high: event.best_ask, top_low: event.best_bid }
}

// Stateful — macro detects state parameter, generates checkpoint wiring
#[stream_accumulator(type = "kafka", topic = "fills", state = f64)]
fn gamma(event: FillEvent, exposure: &mut f64) -> ExposureData {
    match event.side {
        Side::Buy  => *exposure += event.qty,
        Side::Sell => *exposure -= event.qty,
    }
    ExposureData { exposure: *exposure }
}
// Generates: stream consumer loop + init() with checkpoint restore + state persistence
```

Config: `type` (required — backend name from `StreamBackendRegistry`), `topic` (required), `group` (optional — auto-generated if omitted), `state` (optional — type for checkpointed state).

### `#[polling_accumulator]`

Timer-based polling of an external system (database, API, file). The developer writes an async function that returns `Option<Boundary>` — `None` means nothing changed this poll.

```rust
#[polling_accumulator(interval = "5s")]
async fn config_source(ctx: &PollingContext) -> Option<ConfigData> {
    let row = ctx.db.query("SELECT value FROM config WHERE key = 'params'").await.ok()?;
    Some(ConfigData { k1: row.k1, k2: row.k2 })
}
// Generates: timer loop calling the function every 5s, pushes boundary when Some
```

Config: `interval` (required — polling frequency), `state` (optional).

### `#[state_accumulator]`

Bounded history buffer for cyclic state. No external source — only written to by the computation graph (collector or mid-graph writes). Emits the full list as its boundary on every write.

```rust
#[state_accumulator(capacity = 10)]
fn previous_outputs() -> VecDeque<DecisionOutput>;

// capacity = 1:  last run only (reconciliation, skip-if-unchanged)
// capacity = N:  bounded history window (pattern detection, rate limiting)
// capacity = -1: unbounded (use with caution)
// no capacity:   write-only sink, no history emitted back
```

The macro generates a `StateAccumulator<DecisionOutput>` with DAL persistence. The graph's collector writes to it:

```rust
ctx.accumulator("previous_outputs").send(current_output)?;
```

The state accumulator appends, evicts if over capacity, persists to DAL, and emits the full `VecDeque<DecisionOutput>` as its boundary. The reactor's cache updates, and the next graph execution sees the history.

### Custom: `impl Accumulator for ...`

Direct trait implementation for anything the macros don't cover. Developer writes `run()`, `process()`, `init()` themselves.

```rust
struct MyCustomAccumulator { /* ... */ }

impl Accumulator for MyCustomAccumulator {
    type Event = CustomEvent;
    type Output = CustomBoundary;

    fn process(&mut self, event: CustomEvent) -> Option<CustomBoundary> { /* ... */ }

    async fn run(&mut self, ctx: &AccumulatorContext) -> Result<(), AccumulatorError> {
        // Custom event loop — connect to proprietary feed, gRPC stream, etc.
    }
}
```

### Summary

| Macro | Event loop | Socket | State | Reactor signal | Config |
|-------|-----------|--------|-------|---------------|--------|
| `#[passthrough_accumulator]` | No | Yes | No | No | None |
| `#[stream_accumulator]` | Broker consumer | Yes | Optional | No | type, topic, group |
| `#[polling_accumulator]` | Timer-based poll | Yes | Optional | No | interval |
| `#[batch_accumulator]` | Optional (stream) | Yes | Vec buffer | Yes (flush) | type, topic (optional) |
| `#[state_accumulator]` | No | Yes (writes from graph) | Bounded VecDeque (DAL) | No | capacity |
| `impl Accumulator for ...` | Custom | Yes | Custom | Custom | Developer-defined |

The trait is the same underneath. The macros are ergonomic wrappers that generate different `run()` implementations. All accumulators get the receive socket automatically — the runtime wires it in regardless of which macro was used.

## Constraints

### Technical Constraints

- `process()` must be fast — it's called on every event. No blocking I/O in process(). If transformation needs async work, do it in `run()` before calling `process()`.
- Boundary wire format: bincode in release (fast, compact), JSON in debug (readable, inspectable). Same pattern as fidius. For the target workloads (small structs, <1KB), serialization overhead is microseconds in either format.
- Accumulators manage their own checkpoint timing independently. A crash loses any state accumulated since the last checkpoint — acceptable given that replay from the source (broker offset, DAL boundary log) is idempotent.
- The API server WebSocket proxy adds a network hop for push-based events. In the same k8s cluster this is sub-millisecond. For absolute minimum latency, the in-process `mpsc` passthrough avoids the proxy entirely (used for embedded mode and detector handoff).
