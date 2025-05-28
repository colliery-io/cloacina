-- Pipeline execution tracking
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE pipeline_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_name VARCHAR NOT NULL,
    pipeline_version VARCHAR NOT NULL DEFAULT '1.0',
    status VARCHAR NOT NULL CHECK (status IN ('Pending', 'Running', 'Completed', 'Failed', 'Cancelled')),
    context_id UUID REFERENCES contexts(id),
    started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    error_details TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Individual task execution tracking
CREATE TABLE task_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_execution_id UUID NOT NULL REFERENCES pipeline_executions(id),
    task_name VARCHAR NOT NULL,
    status VARCHAR NOT NULL CHECK (status IN ('NotStarted', 'Ready', 'Running', 'Completed', 'Failed', 'Skipped')),
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    attempt INTEGER DEFAULT 1,
    max_attempts INTEGER DEFAULT 1,
    error_details TEXT,
    trigger_rules TEXT DEFAULT '{"type": "Always"}' CHECK (trigger_rules IS NULL OR trigger_rules::json IS NOT NULL),
    task_configuration TEXT DEFAULT '{}' CHECK (task_configuration IS NULL OR task_configuration::json IS NOT NULL),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Indexes for efficient querying
CREATE INDEX task_executions_status_idx ON task_executions(status);
CREATE INDEX task_executions_pipeline_idx ON task_executions(pipeline_execution_id);
CREATE INDEX task_executions_task_name_idx ON task_executions(task_name);
CREATE INDEX pipeline_executions_status_idx ON pipeline_executions(status);
CREATE INDEX pipeline_executions_name_version_idx ON pipeline_executions(pipeline_name, pipeline_version);
