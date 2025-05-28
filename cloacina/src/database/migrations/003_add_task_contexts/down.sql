-- Drop indexes first
DROP INDEX IF EXISTS task_execution_metadata_context_idx;
DROP INDEX IF EXISTS task_execution_metadata_lookup_idx;
DROP INDEX IF EXISTS task_execution_metadata_pipeline_idx;

-- Drop table
DROP TABLE IF EXISTS task_execution_metadata;

-- Remove added columns from task_executions
ALTER TABLE task_executions DROP COLUMN IF EXISTS retry_at;
ALTER TABLE task_executions DROP COLUMN IF EXISTS last_error;
