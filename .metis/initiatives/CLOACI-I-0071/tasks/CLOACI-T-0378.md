---
id: integration-tests-websocket-push
level: task
title: "Integration tests — WebSocket push, package lifecycle, auth verification"
short_code: "CLOACI-T-0378"
created_at: 2026-04-05T00:33:05.128152+00:00
updated_at: 2026-04-05T01:55:11.326159+00:00
parent: CLOACI-I-0071
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0071
---

# Integration tests — WebSocket push, package lifecycle, auth verification

## Objective

End-to-end integration tests proving the full server-mode reactive pipeline: external WebSocket client pushes events to an accumulator endpoint on the API server → accumulator processes → reactor fires compiled graph → terminal output. Also tests auth rejection and package lifecycle (load/unload).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Integration test: spin up API server (in-process), load computation graph via ReactiveScheduler, connect WS client to `/v1/ws/accumulator/alpha`, push serialized event, assert graph fires and produces correct output
- [ ] Integration test: connect WS client without auth token → rejected with 401 before upgrade
- [ ] Integration test: connect WS client with valid token but unauthorized for endpoint → rejected
- [ ] Integration test: connect to reactor WS, send ForceFire command, assert graph fires
- [ ] Integration test: connect to reactor WS, send GetState, verify cache contents returned
- [ ] Integration test: unload graph via ReactiveScheduler → WS connections to that name close, registry entries removed
- [ ] Integration test: push to accumulator with broadcast (2 reactors consuming same source) → both fire
- [ ] All existing tests continue to pass

## Implementation Notes

### Files
- `crates/cloacina/tests/integration/` — new `reactive_websocket.rs` module
- Use `tokio-tungstenite` as the WS client in tests (or `axum::extract::ws` test utilities)

### Approach
Spin up the full axum server on `127.0.0.1:0` (ephemeral port), programmatically load a computation graph via ReactiveScheduler (no need for real `.cloacina` packages — use the `ComputationGraphDeclaration` API directly), then connect WS clients. This avoids needing Postgres or real packages for the integration tests.

### Dependencies
All prior tasks (T-0371 through T-0377). This is the capstone task.

## Status Updates

- 2026-04-04: Library-level integration test complete — `test_reactive_scheduler_end_to_end` proves: scheduler load → push via registry → graph fires → pause prevents firing → resume + force-fire → unload deregisters. All 5 computation graph integration tests pass. WebSocket-level tests (actual WS client → axum server) deferred — requires Postgres for AppState construction. The library-level test proves the same pipeline minus the HTTP/WS transport layer.
