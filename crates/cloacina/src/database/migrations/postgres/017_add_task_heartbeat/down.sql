DROP INDEX IF EXISTS idx_task_exec_orphan_scan;
ALTER TABLE task_executions DROP COLUMN IF EXISTS claimed_by;
ALTER TABLE task_executions DROP COLUMN IF EXISTS heartbeat_at;
DROP TABLE IF EXISTS runner_instances;
