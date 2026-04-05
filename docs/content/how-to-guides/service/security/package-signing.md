---
title: "Package Signing"
weight: 10
---

# Package Signing and Verification

Cloacina supports cryptographic signing of workflow packages to ensure integrity and authenticity. This guide covers how to set up and use package signing in production environments.

## Overview

Package signing provides:

- **Integrity verification**: Detect if a package has been tampered with
- **Authenticity**: Verify packages come from a trusted source
- **Audit trail**: Log all package loads and verification results
- **Multi-tenant trust**: Support organizational trust hierarchies

## Key Concepts

### Signing Keys

Cloacina uses **Ed25519** asymmetric cryptography for package signing:

- **Private key**: Used to sign packages (kept secret, encrypted at rest)
- **Public key**: Used to verify signatures (distributed to verifiers)
- **Key fingerprint**: SHA256 hash of the public key, used for identification

### Trust Model

Keys are managed per-organization:

- **Signing keys**: Owned by an organization, used to sign packages
- **Trusted keys**: Public keys an organization trusts for verification
- **Trust ACLs**: Allow parent organizations to trust child organizations' keys

## Enabling Signature Verification

### Configuration

To require signature verification, configure `SecurityConfig`:

```rust
use cloacina::security::SecurityConfig;

let config = SecurityConfig {
    require_signatures: true,
    key_encryption_key: Some(load_key_from_env("CLOACINA_KEY_ENCRYPTION_KEY")),
};
```

When `require_signatures` is `true`:
- All packages must have valid signatures from trusted keys
- Unsigned packages will fail to load
- Tampered packages will be rejected

### Key Encryption Key

The `key_encryption_key` is a 32-byte AES-256 key used to encrypt private signing keys at rest in the database. Store this securely (e.g., in a secrets manager) and provide it at runtime.

## Key Management API

### Generate a Signing Key

```rust
use cloacina::security::{DbKeyManager, KeyManager};

let key_manager = DbKeyManager::new(dal);
let key_info = key_manager
    .create_signing_key(org_id, "release-key-v1", &master_key)
    .await?;

println!("Key ID: {}", key_info.id);
println!("Fingerprint: {}", key_info.fingerprint);
```

### Export Public Key

Export the public key for distribution to verifiers:

```rust
let export = key_manager.export_public_key(key_info.id).await?;

// PEM format for sharing
println!("{}", export.public_key_pem);
```

### Trust a Public Key

Before verifying packages signed by a key, trust its public key:

```rust
// From raw bytes
key_manager
    .trust_public_key(org_id, &public_key_bytes, Some("Release Key"))
    .await?;

// From PEM
key_manager
    .trust_public_key_pem(org_id, &pem_string, Some("Release Key"))
    .await?;
```

### Multi-Organization Trust

Grant trust from a parent organization to a child:

```rust
// Parent org will now trust all keys trusted by child org
key_manager.grant_trust(parent_org_id, child_org_id).await?;
```

## Package Signing

### Sign a Package

```rust
use cloacina::security::{DbPackageSigner, PackageSigner};

let signer = DbPackageSigner::new(dal);

// Sign with a database-stored key
let signature = signer
    .sign_package_with_db_key(
        &package_path,
        key_id,
        &master_key,
        true, // Store signature in database
    )
    .await?;

println!("Package hash: {}", signature.package_hash);
println!("Signer: {}", signature.key_fingerprint);
```

### Detached Signatures

Create a `.sig` file for distribution alongside packages:

```rust
use cloacina::security::DetachedSignature;

let detached = DetachedSignature::from_signature_info(&signature);
detached.write_to_file("my-package.so.sig")?;
```

The signature file is JSON:

```json
{
  "version": 1,
  "algorithm": "ed25519",
  "package_hash": "abc123...",
  "key_fingerprint": "def456...",
  "signature": "base64...",
  "signed_at": "2026-01-28T12:00:00Z"
}
```

## Package Verification

### Online Verification (Database)

When the database is available:

```rust
use cloacina::security::{verify_package, SignatureSource};

let result = verify_package(
    &package_path,
    org_id,
    SignatureSource::Auto, // Try .sig file, then database
    &package_signer,
    &key_manager,
)
.await?;

println!("Verified by: {}", result.signer_fingerprint);
```

### Offline Verification

When only the public key is available:

```rust
use cloacina::security::verify_package_offline;

let result = verify_package_offline(
    &package_path,
    &signature_path,
    &public_key_bytes,
)?;
```

## Key Rotation

To rotate signing keys:

1. **Generate new key**:
   ```rust
   let new_key = key_manager.create_signing_key(org_id, "release-v2", &master_key).await?;
   ```

2. **Trust new key**:
   ```rust
   key_manager.trust_public_key(org_id, &new_key.public_key, Some("release-v2")).await?;
   ```

3. **Update CI to use new key** for signing new packages

4. **During transition**: Both old and new signatures verify

5. **After transition**: Revoke old trusted key
   ```rust
   key_manager.revoke_trusted_key(old_trusted_key_id).await?;
   ```

6. **Optionally**: Revoke old signing key to prevent new signatures
   ```rust
   key_manager.revoke_signing_key(old_signing_key_id).await?;
   ```

## Audit Logging

All security operations are logged with structured fields for SIEM integration:

```json
{
  "event_type": "package.load.success",
  "org_id": "550e8400-e29b-41d4-a716-446655440000",
  "package_path": "/packages/workflow.so",
  "package_hash": "abc123...",
  "signer_fingerprint": "def456...",
  "signature_verified": true
}
```

Event types include:
- `package.load.success` / `package.load.failure`
- `package.signed` / `package.sign.failure`
- `key.signing.created` / `key.signing.revoked`
- `key.trusted.added` / `key.trusted.revoked`
- `key.trust_acl.granted` / `key.trust_acl.revoked`
- `verification.success` / `verification.failure`

## Security Errors

The verification system provides specific error types:

| Error | Meaning |
|-------|---------|
| `TamperedPackage` | Package content doesn't match signature hash |
| `UntrustedSigner` | Signature from a key not trusted by this organization |
| `InvalidSignature` | Cryptographic verification failed |
| `SignatureNotFound` | No signature found for this package |
| `MalformedSignature` | Signature file is corrupt or invalid format |

## Best Practices

1. **Rotate keys regularly**: Create new signing keys periodically
2. **Minimize key access**: Only CI/CD systems should have signing key access
3. **Use trust ACLs**: Delegate trust to organizational units
4. **Monitor audit logs**: Set up alerts for verification failures
5. **Secure master key**: Store the key encryption key in a secrets manager
6. **Test verification**: Verify signature checking works in staging before production
