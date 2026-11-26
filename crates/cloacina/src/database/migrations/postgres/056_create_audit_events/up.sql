-- CLOACINA-T-1024: durable audit record of who did what, when, in which tenant.
--
-- One row per control-plane action (package upload, manual run, pause, key
-- create, ...). The table lives in the admin schema with a tenant column, like
-- `api_keys`: dropping a tenant schema must not drop that tenant's audit rows,
-- and platform actions (tenant create, global keys) have no tenant schema.
--
-- `detail` is a small JSON object (version, content hash, execution id). No
-- secret values, no request bodies, no context payloads.
CREATE TABLE audit_events (
    id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL,
    tenant_id TEXT NULL,
    actor_key_id TEXT NOT NULL,
    actor_key_name TEXT NOT NULL,
    -- The key's `issued_via` (`oidc:<issuer>:<sub>`, `local:<account_id>`);
    -- NULL for a manually created key.
    actor_identity TEXT NULL,
    action TEXT NOT NULL,
    target_kind TEXT NOT NULL,
    target TEXT NULL,
    outcome TEXT NOT NULL,
    detail TEXT NOT NULL DEFAULT '{}'
);

-- The list: one tenant, newest first, paged by id.
CREATE INDEX idx_audit_events_tenant_id
    ON audit_events (tenant_id, id);

-- "Who uploaded this package": one action and target in one tenant.
CREATE INDEX idx_audit_events_tenant_action_target
    ON audit_events (tenant_id, action, target);
