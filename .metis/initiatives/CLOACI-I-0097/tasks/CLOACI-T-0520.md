---
id: t2-extract-cloacina-compiler
level: task
title: "T2: Extract cloacina-compiler binary + library crate"
short_code: "CLOACI-T-0520"
created_at: 2026-04-18T01:50:00+00:00
updated_at: 2026-04-18T01:50:00+00:00
parent: CLOACI-I-0097
blocked_by: [CLOACI-T-0519]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0097
---

# T2: Extract cloacina-compiler binary + library crate

## Parent Initiative

CLOACI-I-0097 — Compiler Service

## Objective

Create the `cloacina-compiler` crate: library with the service's main loop hookpoint, plus a binary `main.rs` that parses flags and starts the service. This task only stubs the loop body — T3 fills in the actual build execution. Gives T3/T4/T9 a real target to link against.

## Acceptance Criteria

- [ ] New crate `crates/cloacina-compiler/` with:
  - `Cargo.toml` — workspace member, dep on `cloacina` with `postgres`+`sqlite` features.
  - `src/lib.rs` — exposes `run(config: CompilerConfig) -> Result<()>`; a placeholder loop that polls the build queue, prints "claimed row N" for each claim, and immediately marks success with an empty `compiled_data` (real logic lands in T3).
  - `src/main.rs` — clap entry point that parses flags + env vars (DATABASE_URL, poll/heartbeat/sweep intervals, bind addr), builds a `CompilerConfig`, calls `cloacina_compiler::run(...)`.
  - Local `/health` HTTP endpoint on `--bind` (default `127.0.0.1:9000`) for T7's status/health verbs. Returns `200 {"status":"ok"}`.
- [ ] Workspace `Cargo.toml` gains `crates/cloacina-compiler` in the members list.
- [ ] `cargo build -p cloacina-compiler` produces the binary.
- [ ] `cloacina-compiler --help` shows the flag surface defined in the spec:
  ```
  --verbose, --home PATH, --database-url URL, --bind ADDR,
  --poll-interval-ms N, --heartbeat-interval-s N, --stale-threshold-s N,
  --sweep-interval-s N, --cargo-flag <FLAG>...
  ```
- [ ] `angreal check all-crates` passes.

## Implementation Notes

### Crate layout

Mirror `cloacina-server` from I-0098:
```
crates/cloacina-compiler/
  Cargo.toml
  src/
    main.rs   — clap + tokio main, builds Config, calls run()
    lib.rs    — pub fn run(config: CompilerConfig) -> Result<()>
    config.rs — CompilerConfig struct + file loader
    health.rs — tiny axum /health router, runs in background
    loop.rs   — build loop placeholder (stubbed; T3 fills in)
```

### Shared types

All DB interaction goes through `cloacina::registry::workflow_registry` DAL helpers from T1. No direct SQL in this crate.

### Logging

Mirror `cloacinactl daemon` — file logging to `$home/logs/compiler.log.YYYY-MM-DD` + stderr. Tracing spans on claim/build/success/failure.

### Feature flags

`default = ["postgres"]`; `postgres` and `sqlite` both forward to the same `cloacina` feature.

### Stub loop behavior

For this task, `run()` ticks every `poll_interval_ms`, calls `dal.claim_next_build()` (from T1). If it gets a row, logs the claim and immediately calls `dal.mark_build_success(id, &[])` with empty bytes. This proves the wiring; T3 replaces the body with real compilation.

## Status Updates

*To be added during implementation*
