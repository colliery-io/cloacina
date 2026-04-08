---
id: stream-backend-kafka-consumer
level: initiative
title: "Stream Backend — Kafka Consumer Accumulator"
short_code: "CLOACI-I-0084"
created_at: 2026-04-07T15:45:48.216179+00:00
updated_at: 2026-04-08T00:57:24.555955+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: stream-backend-kafka-consumer
---

# Stream Backend — Kafka Consumer Accumulator

## Context

The accumulator system has four types:

- **Passthrough** — forwards events unchanged. Socket/channel-fed, zero state.
- **Stream** — backend-driven event loop via `StreamBackend` trait. Currently only `MockStreamBackend` exists — no real broker implementation.
- **Polling** — timer-based async poll function (`Option<T>` return, skips emission on None).
- **Batch** — buffers events, flushes on interval or size threshold.

The `StreamBackend` trait exists (`crates/cloacina/src/computation_graph/accumulator.rs`) with `connect()`, `poll()`, and `commit_offset()` — designed for broker-backed sources. The accumulator runtime already handles stream backends. What's missing is a real implementation that talks to Kafka.

The I-0069 design spec calls for stream-backed accumulators that consume from Kafka/Redpanda topics with durable offset tracking. This is the primary missing capability for production reactive workloads: an event stream feeding a computation graph continuously, surviving restarts by resuming from the last committed offset.

### What exists

- `StreamBackend` trait with `connect`, `poll`, `commit_offset` methods
- `#[stream_accumulator]` macro that generates an accumulator with a `StreamBackend`
- `MockStreamBackend` for testing
- `StreamAccumulatorConfig` with `backend_type`, `config` HashMap, `poll_interval`
- The accumulator runtime already handles stream backends — `accumulator_runtime` checks for a stream backend and runs a separate event loop for it
- Python `@cloaca.stream_accumulator(type="kafka", topic="...")` decorator exists (metadata only — no backend wiring)

### What's missing

- A real `KafkaStreamBackend` implementation of `StreamBackend`
- Kafka client library integration (rdkafka or a pure-Rust alternative)
- Offset commit integration with the checkpoint DAL
- Consumer group management
- Package metadata for declaring Kafka-sourced accumulators
- Integration test with a real Kafka broker

### Detector handoff (already solved)

The I-0069 design spec describes "detector handoff" as a workflow task pushing boundaries into an accumulator. This is already supported today: a workflow task makes an HTTP/WS call to `POST /v1/ws/accumulator/{name}?token=...` to push events into a passthrough accumulator. No special integration needed — same path as any external producer, same auth.

## Goals & Non-Goals

**Goals:**
- Implement `KafkaStreamBackend` — a `StreamBackend` impl that consumes from a Kafka topic
- Offset tracking via consumer group + checkpoint DAL for restart recovery
- Configuration via `#[stream_accumulator(type = "kafka", topic = "...", group = "...")]` (Rust)
- Configuration via `@cloaca.stream_accumulator(type="kafka", topic="...", group="...")` (Python — decorator already exists, needs backend wiring)
- Package metadata support — declare Kafka-sourced accumulators in `package.toml`
- Integration test with Kafka (Docker-based, similar to Postgres tests)
- End-to-end example: Kafka topic → stream accumulator → computation graph (Rust and Python)

**Non-Goals:**
- Complex stream processing (windowing, watermarks, exactly-once) — not Cloacina's concern
- Multi-partition consumer balancing — single partition per accumulator for simplicity
- Redpanda/Iggy support — Kafka protocol first, others can follow the same trait
- Schema registry integration — accumulators deserialize raw bytes, format is the producer's concern

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### Rust
- [ ] `KafkaStreamBackend` implements `StreamBackend` trait (`connect`, `recv`, `commit`, `current_offset`)
- [ ] Registered in global `StreamBackendRegistry` as `"kafka"`
- [ ] `StreamConfig` fields used: `broker_url`, `topic`, `group`, `extra` (for consumer config overrides)
- [ ] On `connect`: creates Kafka consumer, subscribes to topic, seeks to last committed offset (from checkpoint DAL or consumer group)
- [ ] On `recv`: returns next message as `RawMessage` with payload, offset, timestamp
- [ ] On `commit`: commits offset to Kafka consumer group AND checkpoint DAL
- [ ] Accumulator runtime already handles stream backends — no changes needed to `accumulator_runtime`
- [ ] `#[stream_accumulator(type = "kafka", ...)]` macro works end-to-end with real Kafka
- [ ] Restart recovery: accumulator resumes from last committed offset after server restart
- [ ] Error handling: connection failures, broker unavailable, deserialization errors — all surfaced via `StreamError`

