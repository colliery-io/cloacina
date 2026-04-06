---
id: supervision-and-recovery-start
level: task
title: "Supervision and recovery — start loop, apply backoff, record RecoveryEvents"
short_code: "CLOACI-T-0418"
created_at: 2026-04-06T01:05:51.142720+00:00
updated_at: 2026-04-06T09:14:47.083275+00:00
parent: CLOACI-I-0082
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0082
---

# Supervision and recovery — start loop, apply backoff, record RecoveryEvents

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0082]]

## Objective

Start the supervision loop from the server, make backoff delays actually wait before respawning, and record recovery events in the existing RecoveryEvent DAL table so restarts have an audit trail.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `serve.rs` calls `reactive_scheduler.start_supervision()` during server startup, passing the shutdown signal and a configurable check interval
- [ ] Supervision loop shuts down cleanly on server shutdown (via shared shutdown signal)
- [ ] `check_and_restart_failed()` calls `tokio::time::sleep(backoff)` before respawning a failed component
- [ ] Each supervisor restart (accumulator or reactor) records a `RecoveryEvent` in the database via the existing DAL
- [ ] Recovery events include: graph name, component name, failure count, timestamp, error context if available
- [ ] Unit test: verify backoff duration increases exponentially up to the cap
- [ ] All existing tests pass

## Implementation Notes

### Start supervision (`serve.rs`)
After creating the scheduler, before the axum `serve()` call:
```rust
let supervision_shutdown = shutdown_rx.clone();
let _supervision_handle = scheduler_for_shutdown
    .start_supervision(supervision_shutdown, Duration::from_secs(5));
```

### Apply backoff (`scheduler.rs:311-313, 388-394`)
Add `tokio::time::sleep(backoff).await;` before the respawn logic in both the reactor and accumulator restart branches.

### Record RecoveryEvent (`scheduler.rs`)
The existing `RecoveryEvent` DAL has create methods. The scheduler needs a DAL handle (from T-0417) to record events on each restart.

### Dependencies
- T-0417 (scheduler wiring) must land first — the scheduler needs a DAL handle to record recovery events

## Status Updates

*To be added during implementation*
