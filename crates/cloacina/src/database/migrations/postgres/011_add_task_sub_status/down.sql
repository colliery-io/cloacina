-- Revert sub_status column from task_executions
ALTER TABLE task_executions DROP COLUMN IF EXISTS sub_status;
