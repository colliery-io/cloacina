---
title: "Features Overview"
description: "A catalog of what Cloacina can do, with links into the detailed docs."
weight: 3
---

# Features Overview

A single catalog of Cloacina's capabilities. Each item links to the detailed
tutorial, how-to, reference, or explanation. For *whether* Cloacina fits your
problem, see [When to Use Cloacina]({{< ref "/quick-start/when-to-use" >}}).

## Execution engine

- **Durable, database-backed execution** — task and execution state persist in
  Postgres or SQLite; nothing lives only in memory.
- **At-least-once with recovery** — work is claimed atomically; failed or stalled
  work is recovered. Tasks and graph nodes must be idempotent.
- **Automatic retries** — configurable retry policies per task, including
  [conditional retries]({{< ref "/workflows/how-to-guides/conditional-retries" >}}).
- **Content-versioned workflows** — a workflow's version is derived from its task
  code and structure, so changes are explicit and safe. See
  [Workflow Versioning]({{< ref "/workflows/explanation/workflow-versioning" >}}).
- **Async-first** — built on Tokio; blocking work is offloaded explicitly.
- **Conditional task execution** — per-task trigger rules (`all` / `any` / `none`
  over upstream task states) enable branching and conditional fan-in. See
  [Trigger Rules]({{< ref "/workflows/explanation/trigger-rules" >}}).
- **External config & secrets** — reference connections, secrets, and config by
  name and resolve them from `CLOACINA_VAR_*` environment variables at runtime
  (`cloaca.var()` / `var_or()`). See
  [Variable Registry]({{< ref "/workflows/how-to-guides/variable-registry" >}}).
- **Two database backends, chosen at runtime** — Postgres or SQLite via the
  connection URL, no recompile. See
  [Database Backends]({{< ref "/service/explanation/database-backends" >}}).

## Two execution primitives

- **[Workflows]({{< ref "/workflows" >}})** — durable DAGs of tasks with
  dependencies, retries, and recovery; the task is the unit of scheduling.
- **[Computation Graphs]({{< ref "/computation-graphs" >}})** — in-process,
  deterministic, event-driven DAGs; the whole traversal is the unit.
- **They compose** — workflows can
  [subscribe to a reactor]({{< ref "/workflows/how-to-guides/subscribe-workflow-to-reactor" >}})
  and a task can
  [invoke a computation graph]({{< ref "/workflows/how-to-guides/invoke-computation-graph-from-workflow" >}}).

## Computation-graph capabilities

These build on reactors and accumulators — see [Concepts]({{< ref "/quick-start/concepts" >}})
first if those terms are new.

- **Accumulators** — adapters that turn an external stream or source into
  *boundary events* (the discrete events a graph reacts to), in several flavors
  (passthrough, stream, polling, batch, state). See
  [Accumulator Types]({{< ref "/computation-graphs/how-to-guides/accumulator-types" >}}).
- **Reactors** — fire a graph when their criteria are met (`when_any` /
  [`when_all`]({{< ref "/computation-graphs/how-to-guides/when-all-criteria" >}}))
  with a [`latest` or `sequential`]({{< ref "/computation-graphs/how-to-guides/sequential-strategy" >}})
  input strategy.
- **Routing** — enum-variant routing propagates results conditionally between
  nodes.
- **Kafka stream accumulators** — consume topics via the `kafka` feature; see
  [Kafka Stream]({{< ref "/computation-graphs/tutorials/service/09-kafka-stream" >}}).
- **CEL firing filters** — filter reactor firings with CEL (Common Expression
  Language) expressions; see
  [Filter Reactor Firings with CEL]({{< ref "/computation-graphs/how-to-guides/filter-reactor-firings-with-cel" >}}).

## Scheduling & triggers

- **Cron schedules** — run workflows on a schedule; see
  [Cron Scheduling]({{< ref "/workflows/explanation/cron-scheduling" >}}).
