ALTER TABLE task_executions DROP COLUMN IF EXISTS claimed_by;
ALTER TABLE task_executions DROP COLUMN IF EXISTS heartbeat_at;
DROP INDEX IF EXISTS idx_task_executions_claimed;
