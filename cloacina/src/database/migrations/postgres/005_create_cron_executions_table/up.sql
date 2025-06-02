-- PostgreSQL migration: Create cron_executions table for execution audit trail
-- This table tracks every handoff from cron scheduler to pipeline executor

-- Cron execution audit trail for duplicate prevention and observability
CREATE TABLE cron_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    schedule_id UUID NOT NULL REFERENCES cron_schedules(id) ON DELETE CASCADE,
    pipeline_execution_id UUID REFERENCES pipeline_executions(id) ON DELETE CASCADE,
    scheduled_time TIMESTAMP NOT NULL,              -- The original scheduled execution time
    claimed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,

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
