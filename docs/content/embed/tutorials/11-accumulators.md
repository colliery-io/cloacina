---
title: "11 — Accumulators"
description: "Implement a passthrough accumulator, wire channels, spawn a reactor, and push live events through a compiled graph"
weight: 21
aliases:
  - "/python/computation-graphs/tutorials/10-accumulators/"
  - "/computation-graphs/tutorials/library/08-accumulators/"

---

In this tutorial you'll move beyond calling the compiled graph by hand. You'll implement an `Accumulator`, wire up the channel plumbing between it and a `Reactor`, and watch the graph fire automatically as events arrive.

## What you'll learn

- The `Accumulator` trait: `process()` and the `Output` associated type
- `BoundarySender` — how the accumulator hands data off to the reactor
- `AccumulatorContext` and `AccumulatorRuntimeConfig`
- `accumulator_runtime()` — the three-task merge-channel model
- `shutdown_signal()` for coordinated teardown
- `Reactor` with `ReactionCriteria::WhenAny` and `InputStrategy::Latest`
- Pushing serialized events and observing the graph fire

## Prerequisites

- Completion of [Tutorial 10 — Your First Computation Graph]({{< ref "/embed/tutorials/10-computation-graph/" >}})

## The complete example

The full source lives at:

{{< tabs "src-accumulators" >}}
{{< tab "Rust" >}}
[`examples/tutorials/computation-graphs/library/08-accumulators`](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials/computation-graphs/library/08-accumulators)

```bash
angreal demos tutorials rust 08
```

> The on-disk example keeps its original number (`08`); these tutorials were renumbered, so the command number won't match the tutorial number — that's expected.
{{< /tab >}}
{{< tab "Python" >}}
[`examples/tutorials/engine/computation-graphs/10_accumulators.py`](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials/engine/computation-graphs/10_accumulators.py)

```bash
python examples/tutorials/engine/computation-graphs/10_accumulators.py
```
{{< /tab >}}
{{< /tabs >}}

---

## Step 1: Boundary types and the graph

The graph for this tutorial follows the same structure as the previous one. A pricing update arrives from outside, the accumulator converts it to a pricing signal, and the graph processes and formats it.

{{< tabs "step1-graph" >}}
{{< tab "Rust" >}}
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

#[cloacina_macros::reactor(
    name = "pricing_graph_reactor",
    accumulators = [pricing],
    criteria = when_any(pricing),
)]
pub struct PricingGraphReactor;

#[cloacina_macros::computation_graph(
    trigger = reactor("pricing_graph_reactor"),
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
{{< /tab >}}
{{< tab "Python" >}}
The graph topology is identical to the previous tutorial — only the source name changes. In Python the boundary types are plain dicts rather than declared structs.

```python
# Declare the reactor that fires the graph (CLOACI-I-0101 split — the
# bundled `react={...}` kwarg was removed in favour of first-class
# `@cloaca.reactor` classes).
@cloaca.reactor(
    name="pricing_graph_reactor",
    accumulators=["pricing"],
    mode="when_any",
)
class PricingGraphReactor:
    pass


with cloaca.ComputationGraphBuilder(
    "pricing_graph",
    reactor=PricingGraphReactor,
    graph={
        "ingest": {
            "inputs": ["pricing"],
            "next": "analyze",
        },
        "analyze": {
            "next": "format_signal",
        },
        "format_signal": {},
    },
) as builder:

    @cloaca.node
    def ingest(pricing):
        """Entry node: receive pricing data from accumulator."""
        if pricing is None:
            return {"price": 0.0, "change_pct": 0.0}
        return pricing  # accumulator already shaped the data

    @cloaca.node
    def analyze(input_data):
        """Analyze pricing for large moves."""
        price = input_data["price"]
        change_pct = ((price - 100.0) / 100.0) * 100.0 if price > 100.0 else 0.0
        return {"price": price, "change_pct": change_pct}

    @cloaca.node
    def format_signal(input_data):
        """Terminal node: format the signal."""
        return {
            "message": f"Price: {input_data['price']:.2f}, Change: {input_data['change_pct']:.2f}%",
        }
```

Notice that `ingest` simply passes its input through — the accumulator already did the heavy lifting of shaping `mid_price` into the `{price, change_pct}` structure. This separation keeps nodes focused: accumulators transform raw external data, nodes process structured graph data.
{{< /tab >}}
{{< /tabs >}}

Nothing new in the graph itself — you already know this pattern. The interesting part starts below.

---

## Step 2: Implement a passthrough accumulator

An accumulator sits between an external data source and the reactor. It receives raw events, optionally transforms or filters them, and emits outputs that the reactor can cache.

