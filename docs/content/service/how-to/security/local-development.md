---
title: "Local Development"
weight: 20
aliases:
  - "/platform/how-to-guides/security/local-development/"

---

# Security in Local Development

By default, Cloacina does not require package signatures, making local development straightforward. This guide covers security considerations for development workflows.

## Default Configuration

The default security configuration disables signature requirements:

```rust
use cloacina::security::SecurityConfig;

// Default: no signatures required
let config = SecurityConfig::default();
assert!(!config.require_signatures);

// Explicit spelling of the same thing:
let config = SecurityConfig::development();
```

This allows you to:
- Build, pack, and upload packages without signing
- Iterate quickly during development
- Run tests without key management overhead

## Development Workflow

For day-to-day development, nothing security-related is needed: run your
server **without** `--require-signatures` (the default) and use the
normal authoring loop —

```bash
cloacinactl package new my-workflow --lang rust
cloacinactl package validate my-workflow
cloacinactl package pack my-workflow
cloacinactl package upload my-workflow/my-workflow.cloacina
```

Uploads are accepted unverified; signature checking only happens on
servers started with `--require-signatures` (see
[Require signed packages]({{< ref "/service/how-to/require-signed-packages" >}})).

> **Note:** `cloacinactl package pack --sign` fails hard — CLI-driven
> signing is unimplemented (I-0103). Everything below exercises the
> **library** signing API from Rust code, which is how signing is done
> today.

## Testing Signatures Locally

If you want to test the signing machinery locally, use the library API
from a small Rust program or test:

### 1. Generate a Local Keypair

```rust
use cloacina::crypto::generate_signing_keypair;

let keypair = generate_signing_keypair();
println!("Public key: {} bytes", keypair.public_key.len());
println!("Fingerprint: {}", keypair.fingerprint);

// Save keys for later use
std::fs::write("dev-key.pub", &keypair.public_key)?;
std::fs::write("dev-key.priv", &keypair.private_key)?;
```

### 2. Sign Your Package

```rust
use std::path::Path;
use cloacina::security::{DbPackageSigner, PackageSigner, DetachedSignature};

let signer = DbPackageSigner::new(dal);
let signature = signer.sign_package_with_raw_key(
    Path::new("./my-workflow/my-workflow.cloacina"),
    &private_key,
    &public_key,
)?;

// Write detached signature sidecar
let detached = DetachedSignature::from_signature_info(&signature);
detached.write_to_file(Path::new("./my-workflow/my-workflow.cloacina.sig"))?;
```

### 3. Verify Locally

```rust
use std::path::Path;
use cloacina::security::verify_package_offline;

let result = verify_package_offline(
    Path::new("./my-workflow/my-workflow.cloacina"),
    Path::new("./my-workflow/my-workflow.cloacina.sig"),
    &public_key,
)?;

println!("Verified! Hash: {}", result.package_hash);
```

## Testing Verification Failures

To test that verification correctly rejects invalid packages:

```rust
#[test]
fn test_tampered_package_fails() {
    // Sign a package
    let signature = sign_package(&package_path, &private_key, &public_key)?;

    // Tamper with it
    let mut content = std::fs::read(&package_path)?;
    content[0] ^= 0xFF;
    std::fs::write(&package_path, &content)?;

    // Verification should fail
    let result = verify_package_offline(&package_path, &sig_path, &public_key);
    assert!(matches!(result, Err(VerificationError::TamperedPackage { .. })));
}
```

## CI/CD Integration

For CI environments:

1. **Development/test jobs**: run the server without
   `--require-signatures` (leave `CLOACINA_REQUIRE_SIGNATURES` unset)
   and upload unsigned packages.

2. **Staging/production**: start the server with `--require-signatures`
   **and** `--verification-org-id` (both required together — the server
   fails fast if only one is set), and produce signatures in your
   pipeline with a small Rust helper built on
   `cloacina::security::package_signer` (there is no `cloacinactl`
   signing command yet):

   ```yaml
   # .github/workflows/release.yml (sketch)
   - name: Build and pack
     run: |
       cloacinactl package build my-workflow
       cloacinactl package pack my-workflow

   - name: Sign package
     env:
       SIGNING_KEY: ${{ secrets.SIGNING_PRIVATE_KEY }}
     run: |
       # Your own helper binary wrapping sign_package_with_raw_key +
       # DetachedSignature — produces my-workflow.cloacina.sig
       ./tools/sign-package my-workflow/my-workflow.cloacina
   ```

## Security Checklist for Production

Before deploying to production:

- [ ] Start the server with `--require-signatures` and `--verification-org-id`
- [ ] Generate and securely store signing keys
- [ ] Register trusted public keys against the verification org
- [ ] Store the 32-byte key-encryption master key in a secrets manager
- [ ] Set up audit log monitoring (`package.load.*`, `verification.*`, `key.*` events)
- [ ] Test verification in staging environment
- [ ] Document key rotation procedures

## Troubleshooting

### "Signature not found"

The package has no signature. Either:
- Sign the package before uploading
- Disable signature requirements for development

### "Untrusted signer"

The signing key is not trusted by this organization:
- Trust the public key: `key_manager.trust_public_key(org_id, &pub_key, None).await?`
- Or check if there's a trust ACL issue

### "Tampered package"

The package content has changed since signing:
- Re-sign the package after any modifications
- Verify you're loading the correct file

### "Invalid signature"

Cryptographic verification failed:
- Ensure the correct public key is being used
- Check for data corruption in the signature file
