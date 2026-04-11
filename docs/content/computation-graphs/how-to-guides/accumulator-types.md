---
title: "Choosing and Using Accumulator Types"
description: "How to select and configure the right accumulator type for your computation graph"
weight: 20
---

# Choosing and Using Accumulator Types

This guide explains the four accumulator types available in Cloacina's computation graph system, when to use each, and how to implement them.

## Prerequisites

- Familiarity with computation graph concepts (see the [computation graphs tutorial series]({{< ref "/computation-graphs/tutorials" >}}))
- A computation graph declared with `#[cloacina_macros::computation_graph]`

## What Is an Accumulator?

An accumulator sits between an external data source and a reactor. It receives raw events, transforms them into typed boundaries, and forwards those boundaries to the reactor that drives your graph. Every accumulator exposes a socket channel for external pushes regardless of type.

## Decision Matrix

| Type | Source | Emission pattern | Use when |
|------|--------|-----------------|----------|
| Passthrough | External push (socket only) | One-in, one-out | No broker, lowest overhead |
| Stream | Message broker (Kafka, etc.) | Broker-driven + socket | Continuous high-volume feed from a topic |
| Polling | Timer | Periodic | Pull-based sources (databases, REST APIs) |
| Batch | Socket buffer | Timer / size / reactor signal | Collect events between graph executions |

---

## Passthrough

`#[passthrough_accumulator]` transforms each event as it arrives and emits one boundary per event. It has no active event loop — all events arrive via the socket channel. Health state is reported as `SocketOnly` immediately on startup.

### Function signature

```rust
fn my_accumulator(event: InputType) -> OutputType { ... }
```

### Example

```rust
use cloacina_macros::passthrough_accumulator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPrice {
    pub symbol: String,
    pub price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedPrice {
    pub symbol: String,
    pub price_cents: i64,
}

#[passthrough_accumulator]
fn normalize_price(event: RawPrice) -> NormalizedPrice {
    NormalizedPrice {
        symbol: event.symbol,
        price_cents: (event.price * 100.0).round() as i64,
    }
}
```

The macro generates `NormalizePriceAccumulator` implementing the `Accumulator` trait. Instantiate and run it with `accumulator_runtime`:

```rust
use cloacina::computation_graph::accumulator::{
    accumulator_runtime, AccumulatorContext, AccumulatorRuntimeConfig,
};

let (socket_tx, socket_rx) = tokio::sync::mpsc::channel(64);

let ctx = AccumulatorContext {
    output: boundary_sender,
    name: "normalize_price".to_string(),
    shutdown: shutdown_rx.clone(),
    checkpoint: None,
    health: None,
};

tokio::spawn(accumulator_runtime(
    NormalizePriceAccumulator,
    ctx,
    socket_rx,
    AccumulatorRuntimeConfig::default(),
));

// Push events via the socket channel
use cloacina::computation_graph::types::serialize;
socket_tx.send(serialize(&RawPrice { symbol: "BTC".into(), price: 42000.50 }).unwrap()).await.unwrap();
```

---

## Stream

`#[stream_accumulator(type = "...", topic = "...")]` wires a message broker backend into the accumulator runtime. Events arrive from the broker and from the socket channel. Health transitions through `Starting → Connecting → Live` as the broker connection is established.

### Required arguments

| Argument | Type | Description |
|----------|------|-------------|
| `type` | string | Backend type, e.g. `"kafka"` |
| `topic` | string | Topic or queue name to consume |
| `group` | string (optional) | Consumer group. Defaults to `"{fn_name}_group"` |
| `state` | Rust type (optional) | Carry mutable state across events |

### Stateless example

```rust
use cloacina_macros::stream_accumulator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEvent {
    pub order_id: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSummary {
    pub order_id: String,
    pub quantity: u32,
}

#[stream_accumulator(type = "kafka", topic = "orders")]
fn process_order(event: OrderEvent) -> OrderSummary {
    OrderSummary {
        order_id: event.order_id,
        quantity: event.quantity,
    }
}
```

The macro generates `ProcessOrderAccumulator` with `new()` and `Default` implementations.

### Stateful example — running total

Add `state = Type` to carry mutable state across events. The function receives `&mut State` as its second argument.

```rust
use cloacina_macros::stream_accumulator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleEvent {
    pub amount: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunningTotal {
    pub total: f64,
    pub count: u64,
}

#[stream_accumulator(type = "kafka", topic = "sales", state = RunningTotal)]
fn accumulate_sales(event: SaleEvent, state: &mut RunningTotal) -> RunningTotal {
    state.total += event.amount;
    state.count += 1;
    state.clone()
}
```

The macro generates `AccumulateSalesAccumulator` with `new(initial_state: RunningTotal)`:

```rust
let acc = AccumulateSalesAccumulator::new(RunningTotal::default());
```

Stateful accumulators are well-suited for sliding windows, running totals, and deduplication tables where you need memory across events.

---

## Polling

`#[polling_accumulator(interval = "...")]` calls an async function on a fixed timer and emits a boundary when the function returns `Some`. This is the right choice for pull-based sources such as REST APIs, databases, and configuration services.

