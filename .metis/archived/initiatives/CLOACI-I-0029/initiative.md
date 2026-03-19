---
id: server-phase-1-foundation
level: initiative
title: "Server Phase 1: Foundation — cloacinactl serve"
short_code: "CLOACI-I-0029"
created_at: 2026-03-16T01:32:32.511048+00:00
updated_at: 2026-03-16T13:22:27.552358+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: server-phase-1-foundation
---

# Server Phase 1: Foundation — cloacinactl serve Initiative

**Parent tracker**: [[CLOACI-I-0018]]
**Depends on**: Nothing (first phase)
**Blocks**: All subsequent server phases

## Context

First phase of the Cloacina Server initiative (I-0018). Establishes the `cloacinactl serve` subcommand, HTTP server lifecycle, configuration parsing, and the foundation that all subsequent phases build on.

`cloacinactl` already exists with package/key/admin commands (clap-based). This phase adds the `serve` subcommand with mode selection and wires DefaultRunner into the HTTP server lifecycle.

## Goals

- `cloacinactl serve --mode=all|api|worker|scheduler` subcommand
- axum HTTP server with graceful shutdown (SIGTERM/SIGINT)
- TOML configuration file parsing with env var and CLI flag overrides
- `GET /health` endpoint returning version + status
- DefaultRunner wired into serve lifecycle (start background services, stop on shutdown)
- Mode-based service selection: `--mode=api` starts only HTTP, `--mode=worker` starts only executor, `--mode=scheduler` starts only scheduler, `--mode=all` starts everything

## Detailed Design

### Dependencies to add to `cloacinactl/Cargo.toml`:
- `axum` — HTTP framework
- `tower` + `tower-http` — middleware (cors, tracing, timeout)
- `toml` + `serde` — config parsing
- `tokio-signal` — graceful shutdown (or tokio's built-in signal)
- `utoipa` + `utoipa-swagger-ui` — OpenAPI code-first generation

### Config struct (`cloacina.toml`):
```toml
[server]
bind = "0.0.0.0"
port = 8080
mode = "all"  # all | api | worker | scheduler

[database]
url = "postgres://user:pass@localhost:5432/cloacina"
pool_size = 10

[scheduler]
poll_interval_ms = 100
enable_continuous = false
continuous_poll_interval_ms = 100

[worker]
max_concurrent_tasks = 10
task_timeout_seconds = 300

[logging]
level = "info"
format = "json"  # json | pretty
```

### Serve lifecycle:
1. Parse CLI args → load config file → overlay env vars → overlay CLI flags
2. Create Database + DAL
3. Create DefaultRunner with config
4. Start background services (based on mode)
5. Start axum server (if mode includes api)
6. Wait for shutdown signal
7. Stop axum → stop background services → close DB pool

## Implementation Plan

- [ ] Add `serve` subcommand to cloacinactl with `--mode`, `--config`, `--port`, `--bind` flags
- [ ] Add axum, tower, tower-http, toml, utoipa dependencies
- [ ] Config struct with TOML deserialization + env var overlay (e.g., `CLOACINA_DATABASE_URL`)
- [ ] Config file discovery: `--config` flag, or `./cloacina.toml`, or `~/.config/cloacina/cloacina.toml`
- [ ] Server startup/shutdown lifecycle with tokio signal handling
- [ ] Mode-based service selection (which DefaultRunner services to start)
- [ ] `GET /health` endpoint (200 + JSON: version, mode, uptime, status)
- [ ] Integration test: start serve, hit /health, shutdown cleanly
