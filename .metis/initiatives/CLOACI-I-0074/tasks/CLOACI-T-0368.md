---
id: streambackend-trait-mock-backend
level: task
title: "StreamBackend trait, mock backend, and accumulator macros"
short_code: "CLOACI-T-0368"
created_at: 2026-04-04T22:54:47.730216+00:00
updated_at: 2026-04-04T22:54:47.730216+00:00
parent: CLOACI-I-0074
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0074
---

# StreamBackend trait, mock backend, and accumulator macros

## Objective

Implement the `StreamBackend` trait for pluggable broker backends, a mock backend for testing (no real Kafka needed in CI), the `StreamBackendRegistry` factory, and the `#[stream_accumulator]` and `#[passthrough_accumulator]` proc macros.

Spec: CLOACI-S-0004.

## Acceptance Criteria

- [ ] `StreamBackend` trait: `connect()`, `recv()`, `commit()`, `current_offset()`
- [ ] `StreamConfig` struct: broker_url, topic, group, extra config
- [ ] `RawMessage` struct: payload bytes, offset, timestamp
- [ ] `StreamBackendRegistry`: `register()` and `create()` factory pattern
- [ ] `MockBackend` implementing `StreamBackend` — in-memory mpsc channel that simulates a broker. Push messages in, `recv()` returns them. For testing without real Kafka.
- [ ] `#[stream_accumulator(type = "mock", topic = "...")]` macro generates a stream-backed accumulator using the registry
- [ ] `#[passthrough_accumulator]` macro generates a socket-only accumulator (no event loop, no run() override)
- [ ] Both macros generate structs implementing the `Accumulator` trait from T-0367
- [ ] Stateful `#[stream_accumulator(..., state = T)]` variant generates `init()` + checkpoint restore
- [ ] Unit tests: mock backend recv/commit, registry lookup, macro-generated accumulator compiles and processes events

### Dependencies
T-0367 (Accumulator trait + runtime).

## Status Updates

*To be added during implementation*