- **Event triggers** — start work from external events; see
  [Event Triggers]({{< ref "/python/workflows/tutorials/07-event-triggers" >}}).

## Deployment modes

- **Embedded library** — `cloacina` (Rust) / `cloaca` (Python) in your process.
- **Daemon** — `cloacinactl daemon` watches a directory and runs packages locally.
  See [Running the Daemon]({{< ref "/embed/how-to/running-the-daemon" >}}).
- **Server** — `cloacina-server`, an HTTP + WebSocket control plane. See
  [Deploy a Server]({{< ref "/service/tutorials/01-deploy-a-server" >}}).
- **Compiler service** — `cloacina-compiler` builds uploaded packages in a
  sandbox. See [Running the Compiler]({{< ref "/service/how-to/running-the-compiler" >}}).
- **Execution-agent fleet** — `cloacina-agent` workers run tasks off the server
  for horizontal scale. See
  [Deploy an Execution-Agent Fleet]({{< ref "/service/how-to/deploy-an-execution-agent-fleet" >}})
  and [Execution-Agent Fleet]({{< ref "/service/explanation/execution-agent-fleet" >}}).

## Packaging

- **`.cloacina` packages** — a `package.toml` plus source (Python module tree, or
  Rust compiled on the server at load). Scaffold, validate, pack, and upload with
  `cloacinactl package`. See
  [Creating Your First Package]({{< ref "/service/how-to/creating-your-first-package" >}})
  and the [Package Format]({{< ref "/engine/explanation/package-format" >}}).
- **Reconciler** — the server loads and registers uploaded packages
  automatically. See
  [Reconciler Pipeline]({{< ref "/service/explanation/reconciler-pipeline" >}}).

## Multi-tenancy & auth

- **Schema-per-tenant isolation** (Postgres) — see
  [Multi-Tenancy]({{< ref "/service/explanation/multi-tenancy" >}}) and
  [Configure a Multi-Tenant Deployment]({{< ref "/service/how-to/configure-multi-tenant-deployment" >}}).
- **API keys with roles** — admin / write / read, managed via `cloacinactl key`
  and the API.

## Clients & UI

- **Client SDKs** — Rust, Python, and TypeScript clients for calling a running
  server over HTTP/WebSocket. See [Client SDKs]({{< ref "/sdks" >}}).
- **Web UI** — operate and observe a server: workflows, executions (with a live
  event stream), triggers, computation-graph health, package upload, and API-key
  management. See [The Web UI]({{< ref "/service/tutorials/02-the-web-ui" >}}).

## Operations & security

- **Observability** — Prometheus `/metrics`, structured logs, and optional
  OpenTelemetry export. See
  [Observability]({{< ref "/service/explanation/observability" >}}) and the
  [Metrics Catalog]({{< ref "/reference/metrics-catalog" >}}).
- **Package signing** — optional signature verification, enforced when the server
  runs with signatures required; signing private keys are encrypted at rest
  (AES-256-GCM). See
  [Package Signing]({{< ref "/service/how-to/security/package-signing" >}})
  and [Require Signed Packages]({{< ref "/service/how-to/require-signed-packages" >}}).
- **Build sandboxing** — the compiler builds untrusted packages in an isolated
  environment (Linux). See [Security Model]({{< ref "/service/explanation/security-model" >}}).
- **Horizontal scaling** — stateless schedulers coordinate through the database.
  See [Horizontal Scaling]({{< ref "/service/explanation/horizontal-scaling" >}}).

## Tooling

- **`cloacinactl`** — the operator + developer CLI (daemon, server/compiler
  lifecycle, package, workflow, execution, tenant, key, trigger, graph). See the
  [CLI Reference]({{< ref "/reference/cli" >}}).
- **HTTP + WebSocket API** — see the
  [HTTP API Reference]({{< ref "/reference/http-api" >}}) and
  [WebSocket Protocol]({{< ref "/reference/websocket-protocol" >}}).
