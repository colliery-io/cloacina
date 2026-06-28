-- CLOACI-A-0008 refinement (CLOACI-I-0127): persist the autoscaler's last
-- scale-action timestamp in the DB so the back-pressure cooldown (T-0811) holds
-- across replicas. Leadership rotates per tick, so an in-memory per-replica
-- cooldown lets a later-leading replica bypass it. This wall-clock timestamp is
-- stamped (via SQL now()) by the autoscaler's set_desired_autoscaled write and
-- read back to gate should_act_at. NULL = never autoscaled (manual provisioning
-- via set_desired does NOT arm the cooldown, so it leaves this NULL). Server
-- mode only — Postgres. ADD COLUMN, never recreate.
ALTER TABLE agent_desired_counts ADD COLUMN last_autoscaled_at TIMESTAMPTZ NULL;
