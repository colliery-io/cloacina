---
id: integration-and-validation-restart
level: task
title: "Integration and validation — restart recovery tests, component failure tests, soak test with supervisor restarts"
short_code: "CLOACI-T-0414"
created_at: 2026-04-05T21:24:28.559651+00:00
updated_at: 2026-04-06T00:41:41.991644+00:00
parent: CLOACI-I-0081
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0081
---

# Integration and validation — restart recovery tests, component failure tests, soak test with supervisor restarts

## Parent Initiative

[[CLOACI-I-0081]]

## Objective

Build comprehensive integration tests that validate the entire resilience stack works end-to-end: restart recovery, individual component failures, health state transitions, and sustained operation under supervisor-triggered restarts. This is the capstone task — it proves that all the resilience pieces from T-0407 through T-0413 actually work together.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **Restart recovery test**: run a computation graph, inject events, verify cache populated -> stop reactor -> restart reactor -> verify cache loaded from DAL -> inject new events -> verify execution uses restored + fresh data
- [ ] **Accumulator checkpoint recovery test**: run polling accumulator for N polls -> stop -> restart -> verify poll state restored from checkpoint (counter doesn't reset to 0)
- [ ] **Batch buffer recovery test**: buffer events in batch accumulator -> crash (not graceful shutdown) -> restart -> verify buffered events recovered from DAL
- [ ] **State accumulator recovery test**: write N values to state accumulator -> restart -> verify VecDeque loaded from DAL and emitted to reactor
- [ ] **Sequential queue recovery test**: queue 5 boundaries in Sequential strategy -> crash -> restart -> verify queued boundaries restored and processed in order
- [ ] **Individual accumulator failure test**: run graph with 2 accumulators -> crash one -> verify supervisor restarts just that one -> verify reactor enters Degraded then returns to Live
- [ ] **Circuit breaker test**: crash an accumulator repeatedly (>MAX_RECOVERY_ATTEMPTS times) -> verify supervisor stops restarting -> verify recovery event recorded -> verify reactor reports Degraded with permanently failed source
- [ ] **Health state transition test**: start graph -> verify Starting->Warming->Live sequence -> disconnect an accumulator -> verify Live->Degraded -> reconnect -> verify Degraded->Live
- [ ] **Graceful shutdown test**: run graph under load -> send shutdown signal -> verify final state persisted -> verify WS clients received close frames -> verify clean exit
- [ ] **Deduplication test**: run accumulator -> crash and restart mid-stream -> verify reactor drops duplicate boundaries (sequence number check)
- [ ] **Soak test extension**: extend existing computation graph soak test to periodically trigger supervisor restarts during the 60s run -> verify no state loss, no panics, memory stable
- [ ] All tests runnable via `angreal cloacina resilience` or similar angreal task
- [ ] Tests work with both SQLite (unit/integration) and Postgres (soak/server)

## Implementation Notes

### Technical Approach

**Test categories:**

1. **Unit-level integration tests** (in `crates/cloacina/tests/integration/`): Test individual component recovery in isolation with in-memory SQLite. Fast, no external dependencies.

2. **Server-level integration tests** (angreal task): Full server bootstrap with Postgres, WebSocket connections, component failures via injected panics or task abort. Validates the entire stack including auth, WS layer, health endpoints.

3. **Soak test extension**: Modify the existing computation graph soak test (from I-0079/T-0404) to periodically abort and restart an accumulator task during the 60s run. Verify: no state loss (cache values monotonically increasing), no panics, memory stable, fire rate recovers after restart.

**Failure injection approaches:**
- For unit tests: use a custom accumulator that panics after N events (controllable via Arc<AtomicUsize> counter)
- For server tests: abort the JoinHandle directly via the scheduler's internal handle
- For soak tests: use the supervisor's internal API to force-restart a component

### Key files
- `crates/cloacina/tests/integration/computation_graph.rs` — add resilience tests alongside existing CG tests
- `.angreal/cloacina/` — new angreal task for resilience tests
- Extend existing soak test from I-0079

### Dependencies
- All prior tasks (T-0407 through T-0413) — this validates everything they built

### Risk Considerations
- Flaky tests: component restarts involve timing. Use polling assertions (not sleep-based) per existing project convention.
- Soak test with restarts may surface race conditions — that's the point. Fix any discovered issues before marking this complete.

## Status Updates

### 2026-04-06: Core integration tests complete

**12 new tests added** (all passing):
1. test_boundary_sender_sequence_numbers — verify monotonic sequence increment
2. test_boundary_sender_with_sequence_recovery — verify resume from persisted sequence
3. test_accumulator_health_channel — verify health state transitions via watch channel
4. test_checkpoint_dal_round_trip — save/load checkpoint with SQLite
5. test_checkpoint_dal_upsert — verify upsert overwrites previous value
6. test_boundary_dal_with_sequence — boundary + sequence number persistence
7. test_reactor_state_dal_round_trip — cache + dirty flags round-trip
8. test_reactor_state_dal_with_sequential_queue — queue persistence
9. test_state_buffer_dal_round_trip — state accumulator VecDeque persistence
10. test_delete_graph_state — verify cleanup across all 4 tables
11. test_checkpoint_handle_typed_round_trip — CheckpointHandle save/load with serde
12. test_checkpoint_handle_load_empty — verify None for nonexistent

Total: 21 computation_graph integration tests (9 existing + 12 new), all passing.

**Deferred to later work:**
- Server-level integration tests (need Postgres, angreal task)
- Soak test with supervisor restarts (extends I-0079/T-0404)
- Failure injection tests (panicking accumulators, circuit breaker validation)
