-- CLOACINA-T-1100: the error of each failed attempt of a task.
--
-- `task_executions` has one row per task with an attempt counter, and its
-- `last_error` / `error_details` hold the last failure only. This table keeps
-- one row per failed attempt: the message, and the details (a Python
-- traceback) cut to 16 KiB as for `error_details` (CLOACINA-T-1024).
--
-- No unique key on (task_execution_id, attempt): a reset of the retry state
-- sets the attempt back to 1, so a number can come again. Rows are read in
-- (attempt, failed_at) order. Run-history retention deletes them with their
-- run (CLOACINA-T-1023).
CREATE TABLE task_attempt_errors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_execution_id UUID NOT NULL REFERENCES task_executions(id),
    workflow_execution_id UUID NOT NULL,
    attempt INTEGER NOT NULL,
    message TEXT NOT NULL,
    details TEXT NULL,
    failed_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_task_attempt_errors_task
    ON task_attempt_errors (task_execution_id, attempt);

CREATE INDEX idx_task_attempt_errors_workflow
    ON task_attempt_errors (workflow_execution_id);
