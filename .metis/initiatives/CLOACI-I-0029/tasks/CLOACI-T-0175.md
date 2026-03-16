---
id: add-axum-http-server-with-graceful
level: task
title: "Add axum HTTP server with graceful shutdown lifecycle"
short_code: "CLOACI-T-0175"
created_at: 2026-03-16T01:35:07.202030+00:00
updated_at: 2026-03-16T01:59:15.013071+00:00
parent: CLOACI-I-0029
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0029
---

# Add axum HTTP server with graceful shutdown lifecycle

## Objective

Set up the axum HTTP server skeleton within the `cloacinactl serve` command, with proper graceful shutdown handling. This provides the foundational HTTP runtime that later tasks will add endpoints and middleware to. No real routes are needed yet -- just the ability to bind, accept connections, and shut down cleanly on SIGTERM/SIGINT.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `axum`, `tower`, and `tower-http` added as dependencies in `crates/cloacinactl/Cargo.toml`
- [ ] An axum `Router` is created (initially empty or with a placeholder `/` returning 200)
- [ ] Server binds to the address and port from `ServerConfig` (from CLOACI-T-0174)
- [ ] Graceful shutdown via `tokio::signal::ctrl_c()` and unix SIGTERM handler using `tokio::signal::unix::signal(SignalKind::terminate())`
- [ ] On startup, server logs: `"Cloacina server listening on {bind}:{port}"`
- [ ] On shutdown signal, server logs: `"Shutdown signal received, stopping server..."` and then `"Server stopped cleanly"`
- [ ] Server exits with code 0 after clean shutdown (no panic, no error)
- [ ] No authentication, no real endpoints -- just the HTTP lifecycle skeleton

## Implementation Notes

Wire the axum server into the `commands::serve::run()` function. Use `tokio::net::TcpListener::bind()` and `axum::serve()` with `.with_graceful_shutdown(shutdown_signal())`. The `shutdown_signal` async function should select on both `ctrl_c` and SIGTERM. Store the `TcpListener` local address after bind so the actual port is known (important for port-0 usage in tests). The Router should be constructed in a separate `fn app() -> Router` function to make it testable independently. Depends on CLOACI-T-0173 (serve subcommand) and CLOACI-T-0174 (config).

## Status Updates

### 2026-03-16 — Completed
- Added `axum` 0.8 dependency
- Implemented full serve lifecycle: bind → listen → graceful shutdown
- `shutdown_signal()` handles both Ctrl+C and SIGTERM (unix-only for SIGTERM)
- `app()` function separated from `run()` for testability
- Port 0 support for tests (OS assigns random available port)
- Logs actual bound address after listen
- Verified: server starts on random port, accepts connections, killed by timeout as expected
- 19 cloacinactl tests still pass
