---
title: "Reactor"
description: "Binds accumulators to a computation graph and fires it when its criteria are met."
weight: 23
---

# Reactor

A **Reactor** is the long-lived process that binds a set of named
[Accumulators]({{< ref "/engine/computation-graphs/accumulator" >}}) to a
[Computation Graph]({{< ref "/engine/computation-graphs/computation-graph" >}}),
maintains an input cache from the [Boundary events]({{< ref "/engine/computation-graphs/boundary" >}})
they emit, and **fires the graph when its criteria are satisfied**. The firing
criterion is a top-level primitive: a graph subscribes to a reactor by name.

## Mental model

- A reactor declares which **accumulators** feed it and a **criterion**:
  `when_any` (fire when *any* source has new data) or `when_all` (wait for all).
- It keeps an **input cache** and an **input strategy** — `latest` (overwrite with
  newest) or `sequential`.
- When the criterion is met, it calls the compiled graph with the current cache.

## Interfaces

{{< tabs "reactor-define" >}}
{{< tab "Rust" >}}
`#[reactor]` declares the reactor as a top-level primitive on a struct:

```rust
#[cloacina_macros::reactor(
    name = "pricing_reactor",
    accumulators = [orderbook],
    criteria = when_any(orderbook),
)]
pub struct PricingReactor;
```
(The lower-level runtime exposes `Reactor::new(graph_fn, ReactionCriteria::WhenAny,
InputStrategy::Latest, …)` — see the embedded tutorials.)
{{< /tab >}}
{{< tab "Python" >}}
`@cloaca.reactor` decorates a class; criteria is the `mode` argument:

```python
import cloaca

@cloaca.reactor(
    name="pricing_reactor",
    accumulators=["orderbook"],
    mode="when_any",        # or "when_all"
)
class PricingReactor:
    pass
```
{{< /tab >}}
{{< /tabs >}}

## Key facts

- **Criteria:** `when_any` / `when_all` over the named accumulator sources.
- **Input strategy:** `latest` or `sequential` (Rust `InputStrategy`).
- **Naming:** the `accumulators` names must match the accumulator source names and
  the graph's entry-node source names.

## See also

- [Computation Graph]({{< ref "/engine/computation-graphs/computation-graph" >}}) · [Accumulator]({{< ref "/engine/computation-graphs/accumulator" >}}) · [Boundary event]({{< ref "/engine/computation-graphs/boundary" >}})
- [Reactor-triggered workflows]({{< ref "/computation-graphs/how-to-guides/reactor-triggered-workflows" >}})
