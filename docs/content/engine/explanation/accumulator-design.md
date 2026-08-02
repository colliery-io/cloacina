---
title: "Accumulator Design"
weight: 20
aliases:
  - "/computation-graphs/explanation/accumulator-design/"

---

# Accumulator Design

An accumulator is the boundary between the outside world and the computation graph. It is a long-lived tokio task that owns a connection to a data source, transforms raw events into typed boundary values, and pushes those values to the reactor. This document explains the five accumulator types, why each exists, and how the runtime manages state and health.

## The Core Problem

Computation graphs need data from heterogeneous sources. Some data comes from a Kafka broker with durable replay. Some comes from external systems that can only be polled periodically. Some arrives via direct push from another process. Some needs complex aggregation before it means anything. Some is the graph's own output, fed back as a rolling window.

A single abstraction cannot serve all of these without becoming complex and leaky. The five accumulator types — Passthrough, Stream, Polling, Batch, and State — each address one specific data ingestion pattern. Choosing the right type for a source means the accumulator is simple and the complexity lives where it belongs (the broker, the polling interval, the aggregation logic).

## The Accumulator Trait

The event-driven types implement the same trait (`cloacina::computation_graph::Accumulator`):

```rust
pub trait Accumulator: Send + 'static {
    /// The typed boundary produced for the reactor.
    type Output: Serialize + Send + 'static;

    /// Process raw event bytes and optionally produce a boundary.
    /// The implementor owns deserialization — the runtime is format-agnostic.
    fn process(&mut self, event: Vec<u8>) -> Option<Self::Output>;

    /// Called on startup before first receive.
    /// Use to restore state from last checkpoint.
    async fn init(&mut self, _ctx: &AccumulatorContext) -> Result<(), AccumulatorError> {
        Ok(())
    }
}
```

`process()` is called once per event. It receives the raw event **bytes** — the implementor owns deserialization, which keeps the runtime format-agnostic — and returns `Option<Output>`: `Some(boundary)` to forward to the reactor, `None` to suppress. This is where user-defined transformation and aggregation logic lives. `process()` is called sequentially by the processor task, so `&mut self` access to state is safe without locks. In practice you rarely write this impl by hand: the `#[passthrough_accumulator]`, `#[stream_accumulator]`, `#[batch_accumulator]`, and friends generate it, including the deserialize step, from a typed function you write.

`init()` is called once at startup before any events are processed. It is the place to restore persisted state from a checkpoint. (A trait-level default `process` body over a `DeserializeOwned` bound was investigated and rejected — it would tighten the format-agnostic contract; the boilerplate-free path is the `#[passthrough_accumulator]` macro, which generates the method. See CLOACI-T-0739.)

## The Runtime: Two Input Paths, One Processor

Each accumulator runs as two or three tokio tasks connected by channels:

```
Without event source (socket-only / passthrough):

  [socket task]  ──mpsc──▶  [processor task]  ──boundary──▶  reactor

With event source (stream / polling):

  [event source]  ──mpsc──┐
                           ├──▶  [processor task]  ──boundary──▶  reactor
  [socket task]   ──mpsc──┘
```

The **socket task** is always active. It receives raw bytes pushed in from outside (via WebSocket or mpsc channel) and forwards them to the merge channel; `process()` deserializes them. This means every accumulator type, regardless of its primary source, can also receive push events from external producers — useful for testing and ops injection.

The **event source** (optional) is an independently running task that actively pulls from a backend (Kafka, a timer, a database) and pushes events into the same merge channel. It owns `self` rather than `&mut self` so it can run concurrently with the processor without borrowing conflicts.

The **processor task** runs on the current task (not spawned). It owns `&mut acc` and calls `process()` for every event from the merge channel.

## The Five Types

### Passthrough — Zero State, Lowest Latency

A passthrough accumulator has no event source and no state. It receives a pre-materialized boundary on its socket channel and forwards it to the reactor immediately without any transformation.

```rust
#[passthrough_accumulator]
fn beta(event: PricingUpdate) -> BetaData {
    BetaData { estimate: event.mid_price }
}
```

**When to use it**: when the upstream system has already done all the materialization and aggregation work. The producing system pushes a boundary; Cloacina is just the execution engine. Any external system that can write to a socket can feed the graph. This is also the accumulator type used by the FFI packaging bridge for all host-side accumulators in packaged graphs — the processing logic lives inside the compiled graph plugin.

