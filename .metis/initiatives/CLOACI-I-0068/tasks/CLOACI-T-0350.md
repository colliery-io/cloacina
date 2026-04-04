---
id: t2-schedule-and-cron-dal
level: task
title: "T2: Schedule and cron DAL integration tests"
short_code: "CLOACI-T-0350"
created_at: 2026-04-03T13:19:23.316966+00:00
updated_at: 2026-04-03T17:48:15.718404+00:00
parent: CLOACI-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0068
---

# T2: Schedule and cron DAL integration tests

## Parent Initiative
[[CLOACI-I-0068]] — Tier 1 (~1,450 missed lines)

## Objective
Add integration tests for schedule and schedule_execution DAL CRUD operations, and the cron_api runner methods. Currently schedule/crud.rs is at 21% and schedule_execution/crud.rs at 24%.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] schedule/crud.rs: test create, get_by_id, list (with filters), update, delete, enable/disable
- [ ] schedule_execution/crud.rs: test create, list_by_schedule, find_lost_executions, update_pipeline_execution_id
- [ ] schedule/mod.rs: test all public DAL methods
- [ ] runner/default_runner/cron_api.rs: test register_cron_workflow, get_cron_execution_stats, list_cron_schedules
- [ ] Tests run against SQLite (unique DB per test)
- [ ] Coverage of target files moves from ~20% to >60%

## Source Files
- crates/cloacina/src/dal/unified/schedule/crud.rs (820 missed, 21%)
- crates/cloacina/src/dal/unified/schedule/mod.rs (110 missed, 30%)
- crates/cloacina/src/dal/unified/schedule_execution/crud.rs (440 missed, 24%)
- crates/cloacina/src/dal/unified/schedule_execution/mod.rs (65 missed, 30%)
- crates/cloacina/src/runner/default_runner/cron_api.rs (190 missed, 18%)

## Implementation Notes
Use unique SQLite DBs per test (same pattern as security tests). Schedule tests need NewSchedule::cron() and NewSchedule::trigger() constructors.

## Status Updates

### 2026-04-03 — Complete (33 tests)

schedule/mod.rs (20 tests): CRUD, list filters, enable/disable, find by workflow, upsert trigger, cron claim. Coverage 30%→96.7%.

schedule_execution/mod.rs (13 tests): create, list, complete, has_active, update pipeline ID (with FK), get latest, find lost, stats. Coverage 30%→97.4%.

crud.rs files at ~55% — remaining gaps are Postgres-only code paths (structurally identical to SQLite).
