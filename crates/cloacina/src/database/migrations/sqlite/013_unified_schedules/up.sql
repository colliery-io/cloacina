-- SQLite migration: Create unified schedules tables
-- UUID stored as BLOB (16 bytes), TIMESTAMP stored as TEXT (RFC3339 format)

-- Unified schedule configuration for both cron and trigger-based scheduling
CREATE TABLE schedules (
    id BLOB PRIMARY KEY NOT NULL,
    schedule_type TEXT NOT NULL CHECK (schedule_type IN ('cron', 'trigger')),
    workflow_name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),

    -- Cron-specific fields (NULL for trigger type)
    cron_expression TEXT,
    timezone TEXT DEFAULT 'UTC',
    catchup_policy TEXT DEFAULT 'skip' CHECK (catchup_policy IS NULL OR catchup_policy IN ('skip', 'run_all')),
    start_date TEXT,
    end_date TEXT,

    -- Trigger-specific fields (NULL for cron type)
    trigger_name TEXT UNIQUE,
    poll_interval_ms INTEGER,
    allow_concurrent INTEGER DEFAULT 0 CHECK (allow_concurrent IS NULL OR allow_concurrent IN (0, 1)),

    -- Shared scheduling state
    next_run_at TEXT,
    last_run_at TEXT,
    last_poll_at TEXT,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Index for efficient polling of due cron schedules
CREATE INDEX idx_schedules_cron_polling
ON schedules (enabled, next_run_at)
WHERE enabled = 1 AND schedule_type = 'cron';

-- Index for efficient polling of trigger schedules
CREATE INDEX idx_schedules_trigger_polling
ON schedules (enabled, last_poll_at)
WHERE enabled = 1 AND schedule_type = 'trigger';

-- Index for workflow lookup and management
CREATE INDEX idx_schedules_workflow
ON schedules (workflow_name, enabled);

-- Index for schedule type filtering
CREATE INDEX idx_schedules_type
ON schedules (schedule_type);

-- Unified schedule execution audit trail
CREATE TABLE schedule_executions (
    id BLOB PRIMARY KEY NOT NULL,
    schedule_id BLOB NOT NULL REFERENCES schedules(id) ON DELETE CASCADE,
    pipeline_execution_id BLOB REFERENCES pipeline_executions(id) ON DELETE CASCADE,

    -- Cron-specific (NULL for trigger executions)
    scheduled_time TEXT,
    claimed_at TEXT,

    -- Trigger-specific (NULL for cron executions)
    context_hash TEXT,

    started_at TEXT NOT NULL,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Prevent duplicate cron executions for the same schedule at the same time
CREATE UNIQUE INDEX idx_schedule_executions_cron_dedup
ON schedule_executions (schedule_id, scheduled_time)
WHERE scheduled_time IS NOT NULL;

-- Prevent concurrent trigger executions with same context (when not allowed)
CREATE UNIQUE INDEX idx_schedule_executions_trigger_dedup
ON schedule_executions (schedule_id, context_hash)
WHERE context_hash IS NOT NULL AND completed_at IS NULL;

-- Index for efficient lookups by schedule
CREATE INDEX idx_schedule_executions_schedule
ON schedule_executions (schedule_id, started_at DESC);

-- Index for pipeline execution correlation
CREATE INDEX idx_schedule_executions_pipeline
ON schedule_executions (pipeline_execution_id)
WHERE pipeline_execution_id IS NOT NULL;

-- Index for finding in-progress executions
CREATE INDEX idx_schedule_executions_in_progress
ON schedule_executions (schedule_id)
WHERE completed_at IS NULL;

-- Data migration: copy cron_schedules → schedules
INSERT INTO schedules (id, schedule_type, workflow_name, enabled, cron_expression, timezone, catchup_policy, start_date, end_date, next_run_at, last_run_at, created_at, updated_at)
SELECT id, 'cron', workflow_name, enabled, cron_expression, timezone, catchup_policy, start_date, end_date, next_run_at, last_run_at, created_at, updated_at
FROM cron_schedules;

-- Data migration: copy trigger_schedules → schedules
INSERT INTO schedules (id, schedule_type, workflow_name, enabled, trigger_name, poll_interval_ms, allow_concurrent, last_poll_at, created_at, updated_at)
SELECT id, 'trigger', workflow_name, enabled, trigger_name, poll_interval_ms, allow_concurrent, last_poll_at, created_at, updated_at
FROM trigger_schedules;

-- Data migration: copy cron_executions → schedule_executions
INSERT INTO schedule_executions (id, schedule_id, pipeline_execution_id, scheduled_time, claimed_at, started_at, created_at, updated_at)
SELECT id, schedule_id, pipeline_execution_id, scheduled_time, claimed_at, claimed_at, created_at, updated_at
FROM cron_executions;

-- Data migration: copy trigger_executions → schedule_executions
INSERT INTO schedule_executions (id, schedule_id, pipeline_execution_id, context_hash, started_at, completed_at, created_at, updated_at)
SELECT trig_exec.id, trig_sched.id, trig_exec.pipeline_execution_id, trig_exec.context_hash, trig_exec.started_at, trig_exec.completed_at, trig_exec.created_at, trig_exec.updated_at
FROM trigger_executions trig_exec
JOIN trigger_schedules trig_sched ON trig_exec.trigger_name = trig_sched.trigger_name;
