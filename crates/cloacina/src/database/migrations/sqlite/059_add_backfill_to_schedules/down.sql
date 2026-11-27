-- Reverse CLOACINA-T-1020 backfill: build the table again with the older
-- type check and without the `backfill` column. The backfill rows go, with
-- their schedule execution rows.
DELETE FROM schedule_executions
WHERE schedule_id IN (SELECT id FROM schedules WHERE schedule_type = 'backfill');
DELETE FROM schedules WHERE schedule_type = 'backfill';

CREATE TABLE schedule_executions_keep_059 AS SELECT * FROM schedule_executions;

CREATE TABLE schedules_new (
    id BLOB PRIMARY KEY NOT NULL,
    schedule_type TEXT NOT NULL CHECK (schedule_type IN ('cron', 'trigger')),
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
    max_active_runs INTEGER
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

INSERT INTO schedule_executions
SELECT * FROM schedule_executions_keep_059
WHERE id NOT IN (SELECT id FROM schedule_executions);
DROP TABLE schedule_executions_keep_059;
