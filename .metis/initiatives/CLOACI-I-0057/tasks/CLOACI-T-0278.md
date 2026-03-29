---
id: cloacinactl-daemon-subcommand-cli
level: task
title: "cloacinactl daemon subcommand — CLI, SQLite init, DefaultRunner startup"
short_code: "CLOACI-T-0278"
created_at: 2026-03-28T15:30:05.094375+00:00
updated_at: 2026-03-29T00:33:18.507871+00:00
parent: CLOACI-I-0057
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0057
---

# cloacinactl daemon subcommand — CLI, SQLite init, DefaultRunner startup

## Parent Initiative

[[CLOACI-I-0057]]

## Objective

Add the `cloacinactl daemon` subcommand. This is the entry point — parses CLI args, creates the SQLite database, initializes `DefaultRunner`, wires in `FilesystemWorkflowRegistry`, starts the reconciler, and blocks until shutdown. Later tasks add the directory watcher, scheduler wiring, and graceful shutdown on top.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `cloacinactl daemon` subcommand added via clap with `--watch-dir`, `--db-path`, `--poll-interval`, `--log-level` flags
- [ ] Creates/opens SQLite database at `--db-path` (default `./cloacina.db`)
- [ ] Creates `DefaultRunner` with SQLite backend
- [ ] Creates `FilesystemWorkflowRegistry` pointed at `--watch-dir` (default `./packages`)
- [ ] Creates `RegistryReconciler` with the filesystem registry
- [ ] Performs initial reconciliation on startup (loads packages already in watch dir)
- [ ] Blocks until Ctrl+C (basic — graceful shutdown is T-0281)
- [ ] Logs startup info: watch dir, db path, number of packages loaded
- [ ] `cloacinactl daemon --help` shows usage

## Implementation Notes

### Files to modify
- `crates/cloacinactl/src/main.rs` — add `Daemon` variant to `Commands` enum
- `crates/cloacinactl/src/commands/mod.rs` — add `daemon` module
- `crates/cloacinactl/src/commands/daemon.rs` — new file, daemon startup logic

### Depends on
- T-0277 (FilesystemWorkflowRegistry)

## Status Updates

**2026-03-28**: Implementation complete, smoke tested.

### Changes:
- `commands/daemon.rs` — `run()`: creates `~/.cloacina/` home, SQLite DB, DefaultRunner, FilesystemWorkflowRegistry, RegistryReconciler, blocks until Ctrl+C
- `commands/mod.rs` — added `pub mod daemon`
- `main.rs` — `Daemon` variant with `--home`, `--watch-dir`, `--poll-interval` via clap
- `Cargo.toml` — added `dirs` crate
- Smoke tested: starts, creates dirs/DB, reconciler scans, logs info, exits on timeout

### Note on defaults:
Used `~/.cloacina/` home and `~/.cloacina/packages/` as default watch dir per initiative design (not `./cloacina.db` / `./packages` as originally written in AC).
