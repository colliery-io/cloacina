-- CLOACINA-T-1024: durable audit record of who did what, when, in which tenant.
-- See the sibling postgres migration 056 for the rationale.
--
-- The server writes this table on Postgres only; it exists on SQLite to keep
-- the unified diesel schema the same on both backends and for the DAL tests.
CREATE TABLE audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL,
    tenant_id TEXT NULL,
    actor_key_id TEXT NOT NULL,
    actor_key_name TEXT NOT NULL,
    actor_identity TEXT NULL,
    action TEXT NOT NULL,
    target_kind TEXT NOT NULL,
    target TEXT NULL,
    outcome TEXT NOT NULL,
    detail TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_audit_events_tenant_id
    ON audit_events (tenant_id, id);

CREATE INDEX idx_audit_events_tenant_action_target
    ON audit_events (tenant_id, action, target);
