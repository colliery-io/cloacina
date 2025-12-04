-- SQLite version: Complete execution system tables with all fields
-- UUID stored as BLOB (16 bytes), TIMESTAMP stored as TEXT (RFC3339 format)

-- Pipeline execution tracking
CREATE TABLE pipeline_executions (
    id BLOB PRIMARY KEY NOT NULL,
    pipeline_name TEXT NOT NULL,
    pipeline_version TEXT NOT NULL DEFAULT '1.0',
    status TEXT NOT NULL CHECK (status IN ('Pending', 'Running', 'Completed', 'Failed', 'Cancelled')),
    context_id BLOB REFERENCES contexts(id),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    error_details TEXT,
    recovery_attempts INTEGER DEFAULT 0 NOT NULL,
    last_recovery_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Individual task execution tracking (with all retry and recovery fields)
CREATE TABLE task_executions (
    id BLOB PRIMARY KEY NOT NULL,
    pipeline_execution_id BLOB NOT NULL REFERENCES pipeline_executions(id),
    task_name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('NotStarted', 'Ready', 'Running', 'Completed', 'Failed', 'Skipped')),
    started_at TEXT,
    completed_at TEXT,
    attempt INTEGER DEFAULT 1,
    max_attempts INTEGER DEFAULT 1,
    error_details TEXT,
    trigger_rules TEXT DEFAULT '{"type": "Always"}' CHECK (trigger_rules IS NULL OR json_valid(trigger_rules)),
    task_configuration TEXT DEFAULT '{}' CHECK (task_configuration IS NULL OR json_valid(task_configuration)),
    retry_at TEXT,
    last_error TEXT,
    recovery_attempts INTEGER DEFAULT 0 NOT NULL,
    last_recovery_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Task execution metadata for automated context merging
CREATE TABLE task_execution_metadata (
    id BLOB PRIMARY KEY NOT NULL,
    task_execution_id BLOB NOT NULL REFERENCES task_executions(id),
    pipeline_execution_id BLOB NOT NULL REFERENCES pipeline_executions(id),
    task_name TEXT NOT NULL,
    context_id BLOB REFERENCES contexts(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    -- Constraints
    UNIQUE(task_execution_id),                    -- One metadata per task execution
    UNIQUE(pipeline_execution_id, task_name)      -- Unique task name per pipeline
);

-- Recovery audit trail for debugging and monitoring
CREATE TABLE recovery_events (
    id BLOB PRIMARY KEY NOT NULL,
    pipeline_execution_id BLOB NOT NULL REFERENCES pipeline_executions(id),
    task_execution_id BLOB REFERENCES task_executions(id),
    recovery_type TEXT NOT NULL, -- 'task_reset', 'task_abandoned', 'pipeline_failed'
    recovered_at TEXT NOT NULL,
    details TEXT CHECK (details IS NULL OR json_valid(details)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
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
