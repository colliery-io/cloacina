---
title: "10 - Accumulators"
description: "Use @cloaca.passthrough_accumulator to transform raw events before they reach the computation graph"
weight: 20
---

In this tutorial you'll add an accumulator to your computation graph. Accumulators sit between raw data sources and the graph: they receive events, transform them, and emit the processed values that the graph's entry nodes consume.

## What you'll learn

- `@cloaca.passthrough_accumulator` — defining a simple event transformer
- How to call the accumulator manually to simulate event processing
- Wiring accumulator output into `builder.execute()`
- The separation of concerns between raw events and graph inputs

## Prerequisites

- Completion of [Tutorial 09 — Your First Computation Graph]({{< ref "/python/tutorials/computation-graphs/09-computation-graph/" >}})

## The complete example

The full source lives at [`examples/tutorials/python/computation-graphs/10_accumulators.py`](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials/python/computation-graphs/10_accumulators.py).

To run it:

```bash
python examples/tutorials/python/computation-graphs/10_accumulators.py
```

---

## Step 1: Define a passthrough accumulator

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

The function name (`pricing`) becomes the source name. This must match the key you use in the graph's `react` accumulator list and in `builder.execute()`.

The body receives a raw event dict and returns the processed dict that the graph will see. Returning `None` would suppress the event — useful for filtering duplicates or invalid data.

---

## Step 2: Define the computation graph

The graph topology is identical to Tutorial 09 — only the source name changes.

```python
with cloaca.ComputationGraphBuilder(
    "pricing_graph",
    react={"mode": "when_any", "accumulators": ["pricing"]},
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

---

## Step 3: Push events through the accumulator and graph

In a live deployment the accumulator runs as part of the computation graph runtime. For this tutorial you call it directly to simulate the pipeline.

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

Calling `pricing(event)` invokes your accumulator function and returns the transformed dict. You then pass that dict to `builder.execute()` under the same source name (`"pricing"`).

In a reactive deployment the runtime handles this automatically — the accumulator feeds the boundary channel, the reactor calls `execute()` for you. But calling them manually here makes the data flow explicit.

---

## Expected output

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

---

## The accumulator's role

```
Raw event             Accumulator          Graph entry node
{"mid_price": 99.5} → pricing(event)   → ingest(pricing)
                    ↓                  ↓
               {"price": 99.5,    passed as-is
                "change_pct": 0.0}  to analyze
```

The accumulator is responsible for:

1. **Shape translation** — converting the external event format to what the graph expects
2. **Filtering** — returning `None` to suppress unwanted events
3. **Stateful accumulation** — maintaining state between events (e.g. computing running averages) — the decorator keeps the function's local state alive across calls

---

## Comparing `@passthrough_accumulator` with the Rust `Accumulator` trait

| Concept | Rust | Python |
|---|---|---|
| Define accumulator | `impl Accumulator for MyAcc` | `@cloaca.passthrough_accumulator` |
| Transform event | `fn process(&mut self, event) -> Option<Output>` | function body, `return None` to suppress |
| Source name | `BoundarySender::new(tx, SourceName::new("pricing"))` | function name (`pricing`) |
| Invoke manually | `serialize(event)` → socket channel | `pricing(event)` |

---

## Summary

You've added an accumulator to your pipeline:

- `@cloaca.passthrough_accumulator` wraps a transformation function as a named accumulator
- The function name is the source name — it must match in `react`, `graph`, and `execute()`
- Returning `None` suppresses an event; returning a dict passes it to the graph
- Calling the accumulator directly and feeding its output to `execute()` makes the pipeline explicit during development

## What's next?

- [Tutorial 11 — Routing]({{< ref "/python/tutorials/computation-graphs/11-routing/" >}}): add conditional branching with tuple-based enum dispatch
