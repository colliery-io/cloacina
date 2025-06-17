-- Complete execution system tables with all fields

-- Pipeline execution tracking
CREATE TABLE pipeline_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_name VARCHAR NOT NULL,
    pipeline_version VARCHAR NOT NULL DEFAULT '1.0',
    status VARCHAR NOT NULL CHECK (status IN ('Pending', 'Running', 'Completed', 'Failed', 'Cancelled')),
    context_id UUID REFERENCES contexts(id),
    started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    error_details TEXT,
    recovery_attempts INTEGER DEFAULT 0 NOT NULL,
    last_recovery_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Individual task execution tracking (with all retry and recovery fields)
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
    retry_at TIMESTAMP,
    last_error TEXT,
    recovery_attempts INTEGER DEFAULT 0 NOT NULL,
    last_recovery_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Task execution metadata for automated context merging
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

-- Indexes for efficient querying
CREATE INDEX task_executions_status_idx ON task_executions(status);
CREATE INDEX task_executions_pipeline_idx ON task_executions(pipeline_execution_id);
CREATE INDEX task_executions_task_name_idx ON task_executions(task_name);
CREATE INDEX task_executions_running_idx ON task_executions(status) WHERE status = 'Running';
CREATE INDEX pipeline_executions_status_idx ON pipeline_executions(status);
CREATE INDEX pipeline_executions_name_version_idx ON pipeline_executions(pipeline_name, pipeline_version);
CREATE INDEX task_execution_metadata_pipeline_idx ON task_execution_metadata(pipeline_execution_id);
CREATE INDEX task_execution_metadata_lookup_idx ON task_execution_metadata(pipeline_execution_id, task_name);
CREATE INDEX task_execution_metadata_context_idx ON task_execution_metadata(context_id);
CREATE INDEX recovery_events_pipeline_idx ON recovery_events(pipeline_execution_id);
CREATE INDEX recovery_events_task_idx ON recovery_events(task_execution_id) WHERE task_execution_id IS NOT NULL;
CREATE INDEX recovery_events_type_idx ON recovery_events(recovery_type);
