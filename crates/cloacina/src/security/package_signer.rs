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

//! Package signing and signature storage.
//!
//! This module provides:
//! - [`PackageSigner`] trait for signing packages
//! - [`DbPackageSigner`] database-backed implementation
//! - [`DetachedSignature`] format for standalone signature files

use super::audit;
use crate::crypto::{compute_key_fingerprint, sign_package as crypto_sign, verify_signature};
use crate::dal::unified::models::{NewUnifiedPackageSignature, UnifiedPackageSignature};
use crate::dal::unified::DAL;
use crate::database::schema::unified::package_signatures;
use crate::database::universal_types::{UniversalBinary, UniversalTimestamp, UniversalUuid};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during package signing operations.
#[derive(Debug, Error)]
pub enum PackageSignError {
    #[error("Failed to read package file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("Key not found: {0}")]
    KeyNotFound(UniversalUuid),

    #[error("Key has been revoked")]
    KeyRevoked,

    #[error("Database error: {0}")]
    Database(String),

    #[error("Signature not found for package hash: {0}")]
    SignatureNotFound(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Invalid signature file format: {0}")]
    InvalidSignatureFile(String),
}

/// A package signature with all metadata.
#[derive(Debug, Clone)]
pub struct PackageSignatureInfo {
    /// SHA256 hex hash of the package binary
    pub package_hash: String,
    /// SHA256 hex fingerprint of the signing key
    pub key_fingerprint: String,
    /// 64-byte Ed25519 signature
    pub signature: Vec<u8>,
    /// When the package was signed
    pub signed_at: UniversalTimestamp,
}

/// Detached signature file format.
///
/// This is a JSON-serializable format for standalone `.sig` files
/// that can be distributed alongside packages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetachedSignature {
    /// Format version (currently 1)
    pub version: u32,
    /// Signature algorithm (currently "ed25519")
    pub algorithm: String,
    /// SHA256 hex hash of the package binary
    pub package_hash: String,
    /// SHA256 hex fingerprint of the signing key
    pub key_fingerprint: String,
    /// Base64-encoded 64-byte signature
    pub signature: String,
    /// ISO8601 timestamp of when the signature was created
    pub signed_at: String,
}

impl DetachedSignature {
    /// Current signature format version.
    pub const VERSION: u32 = 1;

    /// Algorithm identifier for Ed25519.
    pub const ALGORITHM: &'static str = "ed25519";

    /// Create a detached signature from signature info.
    pub fn from_signature_info(info: &PackageSignatureInfo) -> Self {
        Self {
            version: Self::VERSION,
            algorithm: Self::ALGORITHM.to_string(),
            package_hash: info.package_hash.clone(),
            key_fingerprint: info.key_fingerprint.clone(),
            signature: BASE64.encode(&info.signature),
            signed_at: info.signed_at.to_rfc3339(),
        }
    }

    /// Parse a detached signature from JSON.
    pub fn from_json(json: &str) -> Result<Self, PackageSignError> {
        serde_json::from_str(json)
            .map_err(|e| PackageSignError::InvalidSignatureFile(e.to_string()))
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, PackageSignError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| PackageSignError::InvalidSignatureFile(e.to_string()))
    }

    /// Get the raw signature bytes.
    pub fn signature_bytes(&self) -> Result<Vec<u8>, PackageSignError> {
        BASE64
            .decode(&self.signature)
            .map_err(|e| PackageSignError::InvalidSignatureFile(e.to_string()))
    }

    /// Write the detached signature to a file.
    pub fn write_to_file(&self, path: &Path) -> Result<(), PackageSignError> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Read a detached signature from a file.
    pub fn read_from_file(path: &Path) -> Result<Self, PackageSignError> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json)
    }
}

/// Trait for signing packages and managing signatures.
#[async_trait]
pub trait PackageSigner: Send + Sync {
    /// Sign a package file using a key from the database.
    ///
    /// # Arguments
    ///
    /// * `package_path` - Path to the package file to sign
    /// * `key_id` - ID of the signing key in the database
    /// * `master_key` - AES-256 key for decrypting the signing key
    /// * `store_signature` - Whether to store the signature in the database
    ///
    /// # Returns
    ///
    /// The signature information.
    async fn sign_package_with_db_key(
        &self,
        package_path: &Path,
        key_id: UniversalUuid,
        master_key: &[u8],
        store_signature: bool,
    ) -> Result<PackageSignatureInfo, PackageSignError>;

