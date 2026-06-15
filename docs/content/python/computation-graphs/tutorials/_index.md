---
title: "Tutorials"
description: "Learning-oriented walkthroughs for the Python computation graph surface."
weight: 10
---

# Python Computation Graph Tutorials

These tutorials mirror the [Rust library tutorials 07–10]({{< ref "/embed/tutorials" >}}) on the Python side. They build on the Python workflow tutorials (01–08) — start there if you are new to Cloacina.

## Sequence

1. [09 — Your First Computation Graph in Python]({{< ref "09-computation-graph" >}}) — A `ComputationGraphBuilder`, one reactor, one passthrough accumulator.
2. [10 — Accumulators]({{< ref "10-accumulators" >}}) — Stream / state / batch / passthrough accumulator types.
3. [11 — Routing]({{< ref "11-routing" >}}) — Multi-source reactors, `when_all` vs `when_any`, terminal node routing.

Each tutorial is a working program; run with `angreal demos tutorials python NN` (where `NN` is the tutorial number).

## See also

- [Python · Workflows · Tutorials]({{< ref "/python/workflows/tutorials" >}}) — tutorials 00–08 (workflow-side).
- [Rust · Computation Graphs · Tutorials]({{< ref "/embed/tutorials" >}}) — the Rust counterparts.
