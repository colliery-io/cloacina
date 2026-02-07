---
id: schema-migrations-for-execution
level: task
title: "Schema migrations for execution_events and task_outbox tables"
short_code: "CLOACI-T-0079"
created_at: 2026-02-03T20:16:43.750711+00:00
updated_at: 2026-02-03T21:51:30.154496+00:00
parent: CLOACI-I-0022
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0022
---

# Schema migrations for execution_events and task_outbox tables

## Parent Initiative

[[CLOACI-I-0022]] - Execution Events and Outbox-Based Task Distribution

## Objective

Add database migrations for both Postgres and SQLite backends to create the `execution_events` and `task_outbox` tables.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Migration creates `execution_events` table with all columns and indexes
- [ ] Migration creates `task_outbox` table with all columns and indexes
- [ ] Both Postgres and SQLite migrations implemented
- [ ] Migrations are idempotent / can be re-run safely
- [ ] Existing tests pass after migration

## Implementation Notes

### execution_events schema

```sql
CREATE TABLE execution_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_execution_id UUID NOT NULL REFERENCES pipeline_executions(id) ON DELETE CASCADE,
    task_execution_id UUID REFERENCES task_executions(id) ON DELETE CASCADE,
    event_type VARCHAR(50) NOT NULL,
    event_data JSONB NOT NULL DEFAULT '{}',
    worker_id VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sequence_num BIGINT GENERATED ALWAYS AS IDENTITY
);

CREATE INDEX idx_events_pipeline ON execution_events(pipeline_execution_id, sequence_num);
CREATE INDEX idx_events_task ON execution_events(task_execution_id, sequence_num);
CREATE INDEX idx_events_created ON execution_events(created_at DESC);
```

### task_outbox schema

```sql
CREATE TABLE task_outbox (
    id BIGSERIAL PRIMARY KEY,
    task_execution_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_outbox_unclaimed ON task_outbox(created_at);
```

### SQLite Considerations

- Use `TEXT` for UUID columns
- Use `INTEGER PRIMARY KEY AUTOINCREMENT` for sequence_num
- JSON stored as TEXT with `json_valid()` check constraint
- No `GENERATED ALWAYS AS IDENTITY` - use trigger or application logic

## Status Updates

### 2026-02-03
- Created Postgres migration `012_create_execution_events_and_outbox`
  - `execution_events` table with all columns and indexes
  - `task_outbox` table with FIFO index
  - LISTEN/NOTIFY trigger on outbox inserts
- Created SQLite migration `011_create_execution_events_and_outbox`
  - Same schema adapted for SQLite (BLOB for UUID, TEXT for timestamps)
  - sequence_num as INTEGER PRIMARY KEY AUTOINCREMENT
- Verified migrations:
  - SQLite syntax check: passed
  - Unit tests (287): all passed
  - Integration tests (SQLite): all passed
