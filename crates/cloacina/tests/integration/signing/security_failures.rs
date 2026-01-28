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

//! Security failure integration tests.
//!
//! These tests verify that the signing system correctly rejects:
//! - Tampered packages
//! - Untrusted signers
//! - Invalid signatures
//! - Malformed signature files

use cloacina::crypto::generate_signing_keypair;
use cloacina::security::{verify_package_offline, DetachedSignature, VerificationError};
use tempfile::NamedTempFile;

/// Test that a tampered package is rejected.
#[test]
fn test_tampered_package_rejected() {
    let keypair = generate_signing_keypair();

    // Create and sign a package
    let package = NamedTempFile::new().unwrap();
    std::fs::write(package.path(), b"original package content").unwrap();

    let sig = sign_package_helper(package.path(), &keypair);
    let sig_file = NamedTempFile::new().unwrap();
    sig.write_to_file(sig_file.path()).unwrap();

    // Tamper with the package after signing
    let mut content = std::fs::read(package.path()).unwrap();
    content[5] ^= 0xFF; // Flip some bits
    std::fs::write(package.path(), &content).unwrap();

    // Verification should fail with TamperedPackage error
    let result = verify_package_offline(package.path(), sig_file.path(), &keypair.public_key);
    assert!(result.is_err());

    match result.unwrap_err() {
        VerificationError::TamperedPackage { expected, actual } => {
            assert_ne!(expected, actual);
        }
        e => panic!("Expected TamperedPackage error, got {:?}", e),
    }
}

/// Test that a package signed by untrusted key is rejected.
#[test]
fn test_untrusted_signer_rejected() {
    let signer_keypair = generate_signing_keypair();
    let verifier_keypair = generate_signing_keypair();

    // Sign package with signer's key
    let package = NamedTempFile::new().unwrap();
    std::fs::write(package.path(), b"package content").unwrap();

    let sig = sign_package_helper(package.path(), &signer_keypair);
    let sig_file = NamedTempFile::new().unwrap();
    sig.write_to_file(sig_file.path()).unwrap();

    // Try to verify with a different (untrusted) key
    let result = verify_package_offline(
        package.path(),
        sig_file.path(),
        &verifier_keypair.public_key,
    );
    assert!(result.is_err());

    // Should fail because the signature's fingerprint doesn't match the provided key
    match result.unwrap_err() {
        VerificationError::UntrustedSigner { fingerprint } => {
            assert_eq!(fingerprint, signer_keypair.fingerprint);
        }
        e => panic!("Expected UntrustedSigner error, got {:?}", e),
    }
}

/// Test that an invalid signature (wrong bytes) is rejected.
#[test]
fn test_invalid_signature_rejected() {
    let keypair = generate_signing_keypair();

    // Create a package
    let package = NamedTempFile::new().unwrap();
    std::fs::write(package.path(), b"package content").unwrap();

    // Create a signature with invalid signature bytes
    let package_data = std::fs::read(package.path()).unwrap();
    let package_hash = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&package_data);
        hex::encode(hasher.finalize())
    };

    let sig = DetachedSignature {
        version: 1,
        algorithm: "ed25519".to_string(),
        package_hash,
        key_fingerprint: keypair.fingerprint.clone(),
        // Invalid signature - random bytes instead of actual Ed25519 signature
        signature: base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &[0xABu8; 64],
        ),
        signed_at: chrono::Utc::now().to_rfc3339(),
    };

    let sig_file = NamedTempFile::new().unwrap();
    sig.write_to_file(sig_file.path()).unwrap();

    // Verification should fail
    let result = verify_package_offline(package.path(), sig_file.path(), &keypair.public_key);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VerificationError::InvalidSignature
    ));
}

/// Test that a signature with wrong hash is rejected.
#[test]
fn test_wrong_hash_in_signature_rejected() {
    let keypair = generate_signing_keypair();

    // Create a package
    let package = NamedTempFile::new().unwrap();
    std::fs::write(package.path(), b"actual package content").unwrap();

    // Create a signature with a different hash (as if signed for different content)
    let wrong_hash = "a".repeat(64); // Wrong hash

    let sig = DetachedSignature {
        version: 1,
        algorithm: "ed25519".to_string(),
        package_hash: wrong_hash,
        key_fingerprint: keypair.fingerprint.clone(),
        signature: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &[0u8; 64]),
        signed_at: chrono::Utc::now().to_rfc3339(),
    };

    let sig_file = NamedTempFile::new().unwrap();
    sig.write_to_file(sig_file.path()).unwrap();

    let result = verify_package_offline(package.path(), sig_file.path(), &keypair.public_key);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VerificationError::TamperedPackage { .. }
    ));
}

/// Test that malformed signature JSON is rejected.
#[test]
fn test_malformed_signature_file_rejected() {
    let keypair = generate_signing_keypair();

    let package = NamedTempFile::new().unwrap();
    std::fs::write(package.path(), b"package content").unwrap();

    // Write malformed JSON to signature file
    let sig_file = NamedTempFile::new().unwrap();
    std::fs::write(sig_file.path(), "{ invalid json }").unwrap();

    let result = verify_package_offline(package.path(), sig_file.path(), &keypair.public_key);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VerificationError::MalformedSignature { .. }
    ));
}

/// Test that missing signature file is handled.
#[test]
fn test_missing_signature_file() {
    let keypair = generate_signing_keypair();

    let package = NamedTempFile::new().unwrap();
    std::fs::write(package.path(), b"package content").unwrap();

    let nonexistent_sig = std::path::Path::new("/nonexistent/signature.sig");
    let result = verify_package_offline(package.path(), nonexistent_sig, &keypair.public_key);
    assert!(result.is_err());
}

/// Test that empty package is handled correctly.
#[test]
fn test_empty_package() {
    let keypair = generate_signing_keypair();

    let package = NamedTempFile::new().unwrap();
    std::fs::write(package.path(), b"").unwrap(); // Empty package

    let sig = sign_package_helper(package.path(), &keypair);
    let sig_file = NamedTempFile::new().unwrap();
    sig.write_to_file(sig_file.path()).unwrap();

    // Empty package should still verify (nothing special about empty content)
    let result = verify_package_offline(package.path(), sig_file.path(), &keypair.public_key);
    assert!(result.is_ok());
}

/// Database-based tests for revoked key rejection.
///
/// This test is marked as ignored because it requires database setup.
#[tokio::test]
#[ignore = "Requires database connection"]
async fn test_revoked_key_rejected() {
    // Expected workflow:
    //
    // 1. Create org, signing key, and trust the key
    // 2. Sign a package
    // 3. Verify succeeds
    // 4. Revoke the trusted key
    // 5. Verify should now FAIL (key is no longer trusted)
    todo!("Implement with test database fixture")
}

/// Helper function to sign a package.
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
