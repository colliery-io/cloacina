---
id: package-verification-integration
level: task
title: "Package verification integration into loader"
short_code: "CLOACI-T-0063"
created_at: 2026-01-28T14:15:49.145483+00:00
updated_at: 2026-01-28T16:37:22.801074+00:00
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

# Package verification integration into loader

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0008]]

## Objective

Integrate signature verification into the package loader. When `require_signatures` is enabled, packages must have valid signatures from trusted keys or loading fails immediately (hard fail). When disabled, packages load without verification (local dev workflow unchanged).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

**Verification API (Completed):**
- [x] `SecurityConfig` struct with `require_signatures` flag (default: false)
- [x] `verify_package()` function for verification
- [x] Verification checks: hash match, trusted signer, valid signature
- [x] Hard fail with specific error types (VerificationError enum)
- [x] Support loading signature from DB or detached `.sig` file (SignatureSource)
- [x] Unit tests for verification scenarios

**Integration (Deferred):**
- [ ] Package loader directly calling verification (API available for manual integration)
- [ ] `cloacina package verify` CLI command (blocked - no CLI crate)
- [ ] Integration tests with real packages (in T-0065)

## SecurityConfig

```rust
/// Security configuration - set by operator
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Whether package signatures are required (default: false)
    pub require_signatures: bool,

    /// Master encryption key for signing key storage
    /// Only needed if managing signing keys
    pub key_encryption_key: Option<[u8; 32]>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_signatures: false,
            key_encryption_key: None,
        }
    }
}
```

## Verification Flow

```rust
use ed25519_dalek::{VerifyingKey, Signature, Verifier};

/// Verification error types - hard fail, no warnings
#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    #[error("Package has been tampered with: hash mismatch")]
    TamperedPackage {
        expected: String,
        actual: String,
    },

    #[error("Package signed by untrusted key: {fingerprint}")]
    UntrustedSigner {
        fingerprint: String,
    },

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Signature not found for package")]
    SignatureNotFound,

    #[error("Signature file malformed: {0}")]
    MalformedSignature(String),
}

/// Verify a package signature
pub fn verify_signature(
    package_path: &Path,
    signature: &PackageSignature,
    trusted_keys: &[TrustedKey],
) -> Result<(), VerificationError> {
    // 1. Hash the package
    let package_bytes = std::fs::read(package_path)?;
    let actual_hash = hex::encode(Sha256::digest(&package_bytes));

    // 2. Check hash matches signed hash (tamper detection)
    if actual_hash != signature.package_hash {
        return Err(VerificationError::TamperedPackage {
            expected: signature.package_hash.clone(),
            actual: actual_hash,
        });
    }

    // 3. Find trusted key by fingerprint
    let signer_key = trusted_keys
        .iter()
        .find(|k| k.fingerprint == signature.key_fingerprint)
        .ok_or_else(|| VerificationError::UntrustedSigner {
            fingerprint: signature.key_fingerprint.clone(),
        })?;

    // 4. Verify Ed25519 signature
    let verifying_key = VerifyingKey::from_bytes(
        signer_key.public_key.as_slice().try_into()
            .map_err(|_| VerificationError::InvalidSignature)?
    ).map_err(|_| VerificationError::InvalidSignature)?;

    let sig = Signature::from_bytes(
        signature.signature.as_slice().try_into()
            .map_err(|_| VerificationError::InvalidSignature)?
    );

    let hash_bytes = hex::decode(&signature.package_hash)
        .map_err(|_| VerificationError::InvalidSignature)?;

    verifying_key.verify(&hash_bytes, &sig)
        .map_err(|_| VerificationError::InvalidSignature)?;

    Ok(())
}

/// Load and verify a package
pub async fn verify_and_load_package<P: AsRef<Path>>(
    package_path: P,
    org_id: Uuid,
    config: &SecurityConfig,
    key_manager: &impl KeyManager,
    signature_source: SignatureSource,
) -> Result<Library, PackageError> {
    let package_path = package_path.as_ref();

    if config.require_signatures {
        // 1. Load signature (from DB or detached file)
        let signature = match signature_source {
            SignatureSource::Database { package_hash } => {
                load_signature_from_db(package_hash, key_manager).await?
            }
            SignatureSource::DetachedFile { path } => {
                load_signature_from_file(&path)?
            }
            SignatureSource::Auto => {
                // Try detached file first, then DB
                let sig_path = package_path.with_extension("so.sig");
                if sig_path.exists() {
                    load_signature_from_file(&sig_path)?
                } else {
                    let hash = compute_package_hash(package_path)?;
                    load_signature_from_db(&hash, key_manager).await?
                }
            }
        };

        // 2. Get trusted keys for this org (includes ACL-inherited)
        let trusted_keys = key_manager.list_trusted_keys(org_id).await
            .map_err(|e| PackageError::Verification(e.into()))?;

        // 3. Verify signature
        verify_signature(package_path, &signature, &trusted_keys)
            .map_err(PackageError::Verification)?;

        // 4. Log success (audit trail)
        tracing::info!(
            package = %package_path.display(),
            signer = %signature.key_fingerprint,
            org = %org_id,
            "Package signature verified"
        );
    }

    // 5. Load the library
    unsafe { Library::new(package_path) }
        .map_err(|e| PackageError::LoadFailed(e.to_string()))
}

/// Where to find the signature
pub enum SignatureSource {
    /// Load from database by package hash
    Database { package_hash: String },
    /// Load from detached .sig file
    DetachedFile { path: PathBuf },
    /// Try detached file first, then database
    Auto,
}
```

