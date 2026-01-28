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

//! Basic sign and verify integration tests.

use cloacina::crypto::generate_signing_keypair;
use cloacina::security::{verify_package_offline, DetachedSignature, SignatureSource};
use tempfile::NamedTempFile;

/// Test signing and verifying a package with raw keys (offline mode).
#[test]
fn test_sign_and_verify_offline() {
    // Create a test package file
    let package_file = NamedTempFile::new().unwrap();
    std::fs::write(package_file.path(), b"test package binary content").unwrap();

    // Generate a signing keypair
    let keypair = generate_signing_keypair();

    // Compute package hash and sign
    let package_data = std::fs::read(package_file.path()).unwrap();
    let package_hash = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&package_data);
        hex::encode(hasher.finalize())
    };

    // Sign the hash
    let hash_bytes = hex::decode(&package_hash).unwrap();
    let signature_bytes =
        cloacina::crypto::sign_package(&hash_bytes, &keypair.private_key).unwrap();

    // Create detached signature
    let sig = DetachedSignature {
        version: 1,
        algorithm: "ed25519".to_string(),
        package_hash: package_hash.clone(),
        key_fingerprint: keypair.fingerprint.clone(),
        signature: base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &signature_bytes,
        ),
        signed_at: chrono::Utc::now().to_rfc3339(),
    };

    // Write signature file
    let sig_file = NamedTempFile::new().unwrap();
    sig.write_to_file(sig_file.path()).unwrap();

    // Verify the package offline
    let result = verify_package_offline(package_file.path(), sig_file.path(), &keypair.public_key);

    assert!(result.is_ok());
    let verification = result.unwrap();
    assert_eq!(verification.package_hash, package_hash);
    assert_eq!(verification.signer_fingerprint, keypair.fingerprint);
}

/// Test that detached signature roundtrip works correctly.
#[test]
fn test_detached_signature_json_roundtrip() {
    let sig = DetachedSignature {
        version: 1,
        algorithm: "ed25519".to_string(),
        package_hash: "abc123def456".to_string(),
        key_fingerprint: "fingerprint789".to_string(),
        signature: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &[0u8; 64]),
        signed_at: chrono::Utc::now().to_rfc3339(),
    };

    // Roundtrip through JSON
    let json = sig.to_json().unwrap();
    let parsed = DetachedSignature::from_json(&json).unwrap();

    assert_eq!(parsed.version, sig.version);
    assert_eq!(parsed.algorithm, sig.algorithm);
    assert_eq!(parsed.package_hash, sig.package_hash);
    assert_eq!(parsed.key_fingerprint, sig.key_fingerprint);
}

/// Test that detached signature file I/O works correctly.
#[test]
fn test_detached_signature_file_roundtrip() {
    let sig = DetachedSignature {
        version: 1,
        algorithm: "ed25519".to_string(),
        package_hash: "abc123def456".to_string(),
        key_fingerprint: "fingerprint789".to_string(),
        signature: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &[0u8; 64]),
        signed_at: chrono::Utc::now().to_rfc3339(),
    };

    let sig_file = NamedTempFile::new().unwrap();
    sig.write_to_file(sig_file.path()).unwrap();

    let loaded = DetachedSignature::read_from_file(sig_file.path()).unwrap();
    assert_eq!(loaded.package_hash, sig.package_hash);
    assert_eq!(loaded.key_fingerprint, sig.key_fingerprint);
}

/// Test signature source default is Auto.
#[test]
fn test_signature_source_default() {
    let source = SignatureSource::default();
    assert!(matches!(source, SignatureSource::Auto));
}
