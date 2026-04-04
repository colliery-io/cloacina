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

//! Database-backed key manager implementation.
//!
//! This module provides a [`KeyManager`] implementation that stores keys
//! in the database using Diesel. Private keys are encrypted at rest using
//! AES-256-GCM.

use super::audit;
use super::key_manager::{KeyError, KeyManager, PublicKeyExport, SigningKeyInfo, TrustedKeyInfo};
use crate::crypto::{
    compute_key_fingerprint, decrypt_private_key, encrypt_private_key, generate_signing_keypair,
};
use crate::dal::unified::models::{
    NewUnifiedKeyTrustAcl, NewUnifiedSigningKey, NewUnifiedTrustedKey, UnifiedKeyTrustAcl,
    UnifiedSigningKey, UnifiedTrustedKey,
};
use crate::dal::unified::DAL;
use crate::database::schema::unified::{key_trust_acls, signing_keys, trusted_keys};
use crate::database::universal_types::{UniversalBinary, UniversalTimestamp, UniversalUuid};
use async_trait::async_trait;
use diesel::prelude::*;

/// PEM tag for Ed25519 public keys.
const ED25519_PEM_TAG: &str = "PUBLIC KEY";

/// ASN.1 DER prefix for Ed25519 public keys (SubjectPublicKeyInfo).
/// This is the standard prefix for Ed25519 keys in PKIX format.
const ED25519_DER_PREFIX: [u8; 12] = [
    0x30, 0x2a, // SEQUENCE, 42 bytes
    0x30, 0x05, // SEQUENCE, 5 bytes (algorithm identifier)
    0x06, 0x03, // OID, 3 bytes
    0x2b, 0x65, 0x70, // 1.3.101.112 (Ed25519)
    0x03, 0x21, // BIT STRING, 33 bytes
    0x00, // unused bits
];

/// Database-backed implementation of the [`KeyManager`] trait.
///
/// This implementation:
/// - Stores signing keys with AES-256-GCM encrypted private keys
/// - Supports trust relationships between organizations via ACLs
/// - Does NOT cache any data to ensure immediate effect of revocations
#[derive(Clone)]
pub struct DbKeyManager {
    dal: DAL,
}

impl DbKeyManager {
    /// Creates a new database-backed key manager.
    pub fn new(dal: DAL) -> Self {
        Self { dal }
    }

    /// Encodes a raw Ed25519 public key to PEM format.
    fn encode_public_key_pem(public_key: &[u8]) -> String {
        // Create DER-encoded SubjectPublicKeyInfo
        let mut der = Vec::with_capacity(ED25519_DER_PREFIX.len() + public_key.len());
        der.extend_from_slice(&ED25519_DER_PREFIX);
        der.extend_from_slice(public_key);

        // Encode as PEM
        let pem = pem::Pem::new(ED25519_PEM_TAG, der);
        pem::encode(&pem)
    }

    /// Decodes a PEM-encoded Ed25519 public key to raw bytes.
    fn decode_public_key_pem(pem_str: &str) -> Result<Vec<u8>, KeyError> {
        let pem = pem::parse(pem_str).map_err(|e| KeyError::InvalidPem(e.to_string()))?;

        if pem.tag() != ED25519_PEM_TAG {
            return Err(KeyError::InvalidPem(format!(
                "Expected tag '{}', got '{}'",
                ED25519_PEM_TAG,
                pem.tag()
            )));
        }

        let der = pem.contents();

        // Verify the DER prefix
        if der.len() != ED25519_DER_PREFIX.len() + 32 {
            return Err(KeyError::InvalidPem(format!(
                "Invalid DER length: expected {}, got {}",
                ED25519_DER_PREFIX.len() + 32,
                der.len()
            )));
        }

        if der[..ED25519_DER_PREFIX.len()] != ED25519_DER_PREFIX {
            return Err(KeyError::InvalidPem(
                "Invalid DER prefix for Ed25519 key".to_string(),
            ));
        }

        // Extract the 32-byte public key
        Ok(der[ED25519_DER_PREFIX.len()..].to_vec())
    }

    /// Convert database model to SigningKeyInfo.
    fn to_signing_key_info(key: UnifiedSigningKey) -> SigningKeyInfo {
        SigningKeyInfo {
            id: key.id,
            org_id: key.org_id,
            key_name: key.key_name,
            fingerprint: key.key_fingerprint,
            public_key: key.public_key.into_inner(),
            created_at: key.created_at,
            revoked_at: key.revoked_at,
        }
    }

    /// Convert database model to TrustedKeyInfo.
    fn to_trusted_key_info(key: UnifiedTrustedKey) -> TrustedKeyInfo {
        TrustedKeyInfo {
            id: key.id,
            org_id: key.org_id,
            fingerprint: key.key_fingerprint,
            public_key: key.public_key.into_inner(),
            key_name: key.key_name,
            trusted_at: key.trusted_at,
            revoked_at: key.revoked_at,
        }
    }
}

