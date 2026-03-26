---
id: implement-package-signing-and
level: initiative
title: "Implement Package Signing and Verification for Dynamic Libraries"
short_code: "CLOACI-I-0008"
created_at: 2025-11-29T02:40:14.993840+00:00
updated_at: 2026-01-28T16:56:52.103940+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: XL
strategy_id: NULL
initiative_id: implement-package-signing-and
---

# Implement Package Signing and Verification for Dynamic Libraries Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

The package loading system in `cloacina/src/packaging/manifest.rs` (lines 121-166) loads and executes arbitrary dynamic libraries without any verification:

```rust
let lib = unsafe { libloading::Library::new(so_path) };  // Line 121
// No signature verification
// No integrity check
// No capability restrictions
```

**Risks:**
- Malicious workflow packages can execute arbitrary code
- No audit trail of package origins
- Supply chain attacks through compromised packages
- No way to verify packages came from trusted sources

## Goals & Non-Goals

**Goals:**
- Implement Ed25519 cryptographic signing for workflow packages
- Verify signatures before loading any dynamic library
- Database-backed key management with API for hot-reload
- Hierarchical trust model with explicit ACLs (org → sub-org)
- Hard fail on any signature verification failure
- Audit logging for all package loads and key operations

**Non-Goals:**
- Runtime sandboxing (separate concern)
- Package distribution/registry infrastructure
- Environment-aware modes (no "dev mode" - operator configures tolerance)

## Detailed Design

### Core Principles

1. **Environment agnostic** - No dev/prod modes. Operator configures signature requirements.
2. **Binary enforcement** - Signatures required or not. No warning mode.
3. **Hard fail** - Any verification failure rejects the package immediately.
4. **Hot-reload** - Key changes via API, no restart required.

### Cryptographic Approach

**Always asymmetric (Ed25519):**
- Private keys for signing (held by authorized signers)
- Public keys for verification (distributed to runners)
- Non-repudiation: can prove who signed what
- Clean rotation: add new pubkey, remove old - no re-signing needed

### Database Schema

```sql
-- Signing keys (private, encrypted at rest)
CREATE TABLE signing_keys (
    id UUID PRIMARY KEY,
    org_id UUID NOT NULL REFERENCES organizations(id),
    key_name VARCHAR(255) NOT NULL,
    encrypted_private_key BYTEA NOT NULL,  -- AES-256-GCM encrypted
    public_key BYTEA NOT NULL,             -- For reference/export
    key_fingerprint VARCHAR(64) NOT NULL,  -- SHA256 of public key
    created_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    UNIQUE(org_id, key_name)
);

-- Trusted keys (public keys for verification)
CREATE TABLE trusted_keys (
    id UUID PRIMARY KEY,
    org_id UUID NOT NULL REFERENCES organizations(id),
    key_fingerprint VARCHAR(64) NOT NULL,
    public_key BYTEA NOT NULL,
    key_name VARCHAR(255),
    trusted_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    UNIQUE(org_id, key_fingerprint)
);

-- Trust chain ACLs (explicit org → sub-org trust)
CREATE TABLE key_trust_acls (
    id UUID PRIMARY KEY,
    parent_org_id UUID NOT NULL REFERENCES organizations(id),
    child_org_id UUID NOT NULL REFERENCES organizations(id),
    granted_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    UNIQUE(parent_org_id, child_org_id)
);

-- Package signatures (stored with package metadata)
CREATE TABLE package_signatures (
    id UUID PRIMARY KEY,
    package_id UUID NOT NULL REFERENCES packages(id),
    key_fingerprint VARCHAR(64) NOT NULL,
    signature BYTEA NOT NULL,
    package_hash VARCHAR(64) NOT NULL,  -- SHA256 of package binary
    signed_at TIMESTAMPTZ NOT NULL,
    UNIQUE(package_id, key_fingerprint)
);
```

### Key Management API

