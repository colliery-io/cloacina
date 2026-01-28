-- SQLite migration: Create signing and verification tables for package security
-- UUID stored as BLOB (16 bytes), TIMESTAMP stored as TEXT (RFC3339 format)

-- Signing keys (private, encrypted at rest)
CREATE TABLE signing_keys (
    id BLOB PRIMARY KEY NOT NULL,
    org_id BLOB NOT NULL,
    key_name TEXT NOT NULL,
    encrypted_private_key BLOB NOT NULL,   -- AES-256-GCM encrypted Ed25519 seed (nonce || ciphertext || tag)
    public_key BLOB NOT NULL,              -- 32 bytes Ed25519 public key
    key_fingerprint TEXT NOT NULL,         -- SHA256 hex of public key
    created_at TEXT NOT NULL,              -- RFC3339 format
    revoked_at TEXT,                       -- NULL = active, RFC3339 = revoked
    UNIQUE(org_id, key_name)
);

CREATE INDEX idx_signing_keys_org ON signing_keys(org_id);
CREATE INDEX idx_signing_keys_fingerprint ON signing_keys(key_fingerprint);

-- Trusted keys (public keys for verification)
CREATE TABLE trusted_keys (
    id BLOB PRIMARY KEY NOT NULL,
    org_id BLOB NOT NULL,
    key_fingerprint TEXT NOT NULL,         -- SHA256 hex of public key
    public_key BLOB NOT NULL,              -- 32 bytes Ed25519 public key
    key_name TEXT,                         -- Optional human-readable name
    trusted_at TEXT NOT NULL,              -- RFC3339 format
    revoked_at TEXT,
    UNIQUE(org_id, key_fingerprint)
);

CREATE INDEX idx_trusted_keys_org ON trusted_keys(org_id);
CREATE INDEX idx_trusted_keys_fingerprint ON trusted_keys(key_fingerprint);

-- Trust chain ACLs (explicit org -> sub-org trust)
CREATE TABLE key_trust_acls (
    id BLOB PRIMARY KEY NOT NULL,
    parent_org_id BLOB NOT NULL,           -- The org granting trust
    child_org_id BLOB NOT NULL,            -- The org being trusted
    granted_at TEXT NOT NULL,              -- RFC3339 format
    revoked_at TEXT,
    UNIQUE(parent_org_id, child_org_id)
);

CREATE INDEX idx_trust_acls_parent ON key_trust_acls(parent_org_id);
CREATE INDEX idx_trust_acls_child ON key_trust_acls(child_org_id);

-- Package signatures
CREATE TABLE package_signatures (
    id BLOB PRIMARY KEY NOT NULL,
    package_hash TEXT NOT NULL,            -- SHA256 hex of package binary
    key_fingerprint TEXT NOT NULL,         -- Which key signed it
    signature BLOB NOT NULL,               -- 64 bytes Ed25519 signature
    signed_at TEXT NOT NULL,               -- RFC3339 format
    UNIQUE(package_hash, key_fingerprint)
);

CREATE INDEX idx_signatures_hash ON package_signatures(package_hash);
CREATE INDEX idx_signatures_key ON package_signatures(key_fingerprint);
