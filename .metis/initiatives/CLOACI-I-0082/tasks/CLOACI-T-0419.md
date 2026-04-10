---
id: health-state-machines-reactor
level: task
title: "Health state machines — reactor startup gating, degraded mode, live endpoints"
short_code: "CLOACI-T-0419"
created_at: 2026-04-06T01:05:52.370841+00:00
updated_at: 2026-04-06T09:18:39.139810+00:00
parent: CLOACI-I-0082
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0082
---

# Health state machines — reactor startup gating, degraded mode, live endpoints

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0082]]

## Objective

Implement reactor startup gating (Warming -> Live), degraded mode (Live -> Degraded on accumulator disconnect), and replace the hard-coded health endpoint strings with actual ReactorHealth state machine values.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Reactor subscribes to accumulator health `watch` channels on startup
- [ ] Reactor reports `Warming { healthy, waiting }` while waiting for accumulators
- [ ] Reactor transitions to `Live` only when all accumulators report `Live` or `SocketOnly`
- [ ] Reactor does NOT evaluate reaction criteria or execute the graph while in `Warming`
- [ ] Reactor transitions to `Degraded { disconnected }` when a previously-live accumulator reports `Disconnected`
- [ ] Reactor continues executing with stale cache data in Degraded mode
- [ ] Reactor transitions back to `Live` when disconnected accumulator recovers
- [ ] `GET /v1/health/reactors` returns actual `ReactorHealth` state, not hard-coded "running"
- [ ] `GraphStatus` struct extended to carry `ReactorHealth` instead of just a `running` bool
- [ ] All existing tests pass

## Implementation Notes

### Reactor changes (`reactor.rs`)
1. Add `accumulator_health_rxs: Vec<(String, watch::Receiver<AccumulatorHealth>)>` field, populated via `.with_accumulator_health()` builder
2. In `Reactor::run()`, after DAL cache load, enter a gating loop: report Warming, select over health receivers + shutdown, break to Live when all healthy
3. In main executor loop, background task watches for Disconnected transitions -> set Degraded

### Health endpoint changes (`health_reactive.rs`)
Replace `if !g.running / g.reactor_paused / else "running"` with `serde_json::to_value(&g.health)` using ReactorHealth's existing serde tag serialization.

### Dependencies
- T-0417 (scheduler wiring) must land first — health channels are created there

## Status Updates

*To be added during implementation*
