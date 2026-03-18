---
title: "22 - Local Daemon Scheduler"
description: "Run workflows locally with SQLite — no Docker, no Postgres, no HTTP API"
weight: 32
draft: true
---

## Overview

The Cloacina daemon is a headless local scheduler that runs workflows using SQLite and filesystem storage. No Docker, no Postgres, no HTTP API — just a single binary watching a directory for `.cloacina` packages and executing them on cron schedules.

This is the lightweight counterpart to the [Cloacina server]({{< ref "/tutorials/20-server-quickstart" >}}). Use the daemon when you need:

- **Local development** — iterate on workflow packages without infrastructure
- **CI/CD pipelines** — run scheduled workflows as part of a build
- **Edge / IoT** — single-binary deployment on constrained hardware
- **Cron replacement** — observable, retry-aware workflow execution

## Prerequisites

- `cloacinactl` binary (built from source or downloaded)
- A `.cloacina` workflow package (see [Tutorial 07: Packaged Workflows]({{< ref "/tutorials/07-packaged-workflows" >}}))

## Quick Start

```bash
# 1. Register a workflow package
cloacinactl daemon register my-workflow.cloacina

# 2. Set a cron schedule
cloacinactl daemon schedule set my_workflow --cron "0 9 * * *"

# 3. Start the daemon
cloacinactl daemon --packages ./workflows/

# 4. Check status (in another terminal)
cloacinactl daemon status
```

That's it. The daemon creates `~/.cloacina/daemon.db` (SQLite) and `~/.cloacina/storage/` (package binaries) automatically.

## Step 1: Build a Workflow Package

If you don't have a package yet, build the included example:

```bash
cd examples/features/simple-packaged
cargo build --release

# Package the shared library into a .cloacina archive
cd target/release
tar czf simple-packaged-demo.cloacina libsimple_packaged_demo.so
# On macOS: tar czf simple-packaged-demo.cloacina libsimple_packaged_demo.dylib
```

## Step 2: Register the Package

```bash
cloacinactl daemon register simple-packaged-demo.cloacina
```

Output:

```
Package registered: 3c96ccaa-f554-46a6-bd1e-f9f23087752e
```

This validates the package, stores the binary in `~/.cloacina/storage/`, and saves metadata to the SQLite database.

### Custom paths

By default, the daemon stores everything under `~/.cloacina/`. Override with flags:

```bash
cloacinactl daemon register my-workflow.cloacina \
  --db /var/lib/cloacina/daemon.db \
  --storage /var/lib/cloacina/storage
```

## Step 3: Set a Cron Schedule

```bash
cloacinactl daemon schedule set data_processing --cron "*/30 * * * * *"
```

Output:

```
Schedule created: data_processing — "*/30 * * * * *" (UTC) [323a7d06-f987-4d92-87b6-2a670d0d86d7]
```

{{< hint type=info title="Workflow Name vs Package Name" >}}
The schedule targets the **workflow name** from the `#[packaged_workflow(name = "data_processing")]` macro, not the package name or filename.
{{< /hint >}}

### Schedule management

```bash
# List all schedules
cloacinactl daemon schedule list

# Delete a schedule by ID
cloacinactl daemon schedule delete 323a7d06-f987-4d92-87b6-2a670d0d86d7
```

### Timezone support

```bash
cloacinactl daemon schedule set my_workflow \
  --cron "0 9 * * 1-5" \
  --timezone "America/New_York"
```

## Step 4: Start the Daemon

```bash
cloacinactl daemon --packages ./workflows/
```

The daemon:

1. Starts a `DefaultRunner` with SQLite + filesystem storage
2. The **registry reconciler** loads all registered packages into the global workflow registry
3. The **cron scheduler** fires workflows according to their schedules
4. The **directory scanner** watches `--packages` for new `.cloacina` files and auto-registers them

