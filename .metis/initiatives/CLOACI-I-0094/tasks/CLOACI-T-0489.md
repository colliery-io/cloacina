---
id: db-migration-dal-rename-for
level: task
title: "DB migration + DAL rename for pipeline-to-workflow"
short_code: "CLOACI-T-0489"
created_at: 2026-04-14T00:57:35.350686+00:00
updated_at: 2026-04-14T02:56:11.074691+00:00
parent: CLOACI-I-0094
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0094
---

# DB migration + DAL rename for pipeline-to-workflow

## Parent Initiative

[[CLOACI-I-0094]]

## Objective

Create Diesel migration to rename `pipeline_executions` table and columns to `workflow_executions`, update `schema.rs`, and update all DAL code to use the new names.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Diesel migration renames `pipeline_executions` → `workflow_executions`
- [ ] Columns renamed: `pipeline_name` → `workflow_name`, `pipeline_version` → `workflow_version`
- [ ] `schema.rs` regenerated/updated to reflect new table/column names
- [ ] All DAL code updated: `pipeline_execution.rs` → `workflow_execution.rs`, all `pipeline_exec*` references in DAL modules
- [ ] `pipeline_execution_id` foreign key columns in related tables renamed
- [ ] All tests pass with both SQLite and Postgres backends
- [ ] `cargo check` passes for all crates

## Implementation Notes

### Key Areas (~300 occurrences in `dal/unified/`)
- `dal/unified/pipeline_execution.rs` — main DAL module
- `dal/unified/models.rs` — Diesel model structs
- `dal/unified/task_execution/` — FK references to pipeline_execution_id
- `dal/unified/execution_event.rs`, `recovery_event.rs`, `schedule_execution/`
- `database/schema.rs` — Diesel schema definitions

### Approach
1. Create migration with `ALTER TABLE RENAME` (Postgres) / recreate table (SQLite)
2. Update `schema.rs`
3. Update all DAL code to reference new names
4. Run tests against both backends

### Dependencies
- T-0488 must be completed first (code-level renames)

## Status Updates

*To be added during implementation*
