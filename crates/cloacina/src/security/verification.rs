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

//! Package verification for secure loading.
//!
//! This module provides:
//! - [`SecurityConfig`] for configuring signature requirements
//! - [`VerificationError`] for specific failure types
//! - [`SignatureSource`] for specifying where to find signatures
//! - [`verify_and_load_package`] for verified package loading

use super::audit;
use crate::crypto::verify_signature;
use crate::database::universal_types::UniversalUuid;
use std::path::{Path, PathBuf};
use thiserror::Error;

use super::package_signer::{DbPackageSigner, DetachedSignature, PackageSigner};
use super::{DbKeyManager, KeyManager};

/// Security configuration for package verification.
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Whether package signatures are required (default: false).
    ///
    /// When `true`, packages without valid signatures from trusted keys
    /// will fail to load with a hard error.
    ///
    /// When `false`, packages load without verification (for local development).
    pub require_signatures: bool,

    /// Master encryption key for decrypting signing keys (optional).
    ///
    /// Only needed if using database-stored signing keys for signing operations.
    /// For verification-only scenarios (loading packages), this is not required.
    pub key_encryption_key: Option<[u8; 32]>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_signatures: false,
            key_encryption_key: None,
        }
    }
}

impl SecurityConfig {
    /// Create a security config that requires signatures.
    pub fn require_signatures() -> Self {
        Self {
            require_signatures: true,
            key_encryption_key: None,
        }
    }

    /// Create a security config with no signature requirements (for development).
    pub fn development() -> Self {
        Self::default()
    }

    /// Set the key encryption key for signing operations.
    pub fn with_encryption_key(mut self, key: [u8; 32]) -> Self {
        self.key_encryption_key = Some(key);
        self
    }
}

/// Errors that occur during package verification.
///
/// These are hard failures - there are no "warnings" for security.
#[derive(Debug, Error)]
pub enum VerificationError {
    #[error("Package has been tampered with: hash mismatch (expected {expected}, got {actual})")]
    TamperedPackage {
        /// Expected hash from the signature
        expected: String,
        /// Actual hash computed from the package
        actual: String,
    },

    #[error("Package signed by untrusted key: {fingerprint}")]
    UntrustedSigner {
        /// Fingerprint of the key that signed the package
        fingerprint: String,
    },

    #[error("Invalid signature: cryptographic verification failed")]
    InvalidSignature,

    #[error("Signature not found for package (hash: {hash})")]
    SignatureNotFound {
        /// Hash of the package we're looking for a signature for
        hash: String,
    },

    #[error("Signature file malformed: {reason}")]
    MalformedSignature {
        /// Description of what's wrong with the signature
        reason: String,
    },

    #[error("Failed to read package file: {error}")]
    FileReadError {
        /// The underlying IO error
        error: String,
    },

    #[error("Failed to compute package hash: {error}")]
    HashError {
        /// Description of the hash computation error
        error: String,
    },

    #[error("Database error: {error}")]
    DatabaseError {
        /// The underlying database error
        error: String,
    },

    #[error("Key manager error: {error}")]
    KeyManagerError {
        /// The underlying key manager error
        error: String,
    },
}

/// Where to find the signature for a package.
#[derive(Debug, Clone)]
pub enum SignatureSource {
    /// Load signature from database by package hash.
    Database,

    /// Load signature from a detached `.sig` file.
    DetachedFile {
        /// Path to the signature file
        path: PathBuf,
    },

    /// Try detached file first (package_path + ".sig"), then database.
    Auto,
}

impl Default for SignatureSource {
    fn default() -> Self {
        Self::Auto
    }
}

/// Result of successful verification.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Hash of the verified package
    pub package_hash: String,
    /// Fingerprint of the key that signed it
    pub signer_fingerprint: String,
    /// Name of the trusted key (if available)
    pub signer_name: Option<String>,
}

