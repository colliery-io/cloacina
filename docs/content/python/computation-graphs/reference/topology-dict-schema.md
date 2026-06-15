---
title: "Topology Dict Schema"
description: "The Python topology dictionary consumed by ComputationGraphBuilder — node spec, inputs, routing, and terminal nodes."
weight: 10
---

# Topology Dict Schema

`cloaca.ComputationGraphBuilder(name, reactor=..., graph={...})` takes a `graph`
keyword whose value is a **topology dictionary**: a map from node name to a node
spec. This page is the reference for that dictionary's shape.

## Shape

```python
graph = {
    "<node_name>": {
        "inputs": ["<accumulator_name>", ...],   # optional
        # exactly one of the following (or neither, for a terminal node):
        "next": "<downstream_node>",              # linear edge
        "routes": {"<Variant>": "<node>", ...},   # conditional routing
    },
    ...
}
```

Each key is a node name that must have a matching `@cloaca.node`-decorated
function registered inside the builder's `with` block.

## Node spec keys

| Key | Type | Required | Meaning |
|-----|------|----------|---------|
| `inputs` | `list[str]` | No | Accumulator names this node reads as cache inputs. Each must be one of the reactor's `accumulators`. Omit for nodes fed only by upstream node outputs. |
| `next` | `str` | No | Name of the single downstream node this node's output flows to. Mutually exclusive with `routes`. |
| `routes` | `dict[str, str]` | No | Conditional routing: maps an output **variant name** to the downstream node for that variant. Mutually exclusive with `next`. |

A node spec with **neither** `next` nor `routes` is a **terminal node** — its
output is collected into the graph result and not forwarded.

## Routing nodes

A node that declares `routes` must return a `(variant_name, value)` tuple: the
`variant_name` (a `str`) selects which route fires, and `value` (a dict) is passed
to that downstream node. For example, a node with
`"routes": {"Trade": "signal_handler", "NoAction": "audit_logger"}` returns either
`("Trade", {...})` or `("NoAction", {...})`.

Non-routing nodes return their output value directly.

## Validation

The builder validates the topology when the `with` block exits and raises if:

- a node in the topology has no registered `@cloaca.node` function (or vice versa);
- a `reactor` is provided and an `inputs` entry isn't one of the reactor's
  `accumulators`;
- no `reactor` is provided (trigger-less graph) but a node declares `inputs`.

## Example

```python
import cloaca

@cloaca.reactor(name="market_maker", accumulators=["orderbook", "pricing"], mode="when_any")
class MarketMakerReactor:
    pass

with cloaca.ComputationGraphBuilder(
    "market_maker",
    reactor=MarketMakerReactor,
    graph={
        "decision": {
            "inputs": ["orderbook", "pricing"],
            "routes": {"Trade": "signal_handler", "NoAction": "audit_logger"},
        },
        "signal_handler": {},   # terminal
        "audit_logger": {},     # terminal
    },
) as builder:

    @cloaca.node
    def decision(orderbook, pricing):
        if orderbook and orderbook.get("spread", 1.0) < 0.2:
            return ("Trade", {"price": orderbook["mid"]})
        return ("NoAction", {"reason": "spread too wide"})

    @cloaca.node
    def signal_handler(signal):
        return {"executed": True, "price": signal["price"]}

    @cloaca.node
    def audit_logger(reason):
        return {"logged": True, "reason": reason["reason"]}
```

## See also

- [Python CG Decorator Surface]({{< ref "/python/computation-graphs/explanation/python-cg-decorator-surface" >}}) — the decorators these nodes use.
- [Computation Graph (Rust reference)]({{< ref "/computation-graphs/reference/computation-graphs" >}}) — the `graph = { ... }` macro this dict mirrors.
- [Python · CG · Tutorial 09]({{< ref "/python/computation-graphs/tutorials/09-computation-graph" >}}) — a first concrete example.
