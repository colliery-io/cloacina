---
title: "Topology Dict Schema"
description: "The Python-side topology dictionary format consumed by `ComputationGraphBuilder` — node spec, input declaration, terminal-node convention."
weight: 10
---

# Topology Dict Schema

<!-- TODO(DOC-G Phase 5): full content deferred. Sources to read: -->
<!--   - crates/cloacina-python/src/computation_graph.rs (topology parser, if separate from Rust) -->
<!--   - examples/tutorials/python/computation-graphs/0[9-11]-*/ (runnable examples showing the shape) -->
<!--   - The Rust parser at crates/cloacina-macros/src/computation_graph/parser.rs (the dict ought to map 1:1 to the macro shape) -->

The Python `ComputationGraphBuilder` takes a `graph=` keyword whose value is a topology dictionary. This page documents that dictionary's schema.

The general shape:

```python
with cloaca.ComputationGraphBuilder("graph_name", graph={
    "node_name": {"inputs": ["source1", "source2"], "next": "downstream_node"},
    ...
}) as builder:
    ...
```

Pending verification:

- Whether `"next"` accepts a list (multiple downstream nodes) or only a single string.
- Whether terminal nodes omit `"next"` or use `"next": []` / `"next": None`.
- Whether source names refer to accumulator names directly or go through a separate mapping.

Until verified, treat the Rust `graph = { ... }` macro shape (in [`#[computation_graph]`]({{< ref "/computation-graphs/reference/computation-graphs" >}})) as the authoritative model — the Python dict mirrors it.

## See also

- [Python · CG · Tutorial 09]({{< ref "/python/computation-graphs/tutorials/09-computation-graph" >}}) — first concrete example.
- [Rust · CG · Reference]({{< ref "/computation-graphs/reference/computation-graphs" >}}) — the `graph = { ... }` macro shape.
