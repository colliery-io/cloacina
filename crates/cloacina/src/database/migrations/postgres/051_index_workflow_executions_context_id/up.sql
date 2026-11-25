-- Index for run-history retention (CLOACINA-T-1023).
--
-- Retention deletes `contexts` rows of old finished runs. Each such delete
-- makes the database check the foreign key from
-- `workflow_executions.context_id`; with no index on that column the check
-- scans the whole table, once per context. `task_execution_metadata.context_id`
-- already has its index.
CREATE INDEX IF NOT EXISTS workflow_executions_context_idx
    ON workflow_executions (context_id);
