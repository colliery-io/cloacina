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

        if &der[..ED25519_DER_PREFIX.len()] != ED25519_DER_PREFIX {
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
    fn test_invalid_pem() {
        let result = DbKeyManager::decode_public_key_pem("not a pem");
        assert!(result.is_err());

        let result = DbKeyManager::decode_public_key_pem(
            "-----BEGIN PRIVATE KEY-----\nYWJj\n-----END PRIVATE KEY-----",
        );
        assert!(matches!(result, Err(KeyError::InvalidPem(_))));
    }
}
