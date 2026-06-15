---
title: "Computation Graph"
description: "An in-process, event-driven dataflow DAG — the whole traversal is the unit of execution."
weight: 21
---

# Computation Graph

A **Computation Graph** is an in-process, deterministic, event-driven DAG. Unlike
a [Workflow]({{< ref "/engine/workflows/workflow" >}}) (where the task is the unit
and state is persisted), a computation graph runs **as a single traversal** in
memory, in response to an event. It is built from [Nodes]({{< ref "/engine/computation-graphs/node" >}})
and is fired either by a [Reactor]({{< ref "/engine/computation-graphs/reactor" >}})
or invoked directly from a workflow task.

## Mental model

- A graph is a set of **nodes** wired by a **topology** (`a -> b`).
- **Entry nodes** read named sources from an `InputCache`; **processing nodes**
  take their upstream node's output; the **terminal node** (no downstream) produces
  the result.
- It compiles to a callable function (`<name>_compiled` in Rust) that the reactor
  invokes; the [Boundary events]({{< ref "/engine/computation-graphs/boundary" >}})
  an [Accumulator]({{< ref "/engine/computation-graphs/accumulator" >}}) emits are
  what populate the cache.

## Interfaces

A two-node pricing graph triggered by a reactor:

{{< tabs "cg-define" >}}
{{< tab "Rust" >}}
The `#[computation_graph]` macro declares the topology and generates the compiled
function; each `pub async fn` in the module is a node:

```rust
#[cloacina_macros::computation_graph(
    trigger = reactor("pricing_reactor"),
    graph = {
        ingest(orderbook) -> format_output,
    }
)]
pub mod pricing {
    use super::*;
    pub async fn ingest(orderbook: Option<&OrderBookSnapshot>) -> SpreadSignal { /* ... */ }
    pub async fn format_output(input: &SpreadSignal) -> FormattedOutput { /* ... */ }
}
// generates pricing_compiled(&InputCache) -> GraphResult
```
{{< /tab >}}
{{< tab "Python" >}}
`ComputationGraphBuilder` is a context manager; nodes are `@cloaca.node`
functions inside it, and the topology is a dict:

```python
import cloaca

@cloaca.reactor(name="pricing_reactor", accumulators=["orderbook"], mode="when_any")
class PricingReactor:
    pass

with cloaca.ComputationGraphBuilder(
    "pricing", reactor=PricingReactor,
    graph={
        "ingest": {"inputs": ["orderbook"], "next": "format_output"},
        "format_output": {},   # no `next` → terminal
    },
) as g:
    @cloaca.node
    def ingest(orderbook):
        return spread_signal(orderbook)

    @cloaca.node
    def format_output(ingest):
        return formatted(ingest)
```

`reactor=` takes a `@cloaca.reactor`-decorated **class**, and each `graph` entry
is a **dict** (`inputs` list, plus `next` / `routes`, or empty for terminal). See
the [topology dict schema]({{< ref "/reference/topology-dict-schema" >}})
for the full format. Python node functions receive **owned** values (the boundary
copies them) where Rust takes references.
{{< /tab >}}
{{< /tabs >}}

## Key facts

- **Reactor-triggered or trigger-less.** A graph either subscribes to a reactor
  (`trigger = reactor("…")`) or is trigger-less and invoked inline from a workflow
  task.
- **Data is serialized.** Values that cross node boundaries implement
  `Serialize`/`Deserialize`; the cache stores bytes.
- **Deterministic.** Given the same cache, the traversal is deterministic.

## Build one

- **Embed it** → [Embed · Tutorials]({{< ref "/embed/tutorials" >}})
- **Ship it to a server** → [Run the Service · Tutorials]({{< ref "/service/tutorials" >}})

## See also

- [Node]({{< ref "/engine/computation-graphs/node" >}}) · [Reactor]({{< ref "/engine/computation-graphs/reactor" >}}) · [Accumulator]({{< ref "/engine/computation-graphs/accumulator" >}}) · [Boundary event]({{< ref "/engine/computation-graphs/boundary" >}})
