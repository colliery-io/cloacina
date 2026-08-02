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

- **A reactor is a specialized trigger** (CLOACI-S-0011): where a workflow
  trigger polls or follows a cron schedule, a reactor consumes accumulator
  boundary events and fires a computation graph.
- **Criteria:** `when_any` / `when_all` over the named accumulator sources.
- **Input strategy:** `latest` or `sequential` (Rust `InputStrategy`). There is
  **no `input_strategy` clause on the `#[reactor]` macro** — set it
  programmatically or in the package manifest; see
  [Sequential input strategy]({{< ref "/engine/computation-graphs/how-to/sequential-strategy" >}}).
- **Naming:** the `accumulators` names must match the accumulator source names and
  the graph's entry-node source names.
- **Operator controls:** manual fire is REST
  (`POST /v1/health/reactors/{name}/fire`, `cloacinactl reactor fire|force-fire`);
  pause/resume is **WebSocket-only** (`/v1/ws/reactor/{name}`) — there is no REST
  pause route, and reactor→workflow subscriptions have no HTTP API at all
  (runner API only).

## See also

- [Computation Graph]({{< ref "/engine/computation-graphs/computation-graph" >}}) · [Accumulator]({{< ref "/engine/computation-graphs/accumulator" >}}) · [Boundary event]({{< ref "/engine/computation-graphs/boundary" >}})
- [Reactor-triggered workflows]({{< ref "/engine/computation-graphs/how-to/reactor-triggered-workflows" >}})
