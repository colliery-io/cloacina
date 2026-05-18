---
title: "How-to Guides"
description: "Task-oriented recipes for computation graph development and operations"
weight: 20
---

# Computation Graph How-to Guides

Practical recipes for solving specific problems with the computation graph system.

## Development

- [Accumulator Types]({{< ref "accumulator-types" >}}) — Choose and configure the right accumulator
- [When-All Criteria]({{< ref "when-all-criteria" >}}) — Fire graphs only when all sources have data
- [Sequential Input Strategy]({{< ref "sequential-strategy" >}}) — Process every boundary in order rather than collapsing to the latest
- [Computation Graph in a Workflow Task]({{< ref "computation-graph-in-workflow" >}}) — Wrap a trigger-less graph as a workflow task with `invokes = computation_graph(...)`

## Reactor subscriptions

- [Reactor-triggered Workflows]({{< ref "reactor-triggered-workflows" >}}) — Dual-path topology overview (in-process CG fast path + DB-backed workflow path)
- [Filter Reactor Firings with CEL]({{< ref "filter-reactor-firings-with-cel" >}}) — Predicate-filter a workflow's reactor subscription (CLOACI-T-0602)

## Operations

- [Computation Graph Health]({{< ref "computation-graph-health" >}}) — Monitor graph execution and diagnose issues
