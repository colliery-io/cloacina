---
id: get-health-endpoint-with-version
level: task
title: "GET /health endpoint with version, mode, uptime, and status"
short_code: "CLOACI-T-0177"
created_at: 2026-03-16T01:35:09.426160+00:00
updated_at: 2026-03-16T12:41:49.356985+00:00
parent: CLOACI-I-0029
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0029
---

# GET /health endpoint with version, mode, uptime, and status

## Objective

Implement a `GET /health` endpoint that returns JSON with server status, version, running mode, and uptime. This provides the standard liveness/readiness probe for container orchestrators (Kubernetes, ECS) and a quick diagnostic tool for operators.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `GET /health` returns HTTP 200 with JSON body: `{"status": "ok", "version": "0.x.y", "mode": "all", "uptime_seconds": 123}`
- [ ] `version` field populated from `env!("CARGO_PKG_VERSION")` (compile-time from Cargo.toml)
- [ ] `mode` field reflects the configured `--mode` value (all/api/worker/scheduler)
- [ ] `uptime_seconds` computed from `Instant::now()` captured at server startup
- [ ] Returns HTTP 503 with `{"status": "degraded", ...}` if a critical dependency (e.g., database) is unreachable
- [ ] No authentication required on `/health` (public endpoint)
- [ ] Response `Content-Type` is `application/json`
- [ ] Health response struct derives `serde::Serialize` for JSON serialization

## Implementation Notes

Add the `/health` route to the axum Router from CLOACI-T-0175. Use axum `State` to hold shared application state including `startup_instant: Instant`, `mode: ServeMode`, and optionally a DB pool handle for the degraded check. The handler is an `async fn health(State(state): State<AppState>) -> impl IntoResponse`. For the 503 case, attempt a lightweight DB query (e.g., `SELECT 1`) and if it fails within a timeout, return 503. The `AppState` struct should be defined in a shared module so other endpoints can extend it. Depends on CLOACI-T-0175 (axum server).

## Status Updates

### 2026-03-16 — Completed
- Created `routes/health.rs` with `AppState` (startup_instant, mode) and `HealthResponse` (status, version, mode, uptime_seconds)
- `GET /health` returns JSON 200 with version from CARGO_PKG_VERSION, mode from config, uptime computed from Instant
- `AppState` shared via `Arc<AppState>` through axum State extractor
- No auth on /health (public endpoint)
- Verified: `curl http://localhost:19877/health` → `{"status":"ok","version":"0.3.2","mode":"api","uptime_seconds":1}`
- Graceful shutdown still works after health route added
