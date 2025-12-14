-- PostgreSQL migration: Create trigger tables for event-based workflow scheduling
-- UUID stored as native UUID type, TIMESTAMP WITH TIME ZONE for proper timezone handling

-- Trigger scheduling configuration and state tracking
CREATE TABLE trigger_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trigger_name VARCHAR NOT NULL UNIQUE,         -- Unique identifier for the trigger
    workflow_name VARCHAR NOT NULL,               -- Workflow to execute when trigger fires
    poll_interval_ms INTEGER NOT NULL,            -- Poll interval in milliseconds
    allow_concurrent BOOLEAN NOT NULL DEFAULT false, -- Whether to allow concurrent executions
    enabled BOOLEAN NOT NULL DEFAULT true,        -- Whether the trigger is active
    last_poll_at TIMESTAMP,                       -- Last time the trigger was polled
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Index for efficient polling of triggers
CREATE INDEX idx_trigger_schedules_polling
ON trigger_schedules (enabled, last_poll_at)
WHERE enabled = true;

-- Index for workflow lookup
CREATE INDEX idx_trigger_schedules_workflow
ON trigger_schedules (workflow_name, enabled);

-- Trigger execution tracking for deduplication and audit trail
CREATE TABLE trigger_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trigger_name VARCHAR NOT NULL,                -- Name of the trigger that fired
    context_hash VARCHAR NOT NULL,                -- Hash of the context for deduplication
    pipeline_execution_id UUID REFERENCES pipeline_executions(id) ON DELETE CASCADE,
    started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    completed_at TIMESTAMP,                       -- NULL while execution is in progress
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
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
