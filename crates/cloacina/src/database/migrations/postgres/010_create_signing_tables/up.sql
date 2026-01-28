-- Create signing and verification tables for package security
--
-- This migration creates the tables needed for:
-- 1. signing_keys: Ed25519 private keys (encrypted with AES-256-GCM)
-- 2. trusted_keys: Public keys trusted for verification
-- 3. key_trust_acls: Explicit trust relationships between organizations
-- 4. package_signatures: Signatures for workflow packages

-- Signing keys (private, encrypted at rest)
CREATE TABLE signing_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL,
    key_name VARCHAR(255) NOT NULL,
    encrypted_private_key BYTEA NOT NULL,  -- AES-256-GCM encrypted Ed25519 seed (nonce || ciphertext || tag)
    public_key BYTEA NOT NULL,             -- 32 bytes Ed25519 public key
    key_fingerprint VARCHAR(64) NOT NULL,  -- SHA256 hex of public key
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at TIMESTAMP,                  -- NULL = active, set = revoked
    UNIQUE(org_id, key_name)
);

CREATE INDEX idx_signing_keys_org ON signing_keys(org_id);
CREATE INDEX idx_signing_keys_fingerprint ON signing_keys(key_fingerprint);
CREATE INDEX idx_signing_keys_revoked ON signing_keys(revoked_at) WHERE revoked_at IS NULL;

-- Trusted keys (public keys for verification)
CREATE TABLE trusted_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL,
    key_fingerprint VARCHAR(64) NOT NULL,  -- SHA256 hex of public key
    public_key BYTEA NOT NULL,             -- 32 bytes Ed25519 public key
    key_name VARCHAR(255),                 -- Optional human-readable name
    trusted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at TIMESTAMP,
    UNIQUE(org_id, key_fingerprint)
);

CREATE INDEX idx_trusted_keys_org ON trusted_keys(org_id);
CREATE INDEX idx_trusted_keys_fingerprint ON trusted_keys(key_fingerprint);
CREATE INDEX idx_trusted_keys_revoked ON trusted_keys(revoked_at) WHERE revoked_at IS NULL;

-- Trust chain ACLs (explicit org -> sub-org trust)
CREATE TABLE key_trust_acls (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_org_id UUID NOT NULL,           -- The org granting trust
    child_org_id UUID NOT NULL,            -- The org being trusted
    granted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at TIMESTAMP,
    UNIQUE(parent_org_id, child_org_id)
);

CREATE INDEX idx_trust_acls_parent ON key_trust_acls(parent_org_id);
CREATE INDEX idx_trust_acls_child ON key_trust_acls(child_org_id);
CREATE INDEX idx_trust_acls_active ON key_trust_acls(parent_org_id, child_org_id) WHERE revoked_at IS NULL;

-- Package signatures
CREATE TABLE package_signatures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    package_hash VARCHAR(64) NOT NULL,     -- SHA256 hex of package binary
    key_fingerprint VARCHAR(64) NOT NULL,  -- Which key signed it
    signature BYTEA NOT NULL,              -- 64 bytes Ed25519 signature
    signed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(package_hash, key_fingerprint)
);

CREATE INDEX idx_signatures_hash ON package_signatures(package_hash);
CREATE INDEX idx_signatures_key ON package_signatures(key_fingerprint);
