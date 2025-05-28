-- Migration: Add recovery tracking and audit trail support

-- Add recovery tracking fields to pipeline_executions
ALTER TABLE pipeline_executions ADD COLUMN recovery_attempts INTEGER DEFAULT 0 NOT NULL;
ALTER TABLE pipeline_executions ADD COLUMN last_recovery_at TIMESTAMP;

-- Add recovery tracking fields to task_executions
ALTER TABLE task_executions ADD COLUMN recovery_attempts INTEGER DEFAULT 0 NOT NULL;
ALTER TABLE task_executions ADD COLUMN last_recovery_at TIMESTAMP;

-- Recovery audit trail for debugging and monitoring
CREATE TABLE recovery_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_execution_id UUID NOT NULL REFERENCES pipeline_executions(id),
    task_execution_id UUID REFERENCES task_executions(id),
    recovery_type VARCHAR NOT NULL, -- 'task_reset', 'task_abandoned', 'pipeline_failed'
    recovered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    details TEXT CHECK (details IS NULL OR details::json IS NOT NULL),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Indexes for efficient recovery queries
CREATE INDEX task_executions_running_idx ON task_executions(status) WHERE status = 'Running';
CREATE INDEX recovery_events_pipeline_idx ON recovery_events(pipeline_execution_id);
CREATE INDEX recovery_events_task_idx ON recovery_events(task_execution_id) WHERE task_execution_id IS NOT NULL;
CREATE INDEX recovery_events_type_idx ON recovery_events(recovery_type);