{{< tabs "step2-accumulator" >}}
{{< tab "Rust" >}}
```rust
use cloacina::computation_graph::accumulator::{
    accumulator_runtime, shutdown_signal, AccumulatorContext, AccumulatorRuntimeConfig,
    BoundarySender,
};
use cloacina::computation_graph::Accumulator;

struct PricingAccumulator;

#[async_trait::async_trait]
impl Accumulator for PricingAccumulator {
    type Output = PricingSignal;   // what goes out to the reactor

    fn process(&mut self, event: Vec<u8>) -> Option<PricingSignal> {
        // The runtime hands `process` the raw serialized bytes; deserialize
        // to the type the sender used (types::serialize == bincode).
        let update: PricingUpdate =
            cloacina::computation_graph::types::deserialize(&event).ok()?;
        // Passthrough: convert PricingUpdate → PricingSignal.
        // Returning None would suppress this event (e.g. for filtering).
        Some(PricingSignal {
            price: update.mid_price,
            change_pct: 0.0, // analysis happens in the graph
        })
    }
}
```
{{< /tab >}}
{{< tab "Python" >}}
A passthrough accumulator transforms one dict shape into another. Decorate a function with `@cloaca.passthrough_accumulator` and give it a name that matches the source name in your graph topology.

```python
import cloaca

@cloaca.passthrough_accumulator
def pricing(event):
    """Transform a raw pricing event into a pricing signal.

    Input event shape:  {"mid_price": float}
    Output shape:       {"price": float, "change_pct": float}
    """
    return {"price": event["mid_price"], "change_pct": 0.0}
```

The function name (`pricing`) becomes the source name. This must match the key you use in the reactor's `accumulators` list, the graph topology, and in `builder.execute()`.
{{< /tab >}}
{{< /tabs >}}

In Rust, `process()` receives the raw serialized bytes off the socket channel — deserialize them to your event type, then return the `Output` the accumulator emits to the boundary channel. In Python the function receives a raw event dict and returns the processed dict. In both languages, returning `None` silently drops the event — useful for filtering or deduplication.

Cloacina ships four accumulator decorators in Python — `@cloaca.passthrough_accumulator`, `@cloaca.stream_accumulator`, `@cloaca.polling_accumulator`, and `@cloaca.batch_accumulator`. The Rust `Accumulator` trait additionally supports a `#[state_accumulator]` form that has no Python equivalent yet (tracked in CLOACI-T-0688).

---

## Step 3: Wire the channels

> **Python note:** Steps 3 through 5 wire the live runtime — channels, the accumulator task, and the reactor — by hand. In Python this plumbing is handled by the runtime for you; for a tutorial you simply call the accumulator function directly and feed its output to `builder.execute()`. Skip ahead to [Step 6](#step-6-push-events-and-observe) for the Python flow, then read Steps 3–5 to understand what the runtime does under the hood.

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
    SourceName::new("pricing"),  // must match accumulators = [pricing] / criteria = when_any(pricing) on the reactor
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

1. Reads raw serialized bytes from `socket_rx`
2. Hands those bytes to `process()`, which deserializes them itself (here to `PricingUpdate`) and returns an `Output` (here `PricingSignal`) — the runtime is format-agnostic
3. Serializes and sends the output via `BoundarySender` to the reactor

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

In Rust you send events through `socket_tx` as serialized bytes and the pipeline handles everything from there. In Python — where there is no live runtime to spawn for a tutorial — you call the accumulator directly and pass its output to `builder.execute()`, making the data flow explicit.

{{< tabs "step6-push" >}}
{{< tab "Rust" >}}
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
{{< /tab >}}
{{< tab "Python" >}}
```python
events = [
    {"mid_price": 99.50},
    {"mid_price": 101.25},
    {"mid_price": 103.75},
]

for i, event in enumerate(events, 1):
    print(f"Event {i}: {event}")

    # Step 1: accumulator transforms the raw event
    processed = pricing(event)
    print(f"  Accumulator output: {processed}")

    # Step 2: graph processes the accumulator's output
    result = builder.execute({"pricing": processed})
    print(f"  Graph result: {result.get('message', 'N/A')}\n")
```

Calling `pricing(event)` invokes your accumulator function and returns the transformed dict. You then pass that dict to `builder.execute()` under the same source name (`"pricing"`). In a reactive deployment the runtime handles this automatically — the accumulator feeds the boundary channel and the reactor calls `execute()` for you — but calling them manually here makes the data flow explicit.
{{< /tab >}}
{{< /tabs >}}

Each event triggers one graph execution. In Rust, after three events you'll see `fire_count` reach 3.

---

## Expected output

{{< tabs "expected-output" >}}
{{< tab "Rust" >}}
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
{{< /tab >}}
{{< tab "Python" >}}
```
=== Python Tutorial 10: Accumulators ===

Event 1: {'mid_price': 99.5}
  Accumulator output: {'price': 99.5, 'change_pct': 0.0}
  Graph result: Price: 99.50, Change: 0.00%

Event 2: {'mid_price': 101.25}
  Accumulator output: {'price': 101.25, 'change_pct': 0.0}
  Graph result: Price: 101.25, Change: 1.25%

Event 3: {'mid_price': 103.75}
  Accumulator output: {'price': 103.75, 'change_pct': 0.0}
  Graph result: Price: 103.75, Change: 3.75%

=== Tutorial 10 Complete ===
```

Event 1 produces `Change: 0.00%` because the price is below 100. Events 2 and 3 compute the percentage above baseline.
{{< /tab >}}
{{< /tabs >}}

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

- [Tutorial 12 — Full Reactive Pipeline]({{< ref "/embed/tutorials/12-full-pipeline/" >}}): connect multiple accumulators to one reactor and handle optional inputs in the graph
