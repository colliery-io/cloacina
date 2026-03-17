---
id: migration-schema-dal-for-pipeline
level: task
title: "Migration + schema + DAL for pipeline_outbox table"
short_code: "CLOACI-T-0204"
created_at: 2026-03-17T01:38:49.594903+00:00
updated_at: 2026-03-17T01:49:35.648150+00:00
parent: CLOACI-I-0034
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0034
---

# Migration + schema + DAL for pipeline_outbox table

## Parent Initiative

[[CLOACI-I-0034]]

## Objective

Create the `pipeline_outbox` database table (Postgres and SQLite migrations), add the corresponding Diesel schema declarations, and define the Rust model structs. This table enables outbox-based pipeline claiming, mirroring the existing `task_outbox` pattern.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Postgres migration `016_create_pipeline_outbox` creates the table with `id BIGSERIAL`, `pipeline_execution_id UUID NOT NULL REFERENCES pipeline_executions(id) ON DELETE CASCADE`, and `created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL`, plus an index on `created_at`
- [ ] SQLite migration `015_create_pipeline_outbox` creates the equivalent table with `id INTEGER PRIMARY KEY AUTOINCREMENT`, `pipeline_execution_id VARCHAR(36)`, and `created_at TEXT NOT NULL DEFAULT (datetime('now'))`, plus an index on `created_at`
- [ ] `pipeline_outbox` table is declared in all three schema sections (unified, postgres, sqlite) in `schema.rs`, following the `task_outbox` pattern
- [ ] `pipeline_outbox` is added to all `allow_tables_to_appear_in_same_query!` blocks
- [ ] `joinable!(pipeline_outbox -> pipeline_executions (pipeline_execution_id))` is declared in all schema sections
- [ ] `UnifiedPipelineOutbox` (Queryable) and `NewUnifiedPipelineOutbox` (Insertable) model structs exist in `models.rs`
- [ ] `cargo check -p cloacina` compiles with the new schema and models

## Implementation Notes

### Technical Approach
- Follow the exact pattern of `task_outbox` in migrations, schema, and models
- The `id` column uses `BigInt` (i64) matching `task_outbox`
- The `pipeline_execution_id` column uses `DbUuid`/`Uuid`/`Binary` matching the backend
- Add a Postgres LISTEN/NOTIFY trigger (`pipeline_outbox_notify`) for push-based notification, mirroring the `task_outbox_notify` trigger

### Files to Modify
- `crates/cloacina/src/database/migrations/postgres/016_create_pipeline_outbox/up.sql` (new)
- `crates/cloacina/src/database/migrations/postgres/016_create_pipeline_outbox/down.sql` (new)
- `crates/cloacina/src/database/migrations/sqlite/015_create_pipeline_outbox/up.sql` (new)
- `crates/cloacina/src/database/migrations/sqlite/015_create_pipeline_outbox/down.sql` (new)
- `crates/cloacina/src/database/schema.rs`
- `crates/cloacina/src/dal/unified/models.rs`

### Dependencies
None -- this is the foundational task.

## Status Updates

*To be added during implementation*
