---
id: t1-extract-cloacina-server-binary
level: task
title: "T1: Extract cloacina-server binary from cloacinactl serve"
short_code: "CLOACI-T-0510"
created_at: 2026-04-17T17:00:00+00:00
updated_at: 2026-04-18T01:40:04.032057+00:00
parent: CLOACI-I-0098
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0098
---

# T1: Extract cloacina-server binary from cloacinactl serve

## Parent Initiative

CLOACI-I-0098 — cloacinactl CLI redesign

## Objective

Extract the HTTP API server code currently wired up under `cloacinactl serve` into its own binary named `cloacina-server`. Behavior is identical — this is pure packaging. Downstream tasks (T2 + T-0501) depend on `cloacina-server` existing as a separable artifact.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New crate `crates/cloacina-server/` (or new `[[bin]]` target on an existing crate) produces a `cloacina-server` binary.
- [ ] Binary accepts the same flags today's `cloacinactl serve` accepts: `--bind`, `--database-url`, `--bootstrap-key`, `--require-signatures`. Env var bindings preserved.
- [ ] `cloacinactl serve` source is removed from `crates/cloacinactl/src/commands/` (the noun-verb replacement comes in T2).
- [ ] `angreal check all-crates` passes.
- [ ] `angreal cloacina server-soak` (short run) passes against the new binary.
- [ ] `cloacinactl` continues to build. Users run `cloacina-server` directly until T2 restores `cloacinactl server start`.

## Implementation Notes

### Code to move

From `crates/cloacinactl/src/`:
- `commands/serve.rs` — becomes `cloacina-server`'s `main.rs`.
- `server/` (whole subdirectory) — HTTP handlers, middleware, websocket. Moves wholesale.

HTTP-server-only deps (axum, tower, etc.) move to the new crate.

### Keep on cloacinactl

Only CLI-client-facing code stays:
- `commands/daemon.rs`, `commands/health.rs`, `commands/status.rs`, `commands/config.rs`, `commands/watcher.rs`, `commands/cleanup_events.rs`
- `main.rs` (rewritten in T2)

### Binary naming and workspace wiring

- `crates/cloacina-server/Cargo.toml` with `[[bin]] name = "cloacina-server"`.
- Register in the workspace `Cargo.toml` members list.
- Pin to workspace version.

### Angreal integration

Update angreal tasks that spawn `cloacinactl serve` to run `cloacina-server`. Check `services/`, `cloacina/server-soak`, demo scripts.

## Status Updates

*To be added during implementation*