```rust
// Key lifecycle operations
trait KeyManager {
    /// Generate new signing keypair, store encrypted in DB
    async fn create_signing_key(&self, org_id: Uuid, name: &str) -> Result<KeyInfo>;

    /// Export public key for distribution
    async fn export_public_key(&self, key_id: Uuid) -> Result<PublicKeyExport>;

    /// Import external public key to trust
    async fn trust_public_key(&self, org_id: Uuid, public_key: &[u8], name: &str) -> Result<()>;

    /// Revoke a key (signing or trusted)
    async fn revoke_key(&self, key_id: Uuid) -> Result<()>;

    /// Grant trust from parent org to child org
    async fn grant_trust(&self, parent_org: Uuid, child_org: Uuid) -> Result<()>;

    /// List trusted keys for an org (including inherited via ACL)
    async fn list_trusted_keys(&self, org_id: Uuid) -> Result<Vec<TrustedKey>>;
}
```

### Signing Flow

```rust
/// Sign a package with an org's signing key
async fn sign_package(
    package_path: &Path,
    signing_key_id: Uuid,
    key_manager: &impl KeyManager,
) -> Result<PackageSignature> {
    // 1. Hash the package
    let package_hash = sha256_file(package_path)?;

    // 2. Load and decrypt signing key
    let signing_key = key_manager.get_signing_key(signing_key_id).await?;

    // 3. Sign the hash
    let signature = ed25519_sign(&signing_key, &package_hash);

    // 4. Return signature metadata
    Ok(PackageSignature {
        key_fingerprint: signing_key.fingerprint,
        signature,
        package_hash,
        signed_at: Utc::now(),
    })
}
```

### Verification Flow

```rust
/// Verify package before loading - hard fail on any issue
async fn verify_and_load_package(
    package_path: &Path,
    signature: &PackageSignature,
    org_id: Uuid,
    config: &SecurityConfig,
    key_manager: &impl KeyManager,
) -> Result<Library, PackageError> {
    // 1. Check if signatures required
    if config.require_signatures {
        // 2. Compute current hash
        let current_hash = sha256_file(package_path)?;

        // 3. Verify hash matches signed hash (tamper detection)
        if current_hash != signature.package_hash {
            return Err(PackageError::TamperedPackage);
        }

        // 4. Get trusted keys (includes inherited via ACL)
        let trusted_keys = key_manager.list_trusted_keys(org_id).await?;

        // 5. Find matching trusted key
        let signer_key = trusted_keys
            .iter()
            .find(|k| k.fingerprint == signature.key_fingerprint)
            .ok_or(PackageError::UntrustedSigner)?;

        // 6. Verify signature
        if !ed25519_verify(&signer_key.public_key, &signature.signature, &current_hash) {
            return Err(PackageError::InvalidSignature);
        }

        // 7. Audit log
        tracing::info!(
            package = %package_path.display(),
            signer = %signature.key_fingerprint,
            org = %org_id,
            "Package signature verified"
        );
    }

    // 8. Load the library
    unsafe { Library::new(package_path) }
        .map_err(|e| PackageError::LoadFailed(e.to_string()))
}
```

### Configuration

