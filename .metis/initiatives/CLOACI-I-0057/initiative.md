---
id: daemon-mode-lightweight-local
level: initiative
title: "Daemon Mode — Lightweight Local Scheduler"
short_code: "CLOACI-I-0057"
created_at: 2026-03-28T14:00:48.736606+00:00
updated_at: 2026-03-29T11:17:38.134669+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: daemon-mode-lightweight-local
---

# Daemon Mode — Lightweight Local Scheduler

## Context

Split from I-0049 (Server & Daemon). The daemon is the lightweight local deployment mode — no HTTP API, no auth, no Postgres. It watches a directory for `.cloacina` packages, loads them via the reconciler, and runs cron + trigger schedules.

Starting with daemon establishes patterns (package loading, cron scheduling, trigger scheduling, graceful shutdown) that the server will reuse.

Prior art on `archive/cloacina-server-week1`: daemon implementation at commit `43968dd`, cron API at `6fb6616`.

### Key learnings from archive
- Cron poll interval should default to 50ms (was 30s — too slow)
- Catchup policy should default to `run_all` (was `skip`)
- Directory watcher needs debouncing for rapid file changes
- Graceful shutdown must drain in-flight pipelines

## Goals & Non-Goals

**Goals:**
- `cloacinactl daemon` command: starts a long-running local scheduler
- SQLite backend (auto-created, no external dependencies)
- Directory watcher: watches a configurable path for `.cloacina` packages
- Package reconciler: loads/unloads packages as they appear/disappear
- Cron scheduling: runs cron schedules defined in packages
- Trigger scheduling: polls triggers defined in packages (I-0056)
- Graceful shutdown on SIGINT/SIGTERM
- Daemon home directory (`~/.cloacina/`) for DB, state, default packages
- Multiple watch directories via repeatable `--watch-dir`
- CLI flags: `--watch-dir`, `--home`, `--poll-interval`, `--log-level`
- Hot reload: SIGHUP or config file watch triggers re-read of configuration
- File logging to `~/.cloacina/logs/` (structured, rotatable)
- Standalone binary distribution (pre-built binaries, install script, Homebrew formula)
- Daemon soak test via angreal

