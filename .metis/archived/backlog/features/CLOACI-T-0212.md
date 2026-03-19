---
id: cloacinactl-daemon-lightweight
level: task
title: "cloacinactl daemon — lightweight local scheduler with SQLite"
short_code: "CLOACI-T-0212"
created_at: 2026-03-18T02:03:22.148013+00:00
updated_at: 2026-03-18T13:22:12.035714+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# cloacinactl daemon — lightweight local scheduler with SQLite

## Objective

Add a `cloacinactl daemon` subcommand that runs a headless local scheduler using SQLite — no HTTP API, no Postgres, no Docker. Drop `.cloacina` packages into a watched directory, the daemon picks them up, registers them, and runs them on schedule. A zero-infrastructure way to run Cloacina workflows.

## Motivation

Today Cloacina has two execution modes:

1. **Library** — embed `DefaultRunner` in your Rust app, call it from code (tutorials 01-08)
2. **Server** — `cloacinactl serve` with Postgres, HTTP API, auth, multi-tenancy (tutorials 20-21)

There's a gap between "write Rust code to run workflows" and "deploy a full server with Postgres." Many use cases need persistent scheduling without the server overhead:

- **Local development** — iterate on workflow packages without Docker/Postgres
- **CI/CD pipelines** — run scheduled workflows as part of a build pipeline
- **Edge / IoT** — single-binary deployment on constrained hardware
- **Single-user / cron replacement** — replace crontab with observable, retry-aware workflow execution

### Priority
- [x] P2 - Medium (nice to have)

### Business Justification
- **User Value**: Lowest-friction path to running packaged workflows. No infrastructure setup, no database admin, no Docker. Just `cloacinactl daemon --packages ./workflows/`.
- **Business Value**: Broadens the user base beyond teams that can run Postgres. Makes Cloacina viable for CI, edge, and developer-machine use cases.
- **Effort Estimate**: M — most of the machinery exists (`DefaultRunner` with SQLite, registry reconciler, filesystem storage). The new work is the CLI subcommand, directory watcher, and lifecycle management.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `cloacinactl daemon --packages <dir>` starts a headless process with SQLite storage, scheduler, executor, cron, and registry reconciler
- [ ] `--db <path>` sets the SQLite database location (defaults to `~/.cloacina/daemon.db`)
- [ ] `--storage <path>` sets the filesystem registry storage location (defaults to `~/.cloacina/storage/`)
- [ ] Directory watcher auto-registers new `.cloacina` files and unregisters removed ones
- [ ] `cloacinactl daemon status --db <path>` shows registered workflows, cron schedules, and recent executions (reads SQLite directly, no running daemon required)
- [ ] `cloacinactl daemon schedule set <name> --cron "..." [--timezone UTC] --db <path>` creates a cron schedule
- [ ] `cloacinactl daemon schedule list --db <path>` lists all cron schedules
- [ ] `cloacinactl daemon schedule delete <id> --db <path>` removes a cron schedule
- [ ] Signal handling: SIGTERM/Ctrl+C triggers graceful shutdown
- [ ] Works with the existing `simple-packaged-demo` example: drop the `.cloacina` file into the packages dir, set a schedule, see it execute

## Implementation Notes

### What already exists

- `DefaultRunner::with_config(sqlite_url, config)` — fully functional with SQLite
- `DefaultRunnerConfig::builder().enable_registry_reconciler(true).registry_storage_backend("filesystem")` — filesystem-based package storage
- `FilesystemRegistryStorage` — stores .cloacina binaries on disk
- `RegistryReconciler` — background loop that loads packages into global registries
- `CronScheduler` — cron-based workflow execution
- `CronRecoveryService` — handles missed executions after restart

### What's new

1. **`cloacinactl daemon` subcommand** — CLI arg parsing, lifecycle management, signal handling
2. **Directory watcher** — polling loop on the packages directory, registers new `.cloacina` files via `WorkflowRegistryImpl`, unregisters removed ones
3. **`cloacinactl daemon status`** — reads SQLite to show registered workflows, schedules, recent executions
4. **`cloacinactl daemon schedule set/list/delete`** — CLI wrappers around `DefaultRunner`'s cron API for managing schedules against the local SQLite DB

### Sketch of the flow

```
~/.cloacina/
├── daemon.db          # SQLite — executions, metadata, cron schedules
├── packages/          # Drop .cloacina files here
│   ├── analytics.cloacina
│   ├── analytics.toml      # optional: schedule + context
│   └── etl-pipeline.cloacina
└── storage/           # Managed by FilesystemRegistryStorage
    └── ...            # Registered package binaries
```

```bash
# Start the daemon
cloacinactl daemon --packages ~/.cloacina/packages

# In another terminal, deploy a workflow
cp my-workflow.cloacina ~/.cloacina/packages/
cp my-workflow.toml ~/.cloacina/packages/   # cron = "*/5 * * * *"

# Check status
cloacinactl daemon status
```

### Design decisions
- **Directory watching**: polling every N seconds (not inotify/kqueue). Simpler, portable, matches the reconciler's existing pattern.
- **Package removal**: deleting a `.cloacina` file from the directory unregisters the workflow. Mirrors the reconciler's "unload packages not in DB" logic.
- **Schedule management**: explicit via `cloacinactl daemon schedule set/list/delete` CLI commands. No auto-scheduling from sidecars. Same cron API as the server (CLOACI-T-0213) but exposed via CLI instead of HTTP.
- **Multi-instance**: SQLite is explicitly single-instance. Document that server mode (Postgres) is needed for multi-instance.

### Related
- `cloacinactl serve` — the full server mode (HTTP + Postgres). Daemon is the complement.
- CLOACI-T-0211 — FFI boundary testing. Daemon mode would benefit from the same FFI test harness.
- Tutorial 08 — library-level registry. Daemon mode automates what tutorial 08 does manually.

## Status Updates

*To be added during implementation*
