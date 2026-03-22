---
id: cron-scheduler-stalls-after
level: task
title: "Cron scheduler stalls after initial catchup burst — no new executions after startup"
short_code: "CLOACI-T-0218"
created_at: 2026-03-21T03:04:28.900537+00:00
updated_at: 2026-03-21T14:42:51.304124+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Cron scheduler stalls after initial catchup burst — no new executions after startup

## Objective

The daemon's cron scheduler fires a burst of catchup executions on startup, then stops scheduling new executions entirely. Subsequent cron windows are never evaluated.

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

### Impact Assessment
- **Affected Users**: All daemon users with cron-scheduled workflows
- **Reproduction Steps**:
  1. Register a workflow with `cloacinactl daemon register`
  2. Create a cron schedule: `cloacinactl daemon schedule create --cron "*/2 * * * * *"`
  3. Start daemon: `cloacinactl daemon start`
  4. Observe: ~5-6 executions fire in first 5 seconds (catchup burst), then nothing for remaining duration
- **Expected**: Continuous executions every 2 seconds for the full duration
- **Actual**: Only catchup burst on startup, then zero new executions. 6 executions in 4 minutes instead of ~120.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Cron scheduler continues evaluating new windows after startup catchup completes
- [ ] `*/2 * * * * *` schedule produces ~1 execution every 2 seconds continuously
- [ ] Daemon soak test with `--cron "*/2 * * * * *" --duration 45s` produces ~20 executions (matching server soak throughput)
- [ ] Existing cron tests still pass

## Implementation Notes

### Likely root cause

The cron scheduler in `crates/cloacina/src/cron_scheduler.rs` tracks `last_executed_at` per schedule. After the catchup burst, `last_executed_at` is set to the current time. On the next poll, `executions_between(last_executed_at, now)` returns empty because the window is too narrow (poll interval ~= cron interval). The scheduler thinks it's caught up and does nothing.

### Key files
- `crates/cloacina/src/cron_scheduler.rs` — `CronScheduler::evaluate_schedules()` and `calculate_execution_times()`
- `crates/cloacina/src/cron_evaluator.rs` — `executions_between()` time window calculation
- `crates/cloacina/src/models/cron_schedule.rs` — `CatchupPolicy` enum

### Discovered during
I-0038 (Native Python Workflow Support) — daemon soak test revealed the bug when trying to match server soak execution counts.

## Status Updates

### 2026-03-21 — Root cause identified and fixed

**Root cause:** Not a scheduler bug. The daemon's cron poll interval was 10 seconds (`cron_poll_interval(Duration::from_secs(10))`). With a `*/2 * * * * *` cron (every 2 seconds), the scheduler would fire a catchup burst on startup (all missed windows), then appear to stall because it only checked for due schedules every 10 seconds. Each poll found 1 due schedule, executed it, and set `next_run_at` 2 seconds ahead. But the next poll was 10 seconds later — by then 4 cron windows had been missed.

**Fix:** Already applied in the I-0038 commit: `cron_poll_interval(Duration::from_secs(2))` in `daemon.rs`. This was committed as part of `a412e0c`.

**Verification:** `*/2 * * * * *` cron with 2s poll interval:
- 45s duration → 18 executions, 17 successful (94.4%)
- Matches server soak throughput (~20 executions)
- Continuous execution, no stalling

**All acceptance criteria met.**
