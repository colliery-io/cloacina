---
id: validation-restart-recovery
level: task
title: "Validation — restart recovery integration tests and crash/restart end-to-end tests"
short_code: "CLOACI-T-0421"
created_at: 2026-04-06T01:05:54.466438+00:00
updated_at: 2026-04-06T09:31:31.742849+00:00
parent: CLOACI-I-0082
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0082
---

# Validation — restart recovery integration tests and crash/restart end-to-end tests

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0082]]

## Objective

Write integration tests that prove the resilience features work end-to-end: reactor cache survives restart, individual accumulator restart works with health transitions, and the full push-crash-restart-verify cycle loses no data.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **Test: Reactor cache recovery** — Push events through accumulator -> reactor fires and persists cache to DAL -> shut down reactor -> create new reactor with same DAL -> verify cache is restored and graph can fire with restored state
- [ ] **Test: Individual accumulator restart** — Load a graph with 2 accumulators -> kill one accumulator's task -> verify supervisor restarts it -> verify the restarted accumulator receives events and the reactor continues firing
- [ ] **Test: Health state transitions** — Load graph -> verify reactor starts in Warming -> accumulators report Live -> verify reactor transitions to Live -> disconnect an accumulator -> verify reactor transitions to Degraded -> reconnect -> verify back to Live
- [ ] **Test: Boundary sequence continuity** — Push N events -> shut down -> restart with same DAL -> verify BoundarySender resumes from last persisted sequence number (no gap, no duplicate)
- [ ] **Test: State accumulator survives restart** — Write values to state accumulator -> verify persisted in DAL -> restart runtime -> verify VecDeque loaded from DAL and initial boundary emitted to reactor
- [ ] **Test: Batch buffer crash recovery** — Buffer events in batch accumulator -> "crash" (drop runtime without flush) -> restart with same DAL -> verify buffered events restored from checkpoint
- [ ] All tests use SQLite in-memory DAL (fast, no external dependencies)
- [ ] Tests added to `crates/cloacina/tests/integration/computation_graph.rs`

## Implementation Notes

### Test infrastructure
Reuse the existing `test_dal()` helper from the resilience tests section of `computation_graph.rs` which creates a temp SQLite database.

### Test patterns
Each test follows: setup with DAL -> push events -> verify state persisted -> simulate restart (drop + recreate with same DAL) -> verify state recovered.

For the supervisor test, use an `AccumulatorFactory` that produces an accumulator which panics after N events, then verify `check_and_restart_failed()` respawns it.

### Dependencies
- All other I-0082 tasks should be complete before this task — it validates their work
- T-0415 (bug fixes), T-0416 (run() ownership), T-0417 (scheduler wiring), T-0418 (supervision), T-0419 (health), T-0420 (persistence)

## Status Updates

*To be added during implementation*
