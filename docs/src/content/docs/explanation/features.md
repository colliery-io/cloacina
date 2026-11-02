---
title: "Features Overview"
description: "A catalog of what Cloacina can do, with links into the detailed docs."
sidebar:
  order: 14
---

A single catalog of Cloacina's capabilities. Each item links to the detailed
tutorial, how-to, reference, or explanation. For *whether* Cloacina fits your
problem, see [When to Use Cloacina](../is-cloacina-for-you/).

## Execution engine

- **Durable, database-backed execution** — task and execution state persist in
  Postgres or SQLite; nothing lives only in memory.
- **At-least-once with recovery** — work is claimed atomically; failed or stalled
  work is recovered. Tasks and graph nodes must be idempotent.
- **Automatic retries** — configurable retry policies per task, including
  [conditional retries](../../how-to/embed/conditional-retries/).
- **Content-versioned workflows** — a workflow's version is derived from its task
  code and structure, so changes are explicit and safe. See
  [Workflow Versioning](../engine/workflow-versioning/).
- **Async-first** — built on Tokio; blocking work is offloaded explicitly.
- **Conditional task execution** — per-task trigger rules (`all` / `any` / `none`
  over upstream task states) enable branching and conditional fan-in. See
  [Trigger Rules](../engine/trigger-rules/).
- **External config & secrets** — reference connections, secrets, and config by
  name and resolve them from `CLOACINA_VAR_*` environment variables at runtime
  (`cloaca.var()` / `var_or()`). See
  [Variable Registry](../../how-to/embed/variable-registry/).
- **Two database backends, chosen at runtime** — Postgres or SQLite via the
  connection URL, no recompile. See
  [Database Backends](../service/database-backends/).

## Two execution primitives

- **[Workflows](../primitives/workflows/)** — durable DAGs of tasks with
  dependencies, retries, and recovery; the task is the unit of scheduling.
- **[Computation Graphs](../primitives/computation-graphs/)** — in-process,
  deterministic, event-driven DAGs; the whole traversal is the unit.
- **They compose** — workflows can
  [subscribe to a reactor](../../how-to/embed/subscribe-workflow-to-reactor/)
  and a task can
  [invoke a computation graph](../../how-to/embed/invoke-computation-graph-from-workflow/).

## Sync

- **[Sync](../primitives/sync/)** — data movement built from the two primitives:
  a sync is a workflow instance, each segment of records is a workflow
  execution, and connectors are constructor providers.
- **Sources and destinations** — connector members under one
  [contract](../../reference/connector-contract/): discover, a streaming read of
  records and checkpoints, a client-streaming write.
- **Cursor after the write** — the cursor advances in the transaction that
  completes the write task, after the destination has accepted the segment, so
  a crash replays at most one segment and never loses one.
- **Dead-letters** — a record that cannot be mapped, coerced or written is kept
  with its reason; the run goes on.
- **Runs on the fleet** — every sync runs on an execution agent, never in the
  server process; connectors run sandboxed in WASM.

## Computation-graph capabilities

These build on reactors and accumulators — see [Concepts](../concepts/)
first if those terms are new.

- **Accumulators** — adapters that turn an external stream or source into
  *boundary events* (the discrete events a graph reacts to), in several flavors
  (passthrough, stream, polling, batch, state). See
  [Accumulator Types](../../how-to/computation-graphs/accumulator-types/).
- **Reactors** — fire a graph when their criteria are met (`when_any` /
  [`when_all`](../../how-to/computation-graphs/when-all-criteria/))
  with a [`latest` or `sequential`](../../how-to/computation-graphs/sequential-strategy/)
  input strategy.
- **Routing** — enum-variant routing propagates results conditionally between
  nodes.
- **Kafka stream accumulators** — consume topics via the bundled
  `cloacina-provider-kafka` constructor provider (the Kafka client ships in the
  provider, not the engine); see
  [Kafka Stream](../../tutorials/service/06-kafka-stream/).