### Python
- [ ] `@cloaca.stream_accumulator(type="kafka", topic="...", group="...")` decorator already stores metadata — wire it to create a `KafkaStreamBackend` when the graph is loaded
- [ ] Python accumulator function receives deserialized events from Kafka topic
- [ ] Python example: Kafka → stream accumulator → computation graph

### Packaging
- [ ] `package.toml` supports `[[metadata.accumulators]]` with `backend = "kafka"`, `topic`, `group` fields
- [ ] Reconciler creates `KafkaStreamBackend` from package metadata when loading CG packages
- [ ] `AccumulatorFactory` in `packaging_bridge.rs` supports stream accumulators (not just passthrough)

### Testing
- [ ] Unit tests with `MockStreamBackend` (already exist — verify they still pass)
- [ ] Integration test with Dockerized Kafka/Redpanda broker
- [ ] End-to-end: produce messages to topic → accumulator consumes → graph fires → verify output
- [ ] Restart test: produce, consume, crash, restart, verify resume from last offset
- [ ] Angreal task: `angreal cloacina kafka-integration`

### Infrastructure
- [ ] Kafka added to `.angreal/docker-compose.yaml` alongside Postgres — KIP-500 mode (KRaft, no ZooKeeper)
- [ ] `angreal services up` starts both Postgres and Kafka
- [ ] Kafka broker accessible at `localhost:9092` for tests

### Soak Tests
- [ ] Server-soak updated: upload a CG package with Kafka-sourced accumulator, produce events to Kafka topic during soak loop, verify graph fires from Kafka events
- [ ] Daemon-soak updated: Kafka-sourced package loaded alongside workflow packages

### CI
- [ ] Kafka service in CI Docker Compose (KIP-500 / KRaft — no ZooKeeper)
- [ ] Integration test runnable in CI with Kafka + Postgres services

## Detailed Design

### KafkaStreamBackend

Implements `StreamBackend` trait:
- `connect(config)` — creates a Kafka consumer with the configured broker, topic, group, and starting offset
- `poll()` — returns the next message from the topic (or None if no new messages)
- `commit_offset(offset)` — commits the consumer offset to Kafka AND the checkpoint DAL

### Kafka client

Options:
1. **rdkafka** (librdkafka wrapper) — mature, full-featured, but requires C library
2. **kafka-rust** — pure Rust, simpler, fewer features
3. **rskafka** — pure Rust, async-native, minimal

Need to evaluate: which fits the `StreamBackend` trait best? rdkafka is battle-tested but adds a C dependency. rskafka is async-native which fits the accumulator runtime.

### Configuration & Usage Patterns

The `StreamBackend` provides the transport (Kafka). The accumulator type determines the processing pattern. All four accumulator types can be backed by a stream — the stream is just the event source.

#### Pattern 1: Stream Passthrough — every message fires the graph

Every Kafka message becomes a boundary immediately. No aggregation, no buffering. Use when the upstream already materialized the data and you just need to react to each event.

```rust
// Rust — each orderbook update fires the graph
#[stream_accumulator(type = "kafka", topic = "market.orderbook", group = "cloacina-mm")]
fn orderbook(event: OrderBookUpdate) -> OrderBookData {
    OrderBookData { best_bid: event.bid, best_ask: event.ask }
}
```

