---
id: accumulator-reactor-embedded
level: initiative
title: "Accumulator & Reactor — Embedded Vertical Slice"
short_code: "CLOACI-I-0074"
created_at: 2026-04-04T17:55:31.160558+00:00
updated_at: 2026-04-04T23:33:35.174978+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: accumulator-reactor-embedded
---

# Accumulator & Reactor — Embedded Vertical Slice Initiative

## Context

Second implementation initiative for CLOACI-I-0069. Wires the computation graph macro from I-0070 to accumulators and a reactor, completing the embedded vertical slice: data flows from source through accumulator through reactor through compiled graph to terminal node output.

Everything wired in code in a single binary — no WebSocket, no reactive scheduler, no packaging, no API server. Embedded mode, like `#[workflow]` before packaged deployment.

Blocked by: CLOACI-I-0070 (computation graph macro must work first).

Specs: CLOACI-S-0004 (Accumulator), CLOACI-S-0005 (Reactor).

## Goals & Non-Goals

**Goals:**
- Implement the `Accumulator` trait and runtime (3-task merge channel model: event loop, socket receiver, processor)
- Implement `#[stream_accumulator]` macro with Kafka backend
- Implement `#[passthrough_accumulator]` macro (socket-only)
- Implement the `StreamBackend` trait and `StreamBackendRegistry` factory
- Implement `KafkaBackend` as the first stream backend
- Implement the Reactor: receiver task + executor task, `Arc<RwLock<InputCache>>`, dirty flags
- Implement `when_any` reaction criteria
- Implement `latest` input strategy
- Wire accumulators → reactor → compiled graph (from I-0070) in a working example binary
- Example runnable with `angreal demos computation-graph`
- Unit tests for accumulator and reactor, integration test for end-to-end flow

**Non-Goals:**
- WebSocket / API server (I-0071)
- Reactive scheduler / reconciler (I-0071)
- Auth (I-0071)
- State accumulator, batch accumulator, polling accumulator (I-0072/I-0073)
- DAL persistence (I-0072)
- Health states (I-0072)
- Python bindings (I-0073)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `Accumulator` trait defined with `process()`, `run()`, `init()`
- [ ] Accumulator runtime spawns 3 tasks (event loop, socket receiver, processor) connected by merge channel
- [ ] `process()` called sequentially — no concurrent access to `&mut self`
- [ ] Event loop and socket receiver run independently, never block each other
- [ ] `#[stream_accumulator]` macro generates stream-backed accumulator
- [ ] `#[passthrough_accumulator]` macro generates socket-only accumulator
- [ ] `StreamBackend` trait defined with `connect()`, `recv()`, `commit()`, `current_offset()`
- [ ] `KafkaBackend` implements `StreamBackend`
- [ ] `StreamBackendRegistry` with factory pattern, `register()` and `create()`
- [ ] Accumulator serializes boundaries via bincode (release) / JSON (debug) before sending to reactor
- [ ] Reactor receiver task: reads from accumulator channel, updates cache, sets dirty flags
- [ ] Reactor executor task: reads from strategy channel, snapshots cache, calls `graph.execute()`, clears dirty flags
- [ ] `when_any` reaction criteria: fire if any dirty flag set
- [ ] `latest` input strategy: cache overwritten on each boundary, intermediate values collapsed
- [ ] Working example: 2 accumulators (stream + passthrough) → reactor → compiled graph with routing → terminal output
- [ ] Example uses mock Kafka or test fixture (no real broker required for CI)
- [ ] Integration test: push events → accumulators process → reactor fires → graph executes → correct terminal outputs
- [ ] All existing tests continue to pass

## Implementation Plan

1. **Accumulator trait** — trait definition, `AccumulatorContext`, `BoundarySender` with dual-format serialization
2. **Accumulator runtime** — 3-task spawn, merge channel, shutdown handling
3. **Stream backend** — `StreamBackend` trait, `StreamConfig`, `RawMessage`, `StreamBackendRegistry`
4. **Kafka backend** — `KafkaBackend` implementation (+ mock backend for testing)
5. **Accumulator macros** — `#[stream_accumulator]` and `#[passthrough_accumulator]` code generation
6. **Reactor** — receiver task, executor task, `InputCache`, `DirtyFlags`, strategy channel
7. **Wire it** — example binary: spawn accumulators, spawn reactor, connect channels, run
8. **Angreal task** — `angreal demos computation-graph`
9. **Tests** — unit tests per component, integration test end-to-end