- **CEL firing filters** — filter reactor firings with CEL (Common Expression
  Language) expressions; see
  [Filter Reactor Firings with CEL](../../how-to/computation-graphs/filter-reactor-firings-with-cel/).

## Scheduling & triggers

- **Cron schedules** — run workflows on a schedule; see
  [Cron Scheduling](../engine/cron-scheduling/).
- **Event triggers** — start work from external events; see
  [Event Triggers](../../tutorials/embed/07-event-triggers/).

## Deployment modes

- **Embedded library** — `cloacina` (Rust) / `cloaca` (Python) in your process.
- **Daemon** — `cloacinactl daemon` watches a directory and runs packages locally.
  See [Running the Daemon](../../how-to/embed/running-the-daemon/).
- **Server** — `cloacina-server`, an HTTP + WebSocket control plane. See
  [Deploy a Server](../../tutorials/service/01-deploy-a-server/).
- **Compiler service** — `cloacina-compiler` builds uploaded packages in a
  sandbox. See [Running the Compiler](../../how-to/service/running-the-compiler/).
- **Execution-agent fleet** — `cloacina-agent` workers run tasks off the server
  for horizontal scale. See
  [Deploy an Execution-Agent Fleet](../../how-to/service/deploy-an-execution-agent-fleet/)
  and [Execution-Agent Fleet](../service/execution-agent-fleet/).

## Packaging

- **`.cloacina` packages** — a `package.toml` plus source (Python module tree, or
  Rust compiled on the server at load). Scaffold, validate, pack, and upload with
  `cloacinactl package`. See
  [Creating Your First Package](../../how-to/service/creating-your-first-package/)
  and the [Package Format](../engine/package-format/).
- **Reconciler** — the server loads and registers uploaded packages
  automatically. See
  [Reconciler Pipeline](../service/reconciler-pipeline/).

## Multi-tenancy & auth

- **Schema-per-tenant isolation** (Postgres) — see
  [Multi-Tenancy](../service/multi-tenancy/) and
  [Configure a Multi-Tenant Deployment](../../how-to/service/configure-multi-tenant-deployment/).
- **API keys with roles** — admin / write / read, managed via `cloacinactl key`
  and the API.

## Clients & UI

- **Client SDKs** — Rust, Python, and TypeScript clients for calling a running
  server over HTTP/WebSocket. See [Client SDKs](../../reference/sdks/).
- **Web UI** — operate and observe a server: workflows, executions (with a live
  event stream), triggers, computation-graph health, package upload, and API-key
  management. See [The Web UI](../../tutorials/service/02-the-web-ui/).

## Operations & security

- **Observability** — Prometheus `/metrics`, structured logs, and optional
  OpenTelemetry export. See
  [Observability](../service/observability/) and the
  [Metrics Catalog](../../reference/metrics-catalog/).
- **Package signature enforcement** — the server can require signatures on
  uploaded packages and manages signing keys (private keys encrypted at rest
  with AES-256-GCM). Producing a signed workflow package from the CLI is not
  yet available — `cloacinactl package pack --sign` fails with a clear error
  rather than silently producing an unsigned archive. See
  [Package Signing](../../how-to/service/security/package-signing/)
  and [Require Signed Packages](../../how-to/service/require-signed-packages/).
- **Build sandboxing** — the compiler builds untrusted packages in an isolated
  environment (Linux). See [Security Model](../service/security-model/).
- **Horizontal scaling** — stateless schedulers coordinate through the database.
  See [Horizontal Scaling](../service/horizontal-scaling/).

## Tooling

- **`cloacinactl`** — the operator + developer CLI (daemon, server/compiler
  lifecycle, package, workflow, execution, tenant, key, trigger, graph). See the
  [CLI Reference](../../reference/cli/).
- **HTTP + WebSocket API** — see the
  [HTTP API Reference](../../reference/http-api/) and
  [WebSocket Protocol](../../reference/websocket-protocol/).
