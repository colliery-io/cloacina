---
title: "Cloacina"
description: "Embedded-first workflow orchestration for Rust and Python — what it is, when to use it, and how to start."
---

# Cloacina Documentation

Cloacina is a **workflow orchestration engine you embed in your application** —
or run as a standalone multi-tenant server when you outgrow embedding. It runs
durable, database-backed task pipelines with automatic retries and recovery, and
ships first-class bindings for both **Rust** and **Python**.

New here? Jump to **[Get started](#get-started)** below, or read on to see whether
Cloacina fits your problem.

## What Cloacina is for

Use Cloacina when you have multi-step work that must run **reliably** — survive
process restarts, retry on failure, and never silently drop a step — and you'd
rather keep that orchestration **inside your app and your database** than stand
up a separate orchestration service with its own brokers and workers.

Typical fits:

- Data processing / ETL pipelines with ordered, dependent steps.
- Background job systems and integration workflows that need durable state.
- Event-driven, in-process pipelines that react to a stream and run a fixed
  sequence of steps.
- Teams that start embedded in one app and later graduate to a shared server.

## Why Cloacina

- **Embedded-first.** Add a library (`cloacina` for Rust, `cloaca` for Python) and
  run workflows in-process — no separate scheduler, workers, or message broker.
- **The database is the only dependency.** State and coordination live in
  Postgres or SQLite; there is no Redis/queue/Zookeeper to operate.
- **Guaranteed execution.** Task state is persisted and claimed atomically; failed
  or stalled work is recovered. Execution is **at-least-once** — steps should be
  safe to run more than once.
- **Rust *and* Python, the same engine.** The Python bindings are full parity, not
  a thin wrapper — the same API and runtime, proven against the Rust crate.
- **Grows with you.** The same packaged workflow runs embedded, under a local
  background process, or on the multi-tenant `cloacina-server` — without a rewrite.

## When *not* to use Cloacina

A few honest edges (full guide:
[When to use Cloacina]({{< ref "/quick-start/when-to-use" >}})):

- **You want a no-code, click-to-build scheduler.** Workflows are written in Rust
  or Python; the web UI operates and observes, it doesn't author pipelines.
- **You need multi-tenant isolation or horizontal scaling.** Those require the
  Postgres-backed server (SQLite is single-process), and the hardened multi-tenant
  server targets Linux.
- **You need exactly-once execution, or a synchronous (non-`async`) task model.**
  Cloacina is at-least-once and async-first.

## Two execution primitives

Cloacina exposes two complementary primitives — pick the one that matches the work:

- **[Workflows]({{< ref "/workflows" >}})** — durable, database-backed pipelines of
  dependent steps (tasks). Each step is scheduled and recorded individually, so
  work survives a process restart and recovers from failure. Pick this for ETL,
  background jobs, and integrations.
- **[Computation Graphs]({{< ref "/computation-graphs" >}})** — fast, in-memory
  pipelines that run as a single unit in response to incoming events. Pick this for
  event-driven, latency-sensitive processing.

The two share the same engine, packaging, and multi-tenant model, and they
compose. New to the vocabulary? Start with
**[Concepts]({{< ref "/quick-start/concepts" >}})**. For the full capability list,
see the **[Features overview]({{< ref "/quick-start/features" >}})**.

## Ways to run it

| Mode | What it is | Backed by |
|------|-----------|-----------|
| **Embedded library** | `cloacina` (Rust) / `cloaca` (Python) in your process | SQLite or Postgres |
| **Daemon** | a local background process that watches a directory and runs packages | SQLite |
| **Server** | `cloacina-server` — HTTP + WebSocket control plane, multi-tenant, web UI | Postgres |
| **Compiler + agent fleet** | build packages and run tasks across workers to scale the server out | Postgres |

The embedded library and the server share the same engine, packaging format, and
multi-tenant model — start embedded and graduate without a rewrite. For *which* to
choose, see [When to use Cloacina]({{< ref "/quick-start/when-to-use" >}}); the two
deployment postures also have deliberately different trust models, covered in the
[Security Model]({{< ref "/platform/explanation/security-model" >}}).

## Get started

Pick the shortest path to a working result:

- **Python — code-only, ~5 minutes.** Install `cloaca` and run a workflow
  in-process. → [Python Quick Start]({{< ref "/python/quick-start" >}})
- **Rust — embedded.** Add the `cloacina` crate and run the first tutorial.
  → [Your First Workflow]({{< ref "/workflows/tutorials/library/01-first-workflow" >}})
- **See it running with a web UI (more setup).** The fastest way to *watch*
  workflows execute live — stand up a server and open the UI.
  → [Deploy a Server]({{< ref "/platform/tutorials/01-deploy-a-server" >}}) then
  [The Web UI]({{< ref "/platform/tutorials/02-the-web-ui" >}})
- **Install the CLI to operate it.**
  → [Installing cloacinactl]({{< ref "/quick-start/install" >}})

Not sure which? The **[Quick Start]({{< ref "/quick-start" >}})** routes you by goal.

## Libraries & tools

- **[Cloacina]({{< ref "/workflows/tutorials/" >}})** — native Rust library.
- **[Cloaca]({{< ref "/python/" >}})** — Python bindings, full parity with Rust.
- **[`cloacinactl`]({{< ref "/quick-start/install" >}})** — operator + developer
  CLI (bundles the daemon).
- **[Client SDKs]({{< ref "/sdks" >}})** — Rust, Python, and TypeScript clients for
  calling a running `cloacina-server` over HTTP/WebSocket.

## See also

- [When to Use Cloacina]({{< ref "/quick-start/when-to-use" >}}) — does it fit your problem?
- [Features Overview]({{< ref "/quick-start/features" >}}) — the full capability catalog.
- [Concepts]({{< ref "/quick-start/concepts" >}}) — the core primitives.
- [Quick Start]({{< ref "/quick-start" >}}) — pick the right tutorial track.
- [Platform]({{< ref "/platform" >}}) — deploy and operate the server, compiler, and fleet.
- [Glossary]({{< ref "/glossary" >}}) — every term used in these docs.
- [Troubleshooting]({{< ref "/troubleshooting" >}}) — common problems and resolutions.
