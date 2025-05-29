-- Drop all execution tables in reverse dependency order
DROP TABLE IF EXISTS recovery_events;
DROP TABLE IF EXISTS task_execution_metadata;
DROP TABLE IF EXISTS task_executions;
DROP TABLE IF EXISTS pipeline_executions;
