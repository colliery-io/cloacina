-- CLOACI-T-0926: make cron recovery durable and single-owner.
--
-- Two residuals from CLOACI-T-0914 (#229), both on the cron recovery path:
--
-- 1. The attempt cap lived in `CronRecoveryService.recovery_attempts`, an
--    in-process HashMap. Every restart handed a reliably-failing schedule a
--    fresh budget, so "max 3 recovery attempts" was really "max 3 per process
--    lifetime". `recovery_attempts` persists the count on the audit row
--    itself. (workflow_executions.recovery_attempts already exists but is
--    workflow-level — the wrong grain for a schedule handoff.)
--
-- 2. `find_lost_executions` is a plain SELECT, so every runner's recovery
--    service saw the same lost handoff and each one re-fired it. This is the
--    last duplicate-fire vector after #229 closed the long-running-workflow
--    case. `recovery_claimed_by` is the CAS claim: exactly one recovery
--    service flips it from NULL, the losers skip.
--
-- The claim carries a HEARTBEAT rather than a fixed expiry window because a
-- recovery re-fire is NOT short: `WorkflowExecutor::execute` blocks until the
-- workflow reaches a terminal state, so the claim is held for the workflow's
-- full duration (unbounded). A static `claimed_at + N minutes` window would
-- expire mid-execution and let a second service re-fire — reintroducing the
-- duplicate. `recovery_heartbeat_at` is refreshed while the re-fire runs, so
-- a stale heartbeat is a true death signal and a crashed recovery service can
-- never permanently lock a handoff.
--
-- ADD COLUMN only; existing rows default to "never attempted, unclaimed".
ALTER TABLE schedule_executions ADD COLUMN recovery_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE schedule_executions ADD COLUMN recovery_claimed_by UUID;
ALTER TABLE schedule_executions ADD COLUMN recovery_heartbeat_at TIMESTAMP;

-- Stale-claim detection during a recovery sweep: open, claimed rows by
-- heartbeat age.
CREATE INDEX idx_schedule_executions_recovery_claim
ON schedule_executions (recovery_heartbeat_at)
WHERE completed_at IS NULL AND recovery_claimed_by IS NOT NULL;
