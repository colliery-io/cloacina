---
id: package-signing-integration-tests
level: task
title: "Package signing integration tests and documentation"
short_code: "CLOACI-T-0065"
created_at: 2026-01-28T14:15:49.595447+00:00
updated_at: 2026-01-28T16:48:28.857701+00:00
parent: CLOACI-I-0008
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0008
---

# Package signing integration tests and documentation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0008]]

## Objective

Create comprehensive integration tests for the package signing system and write operator documentation covering key management, signing workflows, and security configuration.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Integration tests for full signing → verification flow
- [x] Integration tests for trust chain resolution (with database-dependent tests marked ignored)
- [x] Integration tests for key rotation scenarios
- [x] Tests for tampered package detection
- [x] Tests for untrusted signer rejection
- [x] Tests for revoked key handling (database-dependent, marked ignored)
- [x] Operator guide for key management (`docs/content/how-to-guides/security/package-signing.md`)
- [x] Developer guide for local workflow (`docs/content/how-to-guides/security/local-development.md`)
- [x] CI/CD integration examples (included in documentation)

## Integration Tests

### Test Categories

#### 1. Happy Path Tests

```rust
#[tokio::test]
async fn test_sign_and_verify_package() {
    // Setup: Create org, generate signing key, trust the key
    let org_id = create_test_org().await;
    let key_info = key_manager.create_signing_key(org_id, "test-key").await.unwrap();
    key_manager.trust_public_key(org_id, &key_info.public_key, Some("test-key")).await.unwrap();

    // Sign a test package
    let package_path = create_test_package();
    let (pub_key, priv_key) = key_manager.get_signing_key(key_info.id).await.unwrap();
    let signature = sign_package(&package_path, &priv_key, &pub_key).unwrap();

    // Verify and load
    let config = SecurityConfig { require_signatures: true, ..Default::default() };
    let result = verify_and_load_package(
        &package_path,
        org_id,
        &config,
        &key_manager,
        SignatureSource::Memory(signature),
    ).await;

    assert!(result.is_ok());
}
```

#### 2. Trust Chain Tests

```rust
#[tokio::test]
async fn test_trust_chain_acl() {
    // Create parent and child orgs
    let parent_org = create_test_org().await;
    let child_org = create_test_org().await;

    // Child creates signing key and trusts it
    let child_key = key_manager.create_signing_key(child_org, "child-key").await.unwrap();
    key_manager.trust_public_key(child_org, &child_key.public_key, None).await.unwrap();

    // Parent grants trust to child
    key_manager.grant_trust(parent_org, child_org).await.unwrap();

    // Parent should now trust child's key via ACL
    let parent_trusted = key_manager.list_trusted_keys(parent_org).await.unwrap();
    assert!(parent_trusted.iter().any(|k| k.fingerprint == child_key.fingerprint));

    // Sign package with child's key, verify as parent
    let signature = sign_with_key(&package_path, child_key.id).await;
    let result = verify_as_org(parent_org, &package_path, &signature).await;
    assert!(result.is_ok());
}
```

#### 3. Security Failure Tests

```rust
#[tokio::test]
async fn test_tampered_package_rejected() {
    // Sign package
    let signature = sign_package(&package_path, ...).unwrap();

    // Tamper with package after signing
    let mut bytes = std::fs::read(&package_path).unwrap();
    bytes[100] ^= 0xFF;  // Flip some bits
    std::fs::write(&package_path, &bytes).unwrap();

    // Verification should fail
    let result = verify_and_load_package(...).await;
    assert!(matches!(result, Err(PackageError::Verification(VerificationError::TamperedPackage { .. }))));
}

#[tokio::test]
async fn test_untrusted_signer_rejected() {
    // Create two orgs, signing key only trusted by org A
    let org_a = create_test_org().await;
    let org_b = create_test_org().await;

    let key = key_manager.create_signing_key(org_a, "key").await.unwrap();
    key_manager.trust_public_key(org_a, &key.public_key, None).await.unwrap();
    // Note: org_b does NOT trust this key

    // Sign package
    let signature = sign_with_key(&package_path, key.id).await;

    // Verify as org_b should fail
    let result = verify_as_org(org_b, &package_path, &signature).await;
    assert!(matches!(result, Err(PackageError::Verification(VerificationError::UntrustedSigner { .. }))));
}

#[tokio::test]
async fn test_revoked_key_rejected() {
    // Setup: create and trust key
    let key = key_manager.create_signing_key(org_id, "key").await.unwrap();
    key_manager.trust_public_key(org_id, &key.public_key, None).await.unwrap();

    // Sign package
    let signature = sign_with_key(&package_path, key.id).await;

    // Revoke the trusted key
    let trusted_key_id = find_trusted_key_id(org_id, &key.fingerprint).await;
    key_manager.revoke_trusted_key(trusted_key_id).await.unwrap();

    // Verification should now fail (hot-reload)
    let result = verify_as_org(org_id, &package_path, &signature).await;
    assert!(matches!(result, Err(PackageError::Verification(VerificationError::UntrustedSigner { .. }))));
}
```

