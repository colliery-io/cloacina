---
title: "08 - Accumulators"
description: "Implement a passthrough accumulator, wire channels, spawn a reactor, and push live events through a compiled graph"
weight: 20
---

In this tutorial you'll move beyond calling the compiled graph by hand. You'll implement an `Accumulator`, wire up the channel plumbing between it and a `Reactor`, and watch the graph fire automatically as events arrive.

## What you'll learn

- The `Accumulator` trait: `process()`, `Event`, and `Output` associated types
- `BoundarySender` — how the accumulator hands data off to the reactor
- `AccumulatorContext` and `AccumulatorRuntimeConfig`
- `accumulator_runtime()` — the three-task merge-channel model
- `shutdown_signal()` for coordinated teardown
- `Reactor` with `ReactionCriteria::WhenAny` and `InputStrategy::Latest`
- Pushing serialized events and observing the graph fire

## Prerequisites

- Completion of [Tutorial 07 — Your First Computation Graph]({{< ref "/computation-graphs/tutorials/library/07-computation-graph/" >}})

## The complete example

The full source lives at [`examples/tutorials/computation-graphs/library/08-accumulators`](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials/computation-graphs/library/08-accumulators).

To run it:

```bash
angreal demos tutorials rust 08
```

---

## Step 1: Boundary types and the graph

The graph for this tutorial follows the same structure as Tutorial 07. A `PricingUpdate` arrives from outside, the accumulator converts it to a `PricingSignal`, and the graph processes and formats it.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingUpdate {
    pub mid_price: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingSignal {
    pub price: f64,
    pub change_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalOutput {
    pub message: String,
}

#[cloacina_macros::computation_graph(
    react = when_any(pricing),
    graph = {
        ingest(pricing) -> analyze,
        analyze -> format_signal,
    }
)]
pub mod pricing_graph {
    use super::*;

    pub async fn ingest(pricing: Option<&PricingSignal>) -> PricingSignal {
        pricing.expect("pricing data should be present").clone()
    }

    pub async fn analyze(input: &PricingSignal) -> PricingSignal {
        let change_pct = if input.price > 100.0 {
            ((input.price - 100.0) / 100.0) * 100.0
        } else {
            0.0
        };
        PricingSignal { price: input.price, change_pct }
    }

    pub async fn format_signal(input: &PricingSignal) -> SignalOutput {
        SignalOutput {
            message: format!(
                "Price: {:.2}, Change: {:.2}%",
                input.price, input.change_pct
            ),
        }
    }
}
```

Nothing new here — you already know this pattern. The interesting part starts below.

---

## Step 2: Implement a passthrough accumulator

An accumulator sits between an external data source and the reactor. It receives raw events, optionally transforms or filters them, and emits typed outputs that the reactor can cache.

```rust
use cloacina::computation_graph::accumulator::{
    accumulator_runtime, shutdown_signal, AccumulatorContext, AccumulatorRuntimeConfig,
    BoundarySender,
};
use cloacina::computation_graph::Accumulator;

struct PricingAccumulator;

#[async_trait::async_trait]
impl Accumulator for PricingAccumulator {
    type Event = PricingUpdate;    // what comes in from the socket channel
    type Output = PricingSignal;   // what goes out to the reactor

    fn process(&mut self, event: PricingUpdate) -> Option<PricingSignal> {
        // Passthrough: convert PricingUpdate → PricingSignal
        // Returning None would suppress this event (e.g. for filtering)
        Some(PricingSignal {
            price: event.mid_price,
            change_pct: 0.0, // analysis happens in the graph
        })
    }
}
```

`type Event` is the raw type you push into the socket channel. `type Output` is what the accumulator emits to the boundary channel after `process()`. Returning `None` from `process()` silently drops the event — useful for filtering or deduplication.

---

## Step 3: Wire the channels

The runtime model uses three channels:

| Channel | Direction | Purpose |
|---|---|---|
| **Socket** | External → Accumulator | You push serialized events here |
| **Boundary** | Accumulator → Reactor | Accumulator pushes typed, named data here |
| **Manual** | External → Reactor | Direct cache injection (unused in this tutorial) |

There is also a **shutdown** signal — a broadcast channel that all components watch.

```rust
// Boundary channel: accumulator sends named data to reactor
let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(10);

// Socket channel: external code pushes raw events to accumulator
let (socket_tx, socket_rx) = tokio::sync::mpsc::channel(10);

// Manual command channel for the reactor (required but unused here)
let (_manual_tx, manual_rx) = tokio::sync::mpsc::channel(10);

