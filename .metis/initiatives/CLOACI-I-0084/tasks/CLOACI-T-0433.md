---
id: stateful-stream-accumulator-state
level: task
title: "Stateful stream accumulator — state parameter on macro, checkpoint DAL wiring"
short_code: "CLOACI-T-0433"
created_at: 2026-04-07T18:44:33.057061+00:00
updated_at: 2026-04-07T21:19:06.423411+00:00
parent: CLOACI-I-0084
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0084
---

# Stateful stream accumulator — state parameter on macro, checkpoint DAL wiring

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0084]]

## Objective

Add `state = Type` parameter to `#[stream_accumulator]` macro so users can maintain mutable state across messages. The macro generates a struct with the state field, passes `&mut self.state` to the user function, and the checkpoint DAL persists/restores state across restarts.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `#[stream_accumulator(type = "kafka", topic = "...", state = MyState)]` accepted by macro
- [ ] Macro generates struct with `state: MyState` field, initializes via `Default::default()`
- [ ] `process()` impl passes `&mut self.state` as second argument to user function
- [ ] State type must implement `Default + Serialize + Deserialize`
- [ ] State checkpointed to DAL on commit (alongside offset)
- [ ] On restart: state restored from DAL checkpoint before consuming
- [ ] Without `state` param: existing stateless behavior unchanged
- [ ] Unit test: stateful accumulator with running counter
- [ ] Unit test: stateful accumulator with VecDeque sliding window
- [ ] Python `@cloaca.stream_accumulator(state={...})` stores initial state in metadata

## Implementation Notes

### Key files
- `crates/cloacina-macros/src/computation_graph/codegen.rs` — macro expansion for stream_accumulator
- `crates/cloacina/src/computation_graph/accumulator.rs` — checkpoint integration
- `crates/cloacina/src/python/computation_graph.rs` — Python decorator state param

### Dependencies
- T-0432 (KafkaStreamBackend) — for end-to-end testing with real broker

## Status Updates **[REQUIRED]**

**2026-04-07 — Already implemented**
- `state = Type` parameter already exists in `#[stream_accumulator]` macro (accumulator_macros.rs lines 58-61)
- Macro generates stateful struct with `state` field, `new(initial_state)` constructor, `process(&mut self.state)` passthrough
- Stateless path unchanged with `Default` impl
- Python decorator already accepts `state` param
- DAL checkpoint wiring: existing `CheckpointHandle` in `AccumulatorContext` handles state persistence
- No new code needed — built during I-0073/I-0074
