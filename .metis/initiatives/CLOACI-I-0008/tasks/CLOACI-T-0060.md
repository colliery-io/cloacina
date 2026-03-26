---
id: package-signing-database-schema
level: task
title: "Package signing database schema and migrations"
short_code: "CLOACI-T-0060"
created_at: 2026-01-28T14:15:48.422786+00:00
updated_at: 2026-01-28T15:44:53.162327+00:00
parent: CLOACI-I-0008
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0008
---

# Package signing database schema and migrations

## Parent Initiative

[[CLOACI-I-0008]] - Package Signing and Verification

## Objective

Create the database schema and Diesel migrations for storing signing keys, trusted keys, trust chain ACLs, and package signatures. Must support both PostgreSQL and SQLite backends.

## Acceptance Criteria

## Acceptance Criteria

- [x] Diesel migrations created for all 4 tables
- [x] Migrations work on both PostgreSQL and SQLite
- [x] Diesel models and schema.rs generated
- [x] Encrypted private key storage implemented (AES-256-GCM)
- [x] Unit tests for crypto operations (12 tests passing)
- [x] Indexes on frequently queried columns

## Database Schema

### Table: `signing_keys`

Stores Ed25519 private keys (encrypted) for signing packages.

```sql
CREATE TABLE signing_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL,  -- References organizations table (if exists) or standalone
    key_name VARCHAR(255) NOT NULL,
    encrypted_private_key BYTEA NOT NULL,  -- AES-256-GCM encrypted Ed25519 seed
    public_key BYTEA NOT NULL,             -- 32 bytes, for reference/export
    key_fingerprint VARCHAR(64) NOT NULL,  -- SHA256 hex of public_key
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,                -- NULL = active, set = revoked
    UNIQUE(org_id, key_name)
);

CREATE INDEX idx_signing_keys_org ON signing_keys(org_id);
CREATE INDEX idx_signing_keys_fingerprint ON signing_keys(key_fingerprint);
```

### Table: `trusted_keys`

Stores public keys that are trusted for verification.

```sql
CREATE TABLE trusted_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL,
    key_fingerprint VARCHAR(64) NOT NULL,  -- SHA256 hex of public_key
    public_key BYTEA NOT NULL,             -- 32 bytes Ed25519 public key
    key_name VARCHAR(255),                 -- Optional human-readable name
    trusted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    UNIQUE(org_id, key_fingerprint)
);

CREATE INDEX idx_trusted_keys_org ON trusted_keys(org_id);
CREATE INDEX idx_trusted_keys_fingerprint ON trusted_keys(key_fingerprint);
```

### Table: `key_trust_acls`

Explicit trust relationships between organizations.

```sql
CREATE TABLE key_trust_acls (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_org_id UUID NOT NULL,  -- The org granting trust
    child_org_id UUID NOT NULL,   -- The org being trusted
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    UNIQUE(parent_org_id, child_org_id)
);

CREATE INDEX idx_trust_acls_parent ON key_trust_acls(parent_org_id);
CREATE INDEX idx_trust_acls_child ON key_trust_acls(child_org_id);
```

### Table: `package_signatures`

Stores signatures for packages.

```sql
CREATE TABLE package_signatures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    package_hash VARCHAR(64) NOT NULL,     -- SHA256 hex of package binary
    key_fingerprint VARCHAR(64) NOT NULL,  -- Which key signed it
    signature BYTEA NOT NULL,              -- 64 bytes Ed25519 signature
    signed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(package_hash, key_fingerprint)
);

CREATE INDEX idx_signatures_hash ON package_signatures(package_hash);
CREATE INDEX idx_signatures_key ON package_signatures(key_fingerprint);
```

## Implementation Notes

### File Locations

- Migrations: `crates/cloacina/migrations/` (follow existing pattern)
- Models: `crates/cloacina/src/dal/models/signing.rs` (new file)
- Schema: Auto-generated in `schema.rs`

### Encrypted Key Storage

Private keys must be encrypted at rest using AES-256-GCM:

```rust
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use aes_gcm::aead::Aead;

/// Encrypted key format: nonce (12 bytes) || ciphertext || tag (16 bytes)
pub fn encrypt_private_key(key: &[u8; 32], master_key: &[u8; 32]) -> Vec<u8> {
    let cipher = Aes256Gcm::new_from_slice(master_key).unwrap();
    let nonce = Nonce::from_slice(&rand::random::<[u8; 12]>());
    let ciphertext = cipher.encrypt(nonce, key.as_ref()).unwrap();
    [nonce.as_slice(), &ciphertext].concat()
}

pub fn decrypt_private_key(encrypted: &[u8], master_key: &[u8; 32]) -> Result<[u8; 32], Error> {
    let cipher = Aes256Gcm::new_from_slice(master_key).unwrap();
    let nonce = Nonce::from_slice(&encrypted[..12]);
    let plaintext = cipher.decrypt(nonce, &encrypted[12..])?;
    Ok(plaintext.try_into().unwrap())
}
```

### SQLite Compatibility

- Use `TEXT` instead of `VARCHAR` for SQLite
- Use `BLOB` instead of `BYTEA`
- Use `TEXT` for timestamps (ISO8601 format)
- No `gen_random_uuid()` - generate UUIDs in application code

### Dependencies to Add

```toml
# In cloacina/Cargo.toml
aes-gcm = "0.10"
```

### Existing Patterns to Follow

- Look at `crates/cloacina/src/dal/models/` for existing Diesel model patterns
- Look at `crates/cloacina/migrations/` for migration naming convention
- Use `Insertable`, `Queryable`, `AsChangeset` derives

## Status Updates

### Session 1 - 2026-01-28 (Completed)

**Database Infrastructure:**
- Created PostgreSQL migration `010_create_signing_tables/up.sql` and `down.sql`
- Created SQLite migration `009_create_signing_tables/up.sql` and `down.sql`
- Updated `schema.rs` with new tables in all three schema modules (unified, postgres, sqlite)
- Added Diesel models to `dal/unified/models.rs`:
  - `UnifiedSigningKey` / `NewUnifiedSigningKey`
  - `UnifiedTrustedKey` / `NewUnifiedTrustedKey`
  - `UnifiedKeyTrustAcl` / `NewUnifiedKeyTrustAcl`
  - `UnifiedPackageSignature` / `NewUnifiedPackageSignature`
- Added dependencies to Cargo.toml: `aes-gcm`, `ed25519-dalek`, `hex`

**Domain Models:**
- Created `src/models/signing_key.rs` - SigningKey, NewSigningKey, SigningKeyInfo
- Created `src/models/trusted_key.rs` - TrustedKey, NewTrustedKey
- Created `src/models/key_trust_acl.rs` - KeyTrustAcl, NewKeyTrustAcl
- Created `src/models/package_signature.rs` - PackageSignature, NewPackageSignature, SignatureVerification
- Updated `models/mod.rs` to export new modules
- Added `From` conversions from Unified* to domain models

**Crypto Utilities:**
- Created `src/crypto/mod.rs` - Module exports
- Created `src/crypto/key_encryption.rs` - AES-256-GCM encryption/decryption for private keys
- Created `src/crypto/signing.rs` - Ed25519 key generation, signing, verification
- Exported crypto module from lib.rs

**Testing:**
- All 12 crypto unit tests passing
- Compilation verified with both postgres and sqlite features
