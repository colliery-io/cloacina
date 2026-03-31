---
id: drop-old-cron-trigger-tables-dal
level: task
title: "Drop old cron/trigger tables, DAL modules, and scheduler code"
short_code: "CLOACI-T-0311"
created_at: 2026-03-29T22:16:17.928797+00:00
updated_at: 2026-03-31T21:42:23.923063+00:00
parent: CLOACI-I-0059
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0059
---

# Drop old cron/trigger tables, DAL modules, and scheduler code

## Parent Initiative

[[CLOACI-I-0059]]

## Objective

Remove old infrastructure now that the unified scheduler (T-0310) and DAL (T-0309) are in place. Drop the old tables via migration, delete the old DAL modules, delete `CronScheduler` and `TriggerScheduler`, remove old config structs. Clean sweep.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Migration drops `cron_schedules`, `cron_executions`, `trigger_schedules`, `trigger_executions` tables (Postgres + SQLite)
- [ ] Delete `dal/unified/cron_schedule/`, `dal/unified/cron_execution/`, `dal/unified/trigger_schedule/`, `dal/unified/trigger_execution/`
- [ ] Delete `src/cron_scheduler.rs`, `src/trigger_scheduler.rs` (crate root)
- [ ] Delete `CronSchedulerConfig`, `TriggerSchedulerConfig`
- [ ] Delete or refactor `src/runner/default_runner/cron_api.rs` → unified schedule API
- [ ] Remove old table definitions from `schema.rs` (all 3 sections)
- [ ] Remove old models from `models/cron_schedule.rs`, `models/cron_execution.rs`, `models/trigger_schedule.rs`, `models/trigger_execution.rs`
- [ ] Remove old DAL accessors from `dal/unified/mod.rs` (`cron_schedule()`, `cron_execution()`, `trigger_schedule()`, `trigger_execution()`)
- [ ] No remaining references to old types anywhere in codebase (`grep` clean)
- [ ] All tests pass
- [ ] Daemon and server soak tests pass

## Implementation Notes

### Depends on
- T-0310 (unified Scheduler must be fully working before removing old code)

### Approach
- Run `grep` for all old type names to find stragglers
- Migration is destructive — no rollback for dropped tables (data already migrated in T-0308)

## Status Updates

*To be added during implementation*
