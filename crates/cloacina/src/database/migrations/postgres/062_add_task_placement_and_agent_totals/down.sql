DROP INDEX IF EXISTS idx_task_executions_fleet_agent_id;
ALTER TABLE task_executions DROP COLUMN fleet_agent_id;
ALTER TABLE task_executions DROP COLUMN memory_request_bytes;
ALTER TABLE task_executions DROP COLUMN cpu_request_millis;
ALTER TABLE task_executions DROP COLUMN pool;
ALTER TABLE fleet_agents DROP COLUMN memory_bytes;
ALTER TABLE fleet_agents DROP COLUMN cpu_millis;
