-- SQLite doesn't support DROP COLUMN in older versions.
-- This is a best-effort rollback.
DROP INDEX IF EXISTS idx_task_executions_claimed;
