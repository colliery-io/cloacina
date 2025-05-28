-- Rollback recovery support

-- Drop recovery events table and indexes
DROP INDEX IF EXISTS recovery_events_type_idx;
DROP INDEX IF EXISTS recovery_events_task_idx;
DROP INDEX IF EXISTS recovery_events_pipeline_idx;
DROP TABLE IF EXISTS recovery_events;

-- Drop task execution recovery indexes
DROP INDEX IF EXISTS task_executions_running_idx;

-- Remove recovery fields from task_executions
ALTER TABLE task_executions DROP COLUMN IF EXISTS last_recovery_at;
ALTER TABLE task_executions DROP COLUMN IF EXISTS recovery_attempts;

-- Remove recovery fields from pipeline_executions
ALTER TABLE pipeline_executions DROP COLUMN IF EXISTS last_recovery_at;
ALTER TABLE pipeline_executions DROP COLUMN IF EXISTS recovery_attempts;
