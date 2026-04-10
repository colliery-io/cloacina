---
title: "Computation Graphs"
description: "Architecture and design of the reactive computation graph system"
weight: 20
---

# Computation Graph Architecture & Design

The computation graph system is Cloacina's reactive scheduling engine — a parallel track alongside the cron/trigger workflow scheduler, built for workloads where events drive execution rather than schedules.

The core pattern it solves: multiple independent event streams, each producing data at its own rate, need to be correlated into a single decision snapshot that fires a compiled computation function. Market data feeds, sensor streams, pricing models, risk accumulators — anything where the question is "what is the current state across all my inputs, and what should I do about it?"

These explanations cover the internals and design decisions. If you want to build something, start with the tutorials and how-to guides first.

## In This Section

**[Computation Graph Architecture]({{< ref "architecture" >}})** — The reactive model: how accumulators, reactors, and compiled graph functions fit together. Why computation graphs are not workflows. The reactor loop, the event flow from source to graph execution, and the recovery model.

**[Accumulator Design]({{< ref "accumulator-design" >}})** — The four accumulator types (Passthrough, Stream, Polling, Batch) and why each exists. The runtime task model, state management via `CheckpointHandle`, the `StreamBackend` trait, and accumulator health states.

**[Packaging & FFI]({{< ref "packaging" >}})** — How packaged computation graphs work: the `cloacina-computation-graph` thin types crate, the fidius FFI boundary and method indices, the load-once `LoadedGraphPlugin`, the dlclose problem and why we never unload libraries mid-flight, and the reconciler flow from package upload to running reactor.

**[Performance Characteristics]({{< ref "performance" >}})** — Latency and throughput baselines for the embedded pipeline. Why debug and release builds have similar throughput (channel hops, not compute). Kafka stream accumulator soak test numbers. How to run benchmarks with `cg-bench`.

{{< toc-tree >}}
