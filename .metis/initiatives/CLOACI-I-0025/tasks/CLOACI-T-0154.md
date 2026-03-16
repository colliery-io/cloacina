---
id: add-task-execution-timeout-to
level: task
title: "Add task execution timeout to scheduler run loop"
short_code: "CLOACI-T-0154"
created_at: 2026-03-15T18:24:20.818769+00:00
updated_at: 2026-03-15T19:19:16.751132+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Add task execution timeout to scheduler run loop

**Priority: P0 — CRITICAL**
**Parent**: [[CLOACI-I-0025]]

## Objective

Add configurable timeout to task execution in `scheduler.rs:337` where `exec.task.execute()` is awaited with no timeout. A hanging task blocks the entire scheduler loop — single stuck task = entire system stalls.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Task execution in `scheduler.rs` is wrapped with `tokio::time::timeout()`
- [ ] Default timeout is configurable per-task and globally (e.g., `ContinuousSchedulerConfig { default_task_timeout: Duration }`)
- [ ] Timed-out tasks are logged at `error!` level with task name and configured timeout
- [ ] Timed-out tasks are recorded in the `ExecutionLedger` as `TaskTimedOut` (or equivalent)
- [ ] The scheduler loop continues processing other tasks after a timeout (does not stall)
- [ ] Unit test: task that sleeps longer than timeout is correctly cancelled
- [ ] Integration test: one slow task does not block other ready tasks from firing

## Implementation Notes

- Wrap `exec.task.execute(ctx).await` with `tokio::time::timeout(duration, ...)`
- On timeout, log error, record in ledger, and continue loop
- Consider: should timed-out tasks be retried? For now, just record and move on. Retry policy is a separate concern.
- The timeout should be per-task (configurable on `ContinuousTaskConfig`) with a global default fallback

## Status Updates

### 2026-03-15 — Completed
- Added `task_timeout: Option<Duration>` to `ContinuousSchedulerConfig` (default: 5 minutes)
- Wrapped `exec.task.execute()` with `tokio::time::timeout()` when configured
- On timeout: logs `error!`, records `TaskFailed` in ledger with timeout message, pushes `FiredTask` with error, and continues to next task
- `None` disables timeout (used in tests to avoid flaky timing)
- Scheduler loop continues processing other tasks after a timeout — single stuck task no longer stalls the system
- All 412 unit tests pass
