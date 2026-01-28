---
id: package-signing-implementation-and
level: task
title: "Package signing implementation and CLI commands"
short_code: "CLOACI-T-0062"
created_at: 2026-01-28T14:15:48.914398+00:00
updated_at: 2026-01-28T16:22:07.223644+00:00
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

# Package signing implementation and CLI commands

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0008]]

## Objective

Implement package signing functionality and CLI commands. Support both API-based signing (key stays in DB) and offline signing (exported key file). Output signatures to database and/or detached `.sig` files.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

**Core API (Completed):**
- [x] `sign_package()` function for programmatic signing
- [x] Support API signing (key from database)
- [x] Support offline signing (raw key bytes)
- [x] Output to DB (store_signature) and/or detached file (DetachedSignature)
- [x] Unit tests for signing operations

**CLI Commands (Blocked - requires CLI crate):**
- [ ] `cloacina package sign` CLI command
- [ ] `cloacina keys generate` CLI command
- [ ] `cloacina keys export` CLI command
- [ ] `cloacina keys list` CLI command
- [ ] CLI integration tests

## CLI Commands

### `cloacina keys generate`

Generate a new signing keypair.

```bash
# API mode - store in database
cloacina keys generate --name "release-key" --org <org-id>
# Output: Key ID, fingerprint

# Offline mode - write to files
cloacina keys generate --name "dev-key" --output ./dev-key.pem
# Output: ./dev-key.pem (private), ./dev-key.pub.pem (public)
```

### `cloacina keys export`

Export public key for distribution.

```bash
cloacina keys export <key-id> --format pem > release-key.pub.pem
cloacina keys export <key-id> --format raw > release-key.pub
```

### `cloacina keys list`

List keys for an organization.

```bash
cloacina keys list --org <org-id>
# Output: table of key ID, name, fingerprint, created, status

cloacina keys list --org <org-id> --json
# Output: JSON array
```

### `cloacina package sign`

Sign a package.

```bash
# API signing - key stays in database
cloacina package sign ./target/release/libworkflow.so \
  --key-id <key-id> \
  --api-url http://localhost:8080

# Offline signing - use local key file
cloacina package sign ./target/release/libworkflow.so \
  --key ./signing-key.pem

# Output options
cloacina package sign ./pkg.so --key ./key.pem --output ./pkg.so.sig  # detached file
cloacina package sign ./pkg.so --key-id <id> --store                  # store in DB
cloacina package sign ./pkg.so --key ./key.pem --output ./pkg.so.sig --store  # both
```

## Core Signing Implementation

```rust
use ed25519_dalek::{SigningKey, Signature, Signer};
use sha2::{Sha256, Digest};

/// Package signature data
#[derive(Debug, Clone)]
pub struct PackageSignature {
    pub package_hash: String,      // SHA256 hex of package
    pub key_fingerprint: String,   // Which key signed it
    pub signature: Vec<u8>,        // 64 bytes Ed25519 signature
    pub signed_at: DateTime<Utc>,
}

/// Sign a package file
pub fn sign_package(
    package_path: &Path,
    private_key: &[u8],
    public_key: &[u8],
) -> Result<PackageSignature, SignError> {
    // 1. Read and hash the package
    let package_bytes = std::fs::read(package_path)?;
    let package_hash = hex::encode(Sha256::digest(&package_bytes));

    // 2. Create signing key from bytes
    let signing_key = SigningKey::from_bytes(
        private_key.try_into().map_err(|_| SignError::InvalidKey)?
    );

    // 3. Sign the hash (not the raw file - more efficient)
    let hash_bytes = hex::decode(&package_hash)?;
    let signature: Signature = signing_key.sign(&hash_bytes);

    // 4. Compute key fingerprint
    let fingerprint = hex::encode(Sha256::digest(public_key));

    Ok(PackageSignature {
        package_hash,
        key_fingerprint: fingerprint,
        signature: signature.to_bytes().to_vec(),
        signed_at: Utc::now(),
    })
}
```

## Detached Signature File Format

JSON format for `.sig` files:

```json
{
  "version": 1,
  "algorithm": "ed25519",
  "package_hash": "abc123...",
  "key_fingerprint": "def456...",
  "signature": "base64-encoded-64-bytes",
  "signed_at": "2026-01-28T12:00:00Z"
}
```

```rust
#[derive(Serialize, Deserialize)]
pub struct DetachedSignature {
    pub version: u32,
    pub algorithm: String,
    pub package_hash: String,
    pub key_fingerprint: String,
    #[serde(with = "base64")]
    pub signature: Vec<u8>,
    pub signed_at: DateTime<Utc>,
}

impl From<PackageSignature> for DetachedSignature {
    fn from(sig: PackageSignature) -> Self {
        Self {
            version: 1,
            algorithm: "ed25519".to_string(),
            package_hash: sig.package_hash,
            key_fingerprint: sig.key_fingerprint,
            signature: sig.signature,
            signed_at: sig.signed_at,
        }
    }
}
```

## File Locations

- CLI commands: `crates/cloacina-cli/src/commands/keys.rs`, `crates/cloacina-cli/src/commands/package.rs`
- Signing logic: `crates/cloacina/src/security/signing.rs`
- Signature types: `crates/cloacina/src/security/types.rs`

## Dependencies

```toml
# cloacina-cli/Cargo.toml (or wherever CLI lives)
clap = { version = "4.0", features = ["derive"] }
```

## Requires

- CLOACI-T-0060 (database schema)
- CLOACI-T-0061 (key management API)

## Status Updates

### Session 1 - 2026-01-28

**Core API Implementation (Completed):**
- Created `src/security/package_signer.rs` with:
  - `PackageSigner` trait for signing packages
  - `DbPackageSigner` database-backed implementation
  - `PackageSignatureInfo` for signature metadata
  - `DetachedSignature` JSON format for standalone `.sig` files
  - `PackageSignError` error types
- Full PostgreSQL and SQLite support
- Signing with database keys (API mode) and raw keys (offline mode)
- Signature storage in database
- Package verification against trusted keys
- Detached signature file read/write

**Testing:**
- 3 unit tests passing:
  - `test_sign_and_verify_with_raw_key`
  - `test_detached_signature_roundtrip`
  - `test_detached_signature_file_io`

**Not Yet Implemented (Requires CLI crate):**
- CLI commands (`cloacina keys generate`, `cloacina package sign`, etc.)
- CLI integration tests

The core signing API is complete. CLI commands will be added when a CLI crate is created (separate task or future work).
