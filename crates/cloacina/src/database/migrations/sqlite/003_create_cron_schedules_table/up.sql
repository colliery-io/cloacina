-- SQLite migration: Create cron_schedules table for time-based workflow scheduling
-- UUID stored as BLOB (16 bytes), TIMESTAMP stored as TEXT (RFC3339 format)

-- Cron scheduling configuration and state tracking
CREATE TABLE cron_schedules (
    id BLOB PRIMARY KEY NOT NULL,
    workflow_name TEXT NOT NULL,
    cron_expression TEXT NOT NULL,
    timezone TEXT NOT NULL DEFAULT 'UTC',           -- Timezone for cron interpretation (e.g., 'America/New_York')
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    catchup_policy TEXT NOT NULL DEFAULT 'skip' CHECK (catchup_policy IN ('skip', 'run_once', 'run_all')),
    start_date TEXT,                                -- NULL = immediate start (RFC3339 format)
    end_date TEXT,                                  -- NULL = no end (RFC3339 format)
    next_run_at TEXT NOT NULL,                      -- RFC3339 format
    last_run_at TEXT,                               -- RFC3339 format
    created_at TEXT NOT NULL,                       -- RFC3339 format
    updated_at TEXT NOT NULL                        -- RFC3339 format
);

-- Index for efficient polling of due schedules
CREATE INDEX idx_cron_schedules_polling
ON cron_schedules (enabled, next_run_at)
WHERE enabled = 1;

-- Index for workflow lookup and management
CREATE INDEX idx_cron_schedules_workflow
ON cron_schedules (workflow_name, enabled);

-- Index for time window queries
CREATE INDEX idx_cron_schedules_time_window
ON cron_schedules (start_date, end_date);