```
INFO Starting cloacina daemon db=~/.cloacina/daemon.db storage=~/.cloacina/storage packages=./workflows/
INFO Daemon runner started, background services active
INFO Directory scanner started dir=./workflows/
INFO Startup reconciliation completed: 1 loaded, 0 unloaded, 0 failed
INFO Executing workflow 'data_processing' for schedule 323a7d06-...
INFO Pipeline completed: e271dd41-... (name: data_processing, 3 completed, 0 failed, 0 skipped)
INFO Successfully executed and audited workflow data_processing
```

### Graceful shutdown

Press `Ctrl+C` or send `SIGTERM`. The daemon stops the scheduler, waits for running tasks to complete, and closes the database.

## Step 5: Check Status

In another terminal:

```bash
cloacinactl daemon status
```

Output:

```
Registered Workflows (1):
  simple_demo v1.0.0 — 3 tasks [3c96ccaa-f554-46a6-bd1e-f9f23087752e]

Cron Schedules (1):
  data_processing — "*/30 * * * * *" (enabled) — next: 2026-03-18 04:00:00 UTC [323a7d06-...]

Execution Stats (last 24h):
  Total:      12
  Successful: 12
  Failed:     0
  Success:    100.0%
```

The status command reads the SQLite database directly — it works even when the daemon isn't running.

## Directory Watching

The daemon watches the `--packages` directory for new `.cloacina` files:

```bash
# While the daemon is running, drop a new package in:
cp analytics-pipeline.cloacina ./workflows/

# The daemon detects it within --poll-interval seconds:
# INFO Registering package file=analytics-pipeline.cloacina
# INFO Package registered successfully id=...
```

Removing a file from the directory logs a notice but doesn't auto-unregister the workflow. Use `cloacinactl daemon schedule delete` to manage the lifecycle explicitly.

## Configuration Reference

### `cloacinactl daemon` flags

| Flag | Default | Description |
|------|---------|-------------|
| `--packages <dir>` | (required) | Directory to watch for `.cloacina` files |
| `--db <path>` | `~/.cloacina/daemon.db` | SQLite database path |
| `--storage <path>` | `~/.cloacina/storage/` | Filesystem storage for package binaries |
| `--poll-interval <secs>` | `5` | How often to scan for new packages |

### `cloacinactl daemon status` flags

| Flag | Default | Description |
|------|---------|-------------|
| `--db <path>` | `~/.cloacina/daemon.db` | SQLite database to read |
| `--storage <path>` | `~/.cloacina/storage/` | Storage directory to read |

### `cloacinactl daemon schedule set` flags

| Flag | Default | Description |
|------|---------|-------------|
| `--cron <expr>` | (required) | Cron expression (6-field with seconds) |
| `--timezone <tz>` | `UTC` | IANA timezone name |
| `--db <path>` | `~/.cloacina/daemon.db` | SQLite database |

## Daemon vs Server

| | Daemon | Server |
|---|---|---|
| **Database** | SQLite (local file) | PostgreSQL |
| **Package upload** | `daemon register` CLI or directory drop | `POST /workflows/packages` HTTP |
| **Schedule management** | `daemon schedule` CLI | `POST /workflows/{name}/schedules` HTTP |
| **Execution trigger** | Cron only | Cron + on-demand API |
| **Auth** | None (local only) | PAK + ABAC |
| **Multi-tenancy** | No | Yes (schema isolation) |
| **Horizontal scaling** | Single instance | Multiple workers/schedulers |
| **Infrastructure** | Zero | Docker + Postgres |

Both use the same `DefaultRunner`, `RegistryReconciler`, and `CronScheduler` under the hood. The daemon is a subset of the server — everything that works in the daemon works in the server.

## Next Steps

- [Tutorial 20: Server Quick Start]({{< ref "/tutorials/20-server-quickstart" >}}) — deploy the full server with Docker + Postgres
- [Tutorial 21: Server Workflow Management]({{< ref "/tutorials/21-server-workflow-management" >}}) — the same lifecycle through HTTP API
- [Tutorial 07: Packaged Workflows]({{< ref "/tutorials/07-packaged-workflows" >}}) — create your own workflow packages
