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

//! AES-256-GCM encryption for private key storage at rest.
//!
//! Private keys are stored encrypted in the database using AES-256-GCM.
//! The encrypted format is: `nonce (12 bytes) || ciphertext || tag (16 bytes)`.
//!
//! The encryption key should be derived from a secure source such as:
//! - A master key stored in a hardware security module (HSM)
//! - A key derived from a passphrase using a KDF like Argon2
//! - A key management service (KMS)

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use thiserror::Error;

/// Errors that can occur during key encryption/decryption.
#[derive(Debug, Error)]
pub enum KeyEncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid key length: expected 32 bytes, got {0}")]
    InvalidKeyLength(usize),

    #[error("Invalid encrypted data: too short")]
    InvalidEncryptedData,
}

/// Size of the AES-256-GCM nonce in bytes.
const NONCE_SIZE: usize = 12;

/// Encrypts an Ed25519 private key using AES-256-GCM.
///
/// # Arguments
///
/// * `private_key` - The 32-byte Ed25519 private key seed to encrypt
/// * `encryption_key` - The 32-byte AES-256 encryption key
///
/// # Returns
///
/// The encrypted data in format: `nonce (12 bytes) || ciphertext || tag (16 bytes)`
///
/// # Errors
///
/// Returns `KeyEncryptionError` if encryption fails or key lengths are invalid.
pub fn encrypt_private_key(
    private_key: &[u8],
    encryption_key: &[u8],
) -> Result<Vec<u8>, KeyEncryptionError> {
    if encryption_key.len() != 32 {
        return Err(KeyEncryptionError::InvalidKeyLength(encryption_key.len()));
    }

    let cipher = Aes256Gcm::new_from_slice(encryption_key)
        .map_err(|e| KeyEncryptionError::EncryptionFailed(e.to_string()))?;

    // Generate a random nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt the private key
    let ciphertext = cipher
        .encrypt(nonce, private_key)
        .map_err(|e| KeyEncryptionError::EncryptionFailed(e.to_string()))?;

    // Concatenate nonce || ciphertext (which includes the tag)
    let mut encrypted = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    encrypted.extend_from_slice(&nonce_bytes);
    encrypted.extend_from_slice(&ciphertext);

    Ok(encrypted)
}

/// Decrypts an Ed25519 private key that was encrypted with AES-256-GCM.
///
/// # Arguments
///
/// * `encrypted_data` - The encrypted data in format: `nonce (12 bytes) || ciphertext || tag`
/// * `encryption_key` - The 32-byte AES-256 encryption key
///
/// # Returns
///
/// The decrypted 32-byte Ed25519 private key seed.
///
/// # Errors
///
/// Returns `KeyEncryptionError` if decryption fails or data is invalid.
pub fn decrypt_private_key(
    encrypted_data: &[u8],
    encryption_key: &[u8],
) -> Result<Vec<u8>, KeyEncryptionError> {
    if encryption_key.len() != 32 {
        return Err(KeyEncryptionError::InvalidKeyLength(encryption_key.len()));
    }

    // Minimum size: nonce (12) + tag (16) + at least 1 byte of ciphertext
    if encrypted_data.len() < NONCE_SIZE + 17 {
        return Err(KeyEncryptionError::InvalidEncryptedData);
    }

    let cipher = Aes256Gcm::new_from_slice(encryption_key)
        .map_err(|e| KeyEncryptionError::DecryptionFailed(e.to_string()))?;

    // Extract nonce and ciphertext
    let nonce = Nonce::from_slice(&encrypted_data[..NONCE_SIZE]);
    let ciphertext = &encrypted_data[NONCE_SIZE..];

    // Decrypt
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| KeyEncryptionError::DecryptionFailed(e.to_string()))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let private_key = [0x42u8; 32]; // Dummy private key
        let encryption_key = [0x01u8; 32]; // Dummy encryption key

        let encrypted = encrypt_private_key(&private_key, &encryption_key).unwrap();

        // Encrypted should be larger than original (nonce + tag overhead)
        assert!(encrypted.len() > private_key.len());

        let decrypted = decrypt_private_key(&encrypted, &encryption_key).unwrap();

        assert_eq!(decrypted, private_key);
    }

    #[test]
    fn test_wrong_key_fails() {
        let private_key = [0x42u8; 32];
        let encryption_key = [0x01u8; 32];
        let wrong_key = [0x02u8; 32];

        let encrypted = encrypt_private_key(&private_key, &encryption_key).unwrap();
        let result = decrypt_private_key(&encrypted, &wrong_key);

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_key_length() {
        let private_key = [0x42u8; 32];
        let short_key = [0x01u8; 16];

        let result = encrypt_private_key(&private_key, &short_key);
        assert!(matches!(
            result,
            Err(KeyEncryptionError::InvalidKeyLength(16))
        ));
    }

    #[test]
    fn test_invalid_encrypted_data() {
        let encryption_key = [0x01u8; 32];
        let too_short = [0u8; 20]; // Less than nonce + tag + 1

        let result = decrypt_private_key(&too_short, &encryption_key);
        assert!(matches!(
            result,
            Err(KeyEncryptionError::InvalidEncryptedData)
        ));
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let private_key = [0x42u8; 32];
        let encryption_key = [0x01u8; 32];

        let mut encrypted = encrypt_private_key(&private_key, &encryption_key).unwrap();

        // Tamper with the ciphertext
        encrypted[NONCE_SIZE + 5] ^= 0xFF;

        let result = decrypt_private_key(&encrypted, &encryption_key);
        assert!(result.is_err());
    }
}
