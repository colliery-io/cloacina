---
id: add-shutdown-channel-to
level: task
title: "Add shutdown channel to SchedulerLoop and fix server graceful shutdown (COR-03, OPS-04, OPS-08)"
short_code: "CLOACI-T-0444"
created_at: 2026-04-08T23:30:06.869697+00:00
updated_at: 2026-04-08T23:37:37.680392+00:00
parent: CLOACI-I-0086
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0086
---

# Add shutdown channel to SchedulerLoop and fix server graceful shutdown (COR-03, OPS-04, OPS-08)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0086]]

## Objective

`SchedulerLoop::run()` is the only background loop in the codebase without a shutdown channel. It runs `loop {}` and can only be stopped by aborting its task handle, which may interrupt database operations mid-transaction. Additionally, `cloacinactl serve` never calls `runner.shutdown()` — background services are terminated abruptly on SIGTERM (unlike the daemon which does this correctly).

**Effort**: 3-4 hours

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `SchedulerLoop` accepts a `watch::Receiver<bool>` shutdown channel
- [ ] `run()` uses `tokio::select!` to check shutdown between iterations (following `StaleClaimSweeper` pattern)
- [ ] `SchedulerLoop::run()` logs "SchedulerLoop shutting down" and breaks cleanly on shutdown signal
- [ ] `serve.rs` graceful shutdown calls `runner.shutdown()` with timeout (matching daemon pattern at `daemon.rs:331-354`)
- [ ] Server SIGTERM results in clean shutdown of scheduler, executor, cron recovery, and stale claim sweeper
- [ ] Existing unit and integration tests pass
- [ ] Soak test passes with clean shutdown (no panic/abort in logs)

## Implementation Notes

### Technical Approach

**SchedulerLoop changes** (`crates/cloacina/src/task_scheduler/scheduler_loop.rs`):
1. Add `shutdown_rx: watch::Receiver<bool>` field to `SchedulerLoop`
2. In `run()`, replace `loop { interval.tick().await; ... }` with:
   ```rust
   loop {
       tokio::select! {
           _ = interval.tick() => { /* existing loop body */ }
           _ = self.shutdown_rx.changed() => {
               info!("SchedulerLoop shutting down");
               break;
           }
       }
   }
   ```
3. Update `SchedulerLoop::new()` to accept the receiver
4. Update call sites that create `SchedulerLoop` to pass the shutdown channel

**Server shutdown changes** (`crates/cloacinactl/src/commands/serve.rs`):
1. In the SIGTERM/shutdown handler, call `runner.shutdown()` with a timeout (e.g., 30s)
2. Follow the daemon's pattern: log "Shutting down...", call shutdown, force-exit on timeout

### Dependencies
Do this first in I-0086 — enables clean shutdown testing for other tasks.

## Status Updates

- **2026-04-08**: Added `shutdown_rx: Option<watch::Receiver<bool>>` to `SchedulerLoop` and `TaskScheduler`. Added `with_shutdown()` builder method to both. `run()` now uses `tokio::select!` to break cleanly on shutdown signal. Wired `run_scheduling_loop()` to pass shutdown receiver through. Server graceful shutdown (`serve.rs`) now calls `runner.shutdown()` with 30s timeout after reactive scheduler shuts down. Compiles clean on both backends.
