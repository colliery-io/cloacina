-- Drop indexes first
DROP INDEX IF EXISTS pipeline_executions_name_version_idx;
DROP INDEX IF EXISTS pipeline_executions_status_idx;
DROP INDEX IF EXISTS task_executions_task_name_idx;
DROP INDEX IF EXISTS task_executions_pipeline_idx;
DROP INDEX IF EXISTS task_executions_status_idx;

-- Drop tables in reverse dependency order
DROP TABLE IF EXISTS task_executions;
DROP TABLE IF EXISTS pipeline_executions;
