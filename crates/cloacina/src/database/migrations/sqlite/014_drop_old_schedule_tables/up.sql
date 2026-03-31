-- Drop old cron and trigger tables, replaced by unified schedules/schedule_executions tables
DROP TABLE IF EXISTS cron_executions;
DROP TABLE IF EXISTS trigger_executions;
DROP TABLE IF EXISTS cron_schedules;
DROP TABLE IF EXISTS trigger_schedules;
