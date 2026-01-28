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

//! Key manager trait and associated types.
//!
//! The [`KeyManager`] trait defines the interface for managing Ed25519 signing keys,
//! trusted public keys, and trust relationships between organizations.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use async_trait::async_trait;
use thiserror::Error;

/// Errors that can occur during key management operations.
#[derive(Debug, Error)]
pub enum KeyError {
    #[error("Key not found: {0}")]
    NotFound(UniversalUuid),

    #[error("Key has been revoked: {0}")]
    Revoked(UniversalUuid),

    #[error("Key name already exists for this organization: {0}")]
    DuplicateName(String),

    #[error("Invalid key format: {0}")]
    InvalidFormat(String),

    #[error("Invalid PEM format: {0}")]
    InvalidPem(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("Trust relationship already exists")]
    TrustAlreadyExists,

    #[error("Trust relationship not found")]
    TrustNotFound,

    #[error("Database error: {0}")]
    Database(String),
}

/// Information about a signing key (excludes private key material).
#[derive(Debug, Clone)]
pub struct SigningKeyInfo {
    pub id: UniversalUuid,
    pub org_id: UniversalUuid,
    pub key_name: String,
    /// SHA256 hex fingerprint of the public key
    pub fingerprint: String,
    /// 32-byte Ed25519 public key
    pub public_key: Vec<u8>,
    pub created_at: UniversalTimestamp,
    pub revoked_at: Option<UniversalTimestamp>,
}

impl SigningKeyInfo {
    /// Check if this key is currently active (not revoked).
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }
}

/// Information about a trusted public key for verification.
#[derive(Debug, Clone)]
pub struct TrustedKeyInfo {
    pub id: UniversalUuid,
    pub org_id: UniversalUuid,
    /// SHA256 hex fingerprint of the public key
    pub fingerprint: String,
    /// 32-byte Ed25519 public key
    pub public_key: Vec<u8>,
    /// Optional human-readable name
    pub key_name: Option<String>,
    pub trusted_at: UniversalTimestamp,
    pub revoked_at: Option<UniversalTimestamp>,
}

impl TrustedKeyInfo {
    /// Check if this key is currently trusted (not revoked).
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }
}

/// Public key export in multiple formats.
#[derive(Debug, Clone)]
pub struct PublicKeyExport {
    /// SHA256 hex fingerprint of the public key
    pub fingerprint: String,
    /// PEM-encoded public key (Ed25519 SubjectPublicKeyInfo format)
    pub public_key_pem: String,
    /// Raw 32-byte Ed25519 public key
    pub public_key_raw: Vec<u8>,
}

/// Trait for managing signing keys, trusted keys, and trust relationships.
///
/// Implementations must be thread-safe (`Send + Sync`) and should NOT cache
/// results to ensure key revocations take effect immediately.
#[async_trait]
pub trait KeyManager: Send + Sync {
    /// Generate a new Ed25519 signing keypair and store it encrypted in the database.
    ///
    /// # Arguments
    ///
    /// * `org_id` - Organization that owns this key
    /// * `name` - Human-readable name for the key (must be unique per org)
    /// * `master_key` - 32-byte AES-256 key for encrypting the private key
    ///
    /// # Returns
    ///
    /// Information about the created signing key.
    async fn create_signing_key(
        &self,
        org_id: UniversalUuid,
        name: &str,
        master_key: &[u8],
    ) -> Result<SigningKeyInfo, KeyError>;

    /// Get information about a signing key (without the private key).
    async fn get_signing_key_info(&self, key_id: UniversalUuid)
        -> Result<SigningKeyInfo, KeyError>;

    /// Get the decrypted signing key for signing operations.
    ///
    /// # Arguments
    ///
    /// * `key_id` - The signing key ID
    /// * `master_key` - 32-byte AES-256 key for decrypting the private key
    ///
    /// # Returns
    ///
    /// Tuple of (public_key, private_key) as raw bytes.
    async fn get_signing_key(
        &self,
        key_id: UniversalUuid,
        master_key: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), KeyError>;

    /// Export a public key in multiple formats for distribution.
    async fn export_public_key(&self, key_id: UniversalUuid) -> Result<PublicKeyExport, KeyError>;

    /// Import an external public key to trust for verification.
    ///
    /// # Arguments
    ///
    /// * `org_id` - Organization that will trust this key
    /// * `public_key` - 32-byte Ed25519 public key
    /// * `name` - Optional human-readable name
    async fn trust_public_key(
        &self,
        org_id: UniversalUuid,
        public_key: &[u8],
        name: Option<&str>,
    ) -> Result<TrustedKeyInfo, KeyError>;

    /// Import a public key from PEM format and add it to trusted keys.
    async fn trust_public_key_pem(
        &self,
        org_id: UniversalUuid,
        pem: &str,
        name: Option<&str>,
    ) -> Result<TrustedKeyInfo, KeyError>;

    /// Revoke a signing key (prevents future signing operations).
    async fn revoke_signing_key(&self, key_id: UniversalUuid) -> Result<(), KeyError>;

    /// Revoke a trusted key (prevents future verification with this key).
    async fn revoke_trusted_key(&self, key_id: UniversalUuid) -> Result<(), KeyError>;

    /// Grant trust from a parent organization to a child organization.
    ///
    /// When trust is granted, the parent org implicitly trusts all keys
    /// that are trusted by the child org.
    async fn grant_trust(
        &self,
        parent_org: UniversalUuid,
        child_org: UniversalUuid,
    ) -> Result<(), KeyError>;

    /// Revoke trust grant between organizations.
    async fn revoke_trust(
        &self,
        parent_org: UniversalUuid,
        child_org: UniversalUuid,
    ) -> Result<(), KeyError>;

    /// List all signing keys for an organization.
    async fn list_signing_keys(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Vec<SigningKeyInfo>, KeyError>;

    /// List all trusted keys for an organization, including inherited keys via ACLs.
    async fn list_trusted_keys(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Vec<TrustedKeyInfo>, KeyError>;

    /// Find a trusted key by fingerprint for verification.
    ///
    /// Searches both directly trusted keys and inherited keys via ACLs.
    async fn find_trusted_key(
        &self,
        org_id: UniversalUuid,
        fingerprint: &str,
    ) -> Result<Option<TrustedKeyInfo>, KeyError>;
}
