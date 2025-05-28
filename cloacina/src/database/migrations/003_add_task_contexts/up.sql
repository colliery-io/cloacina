-- Migration: Add task execution metadata for automated context merging

-- Task execution metadata for automated merging
CREATE TABLE task_execution_metadata (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_execution_id UUID NOT NULL REFERENCES task_executions(id),
    pipeline_execution_id UUID NOT NULL REFERENCES pipeline_executions(id),
    task_name VARCHAR NOT NULL,
    context_id UUID REFERENCES contexts(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    -- Constraints
    UNIQUE(task_execution_id),                    -- One metadata per task execution
    UNIQUE(pipeline_execution_id, task_name)      -- Unique task name per pipeline
);

-- Indexes for efficient lazy loading
CREATE INDEX task_execution_metadata_pipeline_idx ON task_execution_metadata(pipeline_execution_id);
CREATE INDEX task_execution_metadata_lookup_idx ON task_execution_metadata(pipeline_execution_id, task_name);
CREATE INDEX task_execution_metadata_context_idx ON task_execution_metadata(context_id);

-- Add retry fields to existing task_executions table
ALTER TABLE task_executions ADD COLUMN retry_at TIMESTAMP;
ALTER TABLE task_executions ADD COLUMN last_error TEXT;