#### 4. Key Rotation Tests

```rust
#[tokio::test]
async fn test_key_rotation_workflow() {
    // Old key signs packages
    let old_key = key_manager.create_signing_key(org_id, "release-v1").await.unwrap();
    key_manager.trust_public_key(org_id, &old_key.public_key, None).await.unwrap();
    let old_sig = sign_with_key(&package_a, old_key.id).await;

    // Generate new key, trust it
    let new_key = key_manager.create_signing_key(org_id, "release-v2").await.unwrap();
    key_manager.trust_public_key(org_id, &new_key.public_key, None).await.unwrap();
    let new_sig = sign_with_key(&package_b, new_key.id).await;

    // Both old and new signatures should verify (during transition)
    assert!(verify_as_org(org_id, &package_a, &old_sig).await.is_ok());
    assert!(verify_as_org(org_id, &package_b, &new_sig).await.is_ok());

    // Revoke old key
    key_manager.revoke_signing_key(old_key.id).await.unwrap();
    // Old trusted key still works (signing key != trusted key)

    // To fully rotate: revoke old trusted key too
    let old_trusted_id = find_trusted_key_id(org_id, &old_key.fingerprint).await;
    key_manager.revoke_trusted_key(old_trusted_id).await.unwrap();

    // Now old signature fails, new still works
    assert!(verify_as_org(org_id, &package_a, &old_sig).await.is_err());
    assert!(verify_as_org(org_id, &package_b, &new_sig).await.is_ok());
}
```

## Documentation

### Operator Guide: Key Management

```markdown
# Package Signing - Operator Guide

## Quick Start

### 1. Enable Signature Verification

```rust
let config = DefaultRunnerConfig {
    security: SecurityConfig {
        require_signatures: true,
        key_encryption_key: Some(load_key_from_env("CLOACINA_KEY_ENCRYPTION_KEY")),
    },
    ..Default::default()
};
```

### 2. Generate Signing Key

```bash
cloacina keys generate --name "release-key" --org <org-id>
# Output: Key ID and fingerprint
```

### 3. Export Public Key for CI

```bash
cloacina keys export <key-id> --format pem > release-key.pub.pem
```

### 4. Trust the Key

```bash
cloacina keys trust --import ./release-key.pub.pem --name "release-key" --org <org-id>
```

### 5. Sign Packages in CI

```bash
cloacina package sign ./target/release/libworkflow.so --key ./signing-key.pem
```

## Key Rotation

1. Generate new key: `cloacina keys generate --name "release-v2" --org <org-id>`
2. Trust new key: `cloacina keys trust --import ./new-key.pub.pem --org <org-id>`
3. Update CI to use new key for signing
4. After transition period, revoke old trusted key: `cloacina keys revoke <old-trusted-key-id>`

## Multi-Org Trust

To allow parent org to trust packages signed by child org:

```bash
cloacina orgs grant-trust --from <parent-org-id> --to <child-org-id>
```
```

### Developer Guide: Local Workflow

```markdown
# Local Development (No Signing)

By default, signature verification is disabled. Your local workflow is unchanged:

```bash
cargo build --release
./my_runner  # Loads packages without signature checks
```

## Testing with Signatures Locally

If you want to test signing locally:

```bash
# Generate local keypair
cloacina keys generate --name "dev" --output ./dev-key.pem

# Sign your package
cloacina package sign ./target/release/libworkflow.so --key ./dev-key.pem

# Run with signature verification
./my_runner --require-signatures --trust-key ./dev-key.pub.pem
```
```

## File Locations

- Integration tests: `crates/cloacina/tests/integration/signing/`
- Operator docs: `docs/operator-guide/package-signing.md`
- Developer docs: `docs/developer-guide/local-development.md`

## Requires

- All other CLOACI-I-0008 tasks complete

## Status Updates

### Implementation Complete

**Integration Tests Created:**
- `crates/cloacina/tests/integration/signing/mod.rs` - Module definition
- `crates/cloacina/tests/integration/signing/sign_and_verify.rs` - Basic signing and verification tests
- `crates/cloacina/tests/integration/signing/trust_chain.rs` - Trust chain and ACL tests
- `crates/cloacina/tests/integration/signing/key_rotation.rs` - Key rotation workflow tests
- `crates/cloacina/tests/integration/signing/security_failures.rs` - Security failure detection tests

**Test Results:** 15 tests pass, 6 ignored (require database connection)

**Documentation Created:**
- `docs/content/how-to-guides/security/_index.md` - Security section index
- `docs/content/how-to-guides/security/package-signing.md` - Comprehensive operator guide
- `docs/content/how-to-guides/security/local-development.md` - Developer guide with CI/CD examples

**Additional Changes:**
- Exported `GeneratedKeypair` from `crypto` module for test use
- Updated `tests/integration/main.rs` to include signing module
