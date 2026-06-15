---
title: "When to Use Cloacina"
description: "Where Cloacina fits, where it doesn't, and how to decide between its modes and primitives."
weight: 2
---

# When to Use Cloacina (and When Not)

Cloacina is an **embedded-first workflow orchestration engine** for Rust and
Python. This page helps you decide whether it fits your problem before you invest
in learning it.

## Cloacina is a good fit when…

- **You have multi-step work that must run reliably.** Steps depend on each other,
  must retry on failure, and must not be silently dropped — and you want that
  durability backed by a database you already run.
- **You'd rather embed orchestration than operate a separate service.** Cloacina
  runs *inside* your Rust or Python application as a library. The only external
  dependency is a database (Postgres or SQLite) — there is no broker, queue, or
  coordinator to stand up.
- **You're in Rust or Python.** Both are first-class: the Python bindings
  (`cloaca`) expose the same engine and API as the Rust crate (`cloacina`),
  not a reduced wrapper.
- **You want to start small and grow.** The same packaged workflow runs embedded,
  under the local daemon, or on the multi-tenant server — without a rewrite.
- **You're building a platform for multiple teams/tenants.** The server provides
  schema-per-tenant isolation, API-key auth, package upload, and a web UI.

## Cloacina is *not* the right tool when…

- **You want a no-code / click-to-build scheduler.** Cloacina workflows are
  authored in Rust or Python code. The [web UI]({{< ref "/platform/tutorials/02-the-web-ui" >}})
  is for *operating and observing* — uploading packages, running workflows,
  watching executions — not for drawing pipelines.
- **You need multi-tenancy or horizontal scaling on SQLite.** SQLite is
  single-process: ideal for embedding and local/dev, but multi-tenant isolation
  and multi-replica coordination require **Postgres**. See
  [Database Backends]({{< ref "/platform/explanation/database-backends" >}}).
- **You need the managed multi-tenant server on Windows/macOS.** The embedded
  library and the daemon are cross-platform; the hardened multi-tenant **server**
  (with build sandboxing) targets **Linux**. See
  [Security Model]({{< ref "/platform/explanation/security-model" >}}).
- **You require exactly-once execution.** Cloacina guarantees **at-least-once**
  with recovery; tasks and graph nodes must be idempotent under redelivery.
- **Your tasks are synchronous and you don't want async.** Tasks and graph nodes
  are `async`; CPU-bound or blocking work must be offloaded
  (`tokio::task::spawn_blocking` in Rust, or a blocking node).

If you simply need a standalone DAG scheduler-as-a-service with a hosted UI for
non-developers, a dedicated orchestrator may serve you better. Cloacina's
trade-off is the opposite: it lives in your app and your database.

## Choosing a mode

| If you want to… | Use | Backed by |
|------------------|-----|-----------|
| Run workflows inside one Rust/Python app | **Embedded library** | SQLite or Postgres |
| Schedule packages locally, single-user | **Daemon** (`cloacinactl daemon`) | SQLite (default) |
| Serve many tenants / a team, with an API + UI | **Server** (`cloacina-server`) | Postgres |
| Scale the server horizontally | **Server + compiler + agent fleet** | Postgres |

The embedded library and the server are **not exclusive** — you can author and
test embedded, then deploy the same package to a server. The two deployment
postures have deliberately different trust models (high-trust single-user vs.
low-trust multi-tenant); see [Security Model]({{< ref "/platform/explanation/security-model" >}}).

## Choosing a primitive

Cloacina exposes two execution primitives:

- **[Workflows]({{< ref "/workflows" >}})** — durable, database-backed DAGs where
  the **task** is the unit of scheduling. Choose this when work must survive
  process restarts and recover from failure (ETL, background jobs, integrations).
- **[Computation Graphs]({{< ref "/computation-graphs" >}})** — in-process,
  deterministic, event-driven DAGs where the **whole traversal** is the unit, with
  in-memory channels between nodes. Choose this for low-latency, event-driven
  processing reacting to a stream.

They compose — a workflow can be triggered by a computation graph, and a workflow
task can run one inline. See the
[Features overview]({{< ref "/quick-start/features" >}}) for the full picture, or
[Concepts]({{< ref "/quick-start/concepts" >}}) for the vocabulary.

## Next

- [Features overview]({{< ref "/quick-start/features" >}}) — everything Cloacina can do.
- [Concepts]({{< ref "/quick-start/concepts" >}}) — the core primitives.
- [Quick Start]({{< ref "/quick-start" >}}) — pick a starting tutorial.
