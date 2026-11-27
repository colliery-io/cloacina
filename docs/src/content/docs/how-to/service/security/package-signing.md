---
title: "Sign and Verify Workflow Packages"
sidebar:
  order: 10
  label: "Package Signing"
---

Cloacina supports cryptographic (Ed25519) signing of workflow packages so you can
detect tampering and verify authenticity. This guide covers the keys a tenant
trusts on a server (`cloacinactl trusted-key`), and the library-side tasks:
enabling verification, generating and trusting keys, signing, verifying, and
rotating keys.

:::caution[No CLI signing path yet]
`cloacinactl package pack --sign` and `package publish --sign` **fail hard**
with an error, and no route stores a signature. A signature row is written
through the **library-side** API on this page. What the server and the CLI do:
*enforcement* (`--require-signatures`) and the management of each tenant's
trusted keys (`cloacinactl trusted-key add|list|revoke`).
:::

- For the **trust model and why signing is optional**, see
  [Security Model](../../../../explanation/service/security-model/#signing-keys-and-the-trust-model).
- For the **full API surface** (method catalog, signature format, error
  variants), see [Package Signing API Reference](../../../../reference/package-signing-api/).
- To **enforce signatures at the server upload boundary**, see
  [Require signed packages](../../require-signed-packages/).

## Manage a tenant's trusted keys on a server

A trusted key is an Ed25519 public key. With `--require-signatures` on the
server, an upload to a tenant passes when its signer is a live trusted key of
that tenant, or of the server-wide org if the operator set one. The lookup
order is in [Require signed packages](../../require-signed-packages/#how-an-upload-is-checked).

A tenant `admin` key manages the keys of its own tenant. A platform admin key
manages the keys of any tenant: name it with `--tenant`. The `read`, `write`
and `agent` roles are refused.

**Add a key**, from a PEM file or from the raw key as 64 hexadecimal
characters:

```sh
cloacinactl trusted-key add --pem-file release-key.pub.pem --name release-key-v1
cloacinactl trusted-key add --public-key 3b6a27bc...29da29 --name ci
```

The answer has the key's `id` and its `fingerprint` (the SHA-256 of the public
key). A tenant holds a key once: adding a key the tenant already has, live or
revoked, is refused with `409 trusted_key_exists`.

**List the keys.** A revoked key stays in the list, with `revoked: true` and
its `revoked_at` time:

```sh
cloacinactl trusted-key list
```

**Revoke a key**, by its id:

```sh
cloacinactl trusted-key revoke 7c9e6679-7425-40de-944b-e07fc1f90ae7
```

A revoke takes effect at the next upload: the server does not cache keys. A
revoked key cannot be added again; trust a new key instead. A key that the
tenant revoked still passes if the server-wide org also trusts it.

The same operations over HTTP are `POST`, `GET` and `DELETE` on
`/v1/tenants/{tenant_id}/trusted-keys`: see the
[HTTP API Reference](../../../../reference/http-api/#trusted-keys). Each add
and each revoke writes a row to the [audit record](../../../../reference/http-api/#audit)
(`trusted_key.add`, `trusted_key.revoke`).

The org id of a tenant in the `trusted_keys` table is
`cloacina::security::tenant_org_id("<tenant>")`. A key added through the
library with that org id is the same row as one added through the route.

## Enable signature verification

To require verification, configure `SecurityConfig`:

```rust
use cloacina::security::SecurityConfig;

let config = SecurityConfig {
    require_signatures: true,
    // 32-byte master key; only needed when SIGNING with database-stored
    // keys — verification-only loading doesn't require it.
    key_encryption_key: Some(master_key_bytes),
    // The server-wide org. `cloacina-server` checks an upload against the
    // trusted keys of its tenant first, then against the keys of this org
    // if one is set. With `None`, each tenant trusts only its own keys.
    verification_org_id: Some(org_id),
};
```

When `require_signatures` is `true`, unsigned packages and packages signed by an
untrusted or tampered key fail to load.

The `key_encryption_key` is a 32-byte key used to encrypt private signing
keys at rest in the database. Store it in a secrets manager and provide it at
runtime.

## Generate and trust a signing key

Generate a signing key for your organization, then trust its public key so
packages signed with it will verify:

```rust
use cloacina::security::{DbKeyManager, KeyManager};

let key_manager = DbKeyManager::new(dal);

// Generate a signing key (private key encrypted under master_key)
let key_info = key_manager
    .create_signing_key(org_id, "release-key-v1", &master_key)
    .await?;

// Export the public key for distribution
let export = key_manager.export_public_key(key_info.id).await?;
println!("{}", export.public_key_pem);

// Trust the public key (from raw bytes or PEM)
key_manager
    .trust_public_key(org_id, &key_info.public_key, Some("Release Key"))
    .await?;
```

To let a parent organization trust everything a child organization trusts, grant
trust between them:

```rust
key_manager.grant_trust(parent_org_id, child_org_id).await?;
```

## Sign a package

```rust
use cloacina::security::{DbPackageSigner, PackageSigner};

let signer = DbPackageSigner::new(dal);

let signature = signer
    .sign_package_with_db_key(&package_path, key_id, &master_key, true)
    .await?;
```

To distribute a detached `.sig` sidecar alongside the package:

```rust
use cloacina::security::DetachedSignature;

let detached = DetachedSignature::from_signature_info(&signature);
detached.write_to_file("my-package.so.sig")?;
```

## Verify a package

With the database available, verify against a stored signature (or an adjacent
`.sig` file) using the organization's trusted keys:

```rust
use cloacina::security::{verify_package, SignatureSource};

let result = verify_package(
    &package_path,
    org_id,
    SignatureSource::Auto, // try .sig file, then database
    &package_signer,
    &key_manager,
)
.await?;
```

When only the public key and a detached signature are available (no database),
verify offline:

```rust
use cloacina::security::verify_package_offline;

let result = verify_package_offline(&package_path, &signature_path, &public_key_bytes)?;
```

## Rotate signing keys

1. **Generate a new key:**
   ```rust
   let new_key = key_manager.create_signing_key(org_id, "release-v2", &master_key).await?;
   ```

2. **Trust the new key:**
   ```rust
   key_manager.trust_public_key(org_id, &new_key.public_key, Some("release-v2")).await?;
   ```

3. **Update CI to sign new packages with the new key.**

4. **During the transition**, both old and new signatures verify (both keys are
   trusted).

5. **After the transition, revoke the old trusted key:**
   ```rust
   key_manager.revoke_trusted_key(org_id, old_trusted_key_id).await?;
   ```

For a tenant's keys on a server, steps 2 and 5 are `cloacinactl trusted-key
add` and `cloacinactl trusted-key revoke`.

6. **Optionally, revoke the old signing key** to prevent new signatures:
   ```rust
   key_manager.revoke_signing_key(old_signing_key_id).await?;
   ```

## See Also

- [Security Model](../../../../explanation/service/security-model/#package-signature-verification) — trust model and threat model.
- [Package Signing API Reference](../../../../reference/package-signing-api/) — full method catalog, signature format, audit events, and error variants.
- [Require signed packages](../../require-signed-packages/) — turn on server-side enforcement.
