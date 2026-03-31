---
id: unified-schedule-schema-new-tables
level: task
title: "Unified schedule schema — new tables + data migration from cron/trigger tables"
short_code: "CLOACI-T-0308"
created_at: 2026-03-29T22:16:14.452096+00:00
updated_at: 2026-03-31T16:09:41.266585+00:00
parent: CLOACI-I-0059
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0059
---

# Unified schedule schema — new tables + data migration from cron/trigger tables

## Parent Initiative

[[CLOACI-I-0059]]

## Objective

Create the unified `schedules` and `schedule_executions` tables for both Postgres and SQLite. Write data migration SQL that copies existing rows from `cron_schedules`/`cron_executions`/`trigger_schedules`/`trigger_executions` into the new tables. Add diesel schema definitions and domain models.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Postgres migration: creates `schedules` table with `schedule_type` discriminator, cron-specific and trigger-specific columns
- [ ] Postgres migration: creates `schedule_executions` table with schedule FK
- [ ] SQLite migration: same schema adapted for SQLite types
- [ ] Data migration SQL: copies `cron_schedules` → `schedules` (type='cron'), `trigger_schedules` → `schedules` (type='trigger')
- [ ] Data migration SQL: copies `cron_executions` → `schedule_executions`, `trigger_executions` → `schedule_executions`
- [ ] Diesel schema definitions in `schema.rs` (postgres, sqlite, unified sections)
- [ ] Domain models: `Schedule`, `NewSchedule`, `ScheduleExecution`, `NewScheduleExecution` in `models/`
- [ ] Old tables remain (dropped in T-0311) — both old and new coexist during transition
- [ ] Migrations run cleanly on fresh DB and on existing DB with data

## Implementation Notes

### Files to create/modify
- `crates/cloacina/src/database/migrations/postgres/014_unified_schedules/up.sql` + `down.sql`
- `crates/cloacina/src/database/migrations/sqlite/013_unified_schedules/up.sql` + `down.sql`
- `crates/cloacina/src/database/schema.rs` — add unified table definitions
- `crates/cloacina/src/models/schedule.rs` — new domain models

### Depends on
- Nothing — schema-only, no code changes to schedulers

## Status Updates

*To be added during implementation*
