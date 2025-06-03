-- SQLite migration rollback: Drop cron_executions table

-- Drop indexes first
DROP INDEX IF EXISTS idx_cron_executions_claimed_at;
DROP INDEX IF EXISTS idx_cron_executions_pipeline;
DROP INDEX IF EXISTS idx_cron_executions_schedule;

-- Drop the table
DROP TABLE IF EXISTS cron_executions;
