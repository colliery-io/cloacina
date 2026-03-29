-- Add claiming columns to task_executions for horizontal scaling.
-- claimed_by: UUID of the runner instance that claimed this task (NULL = unclaimed)
-- heartbeat_at: timestamp of the last heartbeat from the claiming runner (NULL = no heartbeat)

ALTER TABLE task_executions ADD COLUMN claimed_by UUID;
ALTER TABLE task_executions ADD COLUMN heartbeat_at TIMESTAMPTZ;

-- Index for finding stale claims (heartbeat older than threshold)
CREATE INDEX idx_task_executions_claimed ON task_executions(claimed_by) WHERE claimed_by IS NOT NULL;
