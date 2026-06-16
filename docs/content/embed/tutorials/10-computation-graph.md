---
title: "10 — Your First Computation Graph"
description: "Define a computation graph, declare its topology, and execute it with a hand-built InputCache"
weight: 20
aliases:
  - "/python/computation-graphs/tutorials/09-computation-graph/"
  - "/computation-graphs/tutorials/library/07-computation-graph/"

---

In this tutorial you'll build your first computation graph — a pricing pipeline that reads an order book snapshot, computes spread in basis points, and formats the result. You'll learn how Cloacina's two macros work together: `#[reactor]` declares the firing criterion, and `#[computation_graph]` references that reactor by name and wires async functions into a compiled, callable graph. In Python the same pipeline is built with `@cloaca.reactor`, `@cloaca.node`, and `cloaca.ComputationGraphBuilder`.

## What you'll learn

- How to define boundary types (the data that flows between nodes)
- The `#[reactor]` attribute macro: declaring the firing criterion as a top-level primitive
- The `#[computation_graph]` attribute macro and topology declaration syntax — including `trigger = reactor("...")`
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

> The on-disk example keeps its original number (`07`); these tutorials were renumbered, so the command number won't match the tutorial number — that's expected.

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

> **Python:** there are no boundary-type declarations — data flows between nodes as plain `dict`s (e.g. `{"best_bid": 100.50, "best_ask": 100.55}`), so this step has no Python equivalent.

---

## Step 2: Declare the reactor and the computation graph

As of CLOACI-I-0101 a graph's firing criterion is its own top-level primitive. You declare a reactor with `#[reactor]` (giving it a `name`, an `accumulators` list, and a `criteria` expression), then point one or more `#[computation_graph]` declarations at it via `trigger = reactor("name")`. Inside the annotated `mod`, each `pub async fn` becomes a node. In Python the same pieces are a `@cloaca.reactor` class, a `cloaca.ComputationGraphBuilder(...)` context manager whose `graph=` kwarg is a dict-of-dicts topology (`{"node": {"inputs": [...], "next": "..."}}`), and `@cloaca.node`-decorated functions defined inside the `with` block.

{{< tabs "cg10-step2" >}}
{{< tab "Rust" >}}
```rust
#[cloacina_macros::reactor(
    name = "pricing_pipeline_reactor",
    accumulators = [orderbook],
    criteria = when_any(orderbook),
)]
pub struct PricingPipelineReactor;

#[cloacina_macros::computation_graph(
    trigger = reactor("pricing_pipeline_reactor"),
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
{{< /tab >}}
{{< tab "Python" >}}
```python
import cloaca


# Declare the reactor that fires the graph (CLOACI-I-0101 split — the
# bundled `react={...}` kwarg was removed; reactors are now first-class
# `@cloaca.reactor` classes referenced by the builder via `reactor=`).
@cloaca.reactor(
    name="pricing_pipeline_reactor",
    accumulators=["orderbook"],
    mode="when_any",
)
class PricingPipelineReactor:
    pass


with cloaca.ComputationGraphBuilder(
    "pricing_pipeline",
    reactor=PricingPipelineReactor,
    graph={
        "ingest": {
            "inputs": ["orderbook"],   # reads from the cache by this name
            "next": "compute_spread",  # sends output to compute_spread
        },
        "compute_spread": {
            "next": "format_output",
        },
        "format_output": {},           # terminal node — no "next"
    },
) as builder:

    @cloaca.node
    def ingest(orderbook):
        """Entry node: extract key fields from order book."""
        if orderbook is None:
            return {"spread": 0.0, "mid_price": 0.0}
        spread = orderbook["best_ask"] - orderbook["best_bid"]
        mid_price = (orderbook["best_ask"] + orderbook["best_bid"]) / 2.0
        return {"spread": spread, "mid_price": mid_price}

    @cloaca.node
    def compute_spread(input_data):
        """Processing node: compute spread in basis points."""
        mid = input_data["mid_price"]
        if mid == 0:
            return input_data
        spread_bps = (input_data["spread"] / mid) * 10_000
        return {"spread_bps": spread_bps, "mid_price": mid}

    @cloaca.node
    def format_output(input_data):
        """Terminal node: format for display."""
        return {
            "message": f"Mid: {input_data['mid_price']:.2f}, Spread: {input_data['spread_bps']:.1f} bps",
            "mid_price": input_data["mid_price"],
            "spread_bps": input_data["spread_bps"],
        }
