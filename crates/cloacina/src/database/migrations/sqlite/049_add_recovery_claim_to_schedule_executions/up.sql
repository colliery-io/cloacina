-- CLOACI-T-0926: make cron recovery durable and single-owner.
-- See the sibling postgres migration 049 for the full rationale.
--
-- UUID is stored as BLOB (16 bytes), TIMESTAMP as TEXT (RFC3339) per the
-- SQLite conventions in 013_unified_schedules.
--
-- ADD COLUMN only (constant defaults, so SQLite accepts them in place) — no
-- table rebuild, per the project's SQLite migration rule.
ALTER TABLE schedule_executions ADD COLUMN recovery_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE schedule_executions ADD COLUMN recovery_claimed_by BLOB;
ALTER TABLE schedule_executions ADD COLUMN recovery_heartbeat_at TEXT;

CREATE INDEX idx_schedule_executions_recovery_claim
ON schedule_executions (recovery_heartbeat_at)
WHERE completed_at IS NULL AND recovery_claimed_by IS NOT NULL;
