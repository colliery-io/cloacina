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
- **You want to start small and grow.** Your task and node *logic* carries across
  all three modes — embedded, daemon, and server. A `.cloacina` package, once
  built, runs unchanged on any of them; moving from embedded code to a deployable
  package is a *repackaging* step (see the transition note below), not a rewrite
  of your business logic.
- **You're building a platform for multiple teams/tenants.** The server provides
  schema-per-tenant isolation, API-key auth, package upload, and a web UI.

## Cloacina is *not* the right tool when…

- **You want a no-code / click-to-build scheduler.** Cloacina workflows are
  authored in Rust or Python code. The [web UI]({{< ref "/service/tutorials/02-the-web-ui" >}})
  is for *operating and observing* — uploading packages, running workflows,
  watching executions — not for drawing pipelines.
- **You need multi-tenancy or horizontal scaling on SQLite.** SQLite is
  single-process: ideal for embedding and local/dev, but multi-tenant isolation
  and multi-replica coordination require **Postgres**. See
  [Database Backends]({{< ref "/service/explanation/database-backends" >}}).
- **You need the managed multi-tenant server on Windows/macOS.** The embedded
  library and the daemon are cross-platform; the hardened multi-tenant **server**
  (with build sandboxing) targets **Linux**. See
  [Security Model]({{< ref "/service/explanation/security-model" >}}).
- **You require exactly-once execution.** Cloacina guarantees **at-least-once**
  with recovery; tasks and graph nodes must be idempotent under redelivery.
- **Your tasks are synchronous and you don't want async.** Tasks and graph nodes
  are `async`; CPU-bound or blocking work must be offloaded
  (`tokio::task::spawn_blocking` in Rust, or a blocking node).

If you simply need a standalone DAG scheduler-as-a-service with a hosted UI for
non-developers, a dedicated orchestrator may serve you better. Cloacina's
trade-off is the opposite: it lives in your app and your database.

## How Cloacina compares

If you're coming from Airflow, Temporal, or Prefect, the defining difference is
*deployment shape*: those are standalone systems you operate alongside your
application; Cloacina is a library that runs **inside** it. This is a genuine
trade-off, not a strict ranking — pick the side that matches how you want to
run things.

| | **Cloacina** | **Airflow** | **Temporal** | **Prefect** |
|---|---|---|---|---|
| Deployment | Embedded in your app (or an optional server) | Standalone scheduler + workers + metadata DB | Standalone server cluster + workers | Control plane + agents/workers |
| Infra dependencies | Just a database (SQLite or Postgres) | Scheduler, executor, metadata DB | Temporal service + datastore | API/cloud + worker pool |
| Authoring | Rust or Python, in your codebase | Python DAGs | Workflow-as-code (multi-language SDKs) | Python flows |
| Execution model | Durable DAGs (at-least-once) + in-process computation graphs | Scheduled DAGs | Durable execution / long-running workflows | Dynamic Python flows |
| Best when | You want orchestration *in-process* with no extra service to operate | You want a dedicated scheduler with a rich authoring UI | You need long-lived, signal-driven durable executions at scale | You want flexible Python-native flows with a managed control plane |

**Choose Cloacina** when you'd rather not run a separate orchestration service —
when "a library plus the database we already have" beats "another system to
deploy, secure, and monitor." **Choose one of the others** when a dedicated
control plane, a hosted authoring UI for non-developers, or their specific
execution semantics are what you're after. Cloacina deliberately does not try to
be a hosted, click-to-build platform.

## Choosing a mode

| If you want to… | Use | Backed by |
|------------------|-----|-----------|
| Run workflows inside one Rust/Python app | **Embedded library** | SQLite or Postgres |
| Schedule packages locally, single-user | **Daemon** (`cloacinactl daemon`) | SQLite (default) |
| Serve many tenants / a team, with an API + UI | **Server** (`cloacina-server`) | Postgres |
| Scale the server horizontally | **Server + compiler + agent fleet** | Postgres |

The embedded library and the server are **not exclusive** — you can author and
test embedded, then deploy to a server. The two deployment
postures have deliberately different trust models (high-trust single-user vs.
low-trust multi-tenant); see [Security Model]({{< ref "/service/explanation/security-model" >}}).

### Moving from embedded to the server

This is a packaging step, not a rewrite. Conceptually:

- **What stays the same:** your task logic (and any computation-graph node
  logic) — the function bodies and the dependencies between them.
- **What changes:** the code becomes a `.cloacina` *package* with a small
  `package.toml` and a conventional source layout. In **Python**, a packaged
  module declares tasks with **bare `@cloaca.task` decorators** rather than the
  `WorkflowBuilder` context manager — a builder inside a package fails to load,
  because the package loader already supplies the workflow context.

The package is portable: once built, the same artifact runs under the daemon or
the server unchanged. For the actual procedure, see
[Creating Your First Package]({{< ref "/service/how-to/creating-your-first-package" >}})
and, for the Python specifics,
[Packaging Python Workflows]({{< ref "/python/workflows/how-to-guides/packaging-python-workflows" >}}).

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
