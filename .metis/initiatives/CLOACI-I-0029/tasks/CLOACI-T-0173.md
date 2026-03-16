---
id: add-serve-subcommand-to
level: task
title: "Add serve subcommand to cloacinactl with mode selection"
short_code: "CLOACI-T-0173"
created_at: 2026-03-16T01:35:04.565524+00:00
updated_at: 2026-03-16T01:52:24.660831+00:00
parent: CLOACI-I-0029
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0029
---

# Add serve subcommand to cloacinactl with mode selection

## Objective

Add a `serve` subcommand to the existing `cloacinactl` CLI that will become the entry point for running the Cloacina server. The subcommand accepts a `--mode` flag to select which services to start, plus configuration flags for bind address, port, and config file path. This is the foundational CLI surface for the server -- actual service startup is handled in later tasks.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `Serve` variant added to the `Commands` enum in `main.rs` following existing clap patterns (derive-based `#[derive(Subcommand)]`)
- [ ] `--mode` flag accepts values: `all` (default), `api`, `worker`, `scheduler` -- implemented as a clap `ValueEnum`
- [ ] `--config` flag accepts an optional path to a TOML config file
- [ ] `--port` flag accepts a `u16` (default 8080)
- [ ] `--bind` flag accepts a string (default `"0.0.0.0"`)
- [ ] `cloacinactl serve --help` displays all flags with descriptions
- [ ] `cloacinactl serve` executes without panic (stub implementation that logs "Starting cloacina server in {mode} mode on {bind}:{port}" and exits cleanly)
- [ ] Match arm added in `main()` for `Commands::Serve { .. }` that delegates to a new `commands::serve` module

## Implementation Notes

The existing CLI uses `clap` derive macros with `#[derive(Parser)]` / `#[derive(Subcommand)]`. Add a `ServeMode` enum with `#[derive(clap::ValueEnum)]` for the mode flag. The serve handler should live in a new `commands/serve.rs` module. For now the handler just prints startup info and returns `Ok(())` -- the actual axum server and DefaultRunner wiring come in CLOACI-T-0175 and CLOACI-T-0176 respectively.

Key references: `crates/cloacinactl/src/main.rs` (CLI structure), `crates/cloacinactl/src/commands/` (command modules).

## Status Updates

### 2026-03-16 — Completed
- Created `commands/serve.rs` with `ServeMode` enum (ValueEnum), `ServeArgs` struct, and stub `run()` function
- Added `Serve(ServeArgs)` variant to `Commands` enum in main.rs
- Added match arm delegating to `commands::serve::run()`
- `cloacinactl serve --help` shows all flags: --mode (all/api/worker/scheduler), --config, --bind, --port
- `cloacinactl serve` runs cleanly with info logs and exits
- Compiles clean, all pre-existing warnings only
