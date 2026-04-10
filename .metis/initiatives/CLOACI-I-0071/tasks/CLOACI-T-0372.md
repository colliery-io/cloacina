---
id: endpoint-registry-name-to-channel
level: task
title: "Endpoint registry — name-to-channel mapping with broadcast"
short_code: "CLOACI-T-0372"
created_at: 2026-04-05T00:32:57.808395+00:00
updated_at: 2026-04-05T01:09:26.339387+00:00
parent: CLOACI-I-0071
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0071
---

# Endpoint registry — name-to-channel mapping with broadcast

## Objective

Build an in-process registry that maps accumulator/reactor names to their mpsc channel senders. The WebSocket handlers (T-0371) look up names in this registry to route messages to the correct process. Supports broadcast — multiple accumulators registered under the same name all receive the message (fan-out for multiple reactors consuming the same source).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `EndpointRegistry` struct with `Arc<RwLock<...>>` interior (shared across handlers)
- [ ] `register_accumulator(name, sender)` — registers an accumulator's socket sender under a name
- [ ] `register_reactor(name, manual_tx)` — registers a reactor's manual command sender
- [ ] `deregister(name)` — removes a registration (on shutdown/swap)
- [ ] `send_to_accumulator(name, bytes)` — looks up all senders for `name`, broadcasts to all. Returns error if none registered.
- [ ] `send_to_reactor(name, command)` — sends ManualCommand to reactor. Returns error if not registered.
- [ ] `list_accumulators()` / `list_reactors()` — for health endpoints
- [ ] Broadcast: two accumulators registered as "alpha" both receive when a message is sent to "alpha"
- [ ] Unit test: register, send, deregister lifecycle
- [ ] Unit test: broadcast to multiple same-name accumulators
- [ ] Unit test: send to unregistered name returns error

## Implementation Notes

### Files
- `crates/cloacina/src/computation_graph/registry.rs` — new module
- Add to `crates/cloacina/src/computation_graph/mod.rs`

### Design
- Accumulator entries: `HashMap<String, Vec<mpsc::Sender<Vec<u8>>>>` (Vec for broadcast)
- Reactor entries: `HashMap<String, mpsc::Sender<ManualCommand>>` (single reactor per name)
- The registry lives in `AppState` and is passed to both the Reactive Scheduler (for registration) and the WebSocket handlers (for lookup)

### Dependencies
T-0371 (WebSocket layer provides the handlers that consume this registry)

## Status Updates

- 2026-04-04: Implementation complete. `EndpointRegistry` with `Arc<RwLock<RegistryInner>>`, broadcast via `Vec<Sender>` for accumulators, closed channel pruning on send, 7 unit tests all passing.
