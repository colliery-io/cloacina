---
title: "09 - Your First Computation Graph"
description: "Define a Python computation graph with ComputationGraphBuilder and @cloaca.node, then execute it"
weight: 10
---

In this tutorial you'll build your first computation graph in Python — the same pricing pipeline from [Rust Tutorial 07]({{< ref "/tutorials/computation-graphs/library/07-computation-graph/" >}}), using Cloacina's `cloaca` Python bindings. You'll define nodes with a decorator, declare the topology in a `with` block, and execute the graph against live input data.

## What you'll learn

- `cloaca.ComputationGraphBuilder` — the context manager that declares graph topology
- `@cloaca.node` — the decorator that registers a function as a graph node
- Topology declaration via Python dict (`inputs`, `next`)
- Executing a graph with `builder.execute()` and reading results

## Prerequisites

- Python 3.8+
- `cloaca` installed: `pip install cloaca`

## The complete example

The full source lives at [`examples/tutorials/python/computation-graphs/09_computation_graph.py`](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials/python/computation-graphs/09_computation_graph.py).

To run it:

```bash
python examples/tutorials/python/computation-graphs/09_computation_graph.py
```

---

## Step 1: Declare the graph topology

In Python you declare the topology by opening a `ComputationGraphBuilder` context manager. Inside the `with` block you define each node with `@cloaca.node`.

```python
import cloaca

with cloaca.ComputationGraphBuilder(
    "pricing_pipeline",
    react={"mode": "when_any", "accumulators": ["orderbook"]},
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
```

The `graph` dict mirrors the Rust topology syntax:

| Rust | Python equivalent |
|---|---|
| `ingest(orderbook) -> compute_spread` | `"ingest": {"inputs": ["orderbook"], "next": "compute_spread"}` |
| `compute_spread -> format_output` | `"compute_spread": {"next": "format_output"}` |
| `format_output` (terminal) | `"format_output": {}` |

`react` tells the graph when to fire. `"mode": "when_any"` fires whenever any named accumulator delivers new data.

---

## Step 2: Define node functions

Inside the `with` block, decorate each node function with `@cloaca.node`. Node names must match the keys in the `graph` dict exactly.

```python
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

**Node function signatures:**

- **Entry nodes** (`ingest`) receive arguments named after each source listed in `"inputs"`. The value is `None` if that source hasn't been populated yet.
- **Processing nodes** (`compute_spread`, `format_output`) receive a single `input_data` argument — the dict returned by the upstream node.
- **Return values** are plain Python dicts. The terminal node's return dict becomes the `execute()` result.

---

## Step 3: Execute the graph

After the `with` block closes, `builder` holds the configured graph. Call `builder.execute()` with a dict mapping source names to input values.

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

`execute()` takes a dict where each key is a source name and each value is the data to place in the cache for that source. It returns the terminal node's output dict.

---

## Expected output

```
=== Python Tutorial 09: Your First Computation Graph ===

Input: {'best_bid': 100.5, 'best_ask': 100.55}

Result: {'message': 'Mid: 100.52, Spread: 4.9 bps', 'mid_price': 100.525, 'spread_bps': 4.926...}
  Message: Mid: 100.52, Spread: 4.9 bps
  Mid price: 100.525
  Spread: 4.9 bps

=== Tutorial 09 Complete ===
```

---

## Comparing Python and Rust

| Concept | Rust | Python |
|---|---|---|
| Graph declaration | `#[computation_graph(...)] pub mod name { }` | `with ComputationGraphBuilder("name", ...) as builder:` |
| Node definition | `pub async fn node_name(...)` | `@cloaca.node` + `def node_name(...)` |
| Entry node inputs | `Option<&T>` for each source | named argument per source, `None` if absent |
| Calling the graph | `name_compiled(&cache).await` | `builder.execute({...})` |
| Result type | `GraphResult::Completed { outputs }` | plain dict |

---

## Summary

You've defined and executed your first Python computation graph:

- `ComputationGraphBuilder` declares the graph name, reaction mode, and topology in one `with` block
- `@cloaca.node` registers each function and its position in the graph
- Entry nodes receive source values as named arguments (`None` if absent)
- `builder.execute({source: value})` runs the graph and returns the terminal node's output

## What's next?

- [Tutorial 10 — Accumulators]({{< ref "/tutorials/computation-graphs/python/10-accumulators/" >}}): use `@cloaca.passthrough_accumulator` to transform raw events before they reach the graph
