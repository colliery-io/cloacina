---
title: "Explanation"
description: "Architecture and design notes specific to the Python computation graph surface."
weight: 40
---

# Python Computation Graph Explanation

Conceptual material — how the Python CG surface maps onto the underlying Rust runtime.

## In this section

- [Python CG Decorator Surface]({{< ref "python-cg-decorator-surface" >}}) — How `@cloaca.reactor` / `ComputationGraphBuilder` / `@cloaca.accumulator` map to the Rust `#[reactor]` / `#[computation_graph]` / `#[*_accumulator]` macro family (CLOACI-I-0101 / CLOACI-I-0102).

## See also

- [Rust · CG · Explanation]({{< ref "/computation-graphs/explanation" >}}) — every doc here applies to Python as well.
- [Python · Workflows · Explanation · Python Runtime Architecture]({{< ref "/python/workflows/explanation/python-runtime-architecture" >}}) — the PyO3 / GIL / FFI layer all Python surfaces share.
