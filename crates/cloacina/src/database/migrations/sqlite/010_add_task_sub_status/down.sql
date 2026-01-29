-- SQLite doesn't support DROP COLUMN before 3.35.0.
-- Recreate table without sub_status.

CREATE TABLE task_executions_new (
    id BLOB PRIMARY KEY NOT NULL,
    pipeline_execution_id BLOB NOT NULL REFERENCES pipeline_executions(id),
    task_name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('NotStarted', 'Ready', 'Running', 'Completed', 'Failed', 'Skipped')),
    started_at TEXT,
    completed_at TEXT,
    attempt INTEGER DEFAULT 1,
    max_attempts INTEGER DEFAULT 1,
    error_details TEXT,
    trigger_rules TEXT DEFAULT '{"type": "Always"}',
    task_configuration TEXT DEFAULT '{}',
    retry_at TEXT,
    last_error TEXT,
    recovery_attempts INTEGER DEFAULT 0 NOT NULL,
    last_recovery_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT INTO task_executions_new (
    id, pipeline_execution_id, task_name, status, started_at, completed_at,
    attempt, max_attempts, error_details, trigger_rules, task_configuration,
    retry_at, last_error, recovery_attempts, last_recovery_at, created_at, updated_at
)
SELECT
    id, pipeline_execution_id, task_name, status, started_at, completed_at,
    attempt, max_attempts, error_details, trigger_rules, task_configuration,
    retry_at, last_error, recovery_attempts, last_recovery_at, created_at, updated_at
FROM task_executions;

DROP TABLE task_executions;
ALTER TABLE task_executions_new RENAME TO task_executions;

CREATE INDEX task_executions_pipeline_id_idx ON task_executions(pipeline_execution_id);
CREATE INDEX task_executions_status_idx ON task_executions(status);
