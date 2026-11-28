-- CLOACINA-T-1100: the error of each failed attempt of a task. See the
-- sibling postgres migration 070.
CREATE TABLE task_attempt_errors (
    id BLOB PRIMARY KEY NOT NULL,
    task_execution_id BLOB NOT NULL REFERENCES task_executions(id),
    workflow_execution_id BLOB NOT NULL,
    attempt INTEGER NOT NULL,
    message TEXT NOT NULL,
    details TEXT NULL,
    failed_at TEXT NOT NULL
);

CREATE INDEX idx_task_attempt_errors_task
    ON task_attempt_errors (task_execution_id, attempt);

CREATE INDEX idx_task_attempt_errors_workflow
    ON task_attempt_errors (workflow_execution_id);