**Non-Goals:**
- HTTP API (that's the server initiative, I-0049)
- PAK+ABAC auth (server only)
- Postgres backend (server only)
- Multi-tenancy (server only)
- Docker compose (server only)
- Continuous scheduling (I-0053)

## Detailed Design

### CLI Interface

```
cloacinactl daemon [OPTIONS]

Options:
  --watch-dir <PATH>...   Directories to watch for .cloacina packages (repeatable) [default: ~/.cloacina/packages]
  --home <PATH>           Daemon home directory for DB, logs, state [default: ~/.cloacina]
  --poll-interval <MS>    Cron poll interval in milliseconds [default: 50]
  --log-level <LEVEL>     Log level [default: info]
```

### Daemon Home (`~/.cloacina/`)

The daemon uses a home directory for all operational state:
- `~/.cloacina/cloacina.db` — SQLite database (execution history, schedules, state)
- `~/.cloacina/packages/` — default watch directory for `.cloacina` packages
- `~/.cloacina/logs/` — structured log files (JSON or text, rotatable)
- `~/.cloacina/config.toml` — (optional) daemon configuration file for hot reload

Users can register additional watch directories via `--watch-dir` (repeatable). The default `~/.cloacina/packages/` is always included unless `--home` is overridden.

### Catchup Policy

Catchup policy (what to do about missed cron firings) is set **per-workflow in the package manifest**, not globally on the scheduler. The scheduler reads the policy from each `CronSchedule` record. Default is `run_all`.

### Storage Model

**Filesystem for packages, SQLite for state.** The daemon does NOT store `.cloacina` archive blobs in the database. Instead:
- Packages live on disk in `--watch-dir` — the filesystem IS the package store
- SQLite stores operational state: cron schedules, trigger schedules, execution history, task state
- A filesystem-backed `WorkflowRegistry` implementation reads archives from disk when the reconciler needs package data

This requires a `FilesystemWorkflowRegistry` that implements the `WorkflowRegistry` trait:
- Constructed with multiple watch directory paths
- `list_workflows()` — scans all watch directories for `.cloacina` files, peeks manifests for metadata
- `get_workflow(name, version)` — reads the archive bytes from disk
- No `register_workflow_package()` needed — the filesystem handles "registration" (file appears = registered)

### Components

1. **FilesystemWorkflowRegistry** — `WorkflowRegistry` trait impl backed by a directory of `.cloacina` files
2. **Directory Watcher** — watches `--watch-dir` for `.cloacina` file changes, triggers reconciliation
3. **Package Reconciler** — existing `RegistryReconciler`, configured with `FilesystemWorkflowRegistry`
4. **Cron Scheduler** — existing `CronScheduler`, polls at configured interval
5. **Trigger Scheduler** — existing `TriggerScheduler`, polls registered triggers
6. **Pipeline Executor** — existing `DefaultRunner` with SQLite backend
7. **Shutdown Handler** — listens for SIGINT/SIGTERM, drains in-flight work

### Startup Flow

1. Parse CLI args
2. Initialize `~/.cloacina/` home directory (create if needed)
3. Set up file logging to `~/.cloacina/logs/`
4. Create/open SQLite database at `~/.cloacina/cloacina.db`
5. Create `DefaultRunner` with SQLite
6. Create `FilesystemWorkflowRegistry` with all watch directories
7. Start `RegistryReconciler` with filesystem registry (initial scan)
8. Start `CronScheduler`
9. Start `TriggerScheduler`
10. Start directory watcher on all watch dirs (filesystem notify)
11. Set up SIGHUP handler for hot reload
12. Block until shutdown signal
13. Graceful shutdown: stop schedulers, drain pipelines, close DB

### Distribution

The daemon should be installable without building from source:

- **Pre-built binaries** — GitHub Releases with platform-specific binaries (linux-x86_64, linux-arm64, macos-arm64, macos-x86_64)
- **Install script** — `curl -sSL https://install.cloacina.dev | sh` (downloads correct binary, places in PATH)
- **Homebrew** — `brew install colliery-io/tap/cloacinactl` (macOS/Linux)
- **Cargo install** — `cargo install cloacinactl` as fallback for Rust users

The binary is self-contained — SQLite is bundled, no external dependencies. Users just need the `cloacinactl` binary and their `.cloacina` packages.

## Prior Art

Reference implementation on `archive/cloacina-server-week1`:
- Daemon: commit `43968dd`
- Cron API: commit `6fb6616`

## Alternatives Considered

- **Polling-based directory watch**: Rejected in favor of `notify` crate for filesystem events — lower latency, less CPU.
- **Daemon with embedded HTTP**: Rejected to keep daemon minimal. If users need HTTP, they use the server mode.

## Scope Decisions (from discovery)

- **Filesystem for packages, SQLite for state** — don't store archive blobs in the DB. The watch directory IS the package store.
- **FilesystemWorkflowRegistry** — new `WorkflowRegistry` trait impl that reads `.cloacina` files from disk. Supports multiple directories. Enables the existing reconciler to work unchanged.
- **Catchup policy is per-workflow** — set in the package manifest, not globally on the scheduler. Default `run_all`.
- **Daemon home `~/.cloacina/`** — central location for DB, state, default packages. Additional watch dirs via repeatable `--watch-dir`.

## Implementation Plan

1. **FilesystemWorkflowRegistry** — `WorkflowRegistry` trait impl backed by multi-directory scan
2. **CLI + startup** — `cloacinactl daemon` subcommand, `~/.cloacina/` home, SQLite init, DefaultRunner
3. **Directory watcher** — `notify` crate, debounced events, trigger reconciliation on file changes
4. **Scheduler wiring** — Start cron + trigger schedulers from reconciled packages
5. **Graceful shutdown** — Signal handling, drain in-flight, close connections
6. **File logging** — Structured logs to `~/.cloacina/logs/`, rotation, configurable level
7. **Hot reload** — SIGHUP or config file watch, re-read watch dirs and settings without restart
8. **Distribution** — CI release workflow, pre-built binaries, install script, Homebrew formula
9. **Soak test** — `angreal soak --mode daemon` with sustained package loading
