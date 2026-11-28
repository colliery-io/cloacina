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
#[derive(Debug, Clone, Default)]
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

    /// The server-wide organization (CLOACINA-A-0018). An upload to a tenant
    /// is checked against the trusted keys of the tenant's own org first
    /// ([`super::tenant_org_id`]), then against the keys of this org if one
    /// is set: a key in it is trusted for every tenant. `None` means each
    /// tenant trusts only its own keys.
    pub verification_org_id: Option<UniversalUuid>,
}

impl SecurityConfig {
    /// Create a security config that requires signatures.
    pub fn require_signatures() -> Self {
        Self {
            require_signatures: true,
            key_encryption_key: None,
            verification_org_id: None,
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

    /// The signer's key is in the trusted keys of an organization that was
    /// asked, revoked, and no organization that was asked holds it live
    /// (CLOACINA-T-1062).
    #[error("Package signed by a revoked key: {fingerprint}")]
    RevokedSigner {
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
#[derive(Debug, Clone, Default)]
pub enum SignatureSource {
    /// Load signature from database by package hash.
    Database,

    /// Load signature from a detached `.sig` file.
    DetachedFile {
        /// Path to the signature file
        path: PathBuf,
    },

    /// Try detached file first (package_path + ".sig"), then database.
    #[default]
    Auto,
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

    if verify_signature(&hash_bytes, &sig_bytes, &trusted_key.public_key).is_err() {
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

/// Verify a package signature against in-memory bytes. Same flow as
/// `verify_package` but takes a `&[u8]` instead of a path; useful for
/// upload handlers that already have the package buffered (T-0557 Bug 2).
pub async fn verify_package_bytes(
    package_data: &[u8],
    org_id: UniversalUuid,
    signature_source: SignatureSource,
    package_signer: &DbPackageSigner,
    key_manager: &DbKeyManager,
) -> Result<VerificationResult, VerificationError> {
    verify_package_bytes_for_orgs(
        package_data,
        &[org_id],
        signature_source,
        package_signer,
        key_manager,
    )
    .await
    .map(|(result, _)| result)
}

/// Verify a package signature against in-memory bytes, with the signer looked
/// up in each organization of `org_ids`, in order (CLOACINA-A-0018: the
/// tenant's org first, then the server org if one is set).
///
/// The first organization that trusts the signer's key decides: the result
/// names it. A key that no organization of the list knows is
/// [`VerificationError::UntrustedSigner`]; a key that one of them holds
/// revoked, and none live, is [`VerificationError::RevokedSigner`]. An empty
/// list trusts no key.
///
/// The failure lines of the audit log name the first organization of the
/// list.
pub async fn verify_package_bytes_for_orgs(
    package_data: &[u8],
    org_ids: &[UniversalUuid],
    signature_source: SignatureSource,
    package_signer: &DbPackageSigner,
    key_manager: &DbKeyManager,
) -> Result<(VerificationResult, UniversalUuid), VerificationError> {
    let package_hash = compute_package_hash(package_data)?;
    // The org of the failure lines. With an empty list no key is trusted;
    // the nil id marks that no org was asked.
    let log_org = org_ids
        .first()
        .copied()
        .unwrap_or(UniversalUuid(uuid::Uuid::nil()));

    let signature = match signature_source {
        SignatureSource::Database => load_signature_from_db(&package_hash, package_signer).await?,
        SignatureSource::DetachedFile { path } => load_signature_from_file(&path)?,
        SignatureSource::Auto => {
            // No on-disk path to derive a sidecar from — fall back to
            // database lookup. Callers wanting detached-file behavior
            // must pass `DetachedFile` explicitly.
            load_signature_from_db(&package_hash, package_signer).await?
        }
    };

    check_signature_for_orgs(&package_hash, &signature, org_ids, log_org, key_manager).await
}

/// Verify a detached signature that came with the package bytes, with the
/// signer looked up in each organization of `org_ids`, in order
/// (CLOACINA-A-0018, CLOACINA-T-1062: the `signature` field of an upload).
///
/// No signature store is read: the signature is the one given. The checks,
/// in order, and the error of each:
///
/// 1. the format is one this build knows ([`VerificationError::MalformedSignature`]);
/// 2. the signature was made for these bytes ([`VerificationError::TamperedPackage`]);
/// 3. an organization of the list holds the signer's key live
///    ([`VerificationError::RevokedSigner`] when one holds it revoked and
///    none live, else [`VerificationError::UntrustedSigner`]);
/// 4. the signature verifies with that key ([`VerificationError::InvalidSignature`]).
///
/// The result names the organization that trusted the key.
pub async fn verify_detached_signature_for_orgs(
    package_data: &[u8],
    signature: &DetachedSignature,
    org_ids: &[UniversalUuid],
    key_manager: &DbKeyManager,
) -> Result<(VerificationResult, UniversalUuid), VerificationError> {
    let package_hash = compute_package_hash(package_data)?;
    let log_org = org_ids
        .first()
        .copied()
        .unwrap_or(UniversalUuid(uuid::Uuid::nil()));
    check_signature_for_orgs(&package_hash, signature, org_ids, log_org, key_manager).await
}

/// Verify the signature a registry stored for a package, for a load
/// (CLOACINA-T-1062).
///
/// `package_signer` reads the rows where the package lives; `key_manager`
/// reads the trusted keys, which can be another database (a tenant's rows
/// are in the tenant's schema, the trusted keys in the admin schema). Only
/// rows stored for an organization of `org_ids` count. The package passes
/// when one of them passes [`verify_detached_signature_for_orgs`]; else the
/// error is the one of the last row, or
/// [`VerificationError::SignatureNotFound`] with no row.
pub async fn verify_stored_signature_for_orgs(
    package_data: &[u8],
    org_ids: &[UniversalUuid],
    package_signer: &DbPackageSigner,
    key_manager: &DbKeyManager,
) -> Result<(VerificationResult, UniversalUuid), VerificationError> {
    let package_hash = compute_package_hash(package_data)?;
    verify_stored_signature_hash_for_orgs(&package_hash, org_ids, package_signer, key_manager).await
}

/// [`verify_stored_signature_for_orgs`] for a package whose hash is known:
/// the check of a loaded package again after a key is revoked
/// (CLOACINA-T-1119). The bytes are not read again; the hash is the one the
/// load check computed.
pub async fn verify_stored_signature_hash_for_orgs(
    package_hash: &str,
    org_ids: &[UniversalUuid],
    package_signer: &DbPackageSigner,
    key_manager: &DbKeyManager,
) -> Result<(VerificationResult, UniversalUuid), VerificationError> {
    let package_hash = package_hash.to_string();
    let log_org = org_ids
        .first()
        .copied()
        .unwrap_or(UniversalUuid(uuid::Uuid::nil()));
    let rows = package_signer
        .find_signatures_for_orgs(&package_hash, org_ids)
        .await
        .map_err(|e| VerificationError::DatabaseError {
            error: e.to_string(),
        })?;

    let mut last_error = VerificationError::SignatureNotFound {
        hash: package_hash.clone(),
    };
    for (row, _stored_for) in rows {
        let signature = DetachedSignature::from_signature_info(&row);
        match check_signature_for_orgs(&package_hash, &signature, org_ids, log_org, key_manager)
            .await
        {
            Ok(verified) => return Ok(verified),
            Err(e) => last_error = e,
        }
    }
    Err(last_error)
}

/// The checks every verification by organization shares: the format, the
/// hash, the trusted key, the signature. `log_org` is the organization of a
/// failure line written before a key was found.
async fn check_signature_for_orgs(
    package_hash: &str,
    signature: &DetachedSignature,
    org_ids: &[UniversalUuid],
    log_org: UniversalUuid,
    key_manager: &DbKeyManager,
) -> Result<(VerificationResult, UniversalUuid), VerificationError> {
    if signature.version != DetachedSignature::VERSION {
        return Err(VerificationError::MalformedSignature {
            reason: format!("unsupported signature version {}", signature.version),
        });
    }
    if signature.algorithm != DetachedSignature::ALGORITHM {
        return Err(VerificationError::MalformedSignature {
            reason: format!("unsupported signature algorithm '{}'", signature.algorithm),
        });
    }

    if signature.package_hash != package_hash {
        audit::log_verification_failure(
            log_org,
            package_hash,
            "tampered",
            Some(&signature.key_fingerprint),
        );
        return Err(VerificationError::TamperedPackage {
            expected: signature.package_hash.clone(),
            actual: package_hash.to_string(),
        });
    }

    let mut trusted = None;
    for org_id in org_ids {
        let found = key_manager
            .find_trusted_key(*org_id, &signature.key_fingerprint)
            .await
            .map_err(|e| VerificationError::KeyManagerError {
                error: e.to_string(),
            })?;
        if let Some(key) = found {
            trusted = Some((key, *org_id));
            break;
        }
    }
    let Some((trusted_key, org_id)) = trusted else {
        // No organization holds the key live. One that holds it revoked
        // makes the answer "revoked"; a key no organization of the list
        // knows is "untrusted", whoever else trusts it.
        let mut revoked = false;
        for org_id in org_ids {
            let keys = key_manager
                .list_org_trusted_keys(*org_id)
                .await
                .map_err(|e| VerificationError::KeyManagerError {
                    error: e.to_string(),
                })?;
            if keys
                .iter()
                .any(|k| k.fingerprint == signature.key_fingerprint && !k.is_active())
            {
                revoked = true;
                break;
            }
        }
        let reason = if revoked {
            "revoked_signer"
        } else {
            "untrusted_signer"
        };
        audit::log_verification_failure(
            log_org,
            package_hash,
            reason,
            Some(&signature.key_fingerprint),
        );
        return Err(if revoked {
            VerificationError::RevokedSigner {
                fingerprint: signature.key_fingerprint.clone(),
            }
        } else {
            VerificationError::UntrustedSigner {
                fingerprint: signature.key_fingerprint.clone(),
            }
        });
    };

    let hash_bytes = hex::decode(package_hash).map_err(|e| VerificationError::HashError {
        error: e.to_string(),
    })?;

    let sig_bytes = signature.signature_bytes().map_err(|_| {
        audit::log_verification_failure(
            org_id,
            package_hash,
            "invalid_signature",
            Some(&signature.key_fingerprint),
        );
        VerificationError::InvalidSignature
    })?;

    if verify_signature(&hash_bytes, &sig_bytes, &trusted_key.public_key).is_err() {
        audit::log_verification_failure(
            org_id,
            package_hash,
            "invalid_signature",
            Some(&signature.key_fingerprint),
        );
        return Err(VerificationError::InvalidSignature);
    }

    audit::log_verification_success(
        org_id,
        package_hash,
        &signature.key_fingerprint,
        trusted_key.key_name.as_deref(),
    );

    Ok((
        VerificationResult {
            package_hash: package_hash.to_string(),
            signer_fingerprint: signature.key_fingerprint.clone(),
            signer_name: trusted_key.key_name,
        },
        org_id,
    ))
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
            signature: base64::engine::general_purpose::STANDARD.encode([0u8; 64]),
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

    #[test]
    fn test_verify_package_offline_valid_signature() {
        let package_file = NamedTempFile::new().unwrap();
        let content = b"valid package content";
        std::fs::write(package_file.path(), content).unwrap();

        let keypair = generate_signing_keypair();

        // Compute correct hash and sign it
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content);
        let package_hash = hex::encode(hasher.finalize());

        let hash_bytes = hex::decode(&package_hash).unwrap();
        let signature_bytes =
            crate::crypto::sign_package(&hash_bytes, &keypair.private_key).unwrap();

        let sig = DetachedSignature {
            version: 1,
            algorithm: "ed25519".to_string(),
            package_hash: package_hash.clone(),
            key_fingerprint: keypair.fingerprint.clone(),
            signature: base64::engine::general_purpose::STANDARD.encode(&signature_bytes),
            signed_at: chrono::Utc::now().to_rfc3339(),
        };

        let sig_file = NamedTempFile::new().unwrap();
        sig.write_to_file(sig_file.path()).unwrap();

        let result =
            verify_package_offline(package_file.path(), sig_file.path(), &keypair.public_key);
        assert!(result.is_ok());
        let verification = result.unwrap();
        assert_eq!(verification.package_hash, package_hash);
        assert_eq!(verification.signer_fingerprint, keypair.fingerprint);
    }

    #[test]
    fn test_verify_package_offline_tampered_content() {
        let package_file = NamedTempFile::new().unwrap();
        let original = b"original content";
        std::fs::write(package_file.path(), original).unwrap();

        let keypair = generate_signing_keypair();

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(original);
        let package_hash = hex::encode(hasher.finalize());

        let hash_bytes = hex::decode(&package_hash).unwrap();
        let signature_bytes =
            crate::crypto::sign_package(&hash_bytes, &keypair.private_key).unwrap();

        let sig = DetachedSignature {
            version: 1,
            algorithm: "ed25519".to_string(),
            package_hash,
            key_fingerprint: keypair.fingerprint.clone(),
            signature: base64::engine::general_purpose::STANDARD.encode(&signature_bytes),
            signed_at: chrono::Utc::now().to_rfc3339(),
        };

        let sig_file = NamedTempFile::new().unwrap();
        sig.write_to_file(sig_file.path()).unwrap();

        // Tamper with the package content
        std::fs::write(package_file.path(), b"tampered content").unwrap();

        let result =
            verify_package_offline(package_file.path(), sig_file.path(), &keypair.public_key);
        assert!(matches!(
            result,
            Err(VerificationError::TamperedPackage { .. })
        ));
    }

    #[test]
    fn test_verify_package_offline_wrong_key() {
        let package_file = NamedTempFile::new().unwrap();
        let content = b"content";
        std::fs::write(package_file.path(), content).unwrap();

        let signer = generate_signing_keypair();
        let wrong_verifier = generate_signing_keypair();

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content);
        let package_hash = hex::encode(hasher.finalize());

        let hash_bytes = hex::decode(&package_hash).unwrap();
        let signature_bytes =
            crate::crypto::sign_package(&hash_bytes, &signer.private_key).unwrap();

        let sig = DetachedSignature {
            version: 1,
            algorithm: "ed25519".to_string(),
            package_hash,
            key_fingerprint: signer.fingerprint.clone(),
            signature: base64::engine::general_purpose::STANDARD.encode(&signature_bytes),
            signed_at: chrono::Utc::now().to_rfc3339(),
        };

        let sig_file = NamedTempFile::new().unwrap();
        sig.write_to_file(sig_file.path()).unwrap();

        let result = verify_package_offline(
            package_file.path(),
            sig_file.path(),
            &wrong_verifier.public_key,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_package_offline_nonexistent_package() {
        let keypair = generate_signing_keypair();
        let sig_file = NamedTempFile::new().unwrap();

        let sig = DetachedSignature {
            version: 1,
            algorithm: "ed25519".to_string(),
            package_hash: "hash".to_string(),
            key_fingerprint: "fp".to_string(),
            signature: "sig".to_string(),
            signed_at: chrono::Utc::now().to_rfc3339(),
        };
        sig.write_to_file(sig_file.path()).unwrap();

        let result = verify_package_offline(
            std::path::Path::new("/nonexistent/package"),
            sig_file.path(),
            &keypair.public_key,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_package_offline_nonexistent_signature() {
        let keypair = generate_signing_keypair();
        let package_file = NamedTempFile::new().unwrap();
        std::fs::write(package_file.path(), b"content").unwrap();

        let result = verify_package_offline(
            package_file.path(),
            std::path::Path::new("/nonexistent/sig"),
            &keypair.public_key,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_load_signature_from_file_valid() {
        let sig = DetachedSignature {
            version: 1,
            algorithm: "ed25519".to_string(),
            package_hash: "abc".to_string(),
            key_fingerprint: "def".to_string(),
            signature: "c2lnbmF0dXJl".to_string(),
            signed_at: chrono::Utc::now().to_rfc3339(),
        };
        let temp = NamedTempFile::new().unwrap();
        sig.write_to_file(temp.path()).unwrap();

        let result = load_signature_from_file(temp.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().package_hash, "abc");
    }

    #[test]
    fn test_load_signature_from_file_invalid() {
        let temp = NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), "not json").unwrap();

        let result = load_signature_from_file(temp.path());
        assert!(matches!(
            result,
            Err(VerificationError::MalformedSignature { .. })
        ));
    }

    // ── A signature that comes with the bytes, and the load check
    //    (CLOACINA-T-1062, CLOACINA-A-0018) ──────────────────────────

    #[cfg(feature = "sqlite")]
    mod by_org {
        use super::*;
        use crate::dal::unified::DAL;
        use crate::database::Database;
        use crate::security::PackageSigner;
        use crate::security::tenant_org_id;

        async fn new_database() -> Database {
            let url = format!(
                "file:verify_by_org_{}?mode=memory&cache=shared",
                uuid::Uuid::new_v4()
            );
            let db = Database::new(&url, "", 5);
            db.run_migrations()
                .await
                .expect("migrations should succeed");
            db
        }

        const BYTES: &[u8] = b"the bytes of a package";

        fn signed(keypair: &crate::crypto::GeneratedKeypair) -> DetachedSignature {
            DetachedSignature::sign(BYTES, &keypair.private_key).unwrap()
        }

        /// The tenant's org is asked first, then the server org. The result
        /// names the org that trusted the key.
        #[tokio::test]
        async fn test_detached_signature_is_checked_against_the_orgs_in_order() {
            let km = DbKeyManager::new(DAL::new(new_database().await));
            let tenant_org = tenant_org_id("acme");
            let server_org = UniversalUuid::new_v4();
            let orgs = [tenant_org, server_org];
            let tenant_key = generate_signing_keypair();
            let server_key = generate_signing_keypair();
            km.trust_public_key(tenant_org, &tenant_key.public_key, None)
                .await
                .unwrap();
            km.trust_public_key(server_org, &server_key.public_key, None)
                .await
                .unwrap();

            let (result, org) =
                verify_detached_signature_for_orgs(BYTES, &signed(&tenant_key), &orgs, &km)
                    .await
                    .unwrap();
            assert_eq!(org, tenant_org);
            assert_eq!(result.signer_fingerprint, tenant_key.fingerprint);

            let (_, org) =
                verify_detached_signature_for_orgs(BYTES, &signed(&server_key), &orgs, &km)
                    .await
                    .unwrap();
            assert_eq!(org, server_org);

            // The server key is no use when the server org is not asked.
            let alone =
                verify_detached_signature_for_orgs(BYTES, &signed(&server_key), &[tenant_org], &km)
                    .await;
            assert!(
                matches!(alone, Err(VerificationError::UntrustedSigner { .. })),
                "{alone:?}"
            );
            // A key in both orgs is trusted by the first.
            km.trust_public_key(tenant_org, &server_key.public_key, None)
                .await
                .unwrap();
            let (_, org) =
                verify_detached_signature_for_orgs(BYTES, &signed(&server_key), &orgs, &km)
                    .await
                    .unwrap();
            assert_eq!(org, tenant_org);
        }

        /// Each signature that must not pass has its own error.
        #[tokio::test]
        async fn test_each_bad_detached_signature_has_its_own_error() {
            let km = DbKeyManager::new(DAL::new(new_database().await));
            let tenant_org = tenant_org_id("acme");
            let other_tenant_org = tenant_org_id("globex");
            let server_org = UniversalUuid::new_v4();
            let orgs = [tenant_org, server_org];
            let trusted = generate_signing_keypair();
            km.trust_public_key(tenant_org, &trusted.public_key, None)
                .await
                .unwrap();

            // A key nobody trusts, and a key only another tenant trusts.
            let stranger = generate_signing_keypair();
            let others = generate_signing_keypair();
            km.trust_public_key(other_tenant_org, &others.public_key, None)
                .await
                .unwrap();
            for keypair in [&stranger, &others] {
                let result =
                    verify_detached_signature_for_orgs(BYTES, &signed(keypair), &orgs, &km).await;
                assert!(
                    matches!(result, Err(VerificationError::UntrustedSigner { .. })),
                    "{result:?}"
                );
            }

            // A signature for other bytes.
            let other_bytes =
                DetachedSignature::sign(b"other bytes", &trusted.private_key).unwrap();
            let result = verify_detached_signature_for_orgs(BYTES, &other_bytes, &orgs, &km).await;
            assert!(
                matches!(result, Err(VerificationError::TamperedPackage { .. })),
                "{result:?}"
            );

            // The trusted key's fingerprint, signature bytes of another key.
            let mut forged = signed(&trusted);
            forged.signature = signed(&stranger).signature;
            let result = verify_detached_signature_for_orgs(BYTES, &forged, &orgs, &km).await;
            assert!(
                matches!(result, Err(VerificationError::InvalidSignature)),
                "{result:?}"
            );
            // Signature bytes that are not base64.
            let mut not_base64 = signed(&trusted);
            not_base64.signature = "not!base64!".to_string();
            let result = verify_detached_signature_for_orgs(BYTES, &not_base64, &orgs, &km).await;
            assert!(
                matches!(result, Err(VerificationError::InvalidSignature)),
                "{result:?}"
            );

            // A format this build does not know.
            let mut other_algorithm = signed(&trusted);
            other_algorithm.algorithm = "rsa".to_string();
            let mut other_version = signed(&trusted);
            other_version.version = 2;
            for signature in [&other_algorithm, &other_version] {
                let result = verify_detached_signature_for_orgs(BYTES, signature, &orgs, &km).await;
                assert!(
                    matches!(result, Err(VerificationError::MalformedSignature { .. })),
                    "{result:?}"
                );
            }

            // No org asked: no key is trusted.
            let result =
                verify_detached_signature_for_orgs(BYTES, &signed(&trusted), &[], &km).await;
            assert!(
                matches!(result, Err(VerificationError::UntrustedSigner { .. })),
                "{result:?}"
            );
        }

        /// A revoked key is "revoked", not "untrusted". A key the tenant
        /// revoked that the server org holds live still passes.
        #[tokio::test]
        async fn test_a_revoked_signer_is_told_apart_and_the_server_org_can_still_trust_it() {
            let km = DbKeyManager::new(DAL::new(new_database().await));
            let tenant_org = tenant_org_id("acme");
            let server_org = UniversalUuid::new_v4();
            let orgs = [tenant_org, server_org];
            let keypair = generate_signing_keypair();
            let in_tenant = km
                .trust_public_key(tenant_org, &keypair.public_key, None)
                .await
                .unwrap();
            km.revoke_trusted_key(tenant_org, in_tenant.id)
                .await
                .unwrap();

            let result =
                verify_detached_signature_for_orgs(BYTES, &signed(&keypair), &orgs, &km).await;
            assert!(
                matches!(
                    &result,
                    Err(VerificationError::RevokedSigner { fingerprint })
                        if *fingerprint == keypair.fingerprint
                ),
                "{result:?}"
            );

            // Live in the server org: it passes, by the server org.
            let in_server = km
                .trust_public_key(server_org, &keypair.public_key, None)
                .await
                .unwrap();
            let (_, org) = verify_detached_signature_for_orgs(BYTES, &signed(&keypair), &orgs, &km)
                .await
                .unwrap();
            assert_eq!(org, server_org);

            // Revoked there too.
            km.revoke_trusted_key(server_org, in_server.id)
                .await
                .unwrap();
            let result =
                verify_detached_signature_for_orgs(BYTES, &signed(&keypair), &orgs, &km).await;
            assert!(
                matches!(result, Err(VerificationError::RevokedSigner { .. })),
                "{result:?}"
            );
        }

        /// Rows are stored with their org and read by org. One row for a
        /// hash and a key: a second store replaces the first.
        #[tokio::test]
        async fn test_signature_rows_carry_the_org_and_are_read_by_org() {
            let signer = DbPackageSigner::new(DAL::new(new_database().await));
            let org_a = tenant_org_id("acme");
            let org_b = tenant_org_id("globex");
            let keypair = generate_signing_keypair();
            let row = signed(&keypair).to_signature_info().unwrap();

            // A row with no org is not a row of any org.
            signer.store_signature(&row).await.unwrap();
            let rows = signer
                .find_signatures_for_orgs(&row.package_hash, &[org_a, org_b])
                .await
                .unwrap();
            assert!(rows.is_empty());

            // Stored for org A: org A finds it, org B does not.
            signer.store_signature_for_org(org_a, &row).await.unwrap();
            let rows = signer
                .find_signatures_for_orgs(&row.package_hash, &[org_a])
                .await
                .unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].1, org_a);
            assert_eq!(rows[0].0.key_fingerprint, keypair.fingerprint);
            let rows = signer
                .find_signatures_for_orgs(&row.package_hash, &[org_b])
                .await
                .unwrap();
            assert!(rows.is_empty());

            // Stored again, for org B: still one row, now of org B.
            signer.store_signature_for_org(org_b, &row).await.unwrap();
            assert_eq!(
                signer
                    .find_signatures(&row.package_hash)
                    .await
                    .unwrap()
                    .len(),
                1
            );
            let rows = signer
                .find_signatures_for_orgs(&row.package_hash, &[org_a, org_b])
                .await
                .unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].1, org_b);

            // A second key for the same bytes is a second row.
            let second = generate_signing_keypair();
            let second_row = signed(&second).to_signature_info().unwrap();
            signer
                .store_signature_for_org(org_b, &second_row)
                .await
                .unwrap();
            let rows = signer
                .find_signatures_for_orgs(&row.package_hash, &[org_b])
                .await
                .unwrap();
            assert_eq!(rows.len(), 2);
        }

        /// The load check: the rows are in one database (the tenant's), the
        /// trusted keys in another (the admin schema).
        #[tokio::test]
        async fn test_stored_signature_is_verified_with_keys_of_another_database() {
            let tenant_db = new_database().await;
            let admin_db = new_database().await;
            let signer = DbPackageSigner::new(DAL::new(tenant_db.clone()));
            let km = DbKeyManager::new(DAL::new(admin_db));
            let tenant_org = tenant_org_id("acme");
            let server_org = UniversalUuid::new_v4();
            let orgs = [tenant_org, server_org];
            let keypair = generate_signing_keypair();
            let row = signed(&keypair).to_signature_info().unwrap();

            // No row.
            let result = verify_stored_signature_for_orgs(BYTES, &orgs, &signer, &km).await;
            assert!(
                matches!(result, Err(VerificationError::SignatureNotFound { .. })),
                "{result:?}"
            );

            // A row with no org does not count.
            signer.store_signature(&row).await.unwrap();
            let result = verify_stored_signature_for_orgs(BYTES, &orgs, &signer, &km).await;
            assert!(
                matches!(result, Err(VerificationError::SignatureNotFound { .. })),
                "{result:?}"
            );

            // A row for the tenant's org, by a key the tenant does not
            // trust: a row alone proves nothing.
            signer
                .store_signature_for_org(tenant_org, &row)
                .await
                .unwrap();
            let result = verify_stored_signature_for_orgs(BYTES, &orgs, &signer, &km).await;
            assert!(
                matches!(result, Err(VerificationError::UntrustedSigner { .. })),
                "{result:?}"
            );
            // A key in the tenant's own database is not a trusted key: the
            // trusted keys are read from the other database.
            DbKeyManager::new(DAL::new(tenant_db))
                .trust_public_key(tenant_org, &keypair.public_key, None)
                .await
                .unwrap();
            let result = verify_stored_signature_for_orgs(BYTES, &orgs, &signer, &km).await;
            assert!(
                matches!(result, Err(VerificationError::UntrustedSigner { .. })),
                "{result:?}"
            );

            // The tenant trusts the key: the package passes.
            let trusted = km
                .trust_public_key(tenant_org, &keypair.public_key, None)
                .await
                .unwrap();
            let (result, org) = verify_stored_signature_for_orgs(BYTES, &orgs, &signer, &km)
                .await
                .unwrap();
            assert_eq!(org, tenant_org);
            assert_eq!(result.signer_fingerprint, keypair.fingerprint);
            // Other bytes have no row.
            let result = verify_stored_signature_for_orgs(b"other", &orgs, &signer, &km).await;
            assert!(
                matches!(result, Err(VerificationError::SignatureNotFound { .. })),
                "{result:?}"
            );

            // The revoke: the next check refuses the package.
            km.revoke_trusted_key(tenant_org, trusted.id).await.unwrap();
            let result = verify_stored_signature_for_orgs(BYTES, &orgs, &signer, &km).await;
            assert!(
                matches!(result, Err(VerificationError::RevokedSigner { .. })),
                "{result:?}"
            );
        }

        /// `sign_package_with_db_key` with `store_signature` stores the row
        /// for the org that signs, so the load check of that org finds it.
        #[tokio::test]
        async fn test_a_signature_stored_by_the_library_passes_the_load_check_of_its_org() {
            let dal = DAL::new(new_database().await);
            let km = DbKeyManager::new(dal.clone());
            let signer = DbPackageSigner::new(dal);
            let org = UniversalUuid::new_v4();
            let master_key = [3u8; 32];
            let key = km
                .create_signing_key(org, "release", &master_key)
                .await
                .unwrap();
            km.trust_public_key(org, &key.public_key, None)
                .await
                .unwrap();
            let file = NamedTempFile::new().unwrap();
            std::fs::write(file.path(), BYTES).unwrap();
            signer
                .sign_package_with_db_key(file.path(), org, key.id, &master_key, true)
                .await
                .unwrap();

            let (_, trusted_by) = verify_stored_signature_for_orgs(BYTES, &[org], &signer, &km)
                .await
                .unwrap();
            assert_eq!(trusted_by, org);
        }
    }
}
