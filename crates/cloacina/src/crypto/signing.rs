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

//! Ed25519 signing utilities for package signatures.
//!
//! Provides functions for:
//! - Generating Ed25519 signing keypairs
//! - Computing SHA256 key fingerprints
//! - Signing package hashes
//! - Verifying signatures

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Errors that can occur during signing operations.
#[derive(Debug, Error)]
pub enum SigningError {
    #[error("Invalid private key: expected 32 bytes, got {0}")]
    InvalidPrivateKeyLength(usize),

    #[error("Invalid public key: expected 32 bytes, got {0}")]
    InvalidPublicKeyLength(usize),

    #[error("Invalid signature: expected 64 bytes, got {0}")]
    InvalidSignatureLength(usize),

    #[error("Failed to create signing key: {0}")]
    KeyCreationFailed(String),

    #[error("Failed to create signature: {0}")]
    SignatureFailed(String),

    #[error("Signature verification failed")]
    VerificationFailed,
}

/// A generated Ed25519 keypair.
pub struct GeneratedKeypair {
    /// The 32-byte private key seed (should be encrypted before storage)
    pub private_key: Vec<u8>,
    /// The 32-byte public key
    pub public_key: Vec<u8>,
    /// SHA256 hex fingerprint of the public key
    pub fingerprint: String,
}

/// Generates a new Ed25519 signing keypair.
///
/// # Returns
///
/// A `GeneratedKeypair` containing the private key, public key, and fingerprint.
pub fn generate_signing_keypair() -> GeneratedKeypair {
    let mut csprng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();

    let public_key_bytes = verifying_key.to_bytes();
    let fingerprint = compute_key_fingerprint(&public_key_bytes);

    GeneratedKeypair {
        private_key: signing_key.to_bytes().to_vec(),
        public_key: public_key_bytes.to_vec(),
        fingerprint,
    }
}

/// Computes the SHA256 hex fingerprint of a public key.
///
/// # Arguments
///
/// * `public_key` - The 32-byte Ed25519 public key
///
/// # Returns
///
/// A 64-character hex string representing the SHA256 hash of the public key.
pub fn compute_key_fingerprint(public_key: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(public_key);
    let hash = hasher.finalize();
    hex::encode(hash)
}

/// Signs a package hash using an Ed25519 private key.
///
/// # Arguments
///
/// * `package_hash` - The SHA256 hash of the package (as raw bytes or hex string bytes)
/// * `private_key` - The 32-byte Ed25519 private key seed
///
/// # Returns
///
/// The 64-byte Ed25519 signature.
///
/// # Errors
///
/// Returns `SigningError` if the private key is invalid.
pub fn sign_package(package_hash: &[u8], private_key: &[u8]) -> Result<Vec<u8>, SigningError> {
    if private_key.len() != 32 {
        return Err(SigningError::InvalidPrivateKeyLength(private_key.len()));
    }

    let key_bytes: [u8; 32] = private_key
        .try_into()
        .map_err(|_| SigningError::InvalidPrivateKeyLength(private_key.len()))?;

    let signing_key = SigningKey::from_bytes(&key_bytes);
    let signature = signing_key.sign(package_hash);

    Ok(signature.to_bytes().to_vec())
}

/// Verifies a package signature using an Ed25519 public key.
///
/// # Arguments
///
/// * `package_hash` - The SHA256 hash of the package that was signed
/// * `signature` - The 64-byte Ed25519 signature
/// * `public_key` - The 32-byte Ed25519 public key
///
/// # Returns
///
/// `Ok(())` if the signature is valid.
///
/// # Errors
///
/// Returns `SigningError` if the signature is invalid or verification fails.
pub fn verify_signature(
    package_hash: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<(), SigningError> {
    if public_key.len() != 32 {
        return Err(SigningError::InvalidPublicKeyLength(public_key.len()));
    }
    if signature.len() != 64 {
        return Err(SigningError::InvalidSignatureLength(signature.len()));
    }

    let key_bytes: [u8; 32] = public_key
        .try_into()
        .map_err(|_| SigningError::InvalidPublicKeyLength(public_key.len()))?;

    let sig_bytes: [u8; 64] = signature
        .try_into()
        .map_err(|_| SigningError::InvalidSignatureLength(signature.len()))?;

    let verifying_key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| SigningError::KeyCreationFailed(e.to_string()))?;

    let sig = Signature::from_bytes(&sig_bytes);

    verifying_key
        .verify(package_hash, &sig)
        .map_err(|_| SigningError::VerificationFailed)
}

/// Computes the SHA256 hash of package data.
///
/// # Arguments
///
/// * `data` - The package binary data
///
/// # Returns
///
/// A 64-character hex string representing the SHA256 hash.
pub fn compute_package_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    hex::encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let keypair = generate_signing_keypair();

        assert_eq!(keypair.private_key.len(), 32);
        assert_eq!(keypair.public_key.len(), 32);
        assert_eq!(keypair.fingerprint.len(), 64); // SHA256 hex = 64 chars
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = generate_signing_keypair();
        let package_hash = b"test package hash data";

        let signature = sign_package(package_hash, &keypair.private_key).unwrap();

        assert_eq!(signature.len(), 64);

        // Verify should succeed
        let result = verify_signature(package_hash, &signature, &keypair.public_key);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_wrong_key_fails() {
        let keypair1 = generate_signing_keypair();
        let keypair2 = generate_signing_keypair();
        let package_hash = b"test package hash data";

        let signature = sign_package(package_hash, &keypair1.private_key).unwrap();

        // Verify with wrong key should fail
        let result = verify_signature(package_hash, &signature, &keypair2.public_key);
        assert!(matches!(result, Err(SigningError::VerificationFailed)));
    }

    #[test]
    fn test_verify_tampered_data_fails() {
        let keypair = generate_signing_keypair();
        let package_hash = b"test package hash data";
        let tampered_hash = b"tampered package hash";

        let signature = sign_package(package_hash, &keypair.private_key).unwrap();

        // Verify with tampered data should fail
        let result = verify_signature(tampered_hash, &signature, &keypair.public_key);
        assert!(matches!(result, Err(SigningError::VerificationFailed)));
    }

    #[test]
    fn test_fingerprint_is_deterministic() {
        let public_key = [0x42u8; 32];

        let fp1 = compute_key_fingerprint(&public_key);
        let fp2 = compute_key_fingerprint(&public_key);

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_invalid_key_lengths() {
        let package_hash = b"test";

        let result = sign_package(package_hash, &[0u8; 16]);
        assert!(matches!(
            result,
            Err(SigningError::InvalidPrivateKeyLength(16))
        ));

        let result = verify_signature(package_hash, &[0u8; 64], &[0u8; 16]);
        assert!(matches!(
            result,
            Err(SigningError::InvalidPublicKeyLength(16))
        ));

        let result = verify_signature(package_hash, &[0u8; 32], &[0u8; 32]);
        assert!(matches!(
            result,
            Err(SigningError::InvalidSignatureLength(32))
        ));
    }

    #[test]
    fn test_compute_package_hash() {
        let data = b"hello world";
        let hash = compute_package_hash(data);

        // SHA256 of "hello world" is known
        assert_eq!(hash.len(), 64);
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}
