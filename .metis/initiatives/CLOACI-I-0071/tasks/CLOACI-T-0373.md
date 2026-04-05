---
id: accumulator-websocket-bridge
level: task
title: "Accumulator WebSocket bridge — external producer to accumulator socket"
short_code: "CLOACI-T-0373"
created_at: 2026-04-05T00:32:58.923614+00:00
updated_at: 2026-04-05T01:37:13.305495+00:00
parent: CLOACI-I-0071
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0071
---

# Accumulator WebSocket bridge — external producer to accumulator socket

## Objective

Wire the accumulator WebSocket handler (from T-0371) to the endpoint registry (from T-0372). When an external producer sends a binary/text message on the WebSocket, the handler looks up the accumulator name in the registry and forwards the raw bytes to all registered accumulators via their socket channels. This is the external-facing write path for pushing events into accumulators.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Accumulator WS handler (`/v1/ws/accumulator/{name}`) reads incoming messages in a loop
- [ ] Each message forwarded to `EndpointRegistry::send_to_accumulator(name, bytes)`
- [ ] Binary messages forwarded as-is; text messages forwarded as UTF-8 bytes
- [ ] If no accumulator registered for `name`, WebSocket closes with appropriate error frame
- [ ] Handles client disconnect gracefully (log, clean up)
- [ ] Handles accumulator channel full/closed (log warning, don't crash the handler)
- [ ] Test: push bytes via WS → accumulator socket_rx receives them
- [ ] Test: push to unregistered name → WS closes with error

## Implementation Notes

### Files
- `crates/cloacinactl/src/server/ws.rs` — fill in accumulator handler stub from T-0371

### Flow
```
WebSocket client → axum WS handler → EndpointRegistry.send_to_accumulator(name, bytes)
    → mpsc::Sender<Vec<u8>> (accumulator's socket_rx) → accumulator_runtime processes it
```

### Dependencies
T-0371 (WS routes), T-0372 (registry)

## Status Updates

- 2026-04-04: Complete. Handler forwards binary/text messages via `EndpointRegistry::send_to_accumulator`. Unregistered name closes WS with code 4404. EndpointRegistry added to AppState. Full-stack integration tests deferred to T-0378.