    /// Sign a package file using a raw key (for offline signing).
    ///
    /// # Arguments
    ///
    /// * `package_path` - Path to the package file to sign
    /// * `private_key` - 32-byte Ed25519 private key
    /// * `public_key` - 32-byte Ed25519 public key (for fingerprint)
    ///
    /// # Returns
    ///
    /// The signature information.
    fn sign_package_with_raw_key(
        &self,
        package_path: &Path,
        private_key: &[u8],
        public_key: &[u8],
    ) -> Result<PackageSignatureInfo, PackageSignError>;

    /// Sign package data directly (already in memory).
    fn sign_package_data(
        &self,
        package_data: &[u8],
        private_key: &[u8],
        public_key: &[u8],
    ) -> Result<PackageSignatureInfo, PackageSignError>;

    /// Store a signature in the database.
    async fn store_signature(
        &self,
        signature: &PackageSignatureInfo,
    ) -> Result<UniversalUuid, PackageSignError>;

    /// Find a signature by package hash.
    async fn find_signature(
        &self,
        package_hash: &str,
    ) -> Result<Option<PackageSignatureInfo>, PackageSignError>;

    /// Find all signatures for a package hash.
    async fn find_signatures(
        &self,
        package_hash: &str,
    ) -> Result<Vec<PackageSignatureInfo>, PackageSignError>;

    /// Verify a package against a stored signature.
    ///
    /// # Arguments
    ///
    /// * `package_path` - Path to the package file to verify
    /// * `org_id` - Organization to check trusted keys for
    ///
    /// # Returns
    ///
    /// The signature info if verification succeeds.
    async fn verify_package(
        &self,
        package_path: &Path,
        org_id: UniversalUuid,
    ) -> Result<PackageSignatureInfo, PackageSignError>;

    /// Verify a package against a detached signature file.
    fn verify_package_with_detached_signature(
        &self,
        package_path: &Path,
        signature: &DetachedSignature,
        public_key: &[u8],
    ) -> Result<(), PackageSignError>;
}

/// Database-backed package signer implementation.
#[derive(Clone)]
pub struct DbPackageSigner {
    dal: DAL,
}

impl DbPackageSigner {
    /// Create a new database-backed package signer.
    pub fn new(dal: DAL) -> Self {
        Self { dal }
    }

    /// Compute the SHA256 hash of a file.
    fn compute_file_hash(path: &Path) -> Result<String, PackageSignError> {
        let data = std::fs::read(path)?;
        Self::compute_data_hash(&data)
    }

    /// Compute the SHA256 hash of data.
    fn compute_data_hash(data: &[u8]) -> Result<String, PackageSignError> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        Ok(hex::encode(hasher.finalize()))
    }

    /// Convert database model to SignatureInfo.
    fn to_signature_info(sig: UnifiedPackageSignature) -> PackageSignatureInfo {
        PackageSignatureInfo {
            package_hash: sig.package_hash,
            key_fingerprint: sig.key_fingerprint,
            signature: sig.signature.into_inner(),
            signed_at: sig.signed_at,
        }
    }
}