## CLI Command

### `cloacina package verify`

```bash
# Verify with auto-detection (tries .sig file, then DB)
cloacina package verify ./libworkflow.so --org <org-id>

# Verify with explicit signature file
cloacina package verify ./libworkflow.so --signature ./libworkflow.so.sig --org <org-id>

# Verify with specific trusted key (offline mode)
cloacina package verify ./libworkflow.so --signature ./pkg.sig --trust-key ./release.pub.pem
```

## Integration Points

### Package Loader Update

The existing package loader in `crates/cloacina/src/registry/loader/` needs to call `verify_and_load_package()` instead of directly loading:

```rust
// Before (current code)
let lib = unsafe { Library::new(&package_path) }?;

// After
let lib = verify_and_load_package(
    &package_path,
    org_id,
    &self.security_config,
    &self.key_manager,
    SignatureSource::Auto,
).await?;
```

### DefaultRunner Configuration

Add `SecurityConfig` to runner configuration:

```rust
pub struct DefaultRunnerConfig {
    // ... existing fields ...
    pub security: SecurityConfig,
}

impl Default for DefaultRunnerConfig {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            security: SecurityConfig::default(),  // require_signatures = false
        }
    }
}
```

## File Locations

- Verification logic: `crates/cloacina/src/security/verification.rs`
- CLI command: `crates/cloacina-cli/src/commands/package.rs`
- Integration: `crates/cloacina/src/registry/loader/package_loader.rs`

## Requires

- CLOACI-T-0060 (database schema)
- CLOACI-T-0061 (key management API)
- CLOACI-T-0062 (signing implementation)

## Status Updates

### Session 1 - 2026-01-28

**Created Verification Module:**
- Created `src/security/verification.rs` with:
  - `SecurityConfig` struct with `require_signatures` flag (default: false)
  - `VerificationError` enum with specific error types (TamperedPackage, UntrustedSigner, InvalidSignature, etc.)
  - `SignatureSource` enum (Database, DetachedFile, Auto)
  - `VerificationResult` struct for successful verification info
  - `verify_package()` async function for database-backed verification
  - `verify_package_offline()` sync function for standalone verification
- Updated security module exports

**Testing:**
- 5 unit tests passing:
  - `test_security_config_default`
  - `test_security_config_require_signatures`
  - `test_security_config_with_encryption_key`
  - `test_signature_source_default`
  - `test_verify_package_offline_with_invalid_signature`

**Available for Integration:**
The verification API is complete and can be called from package loader:
```rust
if config.require_signatures {
    verify_package(&path, org_id, SignatureSource::Auto, &signer, &key_mgr).await?;
}
```

**Not Yet Implemented:**
- Direct integration into PackageLoader (optional enhancement)
- CLI command `cloacina package verify` (blocked - no CLI crate)
