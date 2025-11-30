---
id: implement-package-signing-and
level: initiative
title: "Implement Package Signing and Verification for Dynamic Libraries"
short_code: "CLOACI-I-0008"
created_at: 2025-11-29T02:40:14.993840+00:00
updated_at: 2025-11-29T02:40:14.993840+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


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
- Implement cryptographic signing for workflow packages
- Verify signatures before loading any dynamic library
- Create key management infrastructure for signing
- Add audit logging for all package loads
- Support signature verification in CI/CD pipelines

**Non-Goals:**
- Runtime sandboxing (separate concern)
- Package distribution/registry infrastructure
- Revoking compromised keys (v2 feature)

## Detailed Design

### Package Signing Format

Use Ed25519 signatures with a detached signature file:

```
my_workflow.so          # The compiled package
my_workflow.so.sig      # Detached signature file
```

Signature file format (JSON):
```json
{
  "version": 1,
  "algorithm": "ed25519",
  "signature": "base64_encoded_signature",
  "signed_hash": "sha256_of_package",
  "signer_id": "key_fingerprint",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### Signing CLI Tool

```bash
# Generate signing keypair
cloacina-sign keygen --output ~/.cloacina/signing_key

# Sign a package
cloacina-sign sign my_workflow.so --key ~/.cloacina/signing_key

# Verify a package
cloacina-sign verify my_workflow.so --pubkey trusted_keys/
```

### Verification Integration

```rust
pub fn load_package(path: &Path, config: &PackageConfig) -> Result<Package, PackageError> {
    // 1. Compute hash of package file
    let package_hash = sha256_file(path)?;
    
    // 2. Load and verify signature
    let sig_path = path.with_extension("so.sig");
    let signature = load_signature(&sig_path)?;
    
    // 3. Verify against trusted keys
    verify_signature(&package_hash, &signature, &config.trusted_keys)?;
    
    // 4. Log successful verification
    tracing::info!(
        package = %path.display(),
        signer = %signature.signer_id,
        "Package signature verified"
    );
    
    // 5. Load the dynamic library
    unsafe { libloading::Library::new(path) }
}
```

### Trust Configuration

```toml
# cloacina.toml
[packages]
require_signatures = true
trusted_keys_dir = "/etc/cloacina/trusted_keys"
allow_unsigned_in_dev = true  # Disable in production
```

## Testing Strategy

- Test with valid signatures
- Test with tampered packages (modified after signing)
- Test with expired/invalid signatures
- Test with unknown signers
- Test key rotation scenarios

## Alternatives Considered

1. **GPG signatures** - More complex tooling, harder to embed
2. **Sigstore/cosign** - External dependency, requires internet
3. **Hash-only verification** - No signer identity, vulnerable to MITM

## Implementation Plan

1. **Phase 1:** Design signature format and CLI tool
2. **Phase 2:** Implement signing tool
3. **Phase 3:** Integrate verification into package loader
4. **Phase 4:** Add configuration options
5. **Phase 5:** Documentation and key management guide
6. **Phase 6:** CI/CD integration examples