/// Verify a package signature.
///
/// This function performs full cryptographic verification:
/// 1. Computes the package hash
/// 2. Loads the signature (from database or file)
/// 3. Finds the trusted key that signed it
/// 4. Verifies the Ed25519 signature
///
/// # Arguments
///
/// * `package_path` - Path to the package file to verify
/// * `org_id` - Organization ID to check trusted keys for
/// * `signature_source` - Where to find the signature
/// * `package_signer` - Package signer for database operations
/// * `key_manager` - Key manager for trusted key lookup
///
/// # Returns
///
/// `Ok(VerificationResult)` if verification succeeds, `Err(VerificationError)` otherwise.
pub async fn verify_package<P: AsRef<Path>>(
    package_path: P,
    org_id: UniversalUuid,
    signature_source: SignatureSource,
    package_signer: &DbPackageSigner,
    key_manager: &DbKeyManager,
) -> Result<VerificationResult, VerificationError> {
    let package_path = package_path.as_ref();

    // 1. Compute package hash
    let package_data =
        std::fs::read(package_path).map_err(|e| VerificationError::FileReadError {
            error: e.to_string(),
        })?;

    let package_hash = compute_package_hash(&package_data)?;

    // 2. Load signature based on source
    let signature = match signature_source {
        SignatureSource::Database => load_signature_from_db(&package_hash, package_signer).await?,

        SignatureSource::DetachedFile { path } => load_signature_from_file(&path)?,

        SignatureSource::Auto => {
            // Try detached file first
            let sig_path = package_path.with_extension(format!(
                "{}.sig",
                package_path
                    .extension()
                    .map(|e| e.to_str().unwrap_or(""))
                    .unwrap_or("")
            ));

            if sig_path.exists() {
                load_signature_from_file(&sig_path)?
            } else {
                load_signature_from_db(&package_hash, package_signer).await?
            }
        }
    };

    // 3. Verify hash matches (tamper detection)
    if signature.package_hash != package_hash {
        audit::log_verification_failure(
            org_id,
            &package_hash,
            "tampered",
            Some(&signature.key_fingerprint),
        );
        return Err(VerificationError::TamperedPackage {
            expected: signature.package_hash,
            actual: package_hash,
        });
    }

    // 4. Find trusted key by fingerprint
    let trusted_key = key_manager
        .find_trusted_key(org_id, &signature.key_fingerprint)
        .await
        .map_err(|e| VerificationError::KeyManagerError {
            error: e.to_string(),
        })?
        .ok_or_else(|| {
            audit::log_verification_failure(
                org_id,
                &package_hash,
                "untrusted_signer",
                Some(&signature.key_fingerprint),
            );
            VerificationError::UntrustedSigner {
                fingerprint: signature.key_fingerprint.clone(),
            }
        })?;

    // 5. Verify Ed25519 signature
    let hash_bytes = hex::decode(&package_hash).map_err(|e| VerificationError::HashError {
        error: e.to_string(),
    })?;

    let sig_bytes = signature.signature_bytes().map_err(|_| {
        audit::log_verification_failure(
            org_id,
            &package_hash,
            "invalid_signature",
            Some(&signature.key_fingerprint),
        );
        VerificationError::InvalidSignature
    })?;

    if let Err(_) = verify_signature(&hash_bytes, &sig_bytes, &trusted_key.public_key) {
        audit::log_verification_failure(
            org_id,
            &package_hash,
            "invalid_signature",
            Some(&signature.key_fingerprint),
        );
        return Err(VerificationError::InvalidSignature);
    }

    // 6. Success - audit log
    audit::log_verification_success(
        org_id,
        &package_hash,
        &signature.key_fingerprint,
        trusted_key.key_name.as_deref(),
    );

    Ok(VerificationResult {
        package_hash,
        signer_fingerprint: signature.key_fingerprint,
        signer_name: trusted_key.key_name,
    })
}