**Why no state**: latency is the primary concern. There is nothing to checkpoint, nothing to replay, nothing to restore. If a boundary is lost during a restart, the upstream system must re-send it (or the loss is accepted as a tradeoff — passthrough semantics are explicit about this).

**Health state**: `SocketOnly` — healthy by definition since there is no backend connection to lose.

### Stream — Broker-Backed with Offset Tracking

A stream accumulator subscribes to a topic on a broker (Kafka, Redpanda, etc.) and consumes messages in order. The consumer offset is the checkpoint — if the process restarts, it resumes from the last committed offset. No boundaries are lost as long as the broker retains the data.

```rust
#[stream_accumulator(type = "kafka", topic = "market.orderbook")]
fn alpha(event: OrderBookUpdate) -> AlphaData {
    AlphaData { top_high: event.best_ask, top_low: event.best_bid }
}

// With running state:
#[stream_accumulator(type = "kafka", topic = "fills", state = f64)]
fn gamma(event: FillEvent, exposure: &mut f64) -> ExposureData {
    match event.side {
        Side::Buy  => *exposure += event.qty,
        Side::Sell => *exposure -= event.qty,
    }
    ExposureData { exposure: *exposure }
}
```

The `state` parameter adds a mutable accumulator value to `process()`. The state is persisted to the DAL via `CheckpointHandle` — on restart, `init()` loads the persisted value and `process()` picks up from where it left off. The consumer offset and the state are checkpointed together, ensuring consistency.

**When to use it**: durable push sources where the broker provides replay. Kafka is the primary implementation, but the `StreamBackend` trait is pluggable — see the StreamBackend section below.

**Health states**: `Connecting` (connecting to broker), `Live` (consuming events), `Disconnected` (lost broker connection, retrying). The reactor gates on health before going live.

### Polling — Timer-Based with `Option<T>` Semantics

A polling accumulator fires on a timer interval and queries an external source. The `Option<T>` return from `process()` is meaningful here: `None` means "no change since last poll" and the boundary is not forwarded to the reactor. Only actual changes trigger graph execution.

```rust
#[polling_accumulator(interval = "5s")]
async fn config() -> Option<ConfigData> {
    let row = query_config_source().await.ok()?;
    Some(ConfigData { /* … */ })
}
```

The poll function takes no arguments and returns `Option<T>` — it owns its own connection to whatever it queries.

**When to use it**: databases, REST APIs, or any system that cannot push data and must be queried. The interval is the latency floor — a `5s` polling interval means up to 5 seconds before the reactor sees a change. This is appropriate for configuration, reference data, or slowly-changing dimensions.

**Why `Option<T>`**: the natural semantics of polling is "check if anything changed." If the query returns the same value as last time, suppressing the boundary is the right behavior — there is nothing new for the reactor to act on. Forcing every poll to produce a boundary would cause constant graph executions even when nothing changed.

**Checkpoint**: the last value seen, used for change detection on restart.

### Batch — Buffer and Flush

A batch accumulator buffers incoming events and flushes them as a single aggregated boundary. A flush happens on any of three conditions: the `flush_interval` timer elapses, the buffer reaches `max_buffer_size`, or the reactor sends a flush signal (which it does after each successful graph execution, via its batch-flush channels).

**When to use it**: aggregation windows, rate limiting, or cases where you want one graph execution per batch rather than one per event. For example: collecting 100 order fill events into a single aggregate before the decision engine runs, rather than running it 100 times.

**The flush signal**: after graph execution completes, the reactor sends a signal to all batch accumulator flush channels. The accumulator drains its buffer and emits the aggregated boundary. The timer and size cap mean a batch accumulator still emits even when the reactor has not fired recently — configure `flush_interval`/`max_buffer_size` to bound staleness and memory.

### State — The Graph's Own Output, Fed Back

A state accumulator (`#[state_accumulator(capacity = N)]`) holds a bounded `VecDeque<T>` that receives values written by the computation graph itself (a collector node or mid-graph write), persists the window to the DAL on every write, and re-emits the full window as a boundary. It enables cyclic patterns where the graph's output on one execution becomes an input on the next — e.g., a rolling window of recent ticks.

Capacity is signed: `capacity > 0` is a bounded window (oldest evicted at capacity); `capacity < 0` is unbounded; `capacity == 0` is a write-only sink that emits no history back. Declared as a bodyless function returning the window type:

```rust
#[state_accumulator(capacity = 100)]
fn tick_window() -> VecDeque<Tick>;
```