#[async_trait]
impl KeyManager for DbKeyManager {
    async fn create_signing_key(
        &self,
        org_id: UniversalUuid,
        name: &str,
        master_key: &[u8],
    ) -> Result<SigningKeyInfo, KeyError> {
        // Generate new keypair
        let keypair = generate_signing_keypair();

        // Encrypt private key
        let encrypted_private_key = encrypt_private_key(&keypair.private_key, master_key)
            .map_err(|e| KeyError::Encryption(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();
        let key_name = name.to_string();
        let fingerprint = keypair.fingerprint.clone();
        let public_key_bytes = keypair.public_key.clone();

        let new_key = NewUnifiedSigningKey {
            id,
            org_id,
            key_name: key_name.clone(),
            encrypted_private_key: UniversalBinary::new(encrypted_private_key),
            public_key: UniversalBinary::new(public_key_bytes.clone()),
            key_fingerprint: fingerprint.clone(),
            created_at: now,
        };

        #[cfg(all(feature = "postgres", feature = "sqlite"))]
        {
            match self.dal.backend() {
                crate::database::BackendType::Postgres => {
                    self.create_signing_key_postgres(new_key).await?
                }
                crate::database::BackendType::Sqlite => {
                    self.create_signing_key_sqlite(new_key).await?
                }
            }
        }
        #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
        {
            self.create_signing_key_postgres(new_key).await?
        }
        #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
        {
            self.create_signing_key_sqlite(new_key).await?
        }

        // Audit log: signing key created
        audit::log_signing_key_created(org_id, id, &fingerprint, &key_name);

        Ok(SigningKeyInfo {
            id,
            org_id,
            key_name,
            fingerprint,
            public_key: public_key_bytes,
            created_at: now,
            revoked_at: None,
        })
    }

    async fn get_signing_key_info(
        &self,
        key_id: UniversalUuid,
    ) -> Result<SigningKeyInfo, KeyError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_signing_key_info_postgres(key_id).await,
            self.get_signing_key_info_sqlite(key_id).await
        )
    }

    async fn get_signing_key(
        &self,
        key_id: UniversalUuid,
        master_key: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), KeyError> {
        let key: UnifiedSigningKey = crate::dispatch_backend!(
            self.dal.backend(),
            self.get_signing_key_raw_postgres(key_id).await,
            self.get_signing_key_raw_sqlite(key_id).await
        )?;

        if key.revoked_at.is_some() {
            return Err(KeyError::Revoked(key_id));
        }

        // Decrypt private key
        let private_key = decrypt_private_key(&key.encrypted_private_key.into_inner(), master_key)
            .map_err(|e| KeyError::Decryption(e.to_string()))?;

        Ok((key.public_key.into_inner(), private_key))
    }

    async fn export_public_key(&self, key_id: UniversalUuid) -> Result<PublicKeyExport, KeyError> {
        let info = self.get_signing_key_info(key_id).await?;

        // Audit log: key exported
        audit::log_key_exported(key_id, &info.fingerprint);

        Ok(PublicKeyExport {
            fingerprint: info.fingerprint.clone(),
            public_key_pem: Self::encode_public_key_pem(&info.public_key),
            public_key_raw: info.public_key,
        })
    }

    async fn trust_public_key(
        &self,
        org_id: UniversalUuid,
        public_key: &[u8],
        name: Option<&str>,
    ) -> Result<TrustedKeyInfo, KeyError> {
        if public_key.len() != 32 {
            return Err(KeyError::InvalidFormat(format!(
                "Expected 32-byte Ed25519 public key, got {} bytes",
                public_key.len()
            )));
        }

        let fingerprint = compute_key_fingerprint(public_key);
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_key = NewUnifiedTrustedKey {
            id,
            org_id,
            key_fingerprint: fingerprint.clone(),
            public_key: UniversalBinary::new(public_key.to_vec()),
            key_name: name.map(|s| s.to_string()),
            trusted_at: now,
        };

        #[cfg(all(feature = "postgres", feature = "sqlite"))]
        {
            match self.dal.backend() {
                crate::database::BackendType::Postgres => {
                    self.create_trusted_key_postgres(new_key).await?
                }
                crate::database::BackendType::Sqlite => {
                    self.create_trusted_key_sqlite(new_key).await?
                }
            }
        }
        #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
        {
            self.create_trusted_key_postgres(new_key).await?
        }
        #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
        {
            self.create_trusted_key_sqlite(new_key).await?
        }

        // Audit log: trusted key added
        audit::log_trusted_key_added(org_id, id, &fingerprint, name);

        Ok(TrustedKeyInfo {
            id,
            org_id,
            fingerprint,
            public_key: public_key.to_vec(),
            key_name: name.map(|s| s.to_string()),
            trusted_at: now,
            revoked_at: None,
        })
    }

    async fn trust_public_key_pem(
        &self,
        org_id: UniversalUuid,
        pem: &str,
        name: Option<&str>,
    ) -> Result<TrustedKeyInfo, KeyError> {
        let public_key = Self::decode_public_key_pem(pem)?;
        self.trust_public_key(org_id, &public_key, name).await
    }

    async fn revoke_signing_key(&self, key_id: UniversalUuid) -> Result<(), KeyError> {
        // Get key info before revoking for audit logging
        let key_info = self.get_signing_key_info(key_id).await?;

        crate::dispatch_backend!(
            self.dal.backend(),
            self.revoke_signing_key_postgres(key_id).await,
            self.revoke_signing_key_sqlite(key_id).await
        )?;

        // Audit log: signing key revoked
        audit::log_signing_key_revoked(
            key_info.org_id,
            key_id,
            &key_info.fingerprint,
            Some(&key_info.key_name),
        );

        Ok(())
    }

