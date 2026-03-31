---
id: unified-scheduler-single-scheduler
level: task
title: "Unified Scheduler — single scheduler replacing CronScheduler + TriggerScheduler"
short_code: "CLOACI-T-0310"
created_at: 2026-03-29T22:16:17.089200+00:00
updated_at: 2026-03-29T22:16:17.089200+00:00
parent: CLOACI-I-0059
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0059
---

# Unified Scheduler — single scheduler replacing CronScheduler + TriggerScheduler

## Parent Initiative

[[CLOACI-I-0059]]

## Objective

Replace `CronScheduler` and `TriggerScheduler` with a single `Scheduler` that handles both schedule types. Single run loop, unified config, same `PipelineExecutor` handoff. Update `DefaultRunner` to start one scheduler instead of two.

## Acceptance Criteria

- [ ] Single `Scheduler` struct replaces `CronScheduler` + `TriggerScheduler`
- [ ] Single run loop: fetches all due schedules (both types), processes each, hands off to `PipelineExecutor`
- [ ] Cron schedules: checks `next_run_at` against now, claims atomically, computes next run
- [ ] Custom triggers: calls registered `Trigger::poll()` at `poll_interval`, deduplicates by context hash
- [ ] Single `SchedulerConfig` replacing `CronSchedulerConfig` + `TriggerSchedulerConfig`
- [ ] `DefaultRunner` starts one `Scheduler` instead of two separate schedulers
- [ ] Graceful shutdown via watch channel (same pattern as current schedulers)
- [ ] Catchup policy honored for cron schedules
- [ ] `allow_concurrent` honored for trigger schedules
- [ ] All existing cron and trigger integration tests pass
- [ ] Daemon soak test passes
- [ ] Server trigger API endpoint updated to query unified DAL

## Implementation Notes

### Files to create/modify
- `crates/cloacina/src/scheduler.rs` — new unified Scheduler (or `scheduler/mod.rs`)
- `crates/cloacina/src/runner/default_runner/services.rs` — replace `start_cron_services()`, `start_cron_recovery()`, and `start_trigger_services()` with single scheduler start
- `crates/cloacina/src/runner/default_runner/config.rs` — unified `SchedulerConfig`
- `crates/cloacina/src/runner/default_runner/cron_api.rs` — update `register_cron_workflow()` to use unified DAL/scheduler

### Note on current code locations
- `CronScheduler` is at crate root: `crates/cloacina/src/cron_scheduler.rs`
- `TriggerScheduler` is at crate root: `crates/cloacina/src/trigger_scheduler.rs`
- Cron recovery is started separately in `services.rs:start_cron_recovery()` — must be folded into unified scheduler
- `cron_api.rs` has `register_cron_workflow()` — needs updating to unified schedule API

### Depends on
- T-0309 (unified DAL must exist)

## Status Updates

*To be added during implementation*