### Function signature

The function must be async and return `Option<OutputType>`. Return `None` when the source has not changed since the last poll.

```rust
async fn my_poller() -> Option<OutputType> { ... }
```

### Interval format

Supports `ms`, `s`, and `m` suffixes: `"100ms"`, `"5s"`, `"1m"`.

### Example — polling a REST API

```rust
use cloacina_macros::polling_accumulator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRate {
    pub base: String,
    pub rate: f64,
}

#[polling_accumulator(interval = "30s")]
async fn fetch_exchange_rate() -> Option<ExchangeRate> {
    // In production, make an HTTP request here.
    // Return None if the rate has not changed.
    Some(ExchangeRate {
        base: "USD".to_string(),
        rate: 1.08,
    })
}
```

The macro generates `FetchExchangeRateAccumulator`. Run it with `polling_accumulator_runtime`:

```rust
use cloacina::computation_graph::accumulator::{
    polling_accumulator_runtime, AccumulatorContext,
};

let (socket_tx, socket_rx) = tokio::sync::mpsc::channel(16);

let ctx = AccumulatorContext {
    output: boundary_sender,
    name: "fetch_exchange_rate".to_string(),
    shutdown: shutdown_rx.clone(),
    checkpoint: None,
    health: None,
};

tokio::spawn(polling_accumulator_runtime(
    FetchExchangeRateAccumulator,
    ctx,
    socket_rx,
));
```

The first poll fires after one full interval, not immediately on startup. The last successful poll result is checkpointed to the DAL when a checkpoint handle is provided, so restarts emit the previously-seen value immediately and only wait one interval for fresh data.

---

## Batch

`#[batch_accumulator(flush_interval = "...")]` buffers incoming socket events and flushes them as a `Vec<Event>` on three triggers: a timer, a maximum buffer size, or a signal from the reactor after graph execution. This is ideal when you want to collect everything that arrived between graph runs and process it as a unit.

### Required arguments

| Argument | Type | Description |
|----------|------|-------------|
| `flush_interval` | string | Timer-based flush interval |
| `max_buffer_size` | integer (optional) | Also flush when buffer reaches this size |

### Function signature

```rust
fn my_batcher(events: Vec<EventType>) -> Option<OutputType> { ... }
```

Return `None` from `process_batch` to suppress emission for empty or uninteresting batches.

### Example — audit log aggregator

```rust
use cloacina_macros::batch_accumulator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub user_id: String,
    pub action: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditBatch {
    pub events: Vec<AuditEvent>,
    pub event_count: usize,
}

#[batch_accumulator(flush_interval = "1s", max_buffer_size = 500)]
fn aggregate_audit(events: Vec<AuditEvent>) -> Option<AuditBatch> {
    if events.is_empty() {
        return None;
    }
    let count = events.len();
    Some(AuditBatch { events, event_count: count })
}
```

The macro generates `AggregateAuditAccumulator`. Run it with `batch_accumulator_runtime`:

```rust
use cloacina::computation_graph::accumulator::{
    batch_accumulator_runtime, flush_signal, AccumulatorContext, BatchAccumulatorConfig,
};
use std::time::Duration;

// Create the flush signal — reactor holds the sender end
let (flush_tx, flush_rx) = flush_signal();

let (socket_tx, socket_rx) = tokio::sync::mpsc::channel(256);

let ctx = AccumulatorContext {
    output: boundary_sender,
    name: "aggregate_audit".to_string(),
    shutdown: shutdown_rx.clone(),
    checkpoint: None,
    health: None,
};

let config = BatchAccumulatorConfig {
    flush_interval: Some(Duration::from_secs(1)),
    max_buffer_size: Some(500),
};

tokio::spawn(batch_accumulator_runtime(
    AggregateAuditAccumulator,
    ctx,
    socket_rx,
    flush_rx,
    config,
));

// Pass flush_tx to Reactor::with_batch_flush_senders so the reactor
// triggers a flush after each graph execution.
```

The reactor signals the flush sender after every successful graph execution. The timer and size threshold serve as fallbacks. On shutdown, any remaining buffered events are flushed automatically.

---

## Health states

All accumulator types report health through a `watch::Receiver<AccumulatorHealth>`. The states are:

| State | Meaning |
|-------|---------|
| `Starting` | Loading checkpoint from DAL |
| `Connecting` | Checkpoint loaded, connecting to broker (stream only) |
| `Live` | Processing events and pushing boundaries |
| `Disconnected` | Lost broker connection, retrying (stream only) |
| `SocketOnly` | No active source — healthy by definition (passthrough) |

Pass a health sender via `AccumulatorContext::health` to enable reporting. The reactor uses these receivers to gate its own startup and detect degraded mode.

## Related

- [Computation graphs tutorial 09 — full pipeline]({{< ref "/computation-graphs/tutorials/library/09-full-pipeline" >}})
- [How to monitor computation graph health]({{< ref "/computation-graphs/how-to-guides/computation-graph-health" >}})
- [How to use when_all reaction criteria]({{< ref "/computation-graphs/how-to-guides/when-all-criteria" >}})
