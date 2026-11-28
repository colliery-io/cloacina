-- CLOACINA-T-1091: the end of what each attempt of a task wrote on stdout
-- and stderr, as one stream, kept with its result.
--
-- One row per attempt that reported output: success and failure alike. The
-- agent keeps the last bytes (the server's `--task-output-max-bytes`, 256 KiB
-- by default) and the server cuts again to that cap. `total_bytes` is what the
-- attempt wrote in all; `cut` is true when `output` is not the whole stream.
--
-- Not `task_attempt_errors` (migration 070): that table holds failed attempts
-- only and goes with its run. These rows have their own retention period
-- (`--task-output-retention-days`, 30 days by default), and run-history
-- retention deletes them with their run before the task rows.
--
-- No unique key on (task_execution_id, attempt), as for 070: a reset of the
-- retry state sets the attempt back to 1.
CREATE TABLE task_attempt_outputs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_execution_id UUID NOT NULL REFERENCES task_executions(id),
    workflow_execution_id UUID NOT NULL,
    attempt INTEGER NOT NULL,
    output TEXT NOT NULL,
    total_bytes BIGINT NOT NULL,
    cut BOOLEAN NOT NULL,
    recorded_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_task_attempt_outputs_task
    ON task_attempt_outputs (task_execution_id, attempt);

CREATE INDEX idx_task_attempt_outputs_workflow
    ON task_attempt_outputs (workflow_execution_id);

CREATE INDEX idx_task_attempt_outputs_recorded
    ON task_attempt_outputs (recorded_at);