```rust
/// Security configuration - set by operator
pub struct SecurityConfig {
    /// Whether package signatures are required (default: false)
    pub require_signatures: bool,

    /// Master encryption key for signing key storage (from env/secrets manager)
    /// Only needed if require_signatures is true or if managing signing keys
    pub key_encryption_key: Option<SecretKey>,
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

No environment-specific flags. Operator sets `require_signatures` based on their security tolerance.

### Local Development Workflow

**Signing does NOT impede local development:**

- `require_signatures` defaults to `false`
- Developer runs their own runner locally with default config
- Packages load without signatures - no friction
- No key infrastructure needed for local testing

```rust
// Local dev - just works, no signing setup needed
let config = SecurityConfig::default();  // require_signatures = false
let runner = Runner::new(config);
runner.load_package("./target/debug/my_workflow.so")?;  // No signature needed
```

The developer IS the operator of their local environment. When they deploy to shared/production infrastructure, that operator chooses to enable `require_signatures = true`.

This is not a "dev mode bypass" - it's simply that signature verification is opt-in. Operators who want security enable it; those who don't (like local dev) don't.

## Testing Strategy

- Test with valid signatures
- Test with tampered packages (modified after signing)
- Test with expired/invalid signatures
- Test with unknown signers
- Test key rotation scenarios

## Technology Decisions

### Cryptography Stack (Pure Rust)

| Component | Crate | Rationale |
|-----------|-------|-----------|
| Ed25519 signing | `ed25519-dalek` | Pure Rust, well-audited, no C deps |
| SHA256 hashing | `sha2` | RustCrypto ecosystem, pure Rust |
| Key encryption | `aes-gcm` | AES-256-GCM, RustCrypto, pure Rust |
| Secure random | `rand` + `getrandom` | Already in dependency tree |

### Key Encoding

| Format | Use |
|--------|-----|
| Raw bytes | DB storage |
| PEM | Export/import for humans and CI |
| SHA256 hex (64 chars) | Key fingerprints |

### CLI Structure

Subcommands under main `cloacina` binary:

```
cloacina keys generate    # Generate keypair
cloacina keys export      # Export public key
cloacina keys trust       # Import/trust external key
cloacina keys revoke      # Revoke a key
cloacina keys list        # List keys

cloacina package sign     # Sign a package
cloacina package verify   # Verify a package signature
cloacina package upload   # Upload package (with signature)
```

### Signing Modes

Both supported:

1. **API signing** - Key never leaves DB, more secure
   ```bash
   cloacina package sign ./pkg.so --key-id <id> --api-url <url>
   ```

2. **Offline signing** - Exported key, simpler for CI
   ```bash
   cloacina package sign ./pkg.so --key ./signing-key.pem
   ```

### Signature Storage

- **Primary**: Stored in DB alongside package metadata
- **Optional**: Detached `.sig` file for portability/offline workflows

```bash
# Output detached signature file
cloacina package sign ./pkg.so --key ./key.pem --output ./pkg.so.sig

# Upload with detached signature
cloacina package upload ./pkg.so --signature ./pkg.so.sig
```

## Alternatives Considered

1. **GPG signatures** - More complex tooling, harder to embed. Rejected.

2. **Sigstore/cosign** - External dependency, requires internet. Rejected.

3. **Hash-only verification** - No signer identity, vulnerable to MITM. Rejected.

4. **Symmetric (HMAC) signing** - Simpler single secret, but anyone who can verify can forge. No non-repudiation. Key rotation requires re-signing everything. Rejected in favor of asymmetric.

5. **File-based key storage** - Simpler but doesn't support hot-reload or multi-tenant. Rejected in favor of database storage.

6. **Environment-aware modes (dev/prod)** - `allow_unsigned_in_dev` flag is a footgun. Rejected - operator explicitly configures tolerance.

## Implementation Plan

### Phase 1: Database Schema & Migrations
- Add signing_keys, trusted_keys, key_trust_acls, package_signatures tables
- Implement encrypted key storage (AES-256-GCM)
- Both PostgreSQL and SQLite support

### Phase 2: Key Management API
- Implement KeyManager trait
- Key generation, export, import, revocation
- Trust chain ACL management
- Hot-reload support (cache invalidation on key changes)

### Phase 3: Signing Infrastructure
- Package hashing (SHA256)
- Ed25519 signing implementation
- sign_package() function
- CLI tool for offline signing (optional - can also sign via API)

### Phase 4: Verification Integration
- Integrate verify_and_load_package() into package loader
- SecurityConfig with require_signatures flag
- Hard fail error types (TamperedPackage, UntrustedSigner, InvalidSignature)

### Phase 5: Audit Logging
- Log all package load attempts (success/failure)
- Log all key operations (create, revoke, trust grant)
- Structured logging for SIEM integration

### Phase 6: Testing & Documentation
- Unit tests for crypto operations
- Integration tests for trust chain resolution
- Key rotation scenarios
- Documentation for operators
