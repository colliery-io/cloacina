---
title: "Package Signing API Reference"
description: "The cloacina::security key-management, signing, and verification API surface, the detached-signature format, audit event types, and verification error variants."
weight: 21

---

# Package Signing API Reference

The `cloacina::security` module provides Ed25519 package signing, key management,
and signature verification. This page is the dry API surface; for the conceptual
model (keys, trust, ACLs) see [Security Model]({{< ref "/service/explanation/security-model" >}}#signing-keys-and-the-trust-model),
and for procedures see [Package signing]({{< ref "/service/how-to/security/package-signing" >}}).

```rust
use cloacina::security::{DbKeyManager, KeyManager, DbPackageSigner, PackageSigner};
```

## KeyManager

Trait for managing signing keys, trusted keys, and trust relationships.
Implementations (e.g. `DbKeyManager`) are `Send + Sync` and do not cache, so
revocations take effect immediately.

| Method | Signature | Description |
|---|---|---|
| `create_signing_key` | `(org_id, name: &str, master_key: &[u8]) -> SigningKeyInfo` | Generate a new Ed25519 signing keypair and store the private key encrypted under `master_key`. `name` must be unique per org. |
| `get_signing_key_info` | `(key_id) -> SigningKeyInfo` | Metadata for a signing key (no private material). |
| `get_signing_key` | `(key_id, master_key: &[u8]) -> (Vec<u8>, Vec<u8>)` | Decrypt and return `(public_key, private_key)` raw bytes for signing. |
| `export_public_key` | `(key_id) -> PublicKeyExport` | Export a public key (fingerprint, PEM, raw) for distribution. |
| `trust_public_key` | `(org_id, public_key: &[u8], name: Option<&str>) -> TrustedKeyInfo` | Trust a raw 32-byte Ed25519 public key for verification. |
| `trust_public_key_pem` | `(org_id, pem: &str, name: Option<&str>) -> TrustedKeyInfo` | Trust a public key supplied in PEM form. |
| `revoke_signing_key` | `(key_id) -> ()` | Prevent future signing with this key. |
| `revoke_trusted_key` | `(key_id) -> ()` | Prevent future verification against this trusted key. |
| `grant_trust` | `(parent_org, child_org) -> ()` | Parent org implicitly trusts all keys the child org trusts. |
| `revoke_trust` | `(parent_org, child_org) -> ()` | Revoke a trust grant between organizations. |

All methods are `async` and return `Result<_, KeyError>`.

## PackageSigner

Trait for signing packages and managing signatures. `DbPackageSigner` is the
database-backed implementation.

| Method | Signature | Description |
|---|---|---|
| `sign_package_with_db_key` | `(package_path, key_id, master_key: &[u8], store_signature: bool) -> PackageSignatureInfo` | Sign a package with a database-stored key; optionally persist the signature. `async`. |
| `sign_package_with_raw_key` | `(package_path, private_key: &[u8], public_key: &[u8]) -> PackageSignatureInfo` | Offline signing with raw key bytes. |
| `sign_package_data` | `(package_data: &[u8], private_key: &[u8], public_key: &[u8]) -> PackageSignatureInfo` | Sign in-memory package bytes. |
| `store_signature` | `(&PackageSignatureInfo) -> UniversalUuid` | Persist a signature. `async`. |
| `find_signature` | `(package_hash: &str) -> Option<PackageSignatureInfo>` | Look up a stored signature by package hash. `async`. |
| `find_signatures` | `(package_hash: &str) -> Vec<PackageSignatureInfo>` | All stored signatures for a package hash. `async`. |
| `verify_package` | `(package_path, org_id) -> PackageSignatureInfo` | Verify against a stored signature using `org_id`'s trusted keys. `async`. |
| `verify_package_with_detached_signature` | `(package_path, &DetachedSignature, public_key: &[u8]) -> ()` | Verify against a detached `.sig` file and an explicit public key. |

Signing/verification methods return `Result<_, PackageSignError>`.

## Free functions

| Function | Description |
|---|---|
| `verify_package(package_path, org_id, SignatureSource, &package_signer, &key_manager)` | High-level online verification. `SignatureSource::Auto` tries the `.sig` file, then the database. Returns a result carrying the `signer_fingerprint`. `async`. |
| `verify_package_offline(package_path, signature_path, public_key: &[u8])` | Verify with only a detached signature file and a public key — no database. |
| `verify_package_bytes(...)` | Verify in-memory package bytes. `async`. |

## Types

- **`SigningKeyInfo`** — `id`, `org_id`, `key_name`, `fingerprint` (SHA-256 hex of
  the public key), `public_key` (32-byte Ed25519), `created_at`, `revoked_at`.
  `is_active()` returns `revoked_at.is_none()`.
- **`TrustedKeyInfo`** — `id`, `org_id`, `fingerprint`, `public_key`, optional
  `key_name`, `trusted_at`, `revoked_at`. `is_active()` as above.
- **`PublicKeyExport`** — `fingerprint`, `public_key_pem` (Ed25519
  SubjectPublicKeyInfo PEM), `public_key_raw` (32 bytes).
- **`PackageSignatureInfo`** — carries `package_hash` and `key_fingerprint`, among
  other fields.
- **`DetachedSignature`** — serializable `.sig` sidecar; `from_signature_info`,
  `write_to_file`, `read_from_file`.

## Detached signature file format

`DetachedSignature::write_to_file` produces a JSON sidecar (conventionally
`<package>.sig`):

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

## Audit event types

Security operations emit structured audit events (JSON, for SIEM ingestion) with
fields such as `org_id`, `package_path`, `package_hash`, `signer_fingerprint`, and
`signature_verified`. Event types:

- `package.load.success` / `package.load.failure`
- `package.signed` / `package.sign.failure`
- `key.signing.created` / `key.signing.revoked`
- `key.trusted.added` / `key.trusted.revoked`
- `key.trust_acl.granted` / `key.trust_acl.revoked`
- `verification.success` / `verification.failure`

## Verification errors

`VerificationError` variants returned by the verification path:

| Error | Meaning |
|-------|---------|
| `TamperedPackage` | Package content doesn't match the signature hash. |
| `UntrustedSigner` | Signature from a key not trusted by this organization. |
| `InvalidSignature` | Cryptographic verification failed. |
| `SignatureNotFound` | No signature found for this package. |
| `MalformedSignature` | Signature file is corrupt or an invalid format. |

Key-management operations return `KeyError` (`NotFound`, `Revoked`,
`DuplicateName`, `InvalidFormat`, `InvalidPem`, `Encryption`, `Decryption`,
`TrustAlreadyExists`, `TrustNotFound`, `Database`); signing operations return
`PackageSignError`.

## See Also

- [Package signing]({{< ref "/service/how-to/security/package-signing" >}}) — task procedures (enable verification, sign, verify, rotate keys).
- [Security Model]({{< ref "/service/explanation/security-model" >}}#package-signature-verification) — trust model and threat model.
- [Require signed packages]({{< ref "/service/how-to/require-signed-packages" >}}) — server-side enforcement.
- [API Error Envelope]({{< ref "/reference/api-error-envelope" >}}#403-forbidden) — HTTP error codes for signature failures.
