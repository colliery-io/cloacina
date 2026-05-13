-- SQLite supports DROP COLUMN as of 3.35.0 (2021). Project minimum
-- SQLite is well past that.
DROP INDEX IF EXISTS idx_execution_events_tenant_created;

ALTER TABLE execution_events DROP COLUMN request_id;
ALTER TABLE execution_events DROP COLUMN runner_id;
ALTER TABLE execution_events DROP COLUMN tenant_id;