// Shutdown signal — shared by all components
let (shutdown_tx, shutdown_rx) = shutdown_signal();
```

`shutdown_signal()` returns a `(Sender<bool>, Receiver<bool>)` pair. The receiver can be cloned — you'll hand one clone to the accumulator and another to the reactor.

---

## Step 4: Spawn the accumulator

`BoundarySender` wraps the boundary channel sender and tags every outgoing message with a `SourceName`. The reactor uses this name to slot data into the correct `InputCache` entry.

```rust
let boundary_sender = BoundarySender::new(
    boundary_tx,
    SourceName::new("pricing"),  // must match react = when_any(pricing)
);

let acc_ctx = AccumulatorContext {
    output: boundary_sender,
    name: "pricing".to_string(),
    shutdown: shutdown_rx.clone(),
    checkpoint: None,
    health: None,
};

let _acc_handle = tokio::spawn(accumulator_runtime(
    PricingAccumulator,
    acc_ctx,
    socket_rx,
    AccumulatorRuntimeConfig::default(),
));
```

`accumulator_runtime` is the function that drives your `Accumulator` implementation. It:

1. Reads serialized bytes from `socket_rx`
2. Deserializes them to `Event` (here `PricingUpdate`)
3. Calls `process()` to produce `Output` (here `PricingSignal`)
4. Serializes and sends the output via `BoundarySender` to the reactor

---

## Step 5: Create and spawn the reactor

The `Reactor` listens on the boundary channel, maintains an `InputCache`, and fires the compiled graph when the reaction criteria are satisfied.

```rust
use cloacina::computation_graph::reactor::{
    CompiledGraphFn, InputStrategy, ReactionCriteria, Reactor,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

// Track how many times the graph fires
let fire_count = Arc::new(AtomicU32::new(0));
let fire_count_inner = fire_count.clone();

let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
    let fc = fire_count_inner.clone();
    Box::pin(async move {
        fc.fetch_add(1, Ordering::SeqCst);
        pricing_graph_compiled(&cache).await
    })
});

let reactor = Reactor::new(
    graph_fn,
    ReactionCriteria::WhenAny,    // fire when any source has new data
    InputStrategy::Latest,         // overwrite cache with newest value
    boundary_rx,
    manual_rx,
    shutdown_rx,
);

let _reactor_handle = tokio::spawn(reactor.run());
```

`CompiledGraphFn` is `Arc<dyn Fn(InputCache) -> BoxFuture<GraphResult>>`. You wrap your `_compiled` function in a closure that fits this signature. The `Arc` lets the reactor call it repeatedly.

`ReactionCriteria::WhenAny` means the graph fires every time any source sends a new value. `InputStrategy::Latest` means each new value for a source overwrites the previous one in the cache.

---

## Step 6: Push events and observe

You send events through `socket_tx` as serialized bytes. The pipeline handles everything from there.

```rust
let events = vec![
    PricingUpdate { mid_price: 99.50, timestamp: 1 },
    PricingUpdate { mid_price: 101.25, timestamp: 2 },
    PricingUpdate { mid_price: 103.75, timestamp: 3 },
];

for (i, event) in events.iter().enumerate() {
    let bytes = serialize(event).expect("serialization should succeed");
    socket_tx.send(bytes).await.expect("socket send should succeed");

    // Brief pause to let the pipeline process
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    println!("Graph has fired {} time(s)", fire_count.load(Ordering::SeqCst));
}

// Graceful shutdown
shutdown_tx.send(true).unwrap();
tokio::time::sleep(std::time::Duration::from_millis(100)).await;
```

Each event triggers one graph execution. After three events you'll see `fire_count` reach 3.

---

## Expected output

```
=== Tutorial 08: Accumulators ===

Spawning accumulator runtime...
Spawning reactor...

Pushing event 1: PricingUpdate { mid_price: 99.5, timestamp: 1 }
  Graph has fired 1 time(s)

Pushing event 2: PricingUpdate { mid_price: 101.25, timestamp: 2 }
  Graph has fired 2 time(s)

Pushing event 3: PricingUpdate { mid_price: 103.75, timestamp: 3 }
  Graph has fired 3 time(s)

Shutting down...

Total graph executions: 3

=== Tutorial 08 Complete ===
```

---

## Summary

You've wired your first live computation pipeline:

- **`Accumulator`** transforms raw events into typed outputs, optionally filtering them with `None`
- **`BoundarySender`** tags each output with a `SourceName` so the reactor knows which cache slot to update
- **`accumulator_runtime()`** drives the accumulator: deserialize → `process()` → serialize → send
- **`Reactor`** listens on the boundary channel, fires the compiled graph when criteria are met
- **`shutdown_signal()`** gives you coordinated teardown across all components

The pattern you've learned here — socket channel → accumulator → boundary channel → reactor → compiled graph — is the foundation for everything in the next tutorial.

## What's next?

- [Tutorial 09 — Full Reactive Pipeline]({{< ref "/computation-graphs/tutorials/library/09-full-pipeline/" >}}): connect multiple accumulators to one reactor and handle optional inputs in the graph
