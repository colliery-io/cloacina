-- Add pause support to pipeline_executions
-- Allows workflows to be paused mid-execution and resumed later
--
-- SQLite doesn't support ALTER TABLE ... DROP CONSTRAINT, so we recreate the table

-- Create new table with updated CHECK constraint and pause columns
CREATE TABLE pipeline_executions_new (
    id BLOB PRIMARY KEY NOT NULL,
    pipeline_name TEXT NOT NULL,
    pipeline_version TEXT NOT NULL DEFAULT '1.0',
    status TEXT NOT NULL CHECK (status IN ('Pending', 'Running', 'Completed', 'Failed', 'Cancelled', 'Paused')),
    context_id BLOB REFERENCES contexts(id),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    error_details TEXT,
    recovery_attempts INTEGER DEFAULT 0 NOT NULL,
    last_recovery_at TEXT,
    paused_at TEXT,
    pause_reason TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Copy existing data
INSERT INTO pipeline_executions_new (
    id, pipeline_name, pipeline_version, status, context_id,
    started_at, completed_at, error_details, recovery_attempts,
    last_recovery_at, created_at, updated_at
)
SELECT
    id, pipeline_name, pipeline_version, status, context_id,
    started_at, completed_at, error_details, recovery_attempts,
    last_recovery_at, created_at, updated_at
FROM pipeline_executions;

-- Drop old table
DROP TABLE pipeline_executions;

-- Rename new table
ALTER TABLE pipeline_executions_new RENAME TO pipeline_executions;

-- Recreate indexes
CREATE INDEX pipeline_executions_status_idx ON pipeline_executions(status);
CREATE INDEX pipeline_executions_name_version_idx ON pipeline_executions(pipeline_name, pipeline_version);
CREATE INDEX pipeline_executions_paused_idx ON pipeline_executions(status) WHERE status = 'Paused';
