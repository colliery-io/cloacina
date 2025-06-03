-- PostgreSQL migration: Create cron_schedules table for time-based workflow scheduling
-- UUID stored as native UUID type, TIMESTAMP WITH TIME ZONE for proper timezone handling

-- Cron scheduling configuration and state tracking
CREATE TABLE cron_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_name VARCHAR NOT NULL,
    cron_expression VARCHAR NOT NULL,
    timezone VARCHAR NOT NULL DEFAULT 'UTC',        -- Timezone for cron interpretation (e.g., 'America/New_York')
    enabled BOOLEAN NOT NULL DEFAULT true,
    catchup_policy VARCHAR NOT NULL DEFAULT 'skip' CHECK (catchup_policy IN ('skip', 'run_all')),
    start_date TIMESTAMP,                           -- NULL = immediate start
    end_date TIMESTAMP,                             -- NULL = no end
    next_run_at TIMESTAMP NOT NULL,
    last_run_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Index for efficient polling of due schedules
CREATE INDEX idx_cron_schedules_polling
ON cron_schedules (enabled, next_run_at)
WHERE enabled = true;

-- Index for workflow lookup and management
CREATE INDEX idx_cron_schedules_workflow
ON cron_schedules (workflow_name, enabled);

-- Index for time window queries
CREATE INDEX idx_cron_schedules_time_window
ON cron_schedules (start_date, end_date);
