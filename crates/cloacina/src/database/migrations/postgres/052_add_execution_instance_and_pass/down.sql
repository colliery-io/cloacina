DROP INDEX IF EXISTS idx_workflow_executions_pass_id;
DROP INDEX IF EXISTS idx_workflow_executions_instance_name;
ALTER TABLE workflow_executions DROP COLUMN pass_id;
ALTER TABLE workflow_executions DROP COLUMN instance_name;
