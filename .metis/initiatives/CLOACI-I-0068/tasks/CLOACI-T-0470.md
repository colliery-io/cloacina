---
id: integration-test-server-startup
level: task
title: "Integration test: server startup → HTTP health check end-to-end (ConnectInfo, rate limiter interaction)"
short_code: "CLOACI-T-0470"
created_at: 2026-04-10T12:45:35.437647+00:00
updated_at: 2026-04-10T12:45:35.437647+00:00
parent: CLOACI-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0068
---

# Integration test: server startup → HTTP health check end-to-end (ConnectInfo, rate limiter interaction)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0068]]

## Objective

The `build_router` function in `serve.rs` uses `axum::serve(listener, app)` which must provide `ConnectInfo<SocketAddr>` for any middleware that needs peer IP. Previously `tower_governor` rate limiting needed this and returned 500 "Unable To Extract Key" on every request when it was missing. Rate limiting has been removed, but this test ensures the server accepts HTTP requests end-to-end after startup.

**Bug:** `axum::serve` was called without `into_make_service_with_connect_info::<SocketAddr>()`. Any layer needing peer IP caused 500.
**Fix:** Added `into_make_service_with_connect_info`. Rate limiter removed entirely (infrastructure concern, not app concern).

**Note:** The existing `angreal cloacina server-soak` test covers this path, but there's no fast unit/integration test. This task adds one to `serve.rs` tests.

## Acceptance Criteria

- [ ] `build_router` test sends GET /health and receives 200 `{"status":"ok"}`
- [ ] Test uses `axum::test` or tower `ServiceExt` (no real TCP needed)
- [ ] Test verifies /ready returns 200 or 503 (not 500)

## Files

- `crates/cloacinactl/src/commands/serve.rs` — existing tests section

## Status Updates

*To be added during implementation*
