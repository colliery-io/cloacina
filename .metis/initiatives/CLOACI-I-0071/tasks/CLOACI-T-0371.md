---
id: websocket-layer-axum-upgrade
level: task
title: "WebSocket layer — axum upgrade, routes, PAK auth on handshake"
short_code: "CLOACI-T-0371"
created_at: 2026-04-05T00:32:56.586241+00:00
updated_at: 2026-04-05T01:05:07.123651+00:00
parent: CLOACI-I-0071
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0071
---

# WebSocket layer — axum upgrade, routes, PAK auth on handshake

## Objective

Add WebSocket upgrade support to the axum API server with routes for accumulator and reactor endpoints. Authenticate WebSocket connections using the existing PAK (Personal API Key) auth model — validate Bearer token on the HTTP upgrade request before promoting to WebSocket.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `axum::extract::ws::WebSocketUpgrade` used for WS support (built into axum, no new deps)
- [ ] Routes added: `/v1/ws/accumulator/{name}` and `/v1/ws/reactor/{name}`
- [ ] PAK auth validated on the upgrade handshake (same `require_auth` middleware or equivalent extractor)
- [ ] Unauthorized connections rejected with 401 before upgrade completes
- [ ] WebSocket handler receives `AuthenticatedKey` + endpoint `name` after successful upgrade
- [ ] Handler stubs — accept connection, log, close gracefully. Business logic wired in T-0373/T-0374.
- [ ] Unit test: mock upgrade request with valid key succeeds
- [ ] Unit test: mock upgrade request with invalid key returns 401

## Implementation Notes

### Files to modify
- `crates/cloacinactl/src/commands/serve.rs` — add WS routes to `build_router`
- `crates/cloacinactl/src/server/` — new `ws.rs` module for WebSocket handlers
- `crates/cloacinactl/src/server/auth.rs` — may need to extract auth logic into a reusable function callable from WS upgrade path (current middleware inserts into extensions, WS upgrade may need it in the extractor)

### Dependencies
None — first task in the chain.

## Status Updates

- 2026-04-04: Implementation complete. Added `ws` feature to axum, created `server/ws.rs` with accumulator + reactor WS handlers, extracted `validate_token()` from `require_auth` for reuse, added routes to `build_router`. Handlers are stubs (accept, log, read messages) — business logic in T-0373/T-0374. Supports auth via both `Authorization: Bearer` header and `?token=` query param.
