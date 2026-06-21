-- CLOACI-T-0749 v1: universal pause (new-execution gating only).
--
-- Pausing a schedule (cron or trigger) stops the scheduler from firing it.
-- Pausing a workflow package blocks new executions of that workflow regardless
-- of source. In-flight executions are unaffected. `paused` is transient
-- operator state, distinct from `enabled` (deliberate on/off). ADD COLUMN only
-- — no table rewrite. (CLOACI-T-0749)
ALTER TABLE schedules ADD COLUMN paused BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE schedules ADD COLUMN paused_at TIMESTAMP;
CREATE INDEX idx_schedules_paused ON schedules (paused) WHERE paused = true;

ALTER TABLE workflow_packages ADD COLUMN paused BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE workflow_packages ADD COLUMN paused_at TIMESTAMP;
CREATE INDEX idx_workflow_packages_paused ON workflow_packages (paused) WHERE paused = true;
