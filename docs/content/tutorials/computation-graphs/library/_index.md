---
title: "Library (Embedded)"
description: "Define and run computation graphs directly in your Rust application"
weight: 10
---

# Computation Graph Tutorials — Library Mode

These tutorials teach you to define computation graphs, create accumulators, wire reactors, and push events through the pipeline — all directly in your Rust application with explicit channel plumbing.

- [Tutorial 07 — Your First Computation Graph]({{< ref "/tutorials/computation-graphs/library/07-computation-graph/" >}}): declare a topology with `#[computation_graph]`, build an `InputCache`, and call the compiled function directly
- [Tutorial 08 — Accumulators]({{< ref "/tutorials/computation-graphs/library/08-accumulators/" >}}): implement the `Accumulator` trait, wire channels, spawn a reactor, and push live events through the graph
- [Tutorial 09 — Full Reactive Pipeline]({{< ref "/tutorials/computation-graphs/library/09-full-pipeline/" >}}): connect multiple accumulators to one reactor and handle optional multi-source inputs
- [Tutorial 10 — Routing]({{< ref "/tutorials/computation-graphs/library/10-routing/" >}}): add conditional branching with enum dispatch using the `=>` topology syntax
