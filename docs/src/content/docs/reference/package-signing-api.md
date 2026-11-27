---
title: "Package Signing API Reference"
description: "The cloacina::security key-management, signing, and verification API surface, the trusted keys of a tenant on a server, the detached-signature format, audit event types, and verification error variants."
sidebar:
  order: 21
---

The `cloacina::security` module provides Ed25519 package signing, key management,
and signature verification. This page is the dry API surface; for the conceptual
model (keys, trust, ACLs) see [Security Model](../../explanation/service/security-model/#signing-keys-and-the-trust-model),
and for procedures see [Package signing](../../how-to/service/security/package-signing/).

```rust
use cloacina::security::{DbKeyManager, KeyManager, DbPackageSigner, PackageSigner};
```

## Trusted keys of a tenant on a server

On `cloacina-server`, each tenant has its own trusted keys. They are the rows
of `trusted_keys` whose `org_id` is `tenant_org_id("<tenant>")`.

| Operation | HTTP route | CLI |
|---|---|---|
| Add a key | `POST /v1/tenants/{tenant_id}/trusted-keys` | `cloacinactl trusted-key add --pem-file <PATH>` or `--public-key <HEX>`, `[--name <NAME>]` |
| List the keys, revoked ones marked | `GET /v1/tenants/{tenant_id}/trusted-keys` | `cloacinactl trusted-key list` |
| Revoke a key | `DELETE /v1/tenants/{tenant_id}/trusted-keys/{key_id}` | `cloacinactl trusted-key revoke <ID> [--force]` |

All three need the permission `trusted_keys.manage` in the tenant of the path:
the `admin` role of that tenant holds it, and a platform admin key has it for
every tenant. Request and response bodies are in the
[HTTP API Reference](../http-api/#trusted-keys).

**Lookup order at upload.** With `--require-signatures`, the server looks the
signer of an upload up in the trusted keys of the tenant's org first, then in
the trusted keys of the server-wide org (`--verification-org-id`) if one is
set. The first org that holds the key, live, decides. A key revoked in the
tenant's org and live in the server-wide org passes.

| Function | Description |
|---|---|
| `tenant_org_id(tenant: &str) -> UniversalUuid` | The org id of a tenant: UUIDv5 of the tenant name under a fixed namespace. The same name gives the same id in every process. The tenant secrets use the same id. |

## KeyManager

Trait for managing signing keys, trusted keys, and trust relationships.
Implementations (e.g. `DbKeyManager`) are `Send + Sync` and do not cache, so
revocations take effect immediately.

| Method | Signature | Description |
|---|---|---|
| `create_signing_key` | `(org_id, name: &str, master_key: &[u8]) -> SigningKeyInfo` | Generate a new Ed25519 signing keypair and store the private key encrypted under `master_key`. `name` must be unique per org. |
| `get_signing_key_info` | `(org_id, key_id) -> SigningKeyInfo` | Metadata for a signing key (no private material). The key must belong to `org_id`; a key of another org gives `KeyError::NotFound`. |
| `get_signing_key` | `(org_id, key_id, master_key: &[u8]) -> (Vec<u8>, Vec<u8>)` | Decrypt and return `(public_key, private_key)` raw bytes for signing. The key must belong to `org_id`; a key of another org gives `KeyError::NotFound`. |
| `export_public_key` | `(org_id, key_id) -> PublicKeyExport` | Export a public key (fingerprint, PEM, raw) for distribution. The key must belong to `org_id`; a key of another org gives `KeyError::NotFound`. |
| `trust_public_key` | `(org_id, public_key: &[u8], name: Option<&str>) -> TrustedKeyInfo` | Trust a raw 32-byte Ed25519 public key for verification. |
| `trust_public_key_pem` | `(org_id, pem: &str, name: Option<&str>) -> TrustedKeyInfo` | Trust a public key supplied in PEM form. |
| `revoke_signing_key` | `(org_id, key_id) -> ()` | Prevent future signing with this key. The key must belong to `org_id`; a key of another org gives `KeyError::NotFound`. |
| `revoke_trusted_key` | `(org_id, key_id) -> ()` | Prevent future verification against this trusted key. The key must be trusted directly by `org_id`; a key of another org gives `KeyError::NotFound`. |
| `grant_trust` | `(parent_org, child_org) -> ()` | Parent org implicitly trusts all keys the child org trusts. |
| `revoke_trust` | `(parent_org, child_org) -> ()` | Revoke a trust grant between organizations. |
| `list_trusted_keys` | `(org_id) -> Vec<TrustedKeyInfo>` | The live keys a verification for `org_id` accepts: its own and the ones inherited through a trust grant. |
| `find_trusted_key` | `(org_id, fingerprint: &str) -> Option<TrustedKeyInfo>` | The live key with this fingerprint that `org_id` trusts, directly or through a trust grant. |

All methods are `async` and return `Result<_, KeyError>`.

`DbKeyManager` adds two functions of its own:

| Function | Signature | Description |
|---|---|---|
| `list_org_trusted_keys` | `(org_id) -> Vec<TrustedKeyInfo>` | The keys `org_id` trusts directly, the revoked ones included, oldest first. No inherited key. `async`. |
| `decode_public_key_pem` | `(pem: &str) -> Result<Vec<u8>, KeyError>` | The raw 32 bytes of an Ed25519 public key in PEM form. |

`trusted_keys` holds one row for a key in an org (`UNIQUE(org_id,
key_fingerprint)`), revoked or not: a revoked key cannot be trusted again in
the same org.

## PackageSigner

Trait for signing packages and managing signatures. `DbPackageSigner` is the
database-backed implementation.

| Method | Signature | Description |
|---|---|---|
| `sign_package_with_db_key` | `(package_path, org_id, key_id, master_key: &[u8], store_signature: bool) -> PackageSignatureInfo` | Sign a package with a database-stored key; optionally persist the signature. The key must belong to `org_id`; a key of another org gives `PackageSignError::KeyNotFound`. `async`. |
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
| `verify_package_bytes(package_data, org_id, SignatureSource, &package_signer, &key_manager)` | Verify in-memory package bytes against the trusted keys of one org. `async`. |
| `verify_package_bytes_for_orgs(package_data, org_ids: &[UniversalUuid], SignatureSource, &package_signer, &key_manager)` | Verify in-memory package bytes, with the signer looked up in each org of `org_ids` in order. Returns the result and the id of the org that trusted the key. An empty list trusts no key. `async`. |

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

- [Package signing](../../how-to/service/security/package-signing/) — task procedures (enable verification, sign, verify, rotate keys).
- [Security Model](../../explanation/service/security-model/#package-signature-verification) — trust model and threat model.
- [Require signed packages](../../how-to/service/require-signed-packages/) — server-side enforcement.
- [API Error Envelope](../api-error-envelope/#403-forbidden) — HTTP error codes for signature failures.