```
{{< /tab >}}
{{< /tabs >}}

**Topology breakdown:**

| Syntax | Meaning |
|---|---|
| `#[reactor(criteria = when_any(orderbook), ...)]` | Declares a reactor that fires whenever the `orderbook` source has new data |
| `trigger = reactor("pricing_pipeline_reactor")` | This graph subscribes to the reactor declared above (referenced by its string name) |
| `ingest(orderbook)` | `ingest` is an entry node; it reads `orderbook` from the cache |
| `-> compute_spread` | `ingest`'s output is passed to `compute_spread` as its input |
| `compute_spread -> format_output` | `format_output` receives `compute_spread`'s output |

**Node function signatures:**

- **Entry nodes** take `Option<&T>` for each named cache source. The `Option` is `None` if that source hasn't been populated yet.
- **Processing nodes** take `&T` where `T` is the return type of their upstream node.
- **The terminal node** is whichever node has no downstream — here `format_output`. Its return value ends up in `GraphResult`.

In Python the same roles hold, but the wiring differs: entry nodes (`ingest`) receive one named argument per source listed in `"inputs"` (`None` if absent); processing nodes (`compute_spread`, `format_output`) receive a single positional argument — the dict returned by their upstream node, conventionally named `input_data`. Node-function names must match the keys in the `graph` dict exactly, and the terminal node's return dict becomes the `execute()` result.

The macro generates a function called `pricing_pipeline_compiled` (the module name plus `_compiled`). In Python the `builder` object returned by the context manager plays that role and is invoked with `builder.execute(...)`.

---

## Step 3: Run the compiled graph

You don't need a reactor or accumulator for the simplest case. Build an `InputCache`, serialize your input into it, and call the generated function directly. In Python you skip the cache entirely: pass a dict of `{source_name: value}` to `builder.execute()`, which returns the terminal node's output dict.

{{< tabs "cg10-step3" >}}
{{< tab "Rust" >}}
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
    // — must match the accumulator name declared on the reactor
    //   (#[reactor(accumulators = [orderbook], criteria = when_any(orderbook), ...)])
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
{{< /tab >}}
{{< tab "Python" >}}
```python
# Input data — a dict matching the structure our entry node expects
orderbook = {"best_bid": 100.50, "best_ask": 100.55}
print(f"Input: {orderbook}\n")

result = builder.execute({"orderbook": orderbook})

print(f"Result: {result}")
print(f"  Message: {result.get('message', 'N/A')}")
print(f"  Mid price: {result.get('mid_price', 'N/A')}")
print(f"  Spread: {result.get('spread_bps', 'N/A')} bps")
```
{{< /tab >}}
{{< /tabs >}}

**Key points:**

- `SourceName::new("orderbook")` must exactly match the accumulator name in the reactor declaration (`accumulators = [orderbook]`) and in the topology (`ingest(orderbook)`).
- `serialize()` converts your value to `Vec<u8>` using the same codec the cache uses internally.
- `GraphResult::Completed { outputs }` carries a `Vec<Box<dyn Any>>`. Use `downcast_ref::<T>()` to get your concrete type back.
- `GraphResult::Error(e)` carries a string describing what went wrong.

In Python `execute()` takes a dict where each key is a source name and each value is the data placed in the cache for that source; it returns the terminal node's output dict directly (no `GraphResult` wrapper or downcast).

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

- **`#[reactor]`** declares the firing criterion as a top-level primitive (`name`, `accumulators`, `criteria`)
- **`#[computation_graph]`** declares the topology, subscribes to a reactor via `trigger = reactor("name")`, and generates the `_compiled` function
- **Entry nodes** receive `Option<&T>` from the `InputCache`; processing nodes receive `&T` from their upstream peer
- **`InputCache`** holds named, serialized data that feeds entry nodes
- **`GraphResult::Completed`** carries boxed terminal outputs; downcast them to your concrete types

The `_compiled` function is the building block for everything that follows. In the next tutorial you'll wrap it in an accumulator and reactor to create a live, event-driven pipeline.

## What's next?

- [Tutorial 11 — Accumulators]({{< ref "/embed/tutorials/11-accumulators/" >}}): wire the compiled graph into a reactor driven by live events
