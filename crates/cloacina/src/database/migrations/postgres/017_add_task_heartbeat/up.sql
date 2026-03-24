-- Add heartbeat columns for task recovery.
-- claimed_by: executor identity (instance UUID or future worker UUID)
-- heartbeat_at: last heartbeat timestamp (stale = orphaned)
ALTER TABLE task_executions ADD COLUMN claimed_by TEXT NULL;
ALTER TABLE task_executions ADD COLUMN heartbeat_at TIMESTAMP NULL;

-- Partial index for recovery sweeper: only scans Running tasks
CREATE INDEX idx_task_exec_orphan_scan
    ON task_executions(status, heartbeat_at)
    WHERE status = 'Running';

-- Runner instances for multi-instance coordination
CREATE TABLE IF NOT EXISTS runner_instances (
    id UUID PRIMARY KEY NOT NULL,
    started_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_heartbeat_at TIMESTAMP NOT NULL DEFAULT NOW(),
    status TEXT NOT NULL DEFAULT 'active',
    mode TEXT NOT NULL DEFAULT 'all'
);
