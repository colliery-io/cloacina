---
title: "Python · Computation Graphs"
description: "Python surface for the Cloacina computation graph runtime — accumulator declarations, reactor triggers, and graph composition from Python."
weight: 30
---

# Python · Computation Graphs

The Python surface for authoring and running [computation graphs]({{< ref "/computation-graphs" >}}) — Cloacina's deterministic DAG primitive. The layout mirrors the [Rust computation-graphs section]({{< ref "/computation-graphs" >}}) at the Diataxis quadrant level.

A computation graph in Python is built with the `ComputationGraphBuilder` context manager. Accumulators register through `@cloaca.accumulator`, reactors through `@cloaca.reactor`, and the graph runs either standalone (reactor-triggered, server-mode) or embedded in a workflow task (`@cloaca.task(invokes=...)`).

## Diataxis quadrants

- **[Tutorials]({{< ref "tutorials" >}})** — Build a CG from scratch in Python (09–11, mirroring Rust library tutorials 07–10).
- **[How-to guides]({{< ref "how-to-guides" >}})** — Recipes for packaging Python CGs and wiring up reactor subscriptions.
- **[Reference]({{< ref "reference" >}})** — Python topology-dict schema and other Python-specific reference material.
- **[Explanation]({{< ref "explanation" >}})** — How the Python CG decorators map onto the Rust runtime.

## See also

- [Python · Workflows]({{< ref "/python/workflows" >}}) — the workflow-side Python surface.
- [Rust · Computation Graphs]({{< ref "/computation-graphs" >}}) — the underlying Rust surface; concepts and macros translate directly.
