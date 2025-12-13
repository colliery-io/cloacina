-- SQLite migration: Create trigger tables for event-based workflow scheduling
-- UUID stored as BLOB (16 bytes), TIMESTAMP stored as TEXT (RFC3339 format)

-- Trigger scheduling configuration and state tracking
CREATE TABLE trigger_schedules (
    id BLOB PRIMARY KEY NOT NULL,
    trigger_name TEXT NOT NULL UNIQUE,            -- Unique identifier for the trigger
    workflow_name TEXT NOT NULL,                  -- Workflow to execute when trigger fires
    poll_interval_ms INTEGER NOT NULL,            -- Poll interval in milliseconds
    allow_concurrent INTEGER NOT NULL DEFAULT 0 CHECK (allow_concurrent IN (0, 1)),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    last_poll_at TEXT,                            -- Last time the trigger was polled (RFC3339 format)
    created_at TEXT NOT NULL,                     -- RFC3339 format
    updated_at TEXT NOT NULL                      -- RFC3339 format
);

-- Index for efficient polling of triggers
CREATE INDEX idx_trigger_schedules_polling
ON trigger_schedules (enabled, last_poll_at)
WHERE enabled = 1;

-- Index for workflow lookup
CREATE INDEX idx_trigger_schedules_workflow
ON trigger_schedules (workflow_name, enabled);

-- Trigger execution tracking for deduplication and audit trail
CREATE TABLE trigger_executions (
    id BLOB PRIMARY KEY NOT NULL,
    trigger_name TEXT NOT NULL,                   -- Name of the trigger that fired
    context_hash TEXT NOT NULL,                   -- Hash of the context for deduplication
    pipeline_execution_id BLOB REFERENCES pipeline_executions(id) ON DELETE CASCADE,
    started_at TEXT NOT NULL,                     -- RFC3339 format
    completed_at TEXT,                            -- NULL while execution is in progress (RFC3339 format)
    created_at TEXT NOT NULL,                     -- RFC3339 format
    updated_at TEXT NOT NULL                      -- RFC3339 format
);

-- Partial unique index for deduplication: prevent concurrent executions with same context
-- Only enforced when completed_at is NULL (execution is in progress)
CREATE UNIQUE INDEX idx_trigger_executions_dedup
ON trigger_executions (trigger_name, context_hash)
WHERE completed_at IS NULL;

-- Index for efficient lookups by trigger name
CREATE INDEX idx_trigger_executions_trigger
ON trigger_executions (trigger_name, started_at DESC);

-- Index for pipeline execution correlation
CREATE INDEX idx_trigger_executions_pipeline
ON trigger_executions (pipeline_execution_id)
WHERE pipeline_execution_id IS NOT NULL;

-- Index for finding in-progress executions
CREATE INDEX idx_trigger_executions_in_progress
ON trigger_executions (trigger_name, context_hash)
WHERE completed_at IS NULL;

-- Index for time-based queries and cleanup
CREATE INDEX idx_trigger_executions_completed_at
ON trigger_executions (completed_at DESC)
WHERE completed_at IS NOT NULL;
