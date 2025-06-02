-- SQLite migration: Create cron_executions table for execution audit trail
-- UUID stored as BLOB (16 bytes), TIMESTAMP stored as TEXT (RFC3339 format)

-- Cron execution audit trail for duplicate prevention and observability
CREATE TABLE cron_executions (
    id BLOB PRIMARY KEY NOT NULL,
    schedule_id BLOB NOT NULL REFERENCES cron_schedules(id) ON DELETE CASCADE,
    pipeline_execution_id BLOB REFERENCES pipeline_executions(id) ON DELETE CASCADE,
    scheduled_time TEXT NOT NULL,              -- The original scheduled execution time
    claimed_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    -- Prevent duplicate executions for the same schedule at the same time
    UNIQUE(schedule_id, scheduled_time)
);

-- Index for efficient lookups by schedule
CREATE INDEX idx_cron_executions_schedule
ON cron_executions (schedule_id, scheduled_time DESC);

-- Index for pipeline execution correlation (only for non-null values)
CREATE INDEX idx_cron_executions_pipeline
ON cron_executions (pipeline_execution_id) WHERE pipeline_execution_id IS NOT NULL;

-- Index for time-based queries
CREATE INDEX idx_cron_executions_claimed_at
ON cron_executions (claimed_at DESC);
