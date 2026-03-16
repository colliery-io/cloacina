-- Authentication tables for PAK + ABAC authorization model.

CREATE TABLE tenants (
    id VARCHAR(36) PRIMARY KEY NOT NULL,
    name VARCHAR(255) UNIQUE NOT NULL,
    schema_name VARCHAR(255) UNIQUE NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE api_keys (
    id VARCHAR(36) PRIMARY KEY NOT NULL,
    tenant_id VARCHAR(36) REFERENCES tenants(id),
    key_hash TEXT NOT NULL,
    key_prefix VARCHAR(32) NOT NULL,
    name VARCHAR(255),
    can_read BOOLEAN NOT NULL DEFAULT 1,
    can_write BOOLEAN NOT NULL DEFAULT 0,
    can_execute BOOLEAN NOT NULL DEFAULT 0,
    can_admin BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT,
    revoked_at TEXT
);

CREATE TABLE api_key_workflow_patterns (
    id VARCHAR(36) PRIMARY KEY NOT NULL,
    api_key_id VARCHAR(36) NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    pattern TEXT NOT NULL
);

CREATE INDEX idx_api_keys_prefix ON api_keys(key_prefix);
CREATE INDEX idx_api_keys_tenant ON api_keys(tenant_id);
CREATE INDEX idx_workflow_patterns_key ON api_key_workflow_patterns(api_key_id);