/// Verify a package using only a detached signature and public key (offline mode).
///
/// This is useful when the database is not available or for offline verification.
///
/// # Arguments
///
/// * `package_path` - Path to the package file to verify
/// * `signature_path` - Path to the detached signature file
/// * `public_key` - The 32-byte Ed25519 public key to verify against
///
/// # Returns
///
/// `Ok(())` if verification succeeds, `Err(VerificationError)` otherwise.
pub fn verify_package_offline<P: AsRef<Path>, S: AsRef<Path>>(
    package_path: P,
    signature_path: S,
    public_key: &[u8],
) -> Result<VerificationResult, VerificationError> {
    let package_path = package_path.as_ref();
    let signature_path = signature_path.as_ref();

    // 1. Load package and compute hash
    let package_data =
        std::fs::read(package_path).map_err(|e| VerificationError::FileReadError {
            error: e.to_string(),
        })?;

    let package_hash = compute_package_hash(&package_data)?;

    // 2. Load signature from file
    let signature = load_signature_from_file(signature_path)?;

    // 3. Verify hash matches
    if signature.package_hash != package_hash {
        return Err(VerificationError::TamperedPackage {
            expected: signature.package_hash,
            actual: package_hash,
        });
    }

    // 4. Verify key fingerprint matches
    let expected_fingerprint = crate::crypto::compute_key_fingerprint(public_key);
    if signature.key_fingerprint != expected_fingerprint {
        return Err(VerificationError::UntrustedSigner {
            fingerprint: signature.key_fingerprint,
        });
    }

    // 5. Verify signature
    let hash_bytes = hex::decode(&package_hash).map_err(|e| VerificationError::HashError {
        error: e.to_string(),
    })?;

    let sig_bytes = signature
        .signature_bytes()
        .map_err(|_| VerificationError::InvalidSignature)?;

    verify_signature(&hash_bytes, &sig_bytes, public_key)
        .map_err(|_| VerificationError::InvalidSignature)?;

    tracing::info!(
        event_type = "verification.success.offline",
        package = %package_path.display(),
        signer_fingerprint = %signature.key_fingerprint,
        "Package signature verified (offline mode)"
    );

    Ok(VerificationResult {
        package_hash,
        signer_fingerprint: signature.key_fingerprint,
        signer_name: None,
    })
}

/// Compute SHA256 hash of package data.
fn compute_package_hash(data: &[u8]) -> Result<String, VerificationError> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(data);
    Ok(hex::encode(hasher.finalize()))
}

/// Load signature from database.
async fn load_signature_from_db(
    package_hash: &str,
    package_signer: &DbPackageSigner,
) -> Result<DetachedSignature, VerificationError> {
    let signature = package_signer
        .find_signature(package_hash)
        .await
        .map_err(|e| VerificationError::DatabaseError {
            error: e.to_string(),
        })?
        .ok_or_else(|| VerificationError::SignatureNotFound {
            hash: package_hash.to_string(),
        })?;

    Ok(DetachedSignature::from_signature_info(&signature))
}

/// Load signature from file.
fn load_signature_from_file(path: &Path) -> Result<DetachedSignature, VerificationError> {
    DetachedSignature::read_from_file(path).map_err(|e| VerificationError::MalformedSignature {
        reason: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::generate_signing_keypair;
    use base64::Engine;
    use tempfile::NamedTempFile;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(!config.require_signatures);
        assert!(config.key_encryption_key.is_none());
    }

    #[test]
    fn test_security_config_require_signatures() {
        let config = SecurityConfig::require_signatures();
        assert!(config.require_signatures);
    }

    #[test]
    fn test_security_config_with_encryption_key() {
        let key = [0x42u8; 32];
        let config = SecurityConfig::default().with_encryption_key(key);
        assert_eq!(config.key_encryption_key, Some(key));
    }

    #[test]
    fn test_verify_package_offline_with_invalid_signature() {
        // Create a test package
        let package_file = NamedTempFile::new().unwrap();
        std::fs::write(package_file.path(), b"test package content").unwrap();

        // Generate a keypair
        let keypair = generate_signing_keypair();

        // Create a signature file with wrong hash
        let sig = DetachedSignature {
            version: 1,
            algorithm: "ed25519".to_string(),
            package_hash: "wrong_hash".to_string(),
            key_fingerprint: keypair.fingerprint.clone(),
            signature: base64::engine::general_purpose::STANDARD.encode(&[0u8; 64]),
            signed_at: chrono::Utc::now().to_rfc3339(),
        };

        let sig_file = NamedTempFile::new().unwrap();
        sig.write_to_file(sig_file.path()).unwrap();

        // Verification should fail due to hash mismatch
        let result =
            verify_package_offline(package_file.path(), sig_file.path(), &keypair.public_key);

        assert!(matches!(
            result,
            Err(VerificationError::TamperedPackage { .. })
        ));
    }

    #[test]
    fn test_signature_source_default() {
        let source = SignatureSource::default();
        assert!(matches!(source, SignatureSource::Auto));
    }
}