    async fn revoke_trusted_key(&self, key_id: UniversalUuid) -> Result<(), KeyError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.revoke_trusted_key_postgres(key_id).await,
            self.revoke_trusted_key_sqlite(key_id).await
        )?;

        // Audit log: trusted key revoked
        audit::log_trusted_key_revoked(key_id);

        Ok(())
    }

    async fn grant_trust(
        &self,
        parent_org: UniversalUuid,
        child_org: UniversalUuid,
    ) -> Result<(), KeyError> {
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_acl = NewUnifiedKeyTrustAcl {
            id,
            parent_org_id: parent_org,
            child_org_id: child_org,
            granted_at: now,
        };

        #[cfg(all(feature = "postgres", feature = "sqlite"))]
        {
            match self.dal.backend() {
                crate::database::BackendType::Postgres => {
                    self.grant_trust_postgres(new_acl).await?
                }
                crate::database::BackendType::Sqlite => self.grant_trust_sqlite(new_acl).await?,
            }
        }
        #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
        {
            self.grant_trust_postgres(new_acl).await?
        }
        #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
        {
            self.grant_trust_sqlite(new_acl).await?
        }

        // Audit log: trust ACL granted
        audit::log_trust_acl_granted(parent_org, child_org);

        Ok(())
    }

    async fn revoke_trust(
        &self,
        parent_org: UniversalUuid,
        child_org: UniversalUuid,
    ) -> Result<(), KeyError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.revoke_trust_postgres(parent_org, child_org).await,
            self.revoke_trust_sqlite(parent_org, child_org).await
        )?;

        // Audit log: trust ACL revoked
        audit::log_trust_acl_revoked(parent_org, child_org);

        Ok(())
    }

    async fn list_signing_keys(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Vec<SigningKeyInfo>, KeyError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_signing_keys_postgres(org_id).await,
            self.list_signing_keys_sqlite(org_id).await
        )
    }

    async fn list_trusted_keys(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Vec<TrustedKeyInfo>, KeyError> {
        // Get directly trusted keys
        let direct_keys: Vec<TrustedKeyInfo> = crate::dispatch_backend!(
            self.dal.backend(),
            self.list_direct_trusted_keys_postgres(org_id).await,
            self.list_direct_trusted_keys_sqlite(org_id).await
        )?;

        // Get trusted child orgs via ACL
        let trusted_children: Vec<UniversalUuid> = crate::dispatch_backend!(
            self.dal.backend(),
            self.get_trusted_child_orgs_postgres(org_id).await,
            self.get_trusted_child_orgs_sqlite(org_id).await
        )?;

        // Get keys from trusted children
        let mut all_keys = direct_keys;
        for child_org in trusted_children {
            let child_keys: Vec<TrustedKeyInfo> = crate::dispatch_backend!(
                self.dal.backend(),
                self.list_direct_trusted_keys_postgres(child_org).await,
                self.list_direct_trusted_keys_sqlite(child_org).await
            )?;

            // Add keys that aren't already present (by fingerprint)
            for key in child_keys {
                if !all_keys.iter().any(|k| k.fingerprint == key.fingerprint) {
                    all_keys.push(key);
                }
            }
        }

        Ok(all_keys)
    }

    async fn find_trusted_key(
        &self,
        org_id: UniversalUuid,
        fingerprint: &str,
    ) -> Result<Option<TrustedKeyInfo>, KeyError> {
        // First check direct trusted keys
        let direct_key: Option<TrustedKeyInfo> = crate::dispatch_backend!(
            self.dal.backend(),
            self.find_direct_trusted_key_postgres(org_id, fingerprint)
                .await,
            self.find_direct_trusted_key_sqlite(org_id, fingerprint)
                .await
        )?;

        if direct_key.is_some() {
            return Ok(direct_key);
        }

        // Check trusted child orgs
        let trusted_children: Vec<UniversalUuid> = crate::dispatch_backend!(
            self.dal.backend(),
            self.get_trusted_child_orgs_postgres(org_id).await,
            self.get_trusted_child_orgs_sqlite(org_id).await
        )?;

        for child_org in trusted_children {
            let child_key: Option<TrustedKeyInfo> = crate::dispatch_backend!(
                self.dal.backend(),
                self.find_direct_trusted_key_postgres(child_org, fingerprint)
                    .await,
                self.find_direct_trusted_key_sqlite(child_org, fingerprint)
                    .await
            )?;

            if child_key.is_some() {
                return Ok(child_key);
            }
        }

        Ok(None)
    }
}

// PostgreSQL implementation
#[cfg(feature = "postgres")]
impl DbKeyManager {
    async fn create_signing_key_postgres(
        &self,
        new_key: NewUnifiedSigningKey,
    ) -> Result<(), KeyError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let key_name = new_key.key_name.clone();

        conn.interact(move |conn| {
            diesel::insert_into(signing_keys::table)
                .values(&new_key)
                .execute(conn)
        })
        .await
        .map_err(|e| KeyError::Database(e.to_string()))?
        .map_err(|e| {
            if e.to_string().contains("duplicate") || e.to_string().contains("UNIQUE") {
                KeyError::DuplicateName(key_name.clone())
            } else {
                KeyError::Database(e.to_string())
            }
        })?;

