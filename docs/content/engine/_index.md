---
title: "Engine & Primitives"
description: "The core objects Cloacina is built from — described once, shared by both the embedded and service paths."
weight: 30
---

# Engine & Primitives

The concepts here are the heart of Cloacina, independent of how you run it. They
apply whether you **[embed the library]({{< ref "/embed" >}})** or
**[run the service]({{< ref "/service" >}})** — each object is described once,
with both its Rust and Python interfaces.

The core primitives:

- **Workflow** — a durable, database-backed DAG; the task is the unit of scheduling.
- **Task** — a unit of work in a workflow.
- **Context** — typed data passed between tasks, persisted and recovered.
- **Computation Graph** — an in-process reactive dataflow; the whole traversal is the unit.
- **Node** — a vertex in a computation graph.
- **Reactor** — binds accumulators to a graph and fires it when criteria are met.
- **Accumulator** — turns a source or stream into boundary events (passthrough, stream, polling, batch, state).
- **Boundary event** — the typed event an accumulator emits and a reactor reacts to.
- **Trigger** — fires a workflow (poll or cron).
- **Cron schedule** — time-based workflow scheduling.
- **Package (`.cloacina`)** — the distributable unit.
- **Runner** — the host that executes workflows against a database.

{{< toc-tree >}}
