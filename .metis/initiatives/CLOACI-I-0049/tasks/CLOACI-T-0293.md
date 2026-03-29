---
id: axum-server-health-endpoints
level: task
title: "axum server + health endpoints — cloacinactl serve, Postgres init, /health /ready /metrics"
short_code: "CLOACI-T-0293"
created_at: 2026-03-29T14:03:25.518171+00:00
updated_at: 2026-03-29T14:03:25.518171+00:00
parent: CLOACI-I-0049
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0049
---

# axum server + health endpoints — cloacinactl serve, Postgres init, /health /ready /metrics

## Parent Initiative

[[CLOACI-I-0049]]

## Objective

Add `cloacinactl serve` subcommand that starts an axum HTTP server backed by Postgres. Serves health/ready/metrics endpoints. This is the foundation all other API tasks build on.

## Acceptance Criteria

- [ ] `cloacinactl serve` subcommand with `--bind` (default `0.0.0.0:8080`), `--database-url` (from config or CLI)
- [ ] Connects to Postgres, runs migrations on startup
- [ ] `GET /health` — returns 200 `{"status":"ok"}` (no auth, no DB check)
- [ ] `GET /ready` — returns 200 if DB is reachable, 503 otherwise (no auth)
- [ ] `GET /metrics` — Prometheus text format (no auth)
- [ ] Graceful shutdown on SIGINT/SIGTERM (reuse daemon patterns)
- [ ] JSON error responses for 404/500
- [ ] `axum` + `tower` dependencies added to cloacinactl
- [ ] Smoke test: start server, curl health, curl ready

## Implementation Notes

### Files to create/modify
- `crates/cloacinactl/src/commands/serve.rs` — new file, server startup
- `crates/cloacinactl/src/main.rs` — add `Serve` variant to `Commands`
- `crates/cloacinactl/Cargo.toml` — add `axum` (with `multipart` feature), `tower`, `tower-http`, `prometheus` deps

### Key design points
- `database_url` resolved via `resolve_database_url()` (CLI → config → error)
- `DefaultRunner::with_config()` with Postgres backend
- axum `Router` with health/ready/metrics routes
- Bind to `0.0.0.0` by default (archive learning)
- Metrics via `prometheus` crate — basic counters for requests, latency histogram

### Depends on
- Nothing — first task

## Status Updates

*To be added during implementation*
