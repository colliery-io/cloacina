---
title: "02 - The Web UI"
description: "Connect the Cloacina web UI to a server, execute a workflow, and watch it run live."
weight: 12
aliases:
  - "/platform/tutorials/02-the-web-ui/"

---

In this tutorial you'll bring up the Cloacina web UI, connect it to a
running `cloacina-server`, and watch a workflow execute in real time.
By the end you'll know how the UI authenticates, where each control
plane view lives, and how the live execution stream works.

## What you'll learn

- How to stand up the full UI demo stack with one command.
- How to connect the UI to a server with a tenant API key.
- How to upload a packaged workflow, execute it, and follow the run
  live as its events stream in.
- Where the demo's continuous workload comes from so there's always
  something to watch.

## Prerequisites

- A checkout of the cloacina repository with `angreal` and Docker
  (including Compose) available. Everything else — the server, compiler,
  execution-agent fleet, and UI — builds and runs inside containers, so
  you don't need Rust or Node installed on your host.

## Step 1 — Bring up the stack

From the repo root:

```bash
angreal ui up
```

This builds and starts the docker compose demo stack: PostgreSQL, Kafka,
a CORS-enabled `cloacina-server`, an in-cluster `cloacina-compiler` (so
packages you upload actually build), a 3-agent execution fleet that runs
the work, a one-shot packer + seed that loads the demo packages, and a
live computation-graph producer. The first run builds images and
compiles the demo packages, so it takes a few minutes. When it's ready
it prints the connection details:

```
  UI:      http://localhost:8082
  Server:  http://localhost:8080
  Connect with →  server:  http://localhost:8080
                  api key: clk_demo_bootstrap_key_0001
                  tenant:  public
```

> **Why the compiler?** `cloacina-server` does not build uploaded
> packages itself — a separate `cloacina-compiler` polls the database
> and compiles them. The demo stack runs one for you; without it,
> uploaded workflows would sit in `pending` forever.

## Step 2 — Connect

Open <http://localhost:8082>. You'll land on the **Connect** screen, with
the server URL and tenant (`public`) prefilled. Paste the bootstrap API
key printed above and click **Connect**. The key is held in
`sessionStorage` for the tab only — closing the tab clears it.

You're now on the **Overview** dashboard: a status rollup, a graph
summary, and the most recent executions.

## Step 3 — The demo is already live

You don't need to seed anything. The stack ships with a continuous
workload: a driver fires `demo_slow_workflow` runs on an interval, a cron
trigger and a poll trigger fire on their own schedules, and the producer
streams market data into the computation graphs so they keep firing. So
the **Executions**, **Triggers**, and **Graphs** views all have live
activity from the moment you connect.

The demo also loads a spread of example packages — including a complex
20-task DAG (`complex_dag_workflow`) and several computation graphs — so
the catalog reflects real structure, not just toy single-task workflows.

## Step 4 — Watch a run live

Go to **Executions**. You'll see runs accruing. Click an in-flight
`demo_slow_workflow` run: its detail view shows a **live** badge and the
**Event log** streams task events in as each of the five steps
completes, then flips to a terminal status when the run finishes. This
is the live delivery stream — the same WebSocket the SDK exposes,
deduplicated on sequence number.

Open a **failed** run to see its error and event history — the
debugging surface for `demo_fail_workflow`.

## Step 5 — Upload and execute your own

Go to **Workflows → Upload**, choose a `.cloacina` package, and upload
it. Once the compiler finishes building it, open its detail page and
click **Execute** (optionally with a JSON context) to start a run and
hand off to the live view.

## What's next

- [Deploy the web UI]({{< ref "/service/how-to/deploy-the-web-ui" >}}) as a
  container against any server.
- Manage tenant API keys from the **API Keys** view (create shows the
  key exactly once — copy it then).
