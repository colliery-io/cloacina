---
id: integration-test-start-serve-hit
level: task
title: "Integration test: start serve, hit /health, shutdown cleanly"
short_code: "CLOACI-T-0179"
created_at: 2026-03-16T01:35:11.771905+00:00
updated_at: 2026-03-16T12:56:01.409610+00:00
parent: CLOACI-I-0029
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0029
---

# Integration test: start serve, hit /health, shutdown cleanly

## Objective

Create an end-to-end integration test that starts the Cloacina server, verifies the health endpoint responds correctly, and confirms the server shuts down cleanly. This validates that the serve command, HTTP server, health endpoint, and graceful shutdown all work together as a cohesive unit.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test spawns `cloacinactl serve --mode=api --port=0` (port 0 lets the OS assign a random available port)
- [ ] Test detects the actual bound port from server startup log output or a readiness mechanism
- [ ] Test sends `GET /health` via `reqwest` (or similar HTTP client) and asserts HTTP 200
- [ ] Response body is valid JSON containing `"status": "ok"`, a `"version"` string, and `"uptime_seconds"` >= 0
- [ ] Test sends SIGTERM to the server process and asserts it exits with code 0
- [ ] No panics or error-level log lines during the entire lifecycle
- [ ] Test has a timeout (e.g., 10 seconds) to avoid hanging if the server fails to start or stop
- [ ] Test is placed in `crates/cloacinactl/tests/` or `crates/cloacina/tests/integration/` following existing test conventions

## Implementation Notes

Two viable approaches: (1) spawn `cloacinactl serve` as a child process via `std::process::Command` / `tokio::process::Command`, capture stdout/stderr, parse the "listening on" log line to extract the port, then use `reqwest::Client` for HTTP assertions and `nix::sys::signal::kill` for SIGTERM; (2) start the server as a tokio task in-process using the same `commands::serve::run()` function with a `CancellationToken` for shutdown. Approach (1) is more realistic but slower; approach (2) is faster and more reliable in CI. Recommend approach (2) as the primary test with approach (1) as an optional smoke test. Use `tokio::time::timeout` to guard against hangs. Depends on all prior tasks: CLOACI-T-0173 through CLOACI-T-0177.

## Status Updates

### 2026-03-16 — Completed
- Used in-process approach: spawn axum server as tokio task with port 0 (OS assigns random port)
- Added `reqwest` dev-dependency for HTTP assertions
- 3 integration tests:
  - `test_serve_health_endpoint_lifecycle` — start server, GET /health (assert 200 + JSON body), GET /api-docs/openapi.json (assert spec has /health path), clean shutdown
  - `test_health_returns_correct_mode` — verify mode field reflects configured mode ("scheduler")
  - `test_unknown_route_returns_404` — verify 404 for non-existent routes
- Safety-net timeout (5s) prevents test hangs
- All 22 cloacinactl tests pass (8 config + 11 pre-existing + 3 new serve)
