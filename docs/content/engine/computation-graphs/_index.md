---
title: "Computation Graphs"
description: "The in-process dataflow cluster: Computation Graph, Node, Reactor, Accumulator, and Boundary events."
weight: 20
---

# Computation Graphs

The in-process dataflow cluster — fast, deterministic, event-driven graphs where
the whole traversal is the unit of execution. A **Computation Graph** is built
from **Nodes**; a **Reactor** fires the graph when its criteria are met;
**Accumulators** turn sources and streams into the **Boundary events** the reactor
reacts to.

Node and Boundary are documented alongside the objects they belong to — a Node
only exists inside a graph, a Boundary only between an Accumulator and a Reactor.

{{< toc-tree >}}
