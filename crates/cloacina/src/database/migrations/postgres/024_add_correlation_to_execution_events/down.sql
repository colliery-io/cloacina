DROP INDEX IF EXISTS idx_execution_events_tenant_created;

ALTER TABLE execution_events DROP COLUMN IF EXISTS request_id;
ALTER TABLE execution_events DROP COLUMN IF EXISTS runner_id;
ALTER TABLE execution_events DROP COLUMN IF EXISTS tenant_id;
