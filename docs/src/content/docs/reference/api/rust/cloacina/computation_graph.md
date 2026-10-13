---
title: "cloacina::computation_graph"
description: "API reference of the Rust module cloacina::computation_graph."
editUrl: false
sidebar:
  order: 39
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Computation Graph Runtime Types**

Types used by compiled computation graphs at runtime. These are the interface
between the `#[computation_graph]` macro-generated code and the reactor.

- [`InputCache`](../../../rust/cloacina-computation-graph/#struct.InputCache) — holds the last-seen boundary per source, used by the reactor
- [`GraphResult`](../../../rust/cloacina-computation-graph/#enum.GraphResult) — returned by the compiled graph function
- [`SourceName`](../../../rust/cloacina-computation-graph/#struct.SourceName) — identifies an accumulator source

## Re-exports

- [`Accumulator`](../../../rust/cloacina/computation_graph/accumulator/#trait.Accumulator)
- [`AccumulatorContext`](../../../rust/cloacina/computation_graph/accumulator/#struct.AccumulatorContext)
- [`AccumulatorError`](../../../rust/cloacina/computation_graph/accumulator/#enum.AccumulatorError)
- [`AccumulatorRuntimeConfig`](../../../rust/cloacina/computation_graph/accumulator/#struct.AccumulatorRuntimeConfig)
- [`BatchAccumulator`](../../../rust/cloacina/computation_graph/accumulator/#trait.BatchAccumulator)
- [`BatchAccumulatorConfig`](../../../rust/cloacina/computation_graph/accumulator/#struct.BatchAccumulatorConfig)
- [`BoundarySender`](../../../rust/cloacina/computation_graph/accumulator/#struct.BoundarySender)
- [`EventSource`](../../../rust/cloacina/computation_graph/accumulator/#trait.EventSource)
- [`PollingAccumulator`](../../../rust/cloacina/computation_graph/accumulator/#trait.PollingAccumulator)
- [`accumulator_runtime`](../../../rust/cloacina/computation_graph/accumulator/#fn.accumulator_runtime)
- [`accumulator_runtime_with_source`](../../../rust/cloacina/computation_graph/accumulator/#fn.accumulator_runtime_with_source)
- [`batch_accumulator_runtime`](../../../rust/cloacina/computation_graph/accumulator/#fn.batch_accumulator_runtime)
- [`flush_signal`](../../../rust/cloacina/computation_graph/accumulator/#fn.flush_signal)
- [`polling_accumulator_runtime`](../../../rust/cloacina/computation_graph/accumulator/#fn.polling_accumulator_runtime)
- [`shutdown_signal`](../../../rust/cloacina/computation_graph/accumulator/#fn.shutdown_signal)
- [`ComputationGraphRegistration`](../../../rust/cloacina-computation-graph/#struct.ComputationGraphRegistration)
- `TriggerlessGraph` (`triggerless::TriggerlessGraph`)
- `TriggerlessGraphFn` (`triggerless::TriggerlessGraphFn`)
- `TriggerlessGraphRegistration` (`triggerless::TriggerlessGraphRegistration`)
- [`GraphError`](../../../rust/cloacina-computation-graph/#enum.GraphError)
- [`GraphResult`](../../../rust/cloacina-computation-graph/#enum.GraphResult)
- [`InputCache`](../../../rust/cloacina-computation-graph/#struct.InputCache)
- [`SourceName`](../../../rust/cloacina-computation-graph/#struct.SourceName)
