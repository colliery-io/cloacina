-- Reverse CLOACI-T-0926 durable recovery accounting + CAS claim.
DROP INDEX IF EXISTS idx_schedule_executions_recovery_claim;
ALTER TABLE schedule_executions DROP COLUMN recovery_heartbeat_at;
ALTER TABLE schedule_executions DROP COLUMN recovery_claimed_by;
ALTER TABLE schedule_executions DROP COLUMN recovery_attempts;
