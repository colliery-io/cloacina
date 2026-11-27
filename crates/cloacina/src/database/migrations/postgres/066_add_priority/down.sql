DROP INDEX IF EXISTS idx_task_executions_waiting_order;
ALTER TABLE task_executions DROP COLUMN task_priority;
ALTER TABLE task_executions DROP COLUMN workflow_priority;
ALTER TABLE workflow_executions DROP COLUMN priority;
ALTER TABLE workflow_executions DROP COLUMN priority_asked;
