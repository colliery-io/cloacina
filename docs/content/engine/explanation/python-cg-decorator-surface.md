---
title: "Python CG Decorator Surface"
description: "How cloaca's computation-graph decorators map onto the Rust macro family, and where they differ."
weight: 10
---

# Python CG Decorator Surface

`cloaca` exposes the computation-graph surface as decorators and a builder that
mirror the Rust macros. This page maps the two and explains where Python differs.

## The mapping

| Rust macro | Python equivalent |
|---|---|
| `#[reactor(name = …, accumulators = […], criteria = when_any/when_all(…))]` | `@cloaca.reactor(name=…, accumulators=[…], mode="when_any"\|"when_all")` (decorates a class) |
| `#[computation_graph(trigger = reactor("…"), graph = { … })]` | `cloaca.ComputationGraphBuilder("name", reactor=…, graph={…})` (a context manager) |
| node functions inside the `#[computation_graph]` module | `@cloaca.node` functions inside the builder's `with` block |
| `#[passthrough_accumulator]` | `@cloaca.passthrough_accumulator` |
| `#[stream_accumulator(type=…, topic=…, group=…)]` | `@cloaca.stream_accumulator(type=…, topic=…, group=…)` |
| `#[polling_accumulator(interval=…)]` | `@cloaca.polling_accumulator(interval=…)` |
| `#[batch_accumulator(flush_interval=…, max_buffer_size=…)]` | `@cloaca.batch_accumulator(flush_interval=…, max_buffer_size=…)` |

Reaction criteria is written as `mode="when_any"` / `mode="when_all"` in Python,
versus the `criteria = when_any(...)` form in Rust.

## Where Python differs from Rust

- **Accumulator types: four, not five.** Python exposes `passthrough`, `stream`,
  `polling`, and `batch`. The Rust-only `#[state_accumulator]` (a bounded
  DAL-persisted history buffer) has no Python decorator.
- **Topology is a dict, parsed at build time.** Rust declares the graph as a
  token-tree literal inside the macro (compile-time); Python passes a
  [topology dict]({{< ref "/reference/topology-dict-schema" >}})
  to `ComputationGraphBuilder`, validated when the `with` block exits.
- **Node values are owned, not borrowed.** Rust node functions take `Option<&T>`
  references into the cache; Python node functions receive owned values (the
  Python/Rust boundary copies them).
- **Routing returns a tuple.** A Python routing node returns
  `(variant_name, value)`; the variant name selects the route.

## One runtime, one package format

Whichever language authors a computation graph, the result registers into the
same engine and packages into the same `.cloacina` format. The server/runner
loads a Python-authored graph and a Rust-authored graph through the same path —
it doesn't care which language wrote it. See
[Package a Python Computation Graph]({{< ref "/engine/computation-graphs/how-to/package-a-python-computation-graph" >}}).

## See also

- [Topology Dict Schema]({{< ref "/reference/topology-dict-schema" >}}) — the `graph={...}` format.
- [Computation Graph (Rust reference)]({{< ref "/reference/computation-graphs" >}}) — the macro family this mirrors.
- [Choosing Accumulator Types]({{< ref "/engine/computation-graphs/how-to/accumulator-types" >}}) — what each accumulator does.
