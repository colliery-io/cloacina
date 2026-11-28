-- CLOACINA-T-1136: the claim a fleet dispatch was recorded under. See the
-- Postgres migration of the same number.
--
-- compat: one nullable column; the previous release does not read it.
ALTER TABLE task_executions ADD COLUMN fleet_dispatch_claimed_by BLOB;
