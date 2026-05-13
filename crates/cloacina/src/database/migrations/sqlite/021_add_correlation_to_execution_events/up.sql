-- SQLite version: add per-request / per-runner / per-tenant correlation
-- columns to execution_events. CLOACI-T-0583 / OPS-16.
--
-- UUIDs stored as BLOB to match the rest of the SQLite schema; tenant_id
-- is TEXT. All nullable to skip backfill and to support the daemon
-- (single-tenant, no per-tenant context).
--
-- Per project convention (feedback_sqlite_migration_recreate): use
-- ALTER TABLE ADD COLUMN, never DROP+CREATE.

ALTER TABLE execution_events ADD COLUMN request_id BLOB;
ALTER TABLE execution_events ADD COLUMN runner_id BLOB;
ALTER TABLE execution_events ADD COLUMN tenant_id TEXT;

CREATE INDEX idx_execution_events_tenant_created
    ON execution_events(tenant_id, created_at DESC)
    WHERE tenant_id IS NOT NULL;
