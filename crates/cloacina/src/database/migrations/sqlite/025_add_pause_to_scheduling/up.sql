-- CLOACI-T-0749 v1: universal pause (new-execution gating only).
--
-- Pausing a schedule (cron or trigger) stops the scheduler from firing it.
-- Pausing a workflow package blocks new executions of that workflow regardless
-- of source. In-flight executions are unaffected. `paused` is transient
-- operator state, distinct from `enabled` (deliberate on/off). ADD COLUMN only.
-- SQLite: bool as INTEGER 0/1, timestamp as TEXT (RFC3339). (CLOACI-T-0749)
ALTER TABLE schedules ADD COLUMN paused INTEGER NOT NULL DEFAULT 0 CHECK (paused IN (0, 1));
ALTER TABLE schedules ADD COLUMN paused_at TEXT;
CREATE INDEX idx_schedules_paused ON schedules (paused) WHERE paused = 1;

ALTER TABLE workflow_packages ADD COLUMN paused INTEGER NOT NULL DEFAULT 0 CHECK (paused IN (0, 1));
ALTER TABLE workflow_packages ADD COLUMN paused_at TEXT;
CREATE INDEX idx_workflow_packages_paused ON workflow_packages (paused) WHERE paused = 1;
