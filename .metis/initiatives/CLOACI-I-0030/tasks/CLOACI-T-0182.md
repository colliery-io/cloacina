---
id: add-continuous-scheduler-shutdown
level: task
title: "Add continuous scheduler shutdown to DefaultRunner shutdown path"
short_code: "CLOACI-T-0182"
created_at: 2026-03-16T13:23:24.676175+00:00
updated_at: 2026-03-16T13:36:48.508189+00:00
parent: CLOACI-I-0030
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0030
---

# Add continuous scheduler shutdown to DefaultRunner shutdown path

## Parent Initiative

[[CLOACI-I-0030]]

## Objective

Integrate the continuous scheduler into the `DefaultRunner::shutdown()` method so that calling `runner.shutdown()` cleanly stops the continuous scheduler, awaits its join handle, and avoids orphaned tokio tasks or panics.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `shutdown()` in `mod.rs` sends `true` on `handles.continuous_shutdown_tx` (the `watch::Sender<bool>`) before awaiting the handle
- [ ] `shutdown()` awaits `handles.continuous_scheduler_handle` after sending the signal
- [ ] Shutdown of the continuous scheduler is ordered after the broadcast `shutdown_sender.send(())` (which stops all other services) -- the watch channel provides the continuous scheduler's own cooperative exit
- [ ] If continuous scheduling was not enabled (handle is `None`), shutdown skips gracefully with no error
- [ ] No panics during shutdown when continuous scheduler is running
- [ ] No orphaned tokio tasks after shutdown completes
- [ ] `Drop` impl log message remains unchanged (users still call `shutdown()` explicitly)

## Implementation Notes

### Technical Approach

**File: `crates/cloacina/src/runner/default_runner/mod.rs`** -- `shutdown()` method (line ~279)

Add two blocks after the existing trigger scheduler shutdown (line ~313), before `self.database.close()`:

```rust
// Send shutdown signal to continuous scheduler (if enabled)
if let Some(tx) = handles.continuous_shutdown_tx.take() {
    let _ = tx.send(true);
}

// Wait for continuous scheduler to finish (if enabled)
if let Some(handle) = handles.continuous_scheduler_handle.take() {
    let _ = handle.await;
}
```

This follows the exact same `take()` + `send()`/`await` pattern used for every other service in the shutdown method. The `watch::Sender<bool>` is the signal mechanism that `ContinuousScheduler::run()` listens on -- sending `true` causes its `watch::Receiver` to resolve, breaking the poll loop.

**Note on dual-signal pattern**: In T-0181, the spawned task uses `tokio::select!` over both the scheduler's `run(shutdown_rx)` and a `broadcast_shutdown_rx.recv()`. The broadcast signal (from `shutdown_sender.send(())`) will fire first since it's sent earlier in the shutdown sequence. The broadcast handler then sends `true` on the watch channel, which causes `scheduler.run()` to exit. The explicit `continuous_shutdown_tx.send(true)` here is a safety net in case the broadcast relay is missed due to task ordering.

### Dependencies

- T-0180 (provides `continuous_scheduler_handle` and `continuous_shutdown_tx` fields in `RuntimeHandles`)
- T-0181 (populates those fields during startup)

### Risk Considerations

- **Ordering**: The continuous scheduler should be shut down after the main task scheduler and cron services to avoid a window where new work could be queued but the continuous scheduler is already gone. Placing it last (before `database.close()`) is correct.
- **Timeout**: If `ContinuousScheduler::run()` does not exit promptly after receiving `true` on the watch channel, the `handle.await` will block indefinitely. The scheduler's poll loop checks the watch receiver each iteration (every `poll_interval`, default 100ms), so this should resolve quickly. If needed, a `tokio::time::timeout` wrapper could be added as a follow-up.

## Status Updates

### 2026-03-16 — Completed
- Added `continuous_shutdown_tx.send(true)` + `continuous_scheduler_handle.await` to `shutdown()` method
- Placed after trigger scheduler shutdown, before `database.close()`
- Both use `.take()` pattern matching all other services
- If continuous scheduling was not enabled (fields are None), shutdown skips gracefully
- Dual-signal safety: broadcast signal fires first (from shutdown_sender), continuous watch signal is safety net
- Compiles clean
