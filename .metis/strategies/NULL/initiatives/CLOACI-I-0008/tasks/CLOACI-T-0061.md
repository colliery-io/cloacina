---
id: key-management-api-and-trait
level: task
title: "Key management API and trait implementation"
short_code: "CLOACI-T-0061"
created_at: 2026-01-28T14:15:48.673682+00:00
updated_at: 2026-01-28T16:14:03.708629+00:00
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

# Key management API and trait implementation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0008]]

## Objective

Implement the `KeyManager` trait and its database-backed implementation for managing signing keys, trusted keys, and trust chain ACLs. This provides the core API for all key operations with hot-reload support.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `KeyManager` trait defined with all required methods
- [x] `DbKeyManager` implementation using Diesel
- [x] Key generation using `ed25519-dalek`
- [x] Key encryption/decryption using `aes-gcm`
- [x] Trust chain resolution (including ACL-inherited keys)
- [x] Key revocation with immediate effect
- [x] PEM export/import for public keys
- [x] Unit tests for PEM operations (integration tests in T-0065)
- [x] Hot-reload: no caching that would prevent immediate key changes

## KeyManager Trait

```rust
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Information about a signing key (excludes private key material)
#[derive(Debug, Clone)]
pub struct SigningKeyInfo {
    pub id: Uuid,
    pub org_id: Uuid,
    pub key_name: String,
    pub fingerprint: String,  // SHA256 hex of public key
    pub public_key: Vec<u8>,  // 32 bytes
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// A trusted public key for verification
#[derive(Debug, Clone)]
pub struct TrustedKey {
    pub id: Uuid,
    pub org_id: Uuid,
    pub fingerprint: String,
    pub public_key: Vec<u8>,
    pub key_name: Option<String>,
    pub trusted_at: DateTime<Utc>,
}

/// Public key export format
#[derive(Debug, Clone)]
pub struct PublicKeyExport {
    pub fingerprint: String,
    pub public_key_pem: String,
    pub public_key_raw: Vec<u8>,
}

#[async_trait::async_trait]
pub trait KeyManager: Send + Sync {
    /// Generate a new Ed25519 signing keypair, store encrypted in DB
    async fn create_signing_key(
        &self,
        org_id: Uuid,
        name: &str,
    ) -> Result<SigningKeyInfo, KeyError>;

    /// Get signing key info (without private key)
    async fn get_signing_key_info(&self, key_id: Uuid) -> Result<SigningKeyInfo, KeyError>;

    /// Get decrypted signing key for signing operations
    /// Returns (public_key, private_key) tuple
    async fn get_signing_key(&self, key_id: Uuid) -> Result<(Vec<u8>, Vec<u8>), KeyError>;

    /// Export public key in PEM format for distribution
    async fn export_public_key(&self, key_id: Uuid) -> Result<PublicKeyExport, KeyError>;

    /// Import an external public key to trust
    async fn trust_public_key(
        &self,
        org_id: Uuid,
        public_key: &[u8],
        name: Option<&str>,
    ) -> Result<TrustedKey, KeyError>;

    /// Import public key from PEM format
    async fn trust_public_key_pem(
        &self,
        org_id: Uuid,
        pem: &str,
        name: Option<&str>,
    ) -> Result<TrustedKey, KeyError>;

    /// Revoke a signing key (prevents future signing)
    async fn revoke_signing_key(&self, key_id: Uuid) -> Result<(), KeyError>;

    /// Revoke a trusted key (prevents future verification)
    async fn revoke_trusted_key(&self, key_id: Uuid) -> Result<(), KeyError>;

    /// Grant trust from parent org to child org's keys
    async fn grant_trust(&self, parent_org: Uuid, child_org: Uuid) -> Result<(), KeyError>;

    /// Revoke trust grant
    async fn revoke_trust(&self, parent_org: Uuid, child_org: Uuid) -> Result<(), KeyError>;

    /// List all trusted keys for an org (including inherited via ACL)
    async fn list_trusted_keys(&self, org_id: Uuid) -> Result<Vec<TrustedKey>, KeyError>;

    /// List signing keys for an org
    async fn list_signing_keys(&self, org_id: Uuid) -> Result<Vec<SigningKeyInfo>, KeyError>;

    /// Find trusted key by fingerprint (for verification)
    async fn find_trusted_key(
        &self,
        org_id: Uuid,
        fingerprint: &str,
    ) -> Result<Option<TrustedKey>, KeyError>;
}
```

