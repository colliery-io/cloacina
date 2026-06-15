---
title: "Is Cloacina for you?"
description: "Where Cloacina fits, where it doesn't, how it compares, and which door to pick."
weight: 12
---

# Is Cloacina for you?

This page helps you decide whether Cloacina fits your problem — and if so, which
door to start at.

## Cloacina is a good fit when…

- **You have multi-step work that must run reliably.** Steps depend on each other,
  must retry on failure, and must not be silently dropped — backed by a database
  you already run.
- **You'd rather not operate a separate orchestration service** (embed it), **or
  you specifically want one** (run the service). Both are first-class.
- **You're in Rust or Python.** Both are first-class; the Python bindings (`cloaca`)
  are the same engine, not a reduced wrapper.
- **You're building a platform for multiple teams/tenants.** The server provides
  schema-per-tenant isolation, API-key auth, package upload, and a web UI.

## Cloacina is *not* the right tool when…

- **You want a no-code / click-to-build scheduler.** Workflows are authored in Rust
  or Python; the [web UI]({{< ref "/service" >}}) operates and observes, it doesn't
  draw pipelines.
- **You need multi-tenancy or horizontal scaling on SQLite.** Those require
  **PostgreSQL** (SQLite is single-process). See
  [Database Backends]({{< ref "/platform/explanation/database-backends" >}}).
- **You need the managed multi-tenant server on Windows/macOS.** The library and
  daemon are cross-platform; the hardened multi-tenant **server** targets **Linux**.
- **You require exactly-once execution.** Cloacina is **at-least-once** with
  recovery; tasks must be idempotent.
- **Your tasks are synchronous and you don't want async.** Tasks and graph nodes
  are `async`; offload blocking/CPU-bound work.

## How Cloacina compares

Coming from Airflow, Temporal, or Prefect, the defining difference is *deployment
shape*: those are standalone systems you operate alongside your app; Cloacina can
be **either** a library inside your app **or** a service you run. A genuine
trade-off, not a ranking.

| | **Cloacina** | **Airflow** | **Temporal** | **Prefect** |
|---|---|---|---|---|
| Deployment | Embedded library **or** a server | Standalone scheduler + workers + metadata DB | Standalone server cluster + workers | Control plane + agents/workers |
| Infra dependencies | Just a database | Scheduler, executor, metadata DB | Temporal service + datastore | API/cloud + worker pool |
| Authoring | Rust or Python, in your codebase | Python DAGs | Workflow-as-code (multi-language SDKs) | Python flows |
| Best when | You want orchestration in-process *or* a self-hosted service with no extra moving parts | You want a dedicated scheduler + authoring UI | You need long-lived, signal-driven durable executions at scale | You want flexible Python-native flows with a managed control plane |

## Choosing a door

| If you want to… | Door | Backed by |
|---|---|---|
| Run workflows inside one Rust/Python app | **[Embed the library]({{< ref "/embed" >}})** | SQLite or Postgres |
| Schedule packages locally, single-user | **Embed** (via `cloacinactl daemon`) | SQLite |
| Serve many tenants / a team, with an API + UI | **[Run the service]({{< ref "/service" >}})** | Postgres |
| Scale the service horizontally | **Run the service** (+ compiler + agent fleet) | Postgres |

The two doors share the same engine, packaging format, and primitives — moving a
`.cloacina` package between them is a repackaging step, not a rewrite. They have
deliberately different trust models (high-trust single-user vs. low-trust
multi-tenant); see [Security Model]({{< ref "/platform/explanation/security-model" >}}).

## Choosing a primitive

- **[Workflows]({{< ref "/engine/workflows/workflow" >}})** — durable DAGs; choose
  for ETL, background jobs, integrations that must survive restarts.
- **[Computation Graphs]({{< ref "/engine/computation-graphs/computation-graph" >}})** —
  in-process event-driven DAGs; choose for low-latency stream processing.

They compose — a workflow can be triggered by a computation graph, and a task can
invoke one inline. See **[Engine & Primitives]({{< ref "/engine" >}})**.
