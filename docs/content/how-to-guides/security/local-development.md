---
title: "Local Development"
weight: 20
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
```

This allows you to:
- Build and load packages without signing
- Iterate quickly during development
- Run tests without key management overhead

## Development Workflow

### 1. Build Your Package

```bash
cargo build --release
```

### 2. Run Without Signatures

```rust
use cloacina::security::SecurityConfig;

let config = SecurityConfig::development(); // Same as default()
let runner = DefaultRunner::new(config, dal).await?;
runner.load_package("./target/release/libworkflow.so").await?;
```

## Testing Signatures Locally

If you want to test the signing workflow locally:

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
use cloacina::security::{DbPackageSigner, PackageSigner, DetachedSignature};

let signer = DbPackageSigner::new(dal);
let signature = signer.sign_package_with_raw_key(
    Path::new("./target/release/libworkflow.so"),
    &private_key,
    &public_key,
)?;

// Write detached signature
let detached = DetachedSignature::from_signature_info(&signature);
detached.write_to_file("./target/release/libworkflow.so.sig")?;
```

### 3. Verify Locally

```rust
use cloacina::security::verify_package_offline;

let result = verify_package_offline(
    Path::new("./target/release/libworkflow.so"),
    Path::new("./target/release/libworkflow.so.sig"),
    &public_key,
)?;

println!("Verified! Hash: {}", result.package_hash);
```

## Environment-Based Configuration

Use environment variables to switch between development and production:

```rust
let config = if std::env::var("CLOACINA_REQUIRE_SIGNATURES").is_ok() {
    SecurityConfig {
        require_signatures: true,
        key_encryption_key: Some(
            load_key_from_env("CLOACINA_KEY_ENCRYPTION_KEY")
        ),
    }
} else {
    SecurityConfig::development()
};
```

## CI/CD Integration

For CI environments, you can either:

1. **Skip verification** (development/test jobs):
   ```yaml
   env:
     CLOACINA_REQUIRE_SIGNATURES: ""  # Empty = disabled
   ```

2. **Enable verification** (staging/production):
   ```yaml
   env:
     CLOACINA_REQUIRE_SIGNATURES: "true"
     CLOACINA_KEY_ENCRYPTION_KEY: ${{ secrets.KEY_ENCRYPTION_KEY }}
   ```

### Signing in CI

```yaml
# .github/workflows/release.yml
jobs:
  build-and-sign:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build package
        run: cargo build --release

      - name: Sign package
        env:
          SIGNING_KEY: ${{ secrets.SIGNING_PRIVATE_KEY }}
        run: |
          # Use your signing tool/script here
          cloacina sign ./target/release/libworkflow.so

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: signed-package
          path: |
            ./target/release/libworkflow.so
            ./target/release/libworkflow.so.sig
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

## Security Checklist for Production

Before deploying to production:

- [ ] Enable `require_signatures: true`
- [ ] Generate and securely store signing keys
- [ ] Distribute public keys to all verification points
- [ ] Configure key encryption key in secrets manager
- [ ] Set up audit log monitoring
- [ ] Test verification in staging environment
- [ ] Document key rotation procedures

## Troubleshooting

### "Signature not found"

The package has no signature. Either:
- Sign the package before loading
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
