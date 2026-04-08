---
id: add-circuit-breaker-with
level: task
title: "Add circuit breaker with exponential backoff to scheduler loop (OPS-05)"
short_code: "CLOACI-T-0447"
created_at: 2026-04-08T23:30:12.005943+00:00
updated_at: 2026-04-08T23:42:29.675166+00:00
parent: CLOACI-I-0086
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0086
---

# Add circuit breaker with exponential backoff to scheduler loop (OPS-05)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0086]]

## Objective

During a sustained database outage, the scheduler loop logs an error on every 100ms tick with no backoff — 600 error lines per minute flooding logs and obscuring diagnostic information. The `ReactiveScheduler` already implements this pattern correctly (exponential backoff, circuit breaker with max failures). Apply the same pattern here.

**Effort**: 2-3 hours

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `SchedulerLoop` has a `consecutive_errors: u32` counter
- [ ] On error, poll interval increases exponentially: `min(base_interval * 2^consecutive_errors, 30s)`
- [ ] On success, counter resets to 0 and interval restores to configured value
- [ ] A rate-limited warning is emitted when backoff kicks in (not on every error)
- [ ] After N consecutive failures (e.g., 5), a circuit-open warning is logged once
- [ ] Unit test verifies backoff behavior (counter increment, interval adjustment, reset on success)

## Implementation Notes

### Technical Approach

Follow the `ReactiveScheduler::check_and_restart_failed()` pattern (`crates/cloacina/src/computation_graph/scheduler.rs`):
1. Add `consecutive_errors: u32` field to `SchedulerLoop`
2. In the error path of `process_active_pipelines` / `dispatch_ready_tasks`:
   - Increment `consecutive_errors`
   - Compute backed-off interval: `min(poll_interval * 2^consecutive_errors, Duration::from_secs(30))`
   - Only log at warn level every N errors (e.g., every 10th) to avoid flooding
3. In the success path: reset `consecutive_errors = 0`, restore original interval

### Dependencies
Should be done after T-0444 (shutdown channel) since both modify `SchedulerLoop`.

## Status Updates

- **2026-04-08**: Added `consecutive_errors: u32` counter to `SchedulerLoop`. On error: increments counter, computes backoff `poll_interval * 2^min(errors, 8)` capped at 30s, sleeps the difference. Circuit-open warning at 5 consecutive errors, rate-limited logging every 10th error after that. On success: resets counter to 0 with recovery log. Constants: `MAX_BACKOFF = 30s`, `CIRCUIT_OPEN_THRESHOLD = 5`. Compiles clean.