#[async_trait]
impl PackageSigner for DbPackageSigner {
    async fn sign_package_with_db_key(
        &self,
        package_path: &Path,
        key_id: UniversalUuid,
        master_key: &[u8],
        store_signature: bool,
    ) -> Result<PackageSignatureInfo, PackageSignError> {
        use super::db_key_manager::DbKeyManager;
        use super::key_manager::KeyManager;

        let path_str = package_path.display().to_string();

        // Get the signing key from the database
        let key_manager = DbKeyManager::new(self.dal.clone());
        let (public_key, private_key) = key_manager
            .get_signing_key(key_id, master_key)
            .await
            .map_err(|e| {
                audit::log_package_sign_failed(&path_str, &e.to_string());
                match e {
                    super::key_manager::KeyError::NotFound(_) => {
                        PackageSignError::KeyNotFound(key_id)
                    }
                    super::key_manager::KeyError::Revoked(_) => PackageSignError::KeyRevoked,
                    e => PackageSignError::SigningFailed(e.to_string()),
                }
            })?;

        // Sign the package
        let signature = self
            .sign_package_with_raw_key(package_path, &private_key, &public_key)
            .map_err(|e| {
                audit::log_package_sign_failed(&path_str, &e.to_string());
                e
            })?;

        // Audit log: package signed successfully
        audit::log_package_signed(
            &path_str,
            &signature.package_hash,
            &signature.key_fingerprint,
        );

        // Optionally store in database
        if store_signature {
            self.store_signature(&signature).await?;
        }

        Ok(signature)
    }

    fn sign_package_with_raw_key(
        &self,
        package_path: &Path,
        private_key: &[u8],
        public_key: &[u8],
    ) -> Result<PackageSignatureInfo, PackageSignError> {
        let package_data = std::fs::read(package_path)?;
        self.sign_package_data(&package_data, private_key, public_key)
    }

    fn sign_package_data(
        &self,
        package_data: &[u8],
        private_key: &[u8],
        public_key: &[u8],
    ) -> Result<PackageSignatureInfo, PackageSignError> {
        // Compute package hash
        let package_hash = Self::compute_data_hash(package_data)?;

        // Sign the hash
        let hash_bytes = hex::decode(&package_hash)
            .map_err(|e| PackageSignError::SigningFailed(e.to_string()))?;

        let signature = crypto_sign(&hash_bytes, private_key)
            .map_err(|e| PackageSignError::SigningFailed(e.to_string()))?;

        // Compute key fingerprint
        let fingerprint = compute_key_fingerprint(public_key);

        Ok(PackageSignatureInfo {
            package_hash,
            key_fingerprint: fingerprint,
            signature,
            signed_at: UniversalTimestamp::now(),
        })
    }

    async fn store_signature(
        &self,
        signature: &PackageSignatureInfo,
    ) -> Result<UniversalUuid, PackageSignError> {
        let id = UniversalUuid::new_v4();

        let new_sig = NewUnifiedPackageSignature {
            id,
            package_hash: signature.package_hash.clone(),
            key_fingerprint: signature.key_fingerprint.clone(),
            signature: UniversalBinary::new(signature.signature.clone()),
            signed_at: signature.signed_at,
        };

        #[cfg(all(feature = "postgres", feature = "sqlite"))]
        {
            match self.dal.backend() {
                crate::database::BackendType::Postgres => {
                    self.store_signature_postgres(new_sig).await?
                }
                crate::database::BackendType::Sqlite => {
                    self.store_signature_sqlite(new_sig).await?
                }
            }
        }
        #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
        {
            self.store_signature_postgres(new_sig).await?
        }
        #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
        {
            self.store_signature_sqlite(new_sig).await?
        }

        Ok(id)
    }

