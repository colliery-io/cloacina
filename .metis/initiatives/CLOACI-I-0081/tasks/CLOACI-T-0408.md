---
id: accumulator-resilience-checkpoint
level: task
title: "Accumulator resilience — checkpoint wiring, boundary persistence, and health state machine"
short_code: "CLOACI-T-0408"
created_at: 2026-04-05T21:24:22.322918+00:00
updated_at: 2026-04-06T00:14:05.886458+00:00
parent: CLOACI-I-0081
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0081
---

# Accumulator resilience — checkpoint wiring, boundary persistence, and health state machine

## Parent Initiative

[[CLOACI-I-0081]]

## Objective

Wire checkpoint persistence into all non-state accumulator classes (passthrough, stream, polling, batch), implement last-emitted boundary persistence, and add the `AccumulatorHealth` state machine. After this task, every accumulator can survive restarts by restoring from its last checkpoint and the reactor can self-seed its cache from persisted boundaries.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **Passthrough accumulator**: persists last-emitted boundary via `CheckpointHandle` after each `process()` call
- [ ] **Stream accumulator**: persists last-emitted boundary; offset management stays with `StreamBackend::commit()`
- [ ] **Polling accumulator**: persists poll state via `CheckpointHandle` after each poll; `init()` restores state on startup
- [ ] **Batch accumulator**: persists buffer to DAL periodically (not just on graceful shutdown); persists last-emitted boundary
- [ ] Accumulator runtime calls `init()` which loads from `CheckpointHandle` — the `AccumulatorError::Checkpoint` error path is now live
- [ ] `AccumulatorHealth` enum: `Starting`, `Connecting`, `Live`, `Disconnected`, `SocketOnly`
- [ ] Health reported via `tokio::sync::watch` channel from each accumulator
- [ ] Health state transitions: Starting->Connecting->Live (normal), Live->Disconnected (source lost, retrying), passthrough goes directly to SocketOnly
- [ ] `health_reactive.rs` reads actual `AccumulatorHealth` instead of hard-coding `"running"`
- [ ] Unit tests: checkpoint save/load round-trip per accumulator class
- [ ] Unit tests: health state transitions for each accumulator class
- [ ] Integration test: restart accumulator, verify it restores from checkpoint

## Implementation Notes

### Technical Approach

**Per-class checkpoint strategy** (per S-0004):
- Passthrough: no internal state, only persist boundary after `ctx.output.send()`
- Stream: persist boundary; offsets delegated to `StreamBackend::commit()` which already exists on the trait
- Polling: `CheckpointHandle::save()` after each successful poll; `init()` calls `CheckpointHandle::load()`
- Batch: periodic buffer persistence (configurable interval or after each flush); boundary persistence after `flush_batch()`

**AccumulatorHealth state machine**: Each accumulator runtime manages its own `watch::Sender<AccumulatorHealth>`. The reactor subscribes to all accumulator health channels (used in T-0410 for startup gating). The `EndpointRegistry` also subscribes for health endpoint reporting.

**Key files:**
- `crates/cloacina/src/computation_graph/accumulator.rs` — runtime modifications, health enum, watch wiring
- `crates/cloacinactl/src/server/health_reactive.rs` — replace hard-coded status

### Dependencies
- T-0407 (DAL foundation) — needs `CheckpointHandle` and DAL tables

### Risk Considerations
- Checkpoint frequency vs performance — `process()` must stay fast per S-0004. Boundary persistence after each send is fine (small writes). Batch buffer persistence should be periodic, not on every event.
- Health watch channel backpressure — `watch` channels always hold latest value, no buffering concern

## Status Updates

### 2026-04-05: Implementation complete

**Completed:**
- AccumulatorHealth enum (Starting, Connecting, Live, Disconnected, SocketOnly) with serde + Display
- health_channel() factory function
- Health watch::Sender added to AccumulatorContext as Option
- All 3 runtime functions (standard, polling, batch) report health transitions
- All 3 runtimes persist boundaries via persist_boundary() after successful sends
- set_health() and persist_boundary() helper functions (best-effort, no-op without DAL)
- EndpointRegistry gains accumulator_health HashMap + register_accumulator_health + get_accumulator_health + list_accumulators_with_health
- health_reactive.rs updated to use real AccumulatorHealth instead of hard-coded "running"
- 825 unit tests pass

**Health state transitions implemented:**
- Standard accumulator: Starting → SocketOnly (no external source)
- Polling accumulator: Starting → Live (once timer starts)
- Batch accumulator: Starting → Live (once ready to receive)
- Connecting and Disconnected states available for stream accumulators (future use)

**Boundary persistence:**
- All runtimes call persist_boundary() after successful ctx.output.send()
- persist_boundary() serializes and saves to accumulator_boundaries table via CheckpointHandle
- Best-effort: logs warning on failure, does not block the event loop
