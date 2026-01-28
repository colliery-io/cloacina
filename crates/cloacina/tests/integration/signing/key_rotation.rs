/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Key rotation integration tests.
//!
//! These tests verify that key rotation workflows work correctly,
//! allowing smooth transition from old keys to new keys.

use cloacina::crypto::generate_signing_keypair;
use cloacina::security::{verify_package_offline, DetachedSignature};
use tempfile::NamedTempFile;

/// Test that multiple keys can sign different packages.
///
/// This simulates a key rotation scenario where:
/// 1. Old key signs old packages
/// 2. New key signs new packages
/// 3. Both can be verified independently
#[test]
fn test_multiple_keys_sign_different_packages() {
    let old_keypair = generate_signing_keypair();
    let new_keypair = generate_signing_keypair();

    // Create two packages
    let old_package = NamedTempFile::new().unwrap();
    std::fs::write(old_package.path(), b"old package content v1.0").unwrap();

    let new_package = NamedTempFile::new().unwrap();
    std::fs::write(new_package.path(), b"new package content v2.0").unwrap();

    // Sign old package with old key
    let old_sig = sign_package_helper(old_package.path(), &old_keypair);
    let old_sig_file = NamedTempFile::new().unwrap();
    old_sig.write_to_file(old_sig_file.path()).unwrap();

    // Sign new package with new key
    let new_sig = sign_package_helper(new_package.path(), &new_keypair);
    let new_sig_file = NamedTempFile::new().unwrap();
    new_sig.write_to_file(new_sig_file.path()).unwrap();

    // Verify old package with old key
    let result = verify_package_offline(
        old_package.path(),
        old_sig_file.path(),
        &old_keypair.public_key,
    );
    assert!(result.is_ok(), "Old package should verify with old key");

    // Verify new package with new key
    let result = verify_package_offline(
        new_package.path(),
        new_sig_file.path(),
        &new_keypair.public_key,
    );
    assert!(result.is_ok(), "New package should verify with new key");

    // Old package should NOT verify with new key (wrong signer)
    let result = verify_package_offline(
        old_package.path(),
        old_sig_file.path(),
        &new_keypair.public_key,
    );
    assert!(
        result.is_err(),
        "Old package should not verify with wrong key"
    );
}

/// Test that re-signing a package with a new key works.
#[test]
fn test_resign_package_with_new_key() {
    let old_keypair = generate_signing_keypair();
    let new_keypair = generate_signing_keypair();

    // Create a package
    let package = NamedTempFile::new().unwrap();
    std::fs::write(package.path(), b"package content to be re-signed").unwrap();

    // Sign with old key
    let old_sig = sign_package_helper(package.path(), &old_keypair);
    let old_sig_file = NamedTempFile::new().unwrap();
    old_sig.write_to_file(old_sig_file.path()).unwrap();

    // Verify with old key works
    assert!(
        verify_package_offline(package.path(), old_sig_file.path(), &old_keypair.public_key)
            .is_ok()
    );

    // Re-sign with new key
    let new_sig = sign_package_helper(package.path(), &new_keypair);
    let new_sig_file = NamedTempFile::new().unwrap();
    new_sig.write_to_file(new_sig_file.path()).unwrap();

    // Verify with new key works
    assert!(
        verify_package_offline(package.path(), new_sig_file.path(), &new_keypair.public_key)
            .is_ok()
    );

    // Package hash should be the same in both signatures
    assert_eq!(old_sig.package_hash, new_sig.package_hash);

    // But fingerprints should differ
    assert_ne!(old_sig.key_fingerprint, new_sig.key_fingerprint);
}

/// Test that database-based key rotation workflow works.
///
/// This test is marked as ignored because it requires database setup.
#[tokio::test]
#[ignore = "Requires database connection"]
async fn test_key_rotation_database_workflow() {
    // Expected workflow:
    //
    // 1. Create org and old signing key
    // 2. Trust old key's public key
    // 3. Sign package A with old key
    //
    // 4. Generate new signing key
    // 5. Trust new key's public key
    // 6. Sign package B with new key
    //
    // 7. Both packages should verify (transition period)
    //
    // 8. Revoke old trusted key
    // 9. Package A should NO LONGER verify
    // 10. Package B should STILL verify
    //
    // 11. Optionally revoke old signing key to prevent new signatures
    todo!("Implement with test database fixture")
}

/// Helper function to sign a package and create a DetachedSignature.
fn sign_package_helper(
    package_path: &std::path::Path,
    keypair: &cloacina::crypto::GeneratedKeypair,
) -> DetachedSignature {
    use sha2::{Digest, Sha256};

    let package_data = std::fs::read(package_path).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(&package_data);
    let package_hash = hex::encode(hasher.finalize());

    let hash_bytes = hex::decode(&package_hash).unwrap();
    let signature_bytes =
        cloacina::crypto::sign_package(&hash_bytes, &keypair.private_key).unwrap();

    DetachedSignature {
        version: 1,
        algorithm: "ed25519".to_string(),
        package_hash,
        key_fingerprint: keypair.fingerprint.clone(),
        signature: base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &signature_bytes,
        ),
        signed_at: chrono::Utc::now().to_rfc3339(),
    }
}
