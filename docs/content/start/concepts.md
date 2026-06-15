---
title: "Concepts"
description: "The core Cloacina primitives and how they relate."
weight: 13
aliases:
  - "/quick-start/concepts/"

---

# Concepts

Cloacina is built from a small set of primitives. This page introduces them and
how they fit together; the [Glossary]({{< ref "/reference/glossary" >}}) has the full,
precise definition of every term.

## The primitives

- **Task** — the unit of work in a workflow: an `async` function that reads and
  writes a shared **context** (defined next). The task is also the unit of
  **scheduling** — the engine claims, runs, and records one task at a time.
- **Context** — the typed key/value state passed along a workflow's edges; how
  data flows from one task to the next.
- **Workflow** — a DAG of tasks with dependencies. Durable and database-backed:
  it survives restarts, retries failures, and recovers. See
  [Workflows]({{< ref "/engine/workflows" >}}).
- **Computation graph** — a DAG whose **entire traversal** is the unit of
  scheduling. Once triggered, all nodes run together with in-memory channels
  between them — deterministic and low-latency. See
  [Computation Graphs]({{< ref "/engine/computation-graphs" >}}).
- **Trigger** — an event source (a cron schedule, an external event) that starts
  a workflow or graph.
- **Accumulator** — an adapter that consumes an external stream or source (Kafka,
  a polled query, pushed events) and emits **boundary events** — the discrete
  events a computation graph reacts to.
- **Reactor** — a specialized trigger that watches one or more accumulators and
  fires a computation graph when its criteria are met.

These names are used consistently across the code, CLI, HTTP API, and docs, so
what you read here matches what you see in every interface.

## How they relate

```
 Trigger ───────────────▶ Workflow   (cron / event starts a durable DAG of tasks)

 Accumulator ──events──▶ Reactor ───fires──▶ Computation graph
                                              (one deterministic traversal)
```

- A **workflow** is driven by **triggers** (e.g. a cron schedule) and runs **tasks**.
- A **computation graph** is driven by a **reactor**, which consumes **accumulator**
  boundary events.
- The two compose: a workflow can **subscribe** to a reactor's firings, and a
  workflow **task** can **invoke** an embedded computation graph.

## Where to go next

- [When to Use Cloacina]({{< ref "/start/is-cloacina-for-you" >}}) — pick the right primitive and mode.
- [Features Overview]({{< ref "/start/features" >}}) — the full capability catalog.
- [Workflows]({{< ref "/engine/workflows" >}}) / [Computation Graphs]({{< ref "/engine/computation-graphs" >}}) — the deep tracks.
- [Glossary]({{< ref "/reference/glossary" >}}) — every term, precisely defined.
