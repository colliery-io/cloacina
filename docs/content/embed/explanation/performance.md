---
title: "Workflow Performance and Design Trade-offs"
description: "Why task granularity, dependency structure, and context size shape the performance of a Cloacina workflow"
weight: 40

---

# Workflow Performance and Design Trade-offs

The way you decompose work into tasks — how coarse each task is, how tasks depend
on each other, and how much data you thread through the context — has a larger
effect on a workflow's throughput than most tuning knobs. This page explains
*why*, so the concrete tuning steps in
[Performance Optimization]({{< ref "/embed/how-to/performance-optimization/" >}})
make sense rather than reading as folklore.

## Task granularity

Every task in Cloacina is scheduled, claimed, executed, and has its result
context persisted to the database. That per-task machinery is fixed overhead
that does not shrink with the amount of work the task does. A task that processes
a single item pays the same scheduling and persistence cost as a task that
processes a thousand.

This produces a straightforward trade-off:

- **Too fine-grained** — thousands of tiny tasks means the framework spends most
  of its time scheduling and persisting rather than doing your work. Throughput
  is dominated by overhead.
- **Too coarse-grained** — one enormous task gives up parallelism and
  restartability. A failure late in a giant task re-runs everything; the
  scheduler has no independent units to spread across workers.

The goal is tasks large enough to amortize the fixed overhead but small enough to
parallelize and to make restarts cheap. Batching many small items into one task
(processing them in a loop) is usually the right move when the per-item work is
small relative to the scheduling cost.

## Dependency structure and parallelism

Cloacina executes tasks as soon as their declared dependencies are satisfied.
Tasks with no dependency relationship between them are eligible to run
concurrently (bounded by `max_concurrent_tasks`). This means the *shape* of your
dependency graph directly determines how much parallelism is available.

Declaring a dependency you don't actually need serializes two tasks that could
have run side by side. The practical rule that follows: **declare only the
dependencies a task genuinely requires.** Independent data-fetch steps should be
siblings with no edges between them, converging only at the aggregation step that
truly needs all of their outputs. A workflow authored this way exposes its
natural parallelism to the scheduler; one over-constrained with incidental
dependencies runs as a slow chain.

## Context size

The context is the data channel between tasks, and it is persisted at task
boundaries so that execution can be recovered after a failure. That durability
has a cost proportional to context size: a large context is serialized and
written more than once over the life of a workflow.

The consequence is that the context is the wrong place for bulk data. Threading a
large dataset through the context makes every task boundary pay to serialize and
store it, even for tasks that never look at it. The better pattern is to keep the
context small — pass *references* (a file path, an object-store key, a row range)
rather than the payload itself, and store per-step summaries rather than full
intermediate results. Bulk data lives in a store built for it; the context
carries the pointers.

## Where this leaves the tuning knobs

Because the dominant costs are structural — scheduling overhead per task,
serialization per context write — runtime tuning (connection-pool sizing,
concurrency limits) matters only once the workflow's *design* is already exposing
its parallelism and keeping its context lean. Tune the design first, then the
runtime. The concrete runtime parameters are in
[Performance Optimization]({{< ref "/embed/how-to/performance-optimization/" >}}).

## Further reading

- [Performance Optimization]({{< ref "/embed/how-to/performance-optimization/" >}}) — the concrete Cloacina tunables (pool params, `DefaultRunnerConfig` sizing).
- [Computation Graph Performance]({{< ref "/engine/explanation/performance" >}}) — measured latency/throughput numbers for the engine pipeline.
- [Configuration Reference]({{< ref "/reference/python-api/configuration/" >}}) — every configuration field.
