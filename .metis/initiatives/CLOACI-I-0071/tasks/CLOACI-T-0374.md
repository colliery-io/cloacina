---
id: reactor-websocket-bridge-manual
level: task
title: "Reactor WebSocket bridge — manual commands and responses"
short_code: "CLOACI-T-0374"
created_at: 2026-04-05T00:33:00.050288+00:00
updated_at: 2026-04-05T01:42:44.166243+00:00
parent: CLOACI-I-0071
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0071
---

# Reactor WebSocket bridge — manual commands and responses

## Objective

Wire the reactor WebSocket handler to the endpoint registry and reactor's `manual_rx` channel. Operators connect via WebSocket and send JSON commands (ForceFire, FireWith, GetState, Pause, Resume). The handler deserializes commands, sends them to the reactor, and returns responses. This requires extending the Reactor to support a response channel for request/reply semantics.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Define `ReactorCommand` and `ReactorResponse` JSON-serializable enums (as specified in S-0007)
- [ ] Reactor WS handler (`/v1/ws/reactor/{name}`) reads incoming text messages, deserializes as `ReactorCommand`
- [ ] `ForceFire` → sends `ManualCommand::ForceFire` to reactor, returns `ReactorResponse::Fired`
- [ ] `FireWith { cache }` → sends `ManualCommand::FireWith(cache)`, returns `ReactorResponse::Fired`
- [ ] `GetState` → reads current cache snapshot, returns `ReactorResponse::State` (JSON-serialized cache)
- [ ] `Pause` / `Resume` → sets/clears reactor pause flag, returns ack
- [ ] Invalid command → returns `ReactorResponse::Error`
- [ ] Extend `Reactor` to expose cache read access for `GetState` (e.g., via `Arc<RwLock<InputCache>>` handle)
- [ ] Extend `Reactor` to support pause/resume (skip graph execution while paused, continue accepting boundaries)
- [ ] Test: send ForceFire via WS → reactor fires → response received
- [ ] Test: send GetState → returns current cache contents

## Implementation Notes

### Files
- `crates/cloacinactl/src/server/ws.rs` — reactor handler
- `crates/cloacina/src/computation_graph/reactor.rs` — add pause/resume, expose cache handle
- New types in `crates/cloacina/src/computation_graph/types.rs` or reactor module

### Design
- `GetState` doesn't go through `manual_rx` — it reads the shared `Arc<RwLock<InputCache>>` directly
- `Pause`/`Resume` use an `AtomicBool` flag checked before graph execution
- `ForceFire`/`FireWith` use the existing `ManualCommand` channel

### Dependencies
T-0371 (WS routes), T-0372 (registry)

## Status Updates

- 2026-04-04: Complete. Added ReactorCommand/ReactorResponse JSON enums, ReactorHandle (shared cache + AtomicBool pause), wired WS handler to dispatch all 5 commands. Reactor executor checks pause flag before firing. EndpointRegistry extended with reactor_handles + get_reactor_handle(). All existing tests pass.
