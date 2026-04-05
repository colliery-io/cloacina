---
id: supervisor-hardening-individual
level: task
title: "Supervisor hardening — individual accumulator restart, failure counting, backoff, recovery events"
short_code: "CLOACI-T-0412"
created_at: 2026-04-05T21:24:26.728252+00:00
updated_at: 2026-04-05T21:24:26.728252+00:00
parent: CLOACI-I-0081
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0081
---

# Supervisor hardening — individual accumulator restart, failure counting, backoff, recovery events

## Parent Initiative

[[CLOACI-I-0081]]

## Objective

Upgrade the ReactiveScheduler's supervision loop to handle individual accumulator restarts (currently only full-graph restart), add failure counting with exponential backoff, circuit breaking after max failures, and record all recovery events in the existing `RecoveryEvent` DAL.

## Acceptance Criteria

- [ ] **Individual accumulator restart**: when a single accumulator task crashes, restart just that accumulator with a new channel and re-wire it to the reactor — without tearing down the entire graph
- [ ] Re-wired accumulator channel is registered in `EndpointRegistry` (old entry replaced)
- [ ] Reactor's `AccumulatorHealth` watch channel updates to reflect the restarted accumulator's new health
- [ ] **Failure counting**: track consecutive failure count per component (accumulator or reactor)
- [ ] **Exponential backoff**: delay between restarts doubles on each consecutive failure (base 1s, max 60s)
- [ ] **Circuit breaking**: after `MAX_RECOVERY_ATTEMPTS` (configurable, default 5) consecutive failures, stop restarting and mark component as permanently failed
- [ ] Permanently failed component triggers reactor `Degraded` state (if accumulator) or graph `Failed` state (if reactor)
- [ ] **Recovery events**: every restart and every permanent failure recorded in the existing `RecoveryEvent` DAL table with structured JSON details (component name, failure count, error message, graph name)
- [ ] `RecoveryType` enum extended with `AccumulatorRestart`, `ReactorRestart`, `ComponentAbandoned` variants (or similar, matching existing pattern)
- [ ] Consecutive failure counter resets to 0 after a component runs successfully for a configurable duration (default 60s)
- [ ] Unit tests: failure counting, backoff calculation, circuit breaker trigger
- [ ] Unit tests: individual accumulator restart with channel re-wiring
- [ ] Integration test: crash an accumulator repeatedly, verify backoff delays increase, verify circuit breaker fires after max attempts

## Implementation Notes

### Technical Approach

**Individual accumulator restart** (the hard part): Currently punted because "we need to re-wire the boundary channel." The fix:
1. Create a new `mpsc::Sender<(SourceName, Vec<u8>)>` for the replacement accumulator
2. The reactor's receiver channel remains the same — it reads from `mpsc::Receiver` which is the other end
3. Key insight: use a shared `Arc<Mutex<mpsc::Sender>>` or a broadcast-style indirection so the reactor doesn't need to know its sender was replaced
4. Alternative: give the reactor a channel that multiplexes from all accumulators via a merge task — replacing one accumulator's sender just means replacing one input to the merge

**Backoff calculation**: `min(base * 2^failures, max_delay)` where base=1s, max=60s.

**Recovery events**: Follow the pattern in `crates/cloacina/src/dal/unified/recovery_event.rs`. The `RecoveryEvent` table and CRUD already exist.

### Key files
- `crates/cloacina/src/computation_graph/scheduler.rs` — `check_and_restart_failed()` rewrite, individual restart, backoff, circuit breaker
- `crates/cloacina/src/dal/unified/recovery_event.rs` — extend with CG-specific recovery types
- `crates/cloacina/src/computation_graph/registry.rs` — re-register accumulator after restart

### Dependencies
- T-0408 (accumulator health) — needs health watch channels for reactor notification of accumulator restart
- T-0410 (reactor health) — reactor must transition to `Degraded` when accumulator is permanently failed

### Risk Considerations
- Channel re-wiring is the trickiest part — must not lose messages during the swap. A brief gap is acceptable (the accumulator was crashed anyway), but the new channel must be wired before the replacement accumulator starts sending.
- Backoff delays must not block the supervision loop for other graphs. Use per-graph timers, not a sleep in the loop.

## Status Updates

*To be added during implementation*
