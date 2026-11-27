-- CLOACINA-T-1027: a task names an agent pool and requests CPU and memory;
-- an agent reports its total CPU and memory; the fleet packs by them.

-- What an agent can hold in all, as it reported at registration. NULL when
-- the agent reported none: it is then packed by its slot count only.
ALTER TABLE fleet_agents ADD COLUMN cpu_millis INTEGER;
ALTER TABLE fleet_agents ADD COLUMN memory_bytes INTEGER;

-- The pool the task runs in (NULL means the pool 'default') and its requests
-- (NULL means none), written when the task row is created. The waiting gauges
-- group by pool.
ALTER TABLE task_executions ADD COLUMN pool TEXT;
ALTER TABLE task_executions ADD COLUMN cpu_request_millis INTEGER;
ALTER TABLE task_executions ADD COLUMN memory_request_bytes INTEGER;

-- The agent the fleet placed the task on, for as long as the dispatch holds
-- its claim. The requests in flight on an agent are the sum over these rows.
ALTER TABLE task_executions ADD COLUMN fleet_agent_id TEXT;
CREATE INDEX idx_task_executions_fleet_agent_id
    ON task_executions (fleet_agent_id)
    WHERE fleet_agent_id IS NOT NULL;
