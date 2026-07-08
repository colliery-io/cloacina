-- CLOACI-I-0133 / T-0857 — encrypted secrets store (envelope encryption, D-7)
--
-- Two tables:
--   tenant_data_keys: the per-tenant data key (DEK) — 32 random bytes wrapped by
--     the server KEK via AES-256-GCM (nonce || ciphertext || tag). One row per org.
--   secrets: a named object of named fields, encrypted at rest under the tenant DEK.
--     `field_names` is PLAINTEXT metadata (the field names only, never the values);
--     `encrypted_fields` is the {field: value} JSON encrypted under the tenant DEK.

CREATE TABLE tenant_data_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL,
    wrapped_dek BYTEA NOT NULL,            -- DEK wrapped by server KEK (AES-256-GCM)
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(org_id)
);

CREATE INDEX idx_tenant_data_keys_org ON tenant_data_keys(org_id);

CREATE TABLE secrets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    field_names TEXT NOT NULL,             -- JSON array of field names (plaintext metadata)
    encrypted_fields BYTEA NOT NULL,       -- {field: value} JSON encrypted under tenant DEK
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(org_id, name)
);

CREATE INDEX idx_secrets_org ON secrets(org_id);
