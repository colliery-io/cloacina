-- CLOACI-I-0133 / T-0857 — encrypted secrets store (envelope encryption, D-7)
-- UUID stored as BLOB (16 bytes), TIMESTAMP stored as TEXT (RFC3339 format)
--
--   tenant_data_keys: the per-tenant data key (DEK) — 32 random bytes wrapped by
--     the server KEK via AES-256-GCM (nonce || ciphertext || tag). One row per org.
--   secrets: a named object of named fields, encrypted at rest under the tenant DEK.
--     `field_names` is PLAINTEXT metadata (the field names only, never the values);
--     `encrypted_fields` is the {field: value} JSON encrypted under the tenant DEK.

CREATE TABLE tenant_data_keys (
    id BLOB PRIMARY KEY NOT NULL,
    org_id BLOB NOT NULL,
    wrapped_dek BLOB NOT NULL,             -- DEK wrapped by server KEK (AES-256-GCM)
    created_at TEXT NOT NULL,              -- RFC3339 format
    UNIQUE(org_id)
);

CREATE INDEX idx_tenant_data_keys_org ON tenant_data_keys(org_id);

CREATE TABLE secrets (
    id BLOB PRIMARY KEY NOT NULL,
    org_id BLOB NOT NULL,
    name TEXT NOT NULL,
    field_names TEXT NOT NULL,             -- JSON array of field names (plaintext metadata)
    encrypted_fields BLOB NOT NULL,        -- {field: value} JSON encrypted under tenant DEK
    created_at TEXT NOT NULL,              -- RFC3339 format
    updated_at TEXT NOT NULL,              -- RFC3339 format
    UNIQUE(org_id, name)
);

CREATE INDEX idx_secrets_org ON secrets(org_id);
