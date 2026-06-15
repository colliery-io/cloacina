---
title: "Node"
description: "A vertex in a computation graph — it only exists inside a graph."
weight: 22
---

# Node

A **Node** is a vertex in a [Computation Graph]({{< ref "/engine/computation-graphs/computation-graph" >}}).
It is **coupled to its graph** — a node has no meaning outside one; the graph's
topology defines what each node receives and where its output goes.

## The three roles

- **Entry node** — reads one or more named sources from the cache. In Rust it
  takes `Option<&T>` per source (the `Option` is `None` until that source is
  populated); in Python it receives the owned value.
- **Processing node** — takes its upstream node's output (`&T` in Rust, owned in
  Python) and returns a new value.
- **Terminal node** — the node with no downstream; its return value is the graph's
  result.

## Interfaces

{{< tabs "node-define" >}}
{{< tab "Rust" >}}
```rust
// entry node: reads the "orderbook" source
pub async fn ingest(orderbook: Option<&OrderBookSnapshot>) -> SpreadSignal { /* ... */ }

// processing/terminal node: takes its upstream's output
pub async fn format_output(input: &SpreadSignal) -> FormattedOutput { /* ... */ }
```
{{< /tab >}}
{{< tab "Python" >}}
```python
@cloaca.node
def ingest(orderbook):          # entry: receives the owned source value
    return spread_signal(orderbook)

@cloaca.node
def format_output(ingest):      # processing: parameter name = upstream node
    return formatted(ingest)
```
Routing nodes return a `(variant_name, value)` tuple; the variant selects the
downstream route.
{{< /tab >}}
{{< /tabs >}}

## See also

- [Computation Graph]({{< ref "/engine/computation-graphs/computation-graph" >}}) — nodes only exist inside one.
- [Boundary event]({{< ref "/engine/computation-graphs/boundary" >}}) — what populates an entry node's source.
