---
title: "Computation Graphs"
description: "The in-process dataflow cluster: Computation Graph, Node, Reactor, Accumulator, and Boundary events."
sidebar:
  order: 20
---

The in-process dataflow cluster — fast, deterministic, event-driven graphs where
the whole traversal is the unit of execution. A **Computation Graph** is built
from **Nodes**; a **Reactor** fires the graph when its criteria are met;
**Accumulators** turn sources and streams into the **Boundary events** the reactor
reacts to.

Node and Boundary are documented alongside the objects they belong to — a Node
only exists inside a graph, a Boundary only between an Accumulator and a Reactor.

- [Computation Graph](computation-graph/)
- [Node](node/)
- [Reactor](reactor/)
- [Accumulator](accumulator/)
- [Boundary event](boundary/)
- [How-to Guides](../../../how-to/computation-graphs/)
  - [Package a Python Computation Graph](../../../how-to/computation-graphs/package-a-python-computation-graph/)
  - [Choosing and Using Accumulator Types](../../../how-to/computation-graphs/accumulator-types/)
  - [Filter Reactor Subscriptions (Python)](../../../how-to/computation-graphs/filter-reactor-subscriptions/)
  - [Monitoring Computation Graph Health](../../../how-to/computation-graphs/computation-graph-health/)
  - [Manually Driving Graph Surfaces](../../../how-to/computation-graphs/drive-graph-surfaces-manually/)
  - [Computation Graph in a Workflow Task](../../../how-to/computation-graphs/computation-graph-in-workflow/)
  - [Triggering Workflows from Reactor Firings](../../../how-to/computation-graphs/reactor-triggered-workflows/)
  - [Using when_all Reaction Criteria](../../../how-to/computation-graphs/when-all-criteria/)
  - [Filter reactor firings with CEL](../../../how-to/computation-graphs/filter-reactor-firings-with-cel/)
  - [Using Sequential Input Strategy](../../../how-to/computation-graphs/sequential-strategy/)
