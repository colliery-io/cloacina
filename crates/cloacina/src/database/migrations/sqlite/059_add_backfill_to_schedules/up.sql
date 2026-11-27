-- CLOACINA-T-1020: the backfill.
-- A backfill is a bounded second schedule for one workflow. It is a row of
-- `schedules` with `schedule_type = 'backfill'`, so the type check takes the
-- new value. `backfill` holds the range of the row as JSON: `start`, `end`
-- (the last fire time of the range), `total`, and for a retry the list
-- `fire_times` and `retry_of`. NULL on every other row.
--
-- SQLite cannot change a CHECK in place, so this migration builds the table
-- again: the columns and indexes of migrations 013, 025, 040 and 054, the
-- new check and the new column.
--
-- Foreign keys are on in the bundled SQLite, and DROP TABLE deletes the rows
-- of the table first, so `schedule_executions` (ON DELETE CASCADE) would lose
-- its rows. They are kept in a copy and written back after the new table
-- has its name.
CREATE TABLE schedule_executions_keep_059 AS SELECT * FROM schedule_executions;

CREATE TABLE schedules_new (
    id BLOB PRIMARY KEY NOT NULL,
    schedule_type TEXT NOT NULL CHECK (schedule_type IN ('cron', 'trigger', 'backfill')),
    workflow_name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    cron_expression TEXT,
    timezone TEXT DEFAULT 'UTC',
    catchup_policy TEXT DEFAULT 'skip' CHECK (catchup_policy IS NULL OR catchup_policy IN ('skip', 'run_all')),
    start_date TEXT,
    end_date TEXT,
    trigger_name TEXT UNIQUE,
    poll_interval_ms INTEGER,
    allow_concurrent INTEGER DEFAULT 0 CHECK (allow_concurrent IS NULL OR allow_concurrent IN (0, 1)),
    next_run_at TEXT,
    last_run_at TEXT,
    last_poll_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    paused INTEGER NOT NULL DEFAULT 0 CHECK (paused IN (0, 1)),
    paused_at TEXT,
    params TEXT,
    instance_name TEXT,
    max_active_runs INTEGER,
    backfill TEXT
);

INSERT INTO schedules_new (
    id, schedule_type, workflow_name, enabled, cron_expression, timezone,
    catchup_policy, start_date, end_date, trigger_name, poll_interval_ms,
    allow_concurrent, next_run_at, last_run_at, last_poll_at, created_at,
    updated_at, paused, paused_at, params, instance_name, max_active_runs
)
SELECT
    id, schedule_type, workflow_name, enabled, cron_expression, timezone,
    catchup_policy, start_date, end_date, trigger_name, poll_interval_ms,
    allow_concurrent, next_run_at, last_run_at, last_poll_at, created_at,
    updated_at, paused, paused_at, params, instance_name, max_active_runs
FROM schedules;

DROP TABLE schedules;
ALTER TABLE schedules_new RENAME TO schedules;

CREATE INDEX idx_schedules_cron_polling
ON schedules (enabled, next_run_at)
WHERE enabled = 1 AND schedule_type = 'cron';
CREATE INDEX idx_schedules_trigger_polling
ON schedules (enabled, last_poll_at)
WHERE enabled = 1 AND schedule_type = 'trigger';
CREATE INDEX idx_schedules_workflow
ON schedules (workflow_name, enabled);
CREATE INDEX idx_schedules_type
ON schedules (schedule_type);
CREATE INDEX idx_schedules_paused ON schedules (paused) WHERE paused = 1;
CREATE UNIQUE INDEX idx_schedules_instance_name
    ON schedules (workflow_name, instance_name)
    WHERE instance_name IS NOT NULL;
-- The scheduler reads the due backfills on each cron pass.
CREATE INDEX idx_schedules_backfill_polling
ON schedules (enabled, next_run_at)
WHERE enabled = 1 AND schedule_type = 'backfill';

-- Write back the rows that the cascade removed. With foreign keys off the
-- rows are still there and the copy adds nothing.
INSERT INTO schedule_executions
SELECT * FROM schedule_executions_keep_059
WHERE id NOT IN (SELECT id FROM schedule_executions);
DROP TABLE schedule_executions_keep_059;