On startup the runtime loads the persisted window from the DAL and emits it to the reactor, so cyclic state survives restarts. Python parity exists as `@cloaca.state_accumulator(capacity=N)` — see `examples/features/computation-graphs/python-stateful-graph` for a packaged example.

## State Management and the CheckpointHandle

Stateful accumulators (stream with `state = T`, polling, batch) persist their state via the `CheckpointHandle`:

```rust
pub struct CheckpointHandle {
    dal: DAL,
    graph_name: String,
    accumulator_name: String,
}

impl CheckpointHandle {
    pub async fn save<T: Serialize>(&self, state: &T) -> Result<(), AccumulatorError>;
    pub async fn load<T: DeserializeOwned>(&self) -> Result<Option<T>, AccumulatorError>;
}
```

The checkpoint is keyed by `(graph_name, accumulator_name)`. It is written after each boundary is emitted — not after each event — which means the checkpoint represents a causally consistent point: state was checkpointed after we successfully notified the reactor. On restart, `init()` calls `checkpoint.load()` and restores the state before event processing begins.

Checkpoint serialization uses the bincode wire format — always, in both debug and release builds — and is transparent to accumulator authors.

## The StreamBackend Trait

Stream accumulators are broker-agnostic. The `StreamBackend` trait defines a minimal interface:

```rust
pub trait StreamBackend: Send + 'static {
    async fn connect(config: &StreamConfig) -> Result<Self, StreamError>;
    async fn recv(&mut self) -> Result<RawMessage, StreamError>;
    async fn commit(&mut self) -> Result<(), StreamError>;
    fn current_offset(&self) -> Option<u64>;
}
```

There is deliberately no `kafka` feature (or any broker implementation) in the core crate — event-source backends ship as constructor **provider** crates. The Kafka implementation lives in `cloacina-provider-kafka`, a native provider that wraps `rdkafka` and exposes the `kafka_source` stream-accumulator constructor. Core stays broker-free (CLOACI-T-0898).

The `StreamBackendRegistry` maps type names to factory functions; backends register through the `Runtime`:

```rust
runtime.register_stream_backend(
    "my-broker".to_string(),
    Box::new(|config| {
        Box::pin(async move {
            let backend = MyBrokerBackend::connect(&config).await?;
            Ok(Box::new(backend) as Box<dyn StreamBackend>)
        })
    }),
);
```

A `#[stream_accumulator(type = "my-broker", ...)]` declaration resolves its backend by that type name at graph load — from an inventory-submitted `StreamBackendEntry` in embedded builds, or from what the runtime has registered. Broker addresses are resolved at runtime (e.g., via the `CLOACINA_VAR_` variable-registry convention) rather than embedded in the compiled package.

## Accumulator Health States

Each accumulator reports its health via a `watch::Sender<AccumulatorHealth>`. The reactor subscribes to all of its accumulators' health channels and uses them for startup gating and degraded mode detection.

```
Starting      →  Connecting  →  Live
                                  │
                                  ▼
                             Disconnected
                             (retrying...)
                                  │
                                  ▼
                               Live (reconnected)
```

- **Starting**: loading checkpoint from DAL
- **Connecting**: checkpoint loaded, connecting to source (socket is already active)
- **Live**: connected, processing events, pushing boundaries
- **Disconnected**: was live, lost source connection, socket still active, retrying
- **SocketOnly**: passthrough accumulator — no backend connection, healthy by definition

The reactor transitions through `Starting → Warming → Live` as its accumulators report healthy. A `Disconnected` accumulator moves the reactor to `Degraded`, where it continues operating with the last-seen cached value for that source rather than blocking entirely.

## What Accumulators Are Not Responsible For

Complex aggregation — windowed, watermarked, multi-partition exactly-once — is not Cloacina's concern. If a workload needs complex aggregation, the right approach is to run whatever upstream process handles it and write the result to a Kafka topic. The stream accumulator then consumes the already-aggregated result. Cloacina treats the topic as a source of boundaries; it does not care what wrote them.

This is a deliberate non-goal. Owning complex stream processing would mean owning a problem Kafka Streams, Flink, and similar systems already solve well. The accumulator interface is intentionally narrow.

## Further Reading

- [Architecture]({{< ref "architecture" >}}) — the full event-driven model and how accumulators fit into it
- [Packaging & FFI]({{< ref "packaging" >}}) — how packaged graphs expose accumulators via FFI
- [Performance Characteristics]({{< ref "performance" >}}) — throughput numbers for stream and batch accumulator types
