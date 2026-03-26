---
id: scaffold-v2-cli-metrics-and
level: task
title: "Scaffold v2 CLI, metrics, and reporting — replace v1 structure"
short_code: "CLOACI-T-0257"
created_at: 2026-03-26T02:36:44.502294+00:00
updated_at: 2026-03-26T02:47:39.877291+00:00
parent: CLOACI-I-0046
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0046
---

# Scaffold v2 CLI, metrics, and reporting — replace v1 structure

## Parent Initiative

[[CLOACI-I-0046]]

## Objective

Replace the v1 scheduler-bench structure with a new CLI supporting `daemon` and `server` subcommands, improved MetricCollector with warmup support, and the same table/JSON reporting. This is the foundation that T-0258/T-0259/T-0260 build on.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Delete all files under `src/scenarios/` (v1 scenario modules)
- [ ] New `main.rs` with `daemon`/`server`/`all` subcommands via clap
- [ ] `daemon` subcommand accepts `--scenario`, `--duration`, `--database-url`
- [ ] `server` subcommand accepts `--scenario`, `--duration`, `--base-url`, `--api-key`
- [ ] `metrics.rs` updated: MetricCollector gains `with_warmup(n)` that discards first N samples
- [ ] `reporting.rs` preserved (table + JSON output)
- [ ] New `daemon.rs` and `server.rs` stub files with `pub async fn run()` returning empty results
- [ ] `Cargo.toml` updated: add `reqwest` for server mode, remove `pyo3-build-config` from build-deps (redundant with cloacina-build)
- [ ] `build.rs` simplified to just `cloacina_build::configure()`
- [ ] `cargo check` passes on the new structure

## Implementation Notes

### Files to modify
- `examples/performance/scheduler-bench/Cargo.toml`
- `examples/performance/scheduler-bench/build.rs`
- `examples/performance/scheduler-bench/src/main.rs`
- `examples/performance/scheduler-bench/src/metrics.rs`

### Files to create
- `examples/performance/scheduler-bench/src/daemon.rs`
- `examples/performance/scheduler-bench/src/server.rs`

### Files to delete
- `examples/performance/scheduler-bench/src/scenarios/` (entire directory)

## Status Updates

- 2026-03-26: Deleted `src/scenarios/` directory. Rewrote `main.rs` with `daemon`/`server`/`all` subcommands. Updated `metrics.rs` with `with_warmup()`. Created `daemon.rs` and `server.rs` stubs. Simplified `build.rs`, removed `pyo3-build-config`. Added `reqwest`. `cargo check` passes.