    async fn find_signature(
        &self,
        package_hash: &str,
    ) -> Result<Option<PackageSignatureInfo>, PackageSignError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.find_signature_postgres(package_hash).await,
            self.find_signature_sqlite(package_hash).await
        )
    }

    async fn find_signatures(
        &self,
        package_hash: &str,
    ) -> Result<Vec<PackageSignatureInfo>, PackageSignError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.find_signatures_postgres(package_hash).await,
            self.find_signatures_sqlite(package_hash).await
        )
    }

    async fn verify_package(
        &self,
        package_path: &Path,
        org_id: UniversalUuid,
    ) -> Result<PackageSignatureInfo, PackageSignError> {
        use super::db_key_manager::DbKeyManager;
        use super::key_manager::KeyManager;

        // Compute package hash
        let package_hash = Self::compute_file_hash(package_path)?;

        // Find signatures for this package
        let signatures = self.find_signatures(&package_hash).await?;
        if signatures.is_empty() {
            return Err(PackageSignError::SignatureNotFound(package_hash));
        }

        // Check if any signature is from a trusted key
        let key_manager = DbKeyManager::new(self.dal.clone());
        let hash_bytes = hex::decode(&package_hash)
            .map_err(|e| PackageSignError::VerificationFailed(e.to_string()))?;

        for sig in signatures {
            // Check if this key is trusted
            if let Ok(Some(trusted_key)) = key_manager
                .find_trusted_key(org_id, &sig.key_fingerprint)
                .await
            {
                // Verify the signature
                if verify_signature(&hash_bytes, &sig.signature, &trusted_key.public_key).is_ok() {
                    return Ok(sig);
                }
            }
        }

        Err(PackageSignError::VerificationFailed(
            "No valid signature from a trusted key".to_string(),
        ))
    }

    fn verify_package_with_detached_signature(
        &self,
        package_path: &Path,
        signature: &DetachedSignature,
        public_key: &[u8],
    ) -> Result<(), PackageSignError> {
        // Verify algorithm
        if signature.algorithm != DetachedSignature::ALGORITHM {
            return Err(PackageSignError::InvalidSignatureFile(format!(
                "Unsupported algorithm: {}",
                signature.algorithm
            )));
        }

        // Compute package hash
        let package_hash = Self::compute_file_hash(package_path)?;

        // Verify hash matches
        if package_hash != signature.package_hash {
            return Err(PackageSignError::VerificationFailed(
                "Package hash does not match signature".to_string(),
            ));
        }

        // Verify key fingerprint matches
        let expected_fingerprint = compute_key_fingerprint(public_key);
        if expected_fingerprint != signature.key_fingerprint {
            return Err(PackageSignError::VerificationFailed(
                "Key fingerprint does not match signature".to_string(),
            ));
        }

        // Verify signature
        let hash_bytes = hex::decode(&package_hash)
            .map_err(|e| PackageSignError::VerificationFailed(e.to_string()))?;
        let sig_bytes = signature.signature_bytes()?;

        verify_signature(&hash_bytes, &sig_bytes, public_key)
            .map_err(|_| PackageSignError::VerificationFailed("Invalid signature".to_string()))?;

        Ok(())
    }
}

// PostgreSQL implementation
#[cfg(feature = "postgres")]
impl DbPackageSigner {
    async fn store_signature_postgres(
        &self,
        new_sig: NewUnifiedPackageSignature,
    ) -> Result<(), PackageSignError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| PackageSignError::Database(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::insert_into(package_signatures::table)
                .values(&new_sig)
                .execute(conn)
        })
        .await
        .map_err(|e| PackageSignError::Database(e.to_string()))?
        .map_err(|e| PackageSignError::Database(e.to_string()))?;

        Ok(())
    }

    async fn find_signature_postgres(
        &self,
        package_hash: &str,
    ) -> Result<Option<PackageSignatureInfo>, PackageSignError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| PackageSignError::Database(e.to_string()))?;

        let hash = package_hash.to_string();

        let sig: Option<UnifiedPackageSignature> = conn
            .interact(move |conn| {
                package_signatures::table
                    .filter(package_signatures::package_hash.eq(&hash))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| PackageSignError::Database(e.to_string()))?
            .map_err(|e| PackageSignError::Database(e.to_string()))?;

        Ok(sig.map(Self::to_signature_info))
    }

    async fn find_signatures_postgres(
        &self,
        package_hash: &str,
    ) -> Result<Vec<PackageSignatureInfo>, PackageSignError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| PackageSignError::Database(e.to_string()))?;

        let hash = package_hash.to_string();

        let sigs: Vec<UnifiedPackageSignature> = conn
            .interact(move |conn| {
                package_signatures::table
                    .filter(package_signatures::package_hash.eq(&hash))
                    .load(conn)
            })
            .await
            .map_err(|e| PackageSignError::Database(e.to_string()))?
            .map_err(|e| PackageSignError::Database(e.to_string()))?;

        Ok(sigs.into_iter().map(Self::to_signature_info).collect())
    }
}