```python
# Python — same pattern
@cloaca.stream_accumulator(type="kafka", topic="market.orderbook", group="cloacina-mm")
def orderbook(event: dict) -> dict:
    return {"best_bid": event["bid"], "best_ask": event["ask"]}
```

#### Pattern 2: Stateful Stream — emit on every message with user-managed state

Each Kafka message fires the graph, but the accumulator maintains state across messages. The `state = Type` macro parameter adds a mutable state argument to the function. State is checkpointed to the DAL for crash recovery.

Two sub-patterns:

**Running total** — simple cumulative state, no expiry:

```rust
// Rust — running net exposure from fill events
#[stream_accumulator(type = "kafka", topic = "fills", group = "cloacina-mm", state = ExposureState)]
fn exposure(event: FillEvent, state: &mut ExposureState) -> ExposureData {
    match event.side {
        Side::Buy  => state.net += event.qty,
        Side::Sell => state.net -= event.qty,
    }
    ExposureData { net_exposure: state.net, last_fill_price: event.price }
}

#[derive(Default, Serialize, Deserialize)]
struct ExposureState { net: f64 }
```

```python
# Python — running exposure
@cloaca.stream_accumulator(type="kafka", topic="fills", group="cloacina-mm", state={"net": 0.0})
def exposure(event: dict, state: dict) -> dict:
    if event["side"] == "buy":
        state["net"] += event["qty"]
    else:
        state["net"] -= event["qty"]
    return {"net_exposure": state["net"], "last_fill_price": event["price"]}
```

**Sliding window** — time-bounded state, old events expire:

```rust
// Rust — 5-minute sliding VWAP, recomputed on every trade
#[stream_accumulator(type = "kafka", topic = "trades", group = "cloacina-mm", state = VwapWindow)]
fn vwap(event: TradeEvent, state: &mut VwapWindow) -> VwapData {
    state.trades.push_back((event.timestamp, event.price, event.qty));

    // Expire events older than 5 minutes
    let cutoff = event.timestamp - chrono::Duration::minutes(5);
    while state.trades.front().map(|t| t.0 < cutoff).unwrap_or(false) {
        state.trades.pop_front();
    }

    let total_value: f64 = state.trades.iter().map(|(_, p, q)| p * q).sum();
    let total_volume: f64 = state.trades.iter().map(|(_, _, q)| q).sum();

    VwapData { vwap: total_value / total_volume, window_size: state.trades.len() }
}

#[derive(Default, Serialize, Deserialize)]
struct VwapWindow {
    trades: VecDeque<(DateTime<Utc>, f64, f64)>,
}
```

```python
# Python — 5-minute sliding VWAP
from collections import deque
from datetime import timedelta

@cloaca.stream_accumulator(type="kafka", topic="trades", group="cloacina-mm", state={"trades": []})
def vwap(event: dict, state: dict) -> dict:
    state["trades"].append((event["timestamp"], event["price"], event["qty"]))

    cutoff = event["timestamp"] - timedelta(minutes=5).total_seconds()
    while state["trades"] and state["trades"][0][0] < cutoff:
        state["trades"].pop(0)

    total_value = sum(p * q for _, p, q in state["trades"])
    total_volume = sum(q for _, _, q in state["trades"])
    return {"vwap": total_value / total_volume, "window_size": len(state["trades"])}
```

In both cases, Cloacina manages the state lifecycle — the macro generates the struct with the state field, `process()` passes `&mut self.state` to the user function, and the checkpoint DAL persists/restores state across restarts. The user owns the window/aggregation logic entirely.

#### Pattern 3: Stream with Batch — collect and fire on flush

Buffers messages from the stream, emits a single boundary on flush. Three flush modes:

**Reactor-driven flush** — buffer everything, drain when the reactor signals after graph completion. The reactor controls the cadence. Use when the graph needs "everything since last run" as a batch.

```rust
// Rust — drain all fills since last graph execution
#[batch_accumulator(type = "kafka", topic = "fills", group = "cloacina-mm")]
fn fill_batch(events: Vec<FillEvent>) -> Option<BatchedFills> {
    if events.is_empty() { return None; }
    Some(BatchedFills {
        count: events.len(),
        total_volume: events.iter().map(|e| e.qty).sum(),
        avg_price: events.iter().map(|e| e.price).sum::<f64>() / events.len() as f64,
    })
}
```

