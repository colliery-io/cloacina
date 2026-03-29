---
id: schema-migration-add-claimed-by
level: task
title: "Schema migration — add claimed_by and heartbeat_at to task_executions"
short_code: "CLOACI-T-0288"
created_at: 2026-03-29T12:33:47.554867+00:00
updated_at: 2026-03-29T12:52:31.272382+00:00
parent: CLOACI-I-0055
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0055
---

# Schema migration — add claimed_by and heartbeat_at to task_executions

## Parent Initiative

[[CLOACI-I-0055]]

## Objective

Add `claimed_by` and `heartbeat_at` columns to the `task_executions` table so runners can atomically claim tasks and heartbeat to prove liveness. Update the Diesel schema and unified models.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Diesel migration adds `claimed_by` (nullable UUID) and `heartbeat_at` (nullable timestamp) to `task_executions`
- [ ] Migration runs on both SQLite and Postgres
- [ ] `schema.rs` updated with new columns
- [ ] `UnifiedTaskExecution` model updated with new fields
- [ ] `TaskExecution` domain model updated with new fields
- [ ] Existing tests still pass (new columns are nullable, no breaking changes)

## Implementation Notes

### Files to modify
- `crates/cloacina/src/database/schema.rs` — add columns to `task_executions` table
- Diesel migration file — `ALTER TABLE task_executions ADD COLUMN ...`
- `crates/cloacina/src/dal/unified/models.rs` — add fields to `UnifiedTaskExecution`
- `crates/cloacina/src/models/task_execution.rs` — add fields to `TaskExecution`

### Depends on
- Nothing — first task in the chain

## Status Updates

**2026-03-29**: Complete. All tests pass (unit + SQLite integration).
