-- Authentication tables for PAK + ABAC authorization model.

CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,
    schema_name VARCHAR(255) UNIQUE NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id),
    key_hash TEXT NOT NULL,
    key_prefix VARCHAR(32) NOT NULL,
    name VARCHAR(255),
    can_read BOOLEAN NOT NULL DEFAULT true,
    can_write BOOLEAN NOT NULL DEFAULT false,
    can_execute BOOLEAN NOT NULL DEFAULT false,
    can_admin BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    expires_at TIMESTAMP,
    revoked_at TIMESTAMP
);

CREATE TABLE api_key_workflow_patterns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    pattern TEXT NOT NULL
);

CREATE INDEX idx_api_keys_prefix ON api_keys(key_prefix);
CREATE INDEX idx_api_keys_tenant ON api_keys(tenant_id);
CREATE INDEX idx_workflow_patterns_key ON api_key_workflow_patterns(api_key_id);
