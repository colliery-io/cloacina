---
id: reactor-resilience-cache
level: task
title: "Reactor resilience — cache persistence, startup DAL loading, health state machine with startup gating and degraded mode"
short_code: "CLOACI-T-0410"
created_at: 2026-04-05T21:24:24.449475+00:00
updated_at: 2026-04-05T21:24:24.449475+00:00
parent: CLOACI-I-0081
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0081
---

# Reactor resilience — cache persistence, startup DAL loading, health state machine with startup gating and degraded mode

## Parent Initiative

[[CLOACI-I-0081]]

## Objective

Make the reactor survive restarts by persisting its `InputCache` and `DirtyFlags` to the DAL, loading them on startup, and implementing the `ReactorHealth` state machine with startup gating (waits for all accumulators healthy before entering Live) and degraded mode (continues with stale data when an accumulator disconnects).

## Acceptance Criteria

- [ ] **Cache persistence**: `InputCache` + `DirtyFlags` persisted to `reactor_state` table after each graph execution
- [ ] **Idle persistence**: if cache updated but criteria not yet met, persist periodically (configurable interval)
- [ ] **Startup loading**: reactor loads cache + dirty flags from DAL on construction — has last known state immediately
- [ ] **Sequential queue persistence**: `VecDeque<(SourceName, Vec<u8>)>` persisted to `sequential_queue` column in `reactor_state`
- [ ] **ReactorHealth** enum: `Starting`, `Warming { healthy, waiting }`, `Live`, `Degraded { disconnected }`
- [ ] **Startup gating** (per S-0005): reactor loads cache from DAL -> spawns accumulators -> waits for all `AccumulatorHealth::Live` or `SocketOnly` -> enters Live -> starts evaluating reaction criteria
- [ ] **Degraded mode**: when an accumulator transitions to `Disconnected`, reactor enters `Degraded` but keeps executing with stale data for that source. Returns to `Live` when accumulator reconnects.
- [ ] Reactor does NOT fire graphs until all accumulators are healthy (no execution during `Starting` or `Warming`)
- [ ] `health_reactive.rs` reports actual `ReactorHealth` state (not hard-coded)
- [ ] DAL handle provided to reactor at construction via the scheduler
- [ ] Unit tests: persist + load round-trip for cache, dirty flags, and sequential queue
- [ ] Unit tests: ReactorHealth state transitions (Starting->Warming->Live, Live->Degraded->Live)
- [ ] Integration test: restart reactor, verify cache restored from DAL and first execution uses fresh + restored data

## Implementation Notes

### Technical Approach

**Persistence timing** (per S-0005):
1. After each graph execution completes: snapshot cache + cleared dirty flags -> persist
2. Periodically during idle: if cache has been updated (new boundaries arrived) but criteria not yet met -> persist
3. On orderly shutdown: final persist (handled by T-0411)

**Startup gating**: The reactor subscribes to all accumulator `watch::Receiver<AccumulatorHealth>` channels (produced by T-0408). A background task monitors all channels. When all report `Live` or `SocketOnly`, the reactor transitions from `Warming` to `Live` and enables the executor.

**Degraded mode**: The accumulator health watcher continues running after `Live`. If any accumulator transitions to `Disconnected`, reactor transitions to `Degraded { disconnected: vec![name] }`. Execution continues — the cache still has the last known boundary for that source. When the accumulator reconnects (`Live` again), reactor returns to `Live`.

### Key files
- `crates/cloacina/src/computation_graph/reactor.rs` — persistence hooks, health state machine, startup gating
- `crates/cloacinactl/src/server/health_reactive.rs` — wire real ReactorHealth

### Dependencies
- T-0407 (DAL foundation) — needs `reactor_state` table
- T-0408 (accumulator health) — needs `AccumulatorHealth` watch channels for startup gating and degraded mode

### Risk Considerations
- Persist-after-execution adds latency to the execution path. Keep it async (spawn a persistence task, don't block the executor).
- Startup gating could deadlock if an accumulator never becomes healthy. Add a configurable timeout with fallback to `Degraded` mode (log warning, start executing with whatever state is available).

## Status Updates

*To be added during implementation*