// SQLite implementation
#[cfg(feature = "sqlite")]
impl DbPackageSigner {
    async fn store_signature_sqlite(
        &self,
        new_sig: NewUnifiedPackageSignature,
    ) -> Result<(), PackageSignError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| PackageSignError::Database(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::insert_into(package_signatures::table)
                .values(&new_sig)
                .execute(conn)
        })
        .await
        .map_err(|e| PackageSignError::Database(e.to_string()))?
        .map_err(|e| PackageSignError::Database(e.to_string()))?;

        Ok(())
    }

    async fn find_signature_sqlite(
        &self,
        package_hash: &str,
    ) -> Result<Option<PackageSignatureInfo>, PackageSignError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| PackageSignError::Database(e.to_string()))?;

        let hash = package_hash.to_string();

        let sig: Option<UnifiedPackageSignature> = conn
            .interact(move |conn| {
                package_signatures::table
                    .filter(package_signatures::package_hash.eq(&hash))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| PackageSignError::Database(e.to_string()))?
            .map_err(|e| PackageSignError::Database(e.to_string()))?;

        Ok(sig.map(Self::to_signature_info))
    }

    async fn find_signatures_sqlite(
        &self,
        package_hash: &str,
    ) -> Result<Vec<PackageSignatureInfo>, PackageSignError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| PackageSignError::Database(e.to_string()))?;

        let hash = package_hash.to_string();

        let sigs: Vec<UnifiedPackageSignature> = conn
            .interact(move |conn| {
                package_signatures::table
                    .filter(package_signatures::package_hash.eq(&hash))
                    .load(conn)
            })
            .await
            .map_err(|e| PackageSignError::Database(e.to_string()))?
            .map_err(|e| PackageSignError::Database(e.to_string()))?;

        Ok(sigs.into_iter().map(Self::to_signature_info).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::generate_signing_keypair;
    use tempfile::NamedTempFile;

    #[test]
    fn test_sign_and_verify_with_raw_key() {
        // Create a temporary file to sign
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"test package content").unwrap();

        // Generate a keypair
        let keypair = generate_signing_keypair();

        // Create a signer (we can't use DbPackageSigner without a database)
        // but we can test the raw key signing functions
        let package_data = std::fs::read(temp_file.path()).unwrap();
        let package_hash = DbPackageSigner::compute_data_hash(&package_data).unwrap();

        // Sign
        let hash_bytes = hex::decode(&package_hash).unwrap();
        let signature = crate::crypto::sign_package(&hash_bytes, &keypair.private_key).unwrap();

        // Verify
        let result = crate::crypto::verify_signature(&hash_bytes, &signature, &keypair.public_key);
        assert!(result.is_ok());
    }

    #[test]
    fn test_detached_signature_roundtrip() {
        let info = PackageSignatureInfo {
            package_hash: "abc123".to_string(),
            key_fingerprint: "def456".to_string(),
            signature: vec![0u8; 64],
            signed_at: UniversalTimestamp::now(),
        };

        let detached = DetachedSignature::from_signature_info(&info);

        // Roundtrip through JSON
        let json = detached.to_json().unwrap();
        let parsed = DetachedSignature::from_json(&json).unwrap();

        assert_eq!(parsed.version, DetachedSignature::VERSION);
        assert_eq!(parsed.algorithm, DetachedSignature::ALGORITHM);
        assert_eq!(parsed.package_hash, info.package_hash);
        assert_eq!(parsed.key_fingerprint, info.key_fingerprint);
        assert_eq!(parsed.signature_bytes().unwrap(), info.signature);
    }

    #[test]
    fn test_detached_signature_file_io() {
        let info = PackageSignatureInfo {
            package_hash: "abc123".to_string(),
            key_fingerprint: "def456".to_string(),
            signature: vec![0u8; 64],
            signed_at: UniversalTimestamp::now(),
        };

        let detached = DetachedSignature::from_signature_info(&info);

        // Write to file
        let temp_file = NamedTempFile::new().unwrap();
        detached.write_to_file(temp_file.path()).unwrap();

        // Read back
        let loaded = DetachedSignature::read_from_file(temp_file.path()).unwrap();
        assert_eq!(loaded.package_hash, info.package_hash);
    }
}
