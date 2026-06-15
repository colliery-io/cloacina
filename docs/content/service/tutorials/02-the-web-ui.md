---
title: "02 - The Web UI"
description: "Connect the Cloacina web UI to a server, execute a workflow, and watch it run live."
weight: 12
---

In this tutorial you'll bring up the Cloacina web UI, connect it to a
running `cloacina-server`, and watch a workflow execute in real time.
By the end you'll know how the UI authenticates, where each control
plane view lives, and how the live execution stream works.

## What you'll learn

- How to stand up the full local UI stack with one command.
- How to connect the UI to a server with a tenant API key.
- How to upload a packaged workflow, execute it, and follow the run
  live as its events stream in.
- How to seed a demo workload so there's always something to watch.

## Prerequisites

- A checkout of the cloacina repository with `angreal`, Rust, Docker,
  and Node 20+ available.

## Step 1 — Bring up the stack

From the repo root:

```bash
angreal ui up
```

This starts PostgreSQL, builds and runs a CORS-enabled
`cloacina-server`, starts a `cloacina-compiler` (so packages you upload
actually build), builds the `@cloacina/client` SDK, and serves the UI
dev server. When it's ready it prints the connection details:

```
  UI:      http://localhost:5173
  Server:  http://localhost:8080
  Connect with →  server:  http://localhost:8080
                  api key: clk_dev_ui_bootstrap_key_0001
                  tenant:  public
```

> **Why the compiler?** `cloacina-server` does not build uploaded
> packages itself — a separate `cloacina-compiler` polls the database
> and compiles them. `angreal ui up` starts one for you; without it,
> uploaded workflows would sit in `pending` forever.

## Step 2 — Connect

Open <http://localhost:5173>. You'll land on the **Connect** screen.
Enter the server URL, the bootstrap API key, and the tenant (`public`)
printed above, then click **Connect**. The key is held in
`sessionStorage` for the tab only — closing the tab clears it.

You're now on the **Overview** dashboard: a status rollup, a graph
summary, and the most recent executions.

## Step 3 — Seed a demo workload

In a second terminal, drive a continuous workload so the UI has live
activity:

```bash
angreal ui seed --loop
```

This uploads the demo workflows and fires a mix of fast, slow, and
intentionally-failing runs every few seconds. (Use `angreal ui seed`
without `--loop` for a one-shot deterministic set: one completed, one
failed, one in-flight run.)

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

- [Deploy the web UI](../how-to-guides/deploy-the-web-ui/) as a
  container against any server.
- Manage tenant API keys from the **API Keys** view (create shows the
  key exactly once — copy it then).
