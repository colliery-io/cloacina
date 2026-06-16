---
title: "Cloacina"
description: "One workflow orchestration engine, two ways to run it — embed it as a library or operate it as a service. For Rust and Python."
---

# Cloacina

**One workflow orchestration engine — two ways to run it.** Cloacina runs durable,
database-backed task pipelines (with retries and recovery) and in-process,
event-driven computation graphs, for **Rust** and **Python**. Its only hard
dependency is a database — Postgres or SQLite. No broker, no queue, no coordinator.

Pick the way that matches how you want it to live in your system — both are
first-class:

{{< columns >}}
## Embed the library

Add `cloacina` (Rust) or `cloaca` (Python) as a dependency and run orchestration
**inside your application**, against a database you already operate. No separate
service to stand up. A permanent, production-legitimate way to run Cloacina.

{{< button relref="/embed" >}}Embed the library →{{< /button >}}

<--->

## Run the service

Operate `cloacina-server` as a **multi-tenant control plane** — HTTP/WebSocket API,
web UI, schema-per-tenant isolation, and horizontal scale. Ship workflows to it as
`.cloacina` packages.

{{< button relref="/service" >}}Run the service →{{< /button >}}

{{< /columns >}}

## New here?

- **[What is Cloacina?]({{< ref "/start/what-is-cloacina" >}})** — the engine, the two ways, and the embedded-first principle.
- **[Is Cloacina for you?]({{< ref "/start/is-cloacina-for-you" >}})** — where it fits, where it doesn't, and which door.
- **[Engine & Primitives]({{< ref "/engine" >}})** — the core objects (workflows, computation graphs, and more), described once for both languages.
- **[Reference]({{< ref "/reference" >}})** — APIs, CLI, HTTP/WebSocket, configuration.

## The two primitives

- **[Workflows]({{< ref "/engine/workflows/workflow" >}})** — durable, database-backed DAGs; the task is the unit of scheduling. For work that must survive restarts and recover from failure.
- **[Computation Graphs]({{< ref "/engine/computation-graphs/computation-graph" >}})** — in-process, event-driven DAGs; the whole traversal is the unit. For low-latency processing that reacts to a stream.

They compose, and both run through either door.
