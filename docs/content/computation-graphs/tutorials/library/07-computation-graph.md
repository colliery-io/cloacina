---
title: "07 - Your First Computation Graph"
description: "Define a computation graph, declare its topology, and execute it with a hand-built InputCache"
weight: 10
---

In this tutorial you'll build your first computation graph — a pricing pipeline that reads an order book snapshot, computes spread in basis points, and formats the result. You'll learn how the `#[computation_graph]` macro wires async functions into a compiled, callable graph.

## What you'll learn

- How to define boundary types (the data that flows between nodes)
- The `#[computation_graph]` attribute macro and topology declaration syntax
- Entry nodes, processing nodes, and terminal nodes
- `InputCache`, `SourceName`, and `serialize()`
- Calling the generated `_compiled()` function and inspecting `GraphResult`

## Prerequisites

- Completion of a Cloacina workflow tutorial, or basic Rust async familiarity
- Rust toolchain installed

## The complete example

The full source lives at [`examples/tutorials/computation-graphs/library/07-computation-graph`](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials/computation-graphs/library/07-computation-graph).

To run it:

```bash
# From the Cloacina repository root
angreal demos tutorials rust 07
```

---

## Step 1: Define your boundary types

Every piece of data that flows between graph nodes must implement `Serialize + Deserialize`. The `InputCache` stores values as serialized bytes, so serde is required throughout.

```rust
use cloacina::computation_graph::types::{serialize, GraphResult, InputCache, SourceName};
use serde::{Deserialize, Serialize};

/// Raw order book snapshot — our input data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub best_bid: f64,
    pub best_ask: f64,
    pub timestamp: u64,
}

/// Computed spread signal — intermediate result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadSignal {
    pub spread: f64,
    pub mid_price: f64,
}

/// Final formatted output — terminal node result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattedOutput {
    pub message: String,
    pub mid_price: f64,
    pub spread_bps: f64,
}
```

You define one struct per data boundary. `OrderBookSnapshot` enters the graph from outside, `SpreadSignal` flows between nodes internally, and `FormattedOutput` is what the graph produces.

---

## Step 2: Declare the computation graph

The `#[computation_graph]` macro takes two arguments: a reaction criterion (`react`) and the topology (`graph`). Inside the annotated `mod`, each `pub async fn` becomes a node.

```rust
#[cloacina_macros::computation_graph(
    react = when_any(orderbook),
    graph = {
        ingest(orderbook) -> compute_spread,
        compute_spread -> format_output,
    }
)]
pub mod pricing_pipeline {
    use super::*;

    /// Entry node: reads the order book from the cache.
    pub async fn ingest(orderbook: Option<&OrderBookSnapshot>) -> SpreadSignal {
        let book = orderbook.expect("orderbook should be present");
        let spread = book.best_ask - book.best_bid;
        let mid_price = (book.best_ask + book.best_bid) / 2.0;
        SpreadSignal { spread, mid_price }
    }

    /// Processing node: converts spread to basis points.
    pub async fn compute_spread(input: &SpreadSignal) -> SpreadSignal {
        let spread_bps = (input.spread / input.mid_price) * 10_000.0;
        SpreadSignal {
            spread: spread_bps,
            mid_price: input.mid_price,
        }
    }

    /// Terminal node: formats the result for display.
    pub async fn format_output(input: &SpreadSignal) -> FormattedOutput {
        FormattedOutput {
            message: format!(
                "Mid: {:.2}, Spread: {:.1} bps",
                input.mid_price, input.spread
            ),
            mid_price: input.mid_price,
            spread_bps: input.spread,
        }
    }
}
```

**Topology breakdown:**

| Syntax | Meaning |
|---|---|
| `react = when_any(orderbook)` | Fire the graph whenever the `orderbook` source has new data |
| `ingest(orderbook)` | `ingest` is an entry node; it reads `orderbook` from the cache |
| `-> compute_spread` | `ingest`'s output is passed to `compute_spread` as its input |
| `compute_spread -> format_output` | `format_output` receives `compute_spread`'s output |

**Node function signatures:**

- **Entry nodes** take `Option<&T>` for each named cache source. The `Option` is `None` if that source hasn't been populated yet.
- **Processing nodes** take `&T` where `T` is the return type of their upstream node.
- **The terminal node** is whichever node has no downstream — here `format_output`. Its return value ends up in `GraphResult`.

The macro generates a function called `pricing_pipeline_compiled` (the module name plus `_compiled`).

---

## Step 3: Run the compiled graph

You don't need a reactor or accumulator for the simplest case. Build an `InputCache`, serialize your input into it, and call the generated function directly.

```rust
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Build an InputCache with our order book data
    let mut cache = InputCache::new();
    let orderbook = OrderBookSnapshot {
        best_bid: 100.50,
        best_ask: 100.55,
        timestamp: 1234567890,
    };

    // Serialize and insert under the source name "orderbook"
    // — must match the name declared in react = when_any(orderbook)
    cache.update(
        SourceName::new("orderbook"),
        serialize(&orderbook).expect("serialization should succeed"),
    );

    // Call the compiled graph
    let result: GraphResult = pricing_pipeline_compiled(&cache).await;

    match result {
        GraphResult::Completed { outputs } => {
            for output in &outputs {
                if let Some(formatted) = output.downcast_ref::<FormattedOutput>() {
                    println!("Output: {}", formatted.message);
                    println!("Mid price: {:.2}", formatted.mid_price);
                    println!("Spread: {:.1} bps", formatted.spread_bps);
                }
            }
        }
        GraphResult::Error(e) => {
            eprintln!("Graph execution failed: {}", e);
        }
    }
}
```

**Key points:**

- `SourceName::new("orderbook")` must exactly match the name you used in the topology — `ingest(orderbook)` and `react = when_any(orderbook)`.
- `serialize()` converts your value to `Vec<u8>` using the same codec the cache uses internally.
- `GraphResult::Completed { outputs }` carries a `Vec<Box<dyn Any>>`. Use `downcast_ref::<T>()` to get your concrete type back.
- `GraphResult::Error(e)` carries a string describing what went wrong.

---

## Expected output

```
=== Tutorial 07: Your First Computation Graph ===

Input: OrderBookSnapshot { best_bid: 100.5, best_ask: 100.55, timestamp: 1234567890 }

Graph completed with 1 terminal output(s)
  Output: Mid: 100.52, Spread: 4.9 bps
  Mid price: 100.52
  Spread: 4.9 bps

=== Tutorial 07 Complete ===
```

---

## Summary

You've built and executed your first computation graph:

- **`#[computation_graph]`** declares the topology and generates the `_compiled` function
- **Entry nodes** receive `Option<&T>` from the `InputCache`; processing nodes receive `&T` from their upstream peer
- **`InputCache`** holds named, serialized data that feeds entry nodes
- **`GraphResult::Completed`** carries boxed terminal outputs; downcast them to your concrete types

The `_compiled` function is the building block for everything that follows. In the next tutorial you'll wrap it in an accumulator and reactor to create a live, event-driven pipeline.

## What's next?

- [Tutorial 08 — Accumulators]({{< ref "/computation-graphs/tutorials/library/08-accumulators/" >}}): wire the compiled graph into a reactor driven by live events
