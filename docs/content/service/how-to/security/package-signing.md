---
title: "Package Signing"
weight: 10
aliases:
  - "/platform/how-to-guides/security/package-signing/"

---

# Sign and Verify Workflow Packages

Cloacina supports cryptographic (Ed25519) signing of workflow packages so you can
detect tampering and verify authenticity. This guide covers the library-side
tasks: enabling verification, generating and trusting keys, signing, verifying,
and rotating keys.

- For the **trust model and why signing is optional**, see
  [Security Model]({{< ref "/service/explanation/security-model" >}}#signing-keys-and-the-trust-model).
- For the **full API surface** (method catalog, signature format, error
  variants), see [Package Signing API Reference]({{< ref "/reference/package-signing-api" >}}).
- To **enforce signatures at the server upload boundary**, see
  [Require signed packages]({{< ref "/service/how-to/require-signed-packages" >}}).

## Enable signature verification

To require verification, configure `SecurityConfig`:

```rust
use cloacina::security::SecurityConfig;

let config = SecurityConfig {
    require_signatures: true,
    key_encryption_key: Some(load_key_from_env("CLOACINA_KEY_ENCRYPTION_KEY")),
};
```

When `require_signatures` is `true`, unsigned packages and packages signed by an
untrusted or tampered key fail to load.

The `key_encryption_key` is a 32-byte AES-256 key used to encrypt private signing
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
   key_manager.revoke_trusted_key(old_trusted_key_id).await?;
   ```

6. **Optionally, revoke the old signing key** to prevent new signatures:
   ```rust
   key_manager.revoke_signing_key(old_signing_key_id).await?;
   ```

## See Also

- [Security Model]({{< ref "/service/explanation/security-model" >}}#package-signature-verification) — trust model and threat model.
- [Package Signing API Reference]({{< ref "/reference/package-signing-api" >}}) — full method catalog, signature format, audit events, and error variants.
- [Require signed packages]({{< ref "/service/how-to/require-signed-packages" >}}) — turn on server-side enforcement.
