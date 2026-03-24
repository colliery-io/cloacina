-- Add heartbeat columns for task recovery.
-- claimed_by: executor identity (instance UUID or future worker UUID)
-- heartbeat_at: last heartbeat timestamp (stale = orphaned)
ALTER TABLE task_executions ADD COLUMN claimed_by TEXT NULL;
ALTER TABLE task_executions ADD COLUMN heartbeat_at TEXT NULL;

-- Runner instances for multi-instance coordination
CREATE TABLE IF NOT EXISTS runner_instances (
    id TEXT PRIMARY KEY NOT NULL,
    started_at TEXT NOT NULL,
    last_heartbeat_at TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    mode TEXT NOT NULL DEFAULT 'all'
);
