-- CLOACINA-T-1091: the end of the output of each attempt of a task. See the
-- sibling postgres migration 078.
CREATE TABLE task_attempt_outputs (
    id BLOB PRIMARY KEY NOT NULL,
    task_execution_id BLOB NOT NULL REFERENCES task_executions(id),
    workflow_execution_id BLOB NOT NULL,
    attempt INTEGER NOT NULL,
    output TEXT NOT NULL,
    total_bytes BIGINT NOT NULL,
    cut BOOLEAN NOT NULL,
    recorded_at TEXT NOT NULL
);

CREATE INDEX idx_task_attempt_outputs_task
    ON task_attempt_outputs (task_execution_id, attempt);

CREATE INDEX idx_task_attempt_outputs_workflow
    ON task_attempt_outputs (workflow_execution_id);

CREATE INDEX idx_task_attempt_outputs_recorded
    ON task_attempt_outputs (recorded_at);
