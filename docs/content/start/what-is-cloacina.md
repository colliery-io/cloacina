---
title: "What is Cloacina?"
description: "One workflow orchestration engine, run two co-equal ways — embedded as a library or operated as a service."
weight: 11
---

# What is Cloacina?

Cloacina is a **workflow orchestration engine** for Rust and Python. It runs
durable, database-backed task pipelines — with retries, recovery, and at-least-once
execution — and in-process, event-driven computation graphs. Its only hard
dependency is a database (PostgreSQL or SQLite): no broker, no queue, no
coordinator.

## One engine, two ways to run it

The same engine runs **two co-equal ways**. Neither is the "real" way; you pick by
how you want it to live in your system:

- **[Embed the library]({{< ref "/embed" >}})** — add `cloacina` (Rust) or `cloaca`
  (Python) as a dependency and run orchestration *inside your application*, against
  a database you already operate.
- **[Run the service]({{< ref "/service" >}})** — operate `cloacina-server` as a
  multi-tenant control plane with an HTTP/WebSocket API, a web UI, and horizontal
  scale; ship workflows to it as `.cloacina` packages.

You don't graduate from one to the other. Embedding is a **permanent,
production-legitimate** way to run Cloacina — chosen because you want orchestration
as a component of your own system, not because you haven't "grown" into the server
yet. Equally, plenty of teams want the service from day one.

## The embedded-first principle

Cloacina is **embedded-first by design**: the engine is a genuine standalone
library, and the server is built *on top of it* — not the other way around. That
ordering is a deliberate commitment. It's why embedding is truly first-class
rather than a stripped-down mode: the library is the product, and the service is
one way to deploy it.

## Two execution primitives

Whichever way you run it, Cloacina exposes two primitives (described once, for both
languages, in **[Engine & Primitives]({{< ref "/engine" >}})**):

- **[Workflows]({{< ref "/engine/workflows/workflow" >}})** — durable, database-backed
  DAGs where the task is the unit of scheduling. For work that must survive restarts
  and recover from failure.
- **[Computation Graphs]({{< ref "/engine/computation-graphs/computation-graph" >}})** —
  in-process, event-driven DAGs where the whole traversal is the unit. For
  low-latency processing that reacts to a stream.

## Next

- **[Is Cloacina for you?]({{< ref "/start/is-cloacina-for-you" >}})** — where it fits, where it doesn't, and which door.
- **[Embed the library]({{< ref "/embed" >}})** · **[Run the service]({{< ref "/service" >}})**