        Ok(())
    }

    async fn get_signing_key_info_postgres(
        &self,
        key_id: UniversalUuid,
    ) -> Result<SigningKeyInfo, KeyError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let key: UnifiedSigningKey = conn
            .interact(move |conn| {
                signing_keys::table
                    .filter(signing_keys::id.eq(key_id))
                    .first(conn)
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| {
                if matches!(e, diesel::result::Error::NotFound) {
                    KeyError::NotFound(key_id)
                } else {
                    KeyError::Database(e.to_string())
                }
            })?;

        Ok(Self::to_signing_key_info(key))
    }

    async fn get_signing_key_raw_postgres(
        &self,
        key_id: UniversalUuid,
    ) -> Result<UnifiedSigningKey, KeyError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        conn.interact(move |conn| {
            signing_keys::table
                .filter(signing_keys::id.eq(key_id))
                .first(conn)
        })
        .await
        .map_err(|e| KeyError::Database(e.to_string()))?
        .map_err(|e| {
            if matches!(e, diesel::result::Error::NotFound) {
                KeyError::NotFound(key_id)
            } else {
                KeyError::Database(e.to_string())
            }
        })
    }

    async fn create_trusted_key_postgres(
        &self,
        new_key: NewUnifiedTrustedKey,
    ) -> Result<(), KeyError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::insert_into(trusted_keys::table)
                .values(&new_key)
                .execute(conn)
        })
        .await
        .map_err(|e| KeyError::Database(e.to_string()))?
        .map_err(|e| KeyError::Database(e.to_string()))?;

        Ok(())
    }

    async fn revoke_signing_key_postgres(&self, key_id: UniversalUuid) -> Result<(), KeyError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let now = UniversalTimestamp::now();

        let updated = conn
            .interact(move |conn| {
                diesel::update(signing_keys::table.filter(signing_keys::id.eq(key_id)))
                    .set(signing_keys::revoked_at.eq(Some(now)))
                    .execute(conn)
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| KeyError::Database(e.to_string()))?;

        if updated == 0 {
            return Err(KeyError::NotFound(key_id));
        }

        Ok(())
    }

    async fn revoke_trusted_key_postgres(&self, key_id: UniversalUuid) -> Result<(), KeyError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let now = UniversalTimestamp::now();

        let updated = conn
            .interact(move |conn| {
                diesel::update(trusted_keys::table.filter(trusted_keys::id.eq(key_id)))
                    .set(trusted_keys::revoked_at.eq(Some(now)))
                    .execute(conn)
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| KeyError::Database(e.to_string()))?;

        if updated == 0 {
            return Err(KeyError::NotFound(key_id));
        }

        Ok(())
    }

    async fn grant_trust_postgres(&self, new_acl: NewUnifiedKeyTrustAcl) -> Result<(), KeyError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::insert_into(key_trust_acls::table)
                .values(&new_acl)
                .execute(conn)
        })
        .await
        .map_err(|e| KeyError::Database(e.to_string()))?
        .map_err(|e| {
            if e.to_string().contains("duplicate") || e.to_string().contains("UNIQUE") {
                KeyError::TrustAlreadyExists
            } else {
                KeyError::Database(e.to_string())
            }
        })?;

        Ok(())
    }

    async fn revoke_trust_postgres(
        &self,
        parent_org: UniversalUuid,
        child_org: UniversalUuid,
    ) -> Result<(), KeyError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let now = UniversalTimestamp::now();

        let updated = conn
            .interact(move |conn| {
                diesel::update(
                    key_trust_acls::table
                        .filter(key_trust_acls::parent_org_id.eq(parent_org))
                        .filter(key_trust_acls::child_org_id.eq(child_org))
                        .filter(key_trust_acls::revoked_at.is_null()),
                )
                .set(key_trust_acls::revoked_at.eq(Some(now)))
                .execute(conn)
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| KeyError::Database(e.to_string()))?;

        if updated == 0 {
            return Err(KeyError::TrustNotFound);
        }

        Ok(())
    }

    async fn list_signing_keys_postgres(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Vec<SigningKeyInfo>, KeyError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let keys: Vec<UnifiedSigningKey> = conn
            .interact(move |conn| {
                signing_keys::table
                    .filter(signing_keys::org_id.eq(org_id))
                    .load(conn)
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| KeyError::Database(e.to_string()))?;

        Ok(keys.into_iter().map(Self::to_signing_key_info).collect())
    }

    async fn list_direct_trusted_keys_postgres(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Vec<TrustedKeyInfo>, KeyError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let keys: Vec<UnifiedTrustedKey> = conn
            .interact(move |conn| {
                trusted_keys::table
                    .filter(trusted_keys::org_id.eq(org_id))
                    .filter(trusted_keys::revoked_at.is_null())
                    .load(conn)
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| KeyError::Database(e.to_string()))?;

        Ok(keys.into_iter().map(Self::to_trusted_key_info).collect())
    }

    async fn get_trusted_child_orgs_postgres(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Vec<UniversalUuid>, KeyError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let acls: Vec<UnifiedKeyTrustAcl> = conn
            .interact(move |conn| {
                key_trust_acls::table
                    .filter(key_trust_acls::parent_org_id.eq(org_id))
                    .filter(key_trust_acls::revoked_at.is_null())
                    .load(conn)
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| KeyError::Database(e.to_string()))?;

        Ok(acls.into_iter().map(|acl| acl.child_org_id).collect())
    }

    async fn find_direct_trusted_key_postgres(
        &self,
        org_id: UniversalUuid,
        fingerprint: &str,
    ) -> Result<Option<TrustedKeyInfo>, KeyError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let fingerprint = fingerprint.to_string();

        let key: Option<UnifiedTrustedKey> = conn
            .interact(move |conn| {
                trusted_keys::table
                    .filter(trusted_keys::org_id.eq(org_id))
                    .filter(trusted_keys::key_fingerprint.eq(&fingerprint))
                    .filter(trusted_keys::revoked_at.is_null())
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| KeyError::Database(e.to_string()))?;

        Ok(key.map(Self::to_trusted_key_info))
    }
}

// SQLite implementation
#[cfg(feature = "sqlite")]
impl DbKeyManager {
    async fn create_signing_key_sqlite(
        &self,
        new_key: NewUnifiedSigningKey,
    ) -> Result<(), KeyError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let key_name = new_key.key_name.clone();

        conn.interact(move |conn| {
            diesel::insert_into(signing_keys::table)
                .values(&new_key)
                .execute(conn)
        })
        .await
        .map_err(|e| KeyError::Database(e.to_string()))?
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                KeyError::DuplicateName(key_name.clone())
            } else {
                KeyError::Database(e.to_string())
            }
        })?;

        Ok(())
    }

    async fn get_signing_key_info_sqlite(
        &self,
        key_id: UniversalUuid,
    ) -> Result<SigningKeyInfo, KeyError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let key: UnifiedSigningKey = conn
            .interact(move |conn| {
                signing_keys::table
                    .filter(signing_keys::id.eq(key_id))
                    .first(conn)
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| {
                if matches!(e, diesel::result::Error::NotFound) {
                    KeyError::NotFound(key_id)
                } else {
                    KeyError::Database(e.to_string())
                }
            })?;

        Ok(Self::to_signing_key_info(key))
    }

    async fn get_signing_key_raw_sqlite(
        &self,
        key_id: UniversalUuid,
    ) -> Result<UnifiedSigningKey, KeyError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        conn.interact(move |conn| {
            signing_keys::table
                .filter(signing_keys::id.eq(key_id))
                .first(conn)
        })
        .await
        .map_err(|e| KeyError::Database(e.to_string()))?
        .map_err(|e| {
            if matches!(e, diesel::result::Error::NotFound) {
                KeyError::NotFound(key_id)
            } else {
                KeyError::Database(e.to_string())
            }
        })
    }

    async fn create_trusted_key_sqlite(
        &self,
        new_key: NewUnifiedTrustedKey,
    ) -> Result<(), KeyError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::insert_into(trusted_keys::table)
                .values(&new_key)
                .execute(conn)
        })
        .await
        .map_err(|e| KeyError::Database(e.to_string()))?
        .map_err(|e| KeyError::Database(e.to_string()))?;

        Ok(())
    }

    async fn revoke_signing_key_sqlite(&self, key_id: UniversalUuid) -> Result<(), KeyError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let now = UniversalTimestamp::now();

        let updated = conn
            .interact(move |conn| {
                diesel::update(signing_keys::table.filter(signing_keys::id.eq(key_id)))
                    .set(signing_keys::revoked_at.eq(Some(now)))
                    .execute(conn)
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| KeyError::Database(e.to_string()))?;

        if updated == 0 {
            return Err(KeyError::NotFound(key_id));
        }

        Ok(())
    }

    async fn revoke_trusted_key_sqlite(&self, key_id: UniversalUuid) -> Result<(), KeyError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let now = UniversalTimestamp::now();

        let updated = conn
            .interact(move |conn| {
                diesel::update(trusted_keys::table.filter(trusted_keys::id.eq(key_id)))
                    .set(trusted_keys::revoked_at.eq(Some(now)))
                    .execute(conn)
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| KeyError::Database(e.to_string()))?;

        if updated == 0 {
            return Err(KeyError::NotFound(key_id));
        }

        Ok(())
    }

    async fn grant_trust_sqlite(&self, new_acl: NewUnifiedKeyTrustAcl) -> Result<(), KeyError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::insert_into(key_trust_acls::table)
                .values(&new_acl)
                .execute(conn)
        })
        .await
        .map_err(|e| KeyError::Database(e.to_string()))?
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                KeyError::TrustAlreadyExists
            } else {
                KeyError::Database(e.to_string())
            }
        })?;

        Ok(())
    }

    async fn revoke_trust_sqlite(
        &self,
        parent_org: UniversalUuid,
        child_org: UniversalUuid,
    ) -> Result<(), KeyError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let now = UniversalTimestamp::now();

        let updated = conn
            .interact(move |conn| {
                diesel::update(
                    key_trust_acls::table
                        .filter(key_trust_acls::parent_org_id.eq(parent_org))
                        .filter(key_trust_acls::child_org_id.eq(child_org))
                        .filter(key_trust_acls::revoked_at.is_null()),
                )
                .set(key_trust_acls::revoked_at.eq(Some(now)))
                .execute(conn)
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| KeyError::Database(e.to_string()))?;

        if updated == 0 {
            return Err(KeyError::TrustNotFound);
        }

        Ok(())
    }

    async fn list_signing_keys_sqlite(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Vec<SigningKeyInfo>, KeyError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let keys: Vec<UnifiedSigningKey> = conn
            .interact(move |conn| {
                signing_keys::table
                    .filter(signing_keys::org_id.eq(org_id))
                    .load(conn)
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| KeyError::Database(e.to_string()))?;

        Ok(keys.into_iter().map(Self::to_signing_key_info).collect())
    }

    async fn list_direct_trusted_keys_sqlite(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Vec<TrustedKeyInfo>, KeyError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let keys: Vec<UnifiedTrustedKey> = conn
            .interact(move |conn| {
                trusted_keys::table
                    .filter(trusted_keys::org_id.eq(org_id))
                    .filter(trusted_keys::revoked_at.is_null())
                    .load(conn)
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| KeyError::Database(e.to_string()))?;

        Ok(keys.into_iter().map(Self::to_trusted_key_info).collect())
    }

    async fn get_trusted_child_orgs_sqlite(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Vec<UniversalUuid>, KeyError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let acls: Vec<UnifiedKeyTrustAcl> = conn
            .interact(move |conn| {
                key_trust_acls::table
                    .filter(key_trust_acls::parent_org_id.eq(org_id))
                    .filter(key_trust_acls::revoked_at.is_null())
                    .load(conn)
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| KeyError::Database(e.to_string()))?;

        Ok(acls.into_iter().map(|acl| acl.child_org_id).collect())
    }

    async fn find_direct_trusted_key_sqlite(
        &self,
        org_id: UniversalUuid,
        fingerprint: &str,
    ) -> Result<Option<TrustedKeyInfo>, KeyError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?;

        let fingerprint = fingerprint.to_string();

        let key: Option<UnifiedTrustedKey> = conn
            .interact(move |conn| {
                trusted_keys::table
                    .filter(trusted_keys::org_id.eq(org_id))
                    .filter(trusted_keys::key_fingerprint.eq(&fingerprint))
                    .filter(trusted_keys::revoked_at.is_null())
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| KeyError::Database(e.to_string()))?
            .map_err(|e| KeyError::Database(e.to_string()))?;

        Ok(key.map(Self::to_trusted_key_info))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    // ── Pure-function unit tests ──────────────────────────────────────

    #[test]
    fn test_pem_roundtrip() {
        let public_key = [0x42u8; 32];

        let pem = DbKeyManager::encode_public_key_pem(&public_key);
        assert!(pem.contains("-----BEGIN PUBLIC KEY-----"));
        assert!(pem.contains("-----END PUBLIC KEY-----"));

        let decoded = DbKeyManager::decode_public_key_pem(&pem).unwrap();
        assert_eq!(decoded, public_key);
    }

    #[test]
    fn test_pem_roundtrip_all_zeros() {
        let public_key = [0u8; 32];
        let pem = DbKeyManager::encode_public_key_pem(&public_key);
        let decoded = DbKeyManager::decode_public_key_pem(&pem).unwrap();
        assert_eq!(decoded, public_key);
    }

    #[test]
    fn test_pem_roundtrip_all_ones() {
        let public_key = [0xFFu8; 32];
        let pem = DbKeyManager::encode_public_key_pem(&public_key);
        let decoded = DbKeyManager::decode_public_key_pem(&pem).unwrap();
        assert_eq!(decoded, public_key);
    }

    #[test]
    fn test_pem_roundtrip_random_key() {
        let keypair = crate::crypto::generate_signing_keypair();
        let pem = DbKeyManager::encode_public_key_pem(&keypair.public_key);
        let decoded = DbKeyManager::decode_public_key_pem(&pem).unwrap();
        assert_eq!(decoded, keypair.public_key);
    }

    #[test]
    fn test_invalid_pem() {
        let result = DbKeyManager::decode_public_key_pem("not a pem");
        assert!(result.is_err());

        let result = DbKeyManager::decode_public_key_pem(
            "-----BEGIN PRIVATE KEY-----\nYWJj\n-----END PRIVATE KEY-----",
        );
        assert!(matches!(result, Err(KeyError::InvalidPem(_))));
    }

    #[test]
    fn test_decode_pem_wrong_length() {
        // Valid PEM structure but wrong key size (16 bytes instead of 32)
        let short_key = [0x42u8; 16];
        let mut der = Vec::with_capacity(ED25519_DER_PREFIX.len() + short_key.len());
        der.extend_from_slice(&ED25519_DER_PREFIX);
        der.extend_from_slice(&short_key);
        let pem = pem::Pem::new(ED25519_PEM_TAG, der);
        let pem_str = pem::encode(&pem);
        let result = DbKeyManager::decode_public_key_pem(&pem_str);
        assert!(matches!(result, Err(KeyError::InvalidPem(_))));
    }

    #[test]
    fn test_decode_pem_wrong_der_prefix() {
        // Correct length but wrong DER prefix
        let mut der = vec![0x00u8; ED25519_DER_PREFIX.len() + 32];
        // Put garbage in prefix area
        for b in der[..ED25519_DER_PREFIX.len()].iter_mut() {
            *b = 0xFF;
        }
        let pem = pem::Pem::new(ED25519_PEM_TAG, der);
        let pem_str = pem::encode(&pem);
        let result = DbKeyManager::decode_public_key_pem(&pem_str);
        assert!(matches!(result, Err(KeyError::InvalidPem(_))));
    }

    #[test]
    fn test_encode_pem_contains_expected_header_footer() {
        let pem = DbKeyManager::encode_public_key_pem(&[0u8; 32]);
        assert!(pem.starts_with("-----BEGIN PUBLIC KEY-----"));
        assert!(pem.trim_end().ends_with("-----END PUBLIC KEY-----"));
    }

    // ── Helper to build a fresh DAL per test ─────────────────────────

    #[cfg(feature = "sqlite")]
    async fn unique_dal() -> DAL {
        let url = format!(
            "sqlite:///tmp/dbkm_test_{}.db?mode=rwc",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    #[cfg(feature = "sqlite")]
    fn master_key() -> [u8; 32] {
        [0u8; 32]
    }

    // ── Database-backed KeyManager tests ─────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_create_signing_key() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let info = km
            .create_signing_key(org_id, "test-key-1", &master_key())
            .await
            .unwrap();

        assert_eq!(info.org_id, org_id);
        assert_eq!(info.key_name, "test-key-1");
        assert!(info.is_active());
        assert_eq!(info.public_key.len(), 32);
        assert!(!info.fingerprint.is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_signing_key_info() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let created = km
            .create_signing_key(org_id, "lookup-key", &master_key())
            .await
            .unwrap();

        let fetched = km.get_signing_key_info(created.id).await.unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.key_name, "lookup-key");
        assert_eq!(fetched.fingerprint, created.fingerprint);
        assert_eq!(fetched.public_key, created.public_key);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_signing_key_info_not_found() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);

        let result = km.get_signing_key_info(UniversalUuid::new_v4()).await;
        assert!(matches!(result, Err(KeyError::NotFound(_))));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_signing_key_decrypt() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let info = km
            .create_signing_key(org_id, "decrypt-key", &master_key())
            .await
            .unwrap();

        let (pub_key, priv_key) = km.get_signing_key(info.id, &master_key()).await.unwrap();
        assert_eq!(pub_key, info.public_key);
        assert_eq!(priv_key.len(), 32);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_signing_key_wrong_master_key() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let info = km
            .create_signing_key(org_id, "wrong-mk-key", &master_key())
            .await
            .unwrap();

        let wrong_key = [1u8; 32];
        let result = km.get_signing_key(info.id, &wrong_key).await;
        assert!(matches!(result, Err(KeyError::Decryption(_))));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_signing_key_revoked_fails() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let info = km
            .create_signing_key(org_id, "revoke-me", &master_key())
            .await
            .unwrap();

        km.revoke_signing_key(info.id).await.unwrap();

        let result = km.get_signing_key(info.id, &master_key()).await;
        assert!(matches!(result, Err(KeyError::Revoked(_))));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_signing_keys() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();
        let other_org = UniversalUuid::new_v4();

        // Empty initially
        let keys = km.list_signing_keys(org_id).await.unwrap();
        assert!(keys.is_empty());

        km.create_signing_key(org_id, "key-a", &master_key())
            .await
            .unwrap();
        km.create_signing_key(org_id, "key-b", &master_key())
            .await
            .unwrap();
        km.create_signing_key(other_org, "key-other", &master_key())
            .await
            .unwrap();

        let keys = km.list_signing_keys(org_id).await.unwrap();
        assert_eq!(keys.len(), 2);

        let other_keys = km.list_signing_keys(other_org).await.unwrap();
        assert_eq!(other_keys.len(), 1);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_revoke_signing_key() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let info = km
            .create_signing_key(org_id, "revoke-test", &master_key())
            .await
            .unwrap();
        assert!(info.is_active());

        km.revoke_signing_key(info.id).await.unwrap();

        let fetched = km.get_signing_key_info(info.id).await.unwrap();
        assert!(!fetched.is_active());
        assert!(fetched.revoked_at.is_some());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_revoke_signing_key_not_found() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);

        let result = km.revoke_signing_key(UniversalUuid::new_v4()).await;
        assert!(matches!(result, Err(KeyError::NotFound(_))));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_export_public_key() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let info = km
            .create_signing_key(org_id, "export-key", &master_key())
            .await
            .unwrap();

        let export = km.export_public_key(info.id).await.unwrap();
        assert_eq!(export.fingerprint, info.fingerprint);
        assert_eq!(export.public_key_raw, info.public_key);
        assert!(export.public_key_pem.contains("-----BEGIN PUBLIC KEY-----"));

        // PEM should decode back to the same raw key
        let decoded = DbKeyManager::decode_public_key_pem(&export.public_key_pem).unwrap();
        assert_eq!(decoded, info.public_key);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_trust_public_key() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let keypair = crate::crypto::generate_signing_keypair();
        let trusted = km
            .trust_public_key(org_id, &keypair.public_key, Some("vendor-key"))
            .await
            .unwrap();

        assert_eq!(trusted.org_id, org_id);
        assert_eq!(trusted.public_key, keypair.public_key);
        assert_eq!(trusted.key_name.as_deref(), Some("vendor-key"));
        assert!(trusted.is_active());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_trust_public_key_invalid_length() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let result = km.trust_public_key(org_id, &[0u8; 16], Some("short")).await;
        assert!(matches!(result, Err(KeyError::InvalidFormat(_))));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_trust_public_key_pem() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let keypair = crate::crypto::generate_signing_keypair();
        let pem = DbKeyManager::encode_public_key_pem(&keypair.public_key);

        let trusted = km
            .trust_public_key_pem(org_id, &pem, Some("pem-key"))
            .await
            .unwrap();

        assert_eq!(trusted.public_key, keypair.public_key);
        assert_eq!(trusted.key_name.as_deref(), Some("pem-key"));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_trust_public_key_pem_invalid() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let result = km.trust_public_key_pem(org_id, "garbage pem", None).await;
        assert!(matches!(result, Err(KeyError::InvalidPem(_))));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_trusted_keys() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        // Empty initially
        let keys = km.list_trusted_keys(org_id).await.unwrap();
        assert!(keys.is_empty());

        let kp1 = crate::crypto::generate_signing_keypair();
        let kp2 = crate::crypto::generate_signing_keypair();

        km.trust_public_key(org_id, &kp1.public_key, Some("vendor-1"))
            .await
            .unwrap();
        km.trust_public_key(org_id, &kp2.public_key, None)
            .await
            .unwrap();

        let keys = km.list_trusted_keys(org_id).await.unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_revoke_trusted_key() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let kp = crate::crypto::generate_signing_keypair();
        let trusted = km
            .trust_public_key(org_id, &kp.public_key, Some("revoke-me"))
            .await
            .unwrap();

        km.revoke_trusted_key(trusted.id).await.unwrap();

        // Revoked keys should not appear in list (list filters revoked)
        let keys = km.list_trusted_keys(org_id).await.unwrap();
        assert!(keys.is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_revoke_trusted_key_not_found() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);

        let result = km.revoke_trusted_key(UniversalUuid::new_v4()).await;
        assert!(matches!(result, Err(KeyError::NotFound(_))));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_find_trusted_key_direct() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let kp = crate::crypto::generate_signing_keypair();
        let trusted = km
            .trust_public_key(org_id, &kp.public_key, Some("findable"))
            .await
            .unwrap();

        let found = km
            .find_trusted_key(org_id, &trusted.fingerprint)
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().fingerprint, trusted.fingerprint);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_find_trusted_key_not_found() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let found = km
            .find_trusted_key(org_id, "nonexistent-fingerprint")
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_find_trusted_key_revoked_not_found() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let kp = crate::crypto::generate_signing_keypair();
        let trusted = km
            .trust_public_key(org_id, &kp.public_key, None)
            .await
            .unwrap();

        km.revoke_trusted_key(trusted.id).await.unwrap();

        let found = km
            .find_trusted_key(org_id, &trusted.fingerprint)
            .await
            .unwrap();
        assert!(found.is_none());
    }

    // ── Trust ACL tests ──────────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_grant_trust() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let parent = UniversalUuid::new_v4();
        let child = UniversalUuid::new_v4();

        km.grant_trust(parent, child).await.unwrap();

        // Child's trusted keys should be visible to parent via ACL
        let kp = crate::crypto::generate_signing_keypair();
        km.trust_public_key(child, &kp.public_key, Some("child-key"))
            .await
            .unwrap();

        let parent_keys = km.list_trusted_keys(parent).await.unwrap();
        assert_eq!(parent_keys.len(), 1);
        assert_eq!(parent_keys[0].key_name.as_deref(), Some("child-key"));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_grant_trust_find_inherited_key() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let parent = UniversalUuid::new_v4();
        let child = UniversalUuid::new_v4();

        let kp = crate::crypto::generate_signing_keypair();
        km.trust_public_key(child, &kp.public_key, None)
            .await
            .unwrap();

        km.grant_trust(parent, child).await.unwrap();

        let fingerprint = crate::crypto::compute_key_fingerprint(&kp.public_key);
        let found = km.find_trusted_key(parent, &fingerprint).await.unwrap();
        assert!(found.is_some());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_revoke_trust() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let parent = UniversalUuid::new_v4();
        let child = UniversalUuid::new_v4();

        let kp = crate::crypto::generate_signing_keypair();
        km.trust_public_key(child, &kp.public_key, None)
            .await
            .unwrap();
        km.grant_trust(parent, child).await.unwrap();

        // Revoke the trust
        km.revoke_trust(parent, child).await.unwrap();

        // Parent should no longer see child's keys
        let parent_keys = km.list_trusted_keys(parent).await.unwrap();
        assert!(parent_keys.is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_revoke_trust_not_found() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let parent = UniversalUuid::new_v4();
        let child = UniversalUuid::new_v4();

        let result = km.revoke_trust(parent, child).await;
        assert!(matches!(result, Err(KeyError::TrustNotFound)));
    }

    // ── End-to-end sign/verify round-trip ────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_create_key_sign_and_verify_roundtrip() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();
        let mk = master_key();

        // Create a signing key
        let info = km
            .create_signing_key(org_id, "sign-verify", &mk)
            .await
            .unwrap();

        // Decrypt the keypair
        let (pub_key, priv_key) = km.get_signing_key(info.id, &mk).await.unwrap();

        // Sign some data
        let data = b"important payload";
        let signature = crate::crypto::sign_package(data, &priv_key).unwrap();

        // Verify
        let result = crate::crypto::verify_signature(data, &signature, &pub_key);
        assert!(result.is_ok());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_export_and_import_roundtrip() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_a = UniversalUuid::new_v4();
        let org_b = UniversalUuid::new_v4();
        let mk = master_key();

        // Org A creates a key and exports it
        let info = km
            .create_signing_key(org_a, "exportable", &mk)
            .await
            .unwrap();
        let export = km.export_public_key(info.id).await.unwrap();

        // Org B imports the public key via PEM
        let trusted = km
            .trust_public_key_pem(org_b, &export.public_key_pem, Some("org-a-key"))
            .await
            .unwrap();

        assert_eq!(trusted.public_key, info.public_key);
        assert_eq!(trusted.fingerprint, info.fingerprint);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_signing_keys_includes_revoked() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_id = UniversalUuid::new_v4();

        let key = km
            .create_signing_key(org_id, "will-revoke", &master_key())
            .await
            .unwrap();
        km.revoke_signing_key(key.id).await.unwrap();

        // list_signing_keys returns all keys including revoked
        let keys = km.list_signing_keys(org_id).await.unwrap();
        assert_eq!(keys.len(), 1);
        assert!(!keys[0].is_active());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_trusted_keys_deduplicates_across_acl() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let parent = UniversalUuid::new_v4();
        let child = UniversalUuid::new_v4();

        let kp = crate::crypto::generate_signing_keypair();

        // Trust the same key in both orgs
        km.trust_public_key(parent, &kp.public_key, Some("direct"))
            .await
            .unwrap();
        km.trust_public_key(child, &kp.public_key, Some("child-copy"))
            .await
            .unwrap();

        km.grant_trust(parent, child).await.unwrap();

        // Should not have duplicates
        let keys = km.list_trusted_keys(parent).await.unwrap();
        assert_eq!(keys.len(), 1);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_multiple_orgs_isolated() {
        let dal = unique_dal().await;
        let km = DbKeyManager::new(dal);
        let org_a = UniversalUuid::new_v4();
        let org_b = UniversalUuid::new_v4();

        km.create_signing_key(org_a, "a-key", &master_key())
            .await
            .unwrap();
        km.create_signing_key(org_b, "b-key", &master_key())
            .await
            .unwrap();

        let a_keys = km.list_signing_keys(org_a).await.unwrap();
        let b_keys = km.list_signing_keys(org_b).await.unwrap();

        assert_eq!(a_keys.len(), 1);
        assert_eq!(b_keys.len(), 1);
        assert_eq!(a_keys[0].key_name, "a-key");
        assert_eq!(b_keys[0].key_name, "b-key");
    }
}