```python
# Python — drain all since last flush
@cloaca.batch_accumulator(type="kafka", topic="fills", group="cloacina-mm")
def fill_batch(events: list) -> dict | None:
    if not events:
        return None
    return {
        "count": len(events),
        "total_volume": sum(e["qty"] for e in events),
        "avg_price": sum(e["price"] for e in events) / len(events),
    }
```

**Time-based flush** — emit every N seconds regardless of count. Tumbling window.

```rust
#[batch_accumulator(type = "kafka", topic = "fills", group = "cloacina-mm", flush_interval = "1s")]
fn fill_batch(events: Vec<FillEvent>) -> Option<BatchedFills> { /* same */ }
```

**Size-based flush** — emit when buffer reaches N events.

```rust
#[batch_accumulator(type = "kafka", topic = "fills", group = "cloacina-mm", max_buffer = 1000)]
fn fill_batch(events: Vec<FillEvent>) -> Option<BatchedFills> { /* same */ }
```

All three can be combined: `flush_interval = "5s", max_buffer = 1000` — whichever triggers first. With neither, it's pure reactor-driven flush.

**Note**: The batch accumulator's event source is currently the socket channel. To back it with Kafka, the stream backend feeds events into the batch buffer instead of the socket. This requires wiring the `StreamBackend` as an alternative event source for `BatchAccumulator`.

#### Pattern 4: Polling with Stream Checkpoint — timer-based read from materialized view

Not strictly a Kafka pattern, but relevant: a polling accumulator that queries a materialized view (populated by a Kafka Streams/ksqlDB pipeline or a CDC consumer) and checkpoints its read position.

```rust
// Rust — poll a materialized view every 5 seconds
#[polling_accumulator(interval = "5s")]
async fn config() -> Option<ConfigData> {
    let row = sqlx::query("SELECT * FROM config_latest").fetch_optional(&pool).await.ok()?;
    row.map(|r| ConfigData { value: r.value })
}
```

### Package metadata

```toml
# Stream passthrough from Kafka
[[metadata.accumulators]]
name = "orderbook"
type = "stream"
backend = "kafka"
topic = "market.orderbook"
group = "cloacina-mm"

# Batch accumulator (event source is Kafka, but processing is batched)
[[metadata.accumulators]]
name = "fill_batch"
type = "batch"
backend = "kafka"
topic = "fills"
group = "cloacina-mm"
flush_interval = "1s"
max_buffer = 1000
```

### Recovery

1. On startup: read last committed offset from checkpoint DAL
2. Seek Kafka consumer to that offset
3. Resume consuming
4. On graph completion: commit offset to both Kafka (consumer group) and checkpoint DAL

### Testing

- Unit tests with `MockStreamBackend` (already exists)
- Integration test with Dockerized Kafka (Redpanda is lighter — Kafka-compatible but single binary)
- Angreal task: `angreal cloacina kafka-integration` similar to `angreal cloacina integration`

## Alternatives Considered

**Alt 1: WebSocket push only (no Kafka consumer)**
Works for low-volume sources where the producer can push. Doesn't work for high-volume streams where Kafka provides durability, replay, and consumer group semantics.

**Alt 2: Generic stream protocol abstraction first**
Design a universal stream protocol before implementing Kafka. Rejected — the `StreamBackend` trait already provides the abstraction. Better to implement one concrete backend (Kafka), learn from it, then add others.

## Implementation Plan

1. **Phase 1** — Kafka client evaluation and selection
2. **Phase 2** — `KafkaStreamBackend` implementation
3. **Phase 3** — Offset tracking + checkpoint DAL integration
4. **Phase 4** — Package metadata + scheduler wiring
5. **Phase 5** — Integration tests with Dockerized broker
6. **Phase 6** — Example: Kafka-sourced computation graph
