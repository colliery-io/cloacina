---
id: unified-schedule-dal-single-module
level: task
title: "Unified schedule DAL — single module replacing 4 cron/trigger DAL modules"
short_code: "CLOACI-T-0309"
created_at: 2026-03-29T22:16:15.788993+00:00
updated_at: 2026-03-31T16:49:10.653692+00:00
parent: CLOACI-I-0059
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0059
---

# Unified schedule DAL — single module replacing 4 cron/trigger DAL modules

## Parent Initiative

[[CLOACI-I-0059]]

## Objective

Create a single `ScheduleDAL` module that replaces `CronScheduleDAL`, `CronExecutionDAL`, `TriggerScheduleDAL`, and `TriggerExecutionDAL`. All CRUD operations target the unified `schedules` and `schedule_executions` tables from T-0308.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ScheduleDAL` with CRUD for `schedules` table — create, get_by_id, list (with schedule_type filter), enable, disable, delete, upsert
- [ ] `ScheduleExecutionDAL` with CRUD for `schedule_executions` — create, get_by_id, list_by_schedule, complete, list_recent
- [ ] Cron-specific queries: `get_due_schedules(now)`, `claim_and_update(id, next_run)`, `update_next_run`
- [ ] Trigger-specific queries: `has_active_execution(trigger_name, context_hash)`, `get_recent(trigger_name, limit)`
- [ ] Postgres and SQLite backends via `dispatch_backend!` macro
- [ ] DAL accessor on `DAL` struct: `dal.schedule()` and `dal.schedule_execution()`
- [ ] All callers (CronScheduler, TriggerScheduler, runner API, server trigger endpoint) updated to use new DAL
- [ ] Existing integration tests pass with new DAL
- [ ] Unit tests for new DAL operations

## Implementation Notes

### Files to create/modify
- `crates/cloacina/src/dal/unified/schedule/mod.rs` + `crud.rs` — new unified DAL
- `crates/cloacina/src/dal/unified/schedule_execution/mod.rs` + `crud.rs`
- `crates/cloacina/src/dal/unified/mod.rs` — register new DAL, keep old ones temporarily

### Depends on
- T-0308 (unified schema must exist)

## Status Updates

*To be added during implementation*