## Implementation Notes

### File Location

- Trait: `crates/cloacina/src/security/key_manager.rs`
- Implementation: `crates/cloacina/src/security/db_key_manager.rs`
- Module: `crates/cloacina/src/security/mod.rs`

### Dependencies

```toml
# Add to cloacina/Cargo.toml
ed25519-dalek = { version = "2.0", features = ["rand_core"] }
sha2 = "0.10"
aes-gcm = "0.10"
pem = "3.0"  # For PEM encoding/decoding
```

### Key Generation

```rust
use ed25519_dalek::{SigningKey, VerifyingKey};
use sha2::{Sha256, Digest};

fn generate_keypair() -> (Vec<u8>, Vec<u8>, String) {
    let signing_key = SigningKey::generate(&mut rand::thread_rng());
    let verifying_key = signing_key.verifying_key();

    let private_key = signing_key.to_bytes().to_vec();
    let public_key = verifying_key.to_bytes().to_vec();

    // Fingerprint is SHA256 of public key in hex
    let fingerprint = hex::encode(Sha256::digest(&public_key));

    (public_key, private_key, fingerprint)
}
```

### Trust Chain Resolution

When listing trusted keys for an org, must include:
1. Keys directly trusted by the org
2. Keys from child orgs that the org has granted trust to (via ACL)

```rust
async fn list_trusted_keys(&self, org_id: Uuid) -> Result<Vec<TrustedKey>, KeyError> {
    // 1. Get directly trusted keys (not revoked)
    let direct_keys = self.get_direct_trusted_keys(org_id).await?;

    // 2. Get child orgs we trust via ACL
    let trusted_children = self.get_trusted_child_orgs(org_id).await?;

    // 3. Get keys from trusted children
    let mut inherited_keys = Vec::new();
    for child_org in trusted_children {
        let child_keys = self.get_direct_trusted_keys(child_org).await?;
        inherited_keys.extend(child_keys);
    }

    // 4. Combine and deduplicate by fingerprint
    let mut all_keys = direct_keys;
    for key in inherited_keys {
        if !all_keys.iter().any(|k| k.fingerprint == key.fingerprint) {
            all_keys.push(key);
        }
    }

    Ok(all_keys)
}
```

### PEM Format

Use standard Ed25519 PEM format:

```
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA<base64-encoded-32-bytes>
-----END PUBLIC KEY-----
```

### Hot-Reload

- NO caching of keys or trust relationships
- Every operation queries the database
- Key revocation takes effect immediately on next verification
- If caching is needed later for performance, use short TTL with explicit invalidation

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("Key not found: {0}")]
    NotFound(Uuid),

    #[error("Key revoked: {0}")]
    Revoked(Uuid),

    #[error("Key name already exists: {0}")]
    DuplicateName(String),

    #[error("Invalid key format: {0}")]
    InvalidFormat(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),
}
```

### Requires

- CLOACI-T-0060 (database schema) must be completed first

## Status Updates

### Session 1 - 2026-01-28 (Completed)

**Created Security Module:**
- Created `src/security/mod.rs` - Module exports
- Created `src/security/key_manager.rs` - `KeyManager` trait and types
- Created `src/security/db_key_manager.rs` - Database-backed implementation

**KeyManager Trait:**
- Full trait definition with all required methods
- Types: `SigningKeyInfo`, `TrustedKeyInfo`, `PublicKeyExport`, `KeyError`
- Async methods for all key operations

**DbKeyManager Implementation:**
- PostgreSQL and SQLite implementations using dispatch macros
- Key generation using `generate_signing_keypair()` from crypto module
- Encryption/decryption using `encrypt_private_key()`/`decrypt_private_key()` from crypto module
- Trust chain resolution with ACL support (inherited keys from child orgs)
- Immediate revocation (no caching)
- PEM export/import for Ed25519 public keys (SubjectPublicKeyInfo format)

**Dependencies Added:**
- `pem = "3.0"` for PEM encoding/decoding

**Testing:**
- 2 unit tests for PEM roundtrip and invalid PEM handling
- All tests passing
