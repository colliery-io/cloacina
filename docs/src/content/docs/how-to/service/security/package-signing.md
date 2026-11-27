---
title: "Sign and Verify Workflow Packages"
sidebar:
  order: 10
  label: "Package Signing"
---

Cloacina supports cryptographic (Ed25519) signing of workflow packages so you can
detect tampering and verify authenticity. This guide covers signing and
publishing a package with `cloacinactl`, the keys a tenant trusts on a server
(`cloacinactl trusted-key`), and the library-side tasks: enabling
verification, generating and trusting keys, signing, verifying, and rotating
keys.

- For the **trust model and why signing is optional**, see
  [Security Model](../../../../explanation/service/security-model/#signing-keys-and-the-trust-model).
- For the **full API surface** (method catalog, signature format, error
  variants), see [Package Signing API Reference](../../../../reference/package-signing-api/).
- To **enforce signatures at the server upload boundary**, see
  [Require signed packages](../../require-signed-packages/).

## Sign and publish a package with cloacinactl

The publisher signs on its own machine with a private key in a file. The
server gets the archive and the signature, never the private key. The tenant
trusts the matching public key.

**1. Make a key pair.** The signing key is an unencrypted Ed25519 private key
in PKCS#8 PEM. OpenSSL writes one:

```sh
openssl genpkey -algorithm ed25519 -out release-key.pem
chmod 600 release-key.pem
openssl pkey -in release-key.pem -pubout -out release-key.pub.pem
```

The permissions of the file are what protects the key. On Unix `cloacinactl`
refuses a key file that the group or others can read. Keep the file out of the
package source and out of version control. In CI, write it from the secret
store to a file with mode `600` for the length of the job.

**2. The tenant trusts the public key.** A tenant `admin` key does this once
for each signing key:

```sh
cloacinactl trusted-key add --pem-file release-key.pub.pem --name release-key-v1
```

**3. Sign and publish.** `publish --sign` builds, packs, signs the archive and
uploads it with the signature:

```sh
cloacinactl package publish ./my-package --sign release-key.pem
```

To sign in one step and upload in another, `pack --sign` writes the signature
beside the archive as `<archive>.sig`, and `upload` sends that file with the
archive when it is there:

```sh
cloacinactl package pack ./my-package --out my-package.cloacina --sign release-key.pem
# my-package.cloacina and my-package.cloacina.sig
cloacinactl package upload my-package.cloacina
```

`upload --signature <SIG_FILE>` sends a signature file from another path. The
signature file is public: it holds the hash of the archive, the fingerprint of
the key and the signature. The archive must not change after it is signed;
`upload` refuses a signature file that was made for other bytes.

The server checks the signature against the archive and against the trusted
keys of the tenant, then of the server-wide org if one is set. The answers of
a refused upload are in
[Require signed packages](../../require-signed-packages/#how-an-upload-is-checked).
A signature that is sent is checked on every server, also on one that does
not require signatures.

The Rust, Python and TypeScript clients send a signature too:
`upload_workflow_signed(package, signature, tenant)`,
`upload_workflow(package, tenant, signature=...)` and
`uploadWorkflow(pkg, tenant, signature)`. The signature is the text of the
`.sig` file.

## Manage a tenant's trusted keys on a server

A trusted key is an Ed25519 public key. An upload to a tenant passes the
signature check when its signer is a live trusted key of that tenant, or of
the server-wide org if the operator set one. The lookup order is in
[Require signed packages](../../require-signed-packages/#how-an-upload-is-checked).

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

A revoke takes effect at the next upload: the server does not cache keys. An
upload signed by the key is refused with `403 signer_revoked`. A revoked key
cannot be added again; trust a new key instead. A key that the tenant revoked
still passes if the server-wide org also trusts it.

A revoke does not stop a package that is already loaded: its workflows keep
running. With `--require-signatures`, the package is checked again the next
time the server loads it, which is at the next start of the server, and that
load refuses a package whose signer is revoked. To stop a loaded package at
once, delete it (`cloacinactl package delete`). To keep it past the next
start, publish it again signed by a key the tenant trusts.

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

A runner with `require_signatures` verifies each package when it loads it. The
package loads when a signature row for its bytes was stored for the org of the
runner's tenant (`tenant_org_id(tenant)`, `public` for an embedded runner) or
for `verification_org_id`, the signer is a live trusted key of one of the two,
and the signature verifies. A row stored with no org does not count. The
trusted keys are read from the runner's database, or from the database given
with `DefaultRunnerConfig::builder().signature_trust_database(...)`.

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
let export = key_manager.export_public_key(org_id, key_info.id).await?;
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

With a key stored in the database. With `true` as the last argument the
signature row is stored for `org_id`:

```rust
use cloacina::security::{DbPackageSigner, PackageSigner};

let signer = DbPackageSigner::new(dal);

let signature = signer
    .sign_package_with_db_key(&package_path, org_id, key_id, &master_key, true)
    .await?;
```

To distribute a detached `.sig` sidecar alongside the package:

```rust
use cloacina::security::DetachedSignature;

let detached = DetachedSignature::from_signature_info(&signature);
detached.write_to_file("my-package.cloacina.sig")?;
```

With a private key held outside the database (the 32-byte Ed25519 seed), which
is what `cloacinactl package pack --sign` does:

```rust
use cloacina::crypto::decode_private_key_pem;
use cloacina::security::DetachedSignature;

let private_key = decode_private_key_pem(&std::fs::read_to_string("release-key.pem")?)?;
let detached = DetachedSignature::sign(&std::fs::read(&package_path)?, &private_key)?;
detached.write_to_file("my-package.cloacina.sig")?;
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
   key_manager.revoke_signing_key(org_id, old_signing_key_id).await?;
   ```

## See Also

- [Security Model](../../../../explanation/service/security-model/#package-signature-verification) — trust model and threat model.
- [Package Signing API Reference](../../../../reference/package-signing-api/) — full method catalog, signature format, audit events, and error variants.
- [Require signed packages](../../require-signed-packages/) — turn on server-side enforcement.
