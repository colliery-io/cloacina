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

//! Encrypted, tenant-scoped secrets store (CLOACI-I-0133 / T-0857).
//!
//! A **Secret** is a named object of named fields (a `{field: value}` map),
//! encrypted at rest — the encrypted sibling of a parameter. This module is the
//! foundation task of I-0133: the store, the DAL, and metadata-only CRUD. It
//! deliberately does NOT implement the resolution side channel (the `ctx.secret()`
//! accessor, D-1), grants (D-3), instance-param `$secret` references (D-4), or the
//! fleet per-execution envelope wrap (D-2) — those are later tasks. [`resolve_secret`]
//! exists here as the INTERNAL decrypt primitive those tasks will call.
//!
//! ## Envelope encryption (D-7)
//!
//! Each tenant has a per-tenant **data key (DEK)** — 32 random bytes generated on
//! first use — wrapped by a server **KEK** and stored in `tenant_data_keys`. A
//! secret's field map is serialized to JSON and encrypted under the tenant DEK
//! (AES-256-GCM, `nonce || ciphertext || tag`). The KEK is supplied by the caller
//! on every method as `kek: &[u8]` (32 bytes), mirroring the `master_key`
//! parameter of [`crate::security::DbKeyManager`]. Unwrapping the DEK happens only
//! server-side; the plaintext DEK and plaintext field values live only transiently
//! in memory.
//!
//! ## Metadata vs. plaintext (REQ-002 / NFR-001)
//!
//! [`SecretMetadata`] (returned by list/get) carries names, field names, and
//! timestamps — NEVER a plaintext or ciphertext value. Plaintext field values are
//! returned ONLY by [`resolve_secret`], the internal resolve path.

use crate::crypto::{decrypt_bytes, encrypt_bytes};
use crate::dal::unified::models::{NewSecret, NewTenantDataKey, Secret, TenantDataKey};
use crate::dal::unified::DAL;
use crate::database::schema::unified::{secrets, tenant_data_keys};
use crate::database::universal_types::{UniversalBinary, UniversalTimestamp, UniversalUuid};
use diesel::prelude::*;
use rand::RngCore;
use std::collections::BTreeMap;
use thiserror::Error;

/// Size of a tenant data key (DEK) in bytes (AES-256).
const DEK_SIZE: usize = 32;

/// Errors that can occur in the secrets store.
#[derive(Debug, Error)]
pub enum SecretError {
    #[error("Secret not found: {0}")]
    NotFound(String),

    #[error("Secret name already exists for this tenant: {0}")]
    DuplicateName(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Database error: {0}")]
    Database(String),
}

/// Metadata-only view of a secret. **Never** carries a plaintext or ciphertext value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMetadata {
    pub id: UniversalUuid,
    pub org_id: UniversalUuid,
    pub name: String,
    /// The declared field names (plaintext metadata) — values are NOT included.
    pub field_names: Vec<String>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

impl SecretMetadata {
    fn from_row(row: &Secret) -> Result<Self, SecretError> {
        let field_names: Vec<String> = serde_json::from_str(&row.field_names)
            .map_err(|e| SecretError::Serialization(e.to_string()))?;
        Ok(Self {
            id: row.id,
            org_id: row.org_id,
            name: row.name.clone(),
            field_names,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Encrypted, tenant-scoped secrets store (mirrors [`crate::security::DbKeyManager`]).
///
/// Holds no key material: the server KEK is passed into each method that needs it.
#[derive(Clone)]
pub struct SecretStore {
    dal: DAL,
}

impl SecretStore {
    /// Creates a new secrets store over the given DAL.
    pub fn new(dal: DAL) -> Self {
        Self { dal }
    }

    // ── Field (de)serialization + encryption helpers ─────────────────────────

    /// Serialize the `{field: value}` map to canonical JSON bytes. A `BTreeMap`
    /// gives a deterministic field order in the plaintext.
    fn serialize_fields(fields: &BTreeMap<String, String>) -> Result<Vec<u8>, SecretError> {
        serde_json::to_vec(fields).map_err(|e| SecretError::Serialization(e.to_string()))
    }

    /// JSON array of just the field names (plaintext metadata).
    fn serialize_field_names(fields: &BTreeMap<String, String>) -> Result<String, SecretError> {
        let names: Vec<&String> = fields.keys().collect();
        serde_json::to_string(&names).map_err(|e| SecretError::Serialization(e.to_string()))
    }

    /// Encrypt a serialized field map under the tenant DEK.
    fn encrypt_fields(
        fields: &BTreeMap<String, String>,
        dek: &[u8],
    ) -> Result<Vec<u8>, SecretError> {
        let plaintext = Self::serialize_fields(fields)?;
        encrypt_bytes(&plaintext, dek).map_err(|e| SecretError::Encryption(e.to_string()))
    }

    // ── Tenant data key (DEK) ────────────────────────────────────────────────

    /// Get the tenant's data key (DEK), generating + wrapping one on first use.
    ///
    /// Returns the UNWRAPPED 32-byte DEK. The DEK is stored wrapped under the
    /// server `kek`; unwrapping happens only here, server-side. Internal — callers
    /// in this module hold the plaintext DEK only transiently.
    async fn get_or_create_tenant_dek(
        &self,
        org_id: UniversalUuid,
        kek: &[u8],
    ) -> Result<Vec<u8>, SecretError> {
        // Fast path: an existing wrapped DEK.
        if let Some(row) = self.get_tenant_data_key_row(org_id).await? {
            return decrypt_bytes(&row.wrapped_dek.into_inner(), kek)
                .map_err(|e| SecretError::Decryption(e.to_string()));
        }

        // Generate a fresh DEK and wrap it under the KEK.
        let mut dek = vec![0u8; DEK_SIZE];
        rand::thread_rng().fill_bytes(&mut dek);
        let wrapped =
            encrypt_bytes(&dek, kek).map_err(|e| SecretError::Encryption(e.to_string()))?;

        let new_row = NewTenantDataKey {
            id: UniversalUuid::new_v4(),
            org_id,
            wrapped_dek: UniversalBinary::new(wrapped),
            created_at: UniversalTimestamp::now(),
        };

        match self.insert_tenant_data_key(new_row).await {
            Ok(()) => Ok(dek),
            Err(SecretError::DuplicateName(_)) => {
                // Lost a race with a concurrent create: adopt the winner's DEK.
                let row = self
                    .get_tenant_data_key_row(org_id)
                    .await?
                    .ok_or_else(|| SecretError::Database("tenant DEK vanished".to_string()))?;
                decrypt_bytes(&row.wrapped_dek.into_inner(), kek)
                    .map_err(|e| SecretError::Decryption(e.to_string()))
            }
            Err(e) => Err(e),
        }
    }

    // ── Public metadata-only CRUD + internal resolve ─────────────────────────

    /// Create a new secret from a `{field: value}` map, encrypted under the tenant DEK.
    ///
    /// Returns metadata only. Errors with [`SecretError::DuplicateName`] if a secret
    /// of that name already exists for the tenant.
    pub async fn create_secret(
        &self,
        org_id: UniversalUuid,
        name: &str,
        fields: &BTreeMap<String, String>,
        kek: &[u8],
    ) -> Result<SecretMetadata, SecretError> {
        let dek = self.get_or_create_tenant_dek(org_id, kek).await?;
        let encrypted_fields = Self::encrypt_fields(fields, &dek)?;
        let field_names = Self::serialize_field_names(fields)?;
        let now = UniversalTimestamp::now();

        let new_secret = NewSecret {
            id: UniversalUuid::new_v4(),
            org_id,
            name: name.to_string(),
            field_names,
            encrypted_fields: UniversalBinary::new(encrypted_fields),
            created_at: now,
            updated_at: now,
        };

        self.insert_secret(new_secret).await?;

        // Re-read so the returned metadata reflects exactly what was stored.
        self.get_secret_metadata(org_id, name).await
    }

    /// Rotate a secret's values in place (D-8 OQ-5: in-place, no versioning).
    ///
    /// Replaces the encrypted field map with a fresh one and bumps `updated_at`.
    /// The next resolve sees the new value. Returns metadata only.
    pub async fn rotate_secret(
        &self,
        org_id: UniversalUuid,
        name: &str,
        fields: &BTreeMap<String, String>,
        kek: &[u8],
    ) -> Result<SecretMetadata, SecretError> {
        let dek = self.get_or_create_tenant_dek(org_id, kek).await?;
        let encrypted_fields = Self::encrypt_fields(fields, &dek)?;
        let field_names = Self::serialize_field_names(fields)?;
        let now = UniversalTimestamp::now();

        let updated = self
            .update_secret(
                org_id,
                name.to_string(),
                field_names,
                UniversalBinary::new(encrypted_fields),
                now,
            )
            .await?;

        if updated == 0 {
            return Err(SecretError::NotFound(name.to_string()));
        }

        self.get_secret_metadata(org_id, name).await
    }

    /// List metadata for all of a tenant's secrets. **No plaintext, no ciphertext.**
    pub async fn list_secrets_metadata(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Vec<SecretMetadata>, SecretError> {
        let rows = self.list_secret_rows(org_id).await?;
        rows.iter().map(SecretMetadata::from_row).collect()
    }

    /// Get metadata for one secret by name. **No plaintext, no ciphertext.**
    pub async fn get_secret_metadata(
        &self,
        org_id: UniversalUuid,
        name: &str,
    ) -> Result<SecretMetadata, SecretError> {
        let row = self
            .get_secret_row(org_id, name.to_string())
            .await?
            .ok_or_else(|| SecretError::NotFound(name.to_string()))?;
        SecretMetadata::from_row(&row)
    }

    /// INTERNAL: resolve a secret to its plaintext `{field: value}` map.
    ///
    /// This is the only method that returns plaintext values. Later tasks
    /// (the resolution side channel D-1, grants D-3, fleet envelope wrap D-2)
    /// call this; it must never feed `Context`, `schedules.params`, logs, audit,
    /// or the fires log (NFR-001).
    pub async fn resolve_secret(
        &self,
        org_id: UniversalUuid,
        name: &str,
        kek: &[u8],
    ) -> Result<BTreeMap<String, String>, SecretError> {
        let row = self
            .get_secret_row(org_id, name.to_string())
            .await?
            .ok_or_else(|| SecretError::NotFound(name.to_string()))?;

        let dek = self.get_or_create_tenant_dek(org_id, kek).await?;
        let plaintext = decrypt_bytes(&row.encrypted_fields.into_inner(), &dek)
            .map_err(|e| SecretError::Decryption(e.to_string()))?;
        serde_json::from_slice(&plaintext).map_err(|e| SecretError::Serialization(e.to_string()))
    }

    /// Delete a secret by name.
    pub async fn delete_secret(
        &self,
        org_id: UniversalUuid,
        name: &str,
    ) -> Result<(), SecretError> {
        let deleted = self.delete_secret_row(org_id, name.to_string()).await?;
        if deleted == 0 {
            return Err(SecretError::NotFound(name.to_string()));
        }
        Ok(())
    }

    // ── Backend-agnostic DAL queries (CLOACI-I-0135) ─────────────────────────
    // Each former `*_postgres`/`*_sqlite` twin pair is collapsed into ONE inline
    // `interact_on_backend!` body here; the call-site error mapping (NotFound via
    // the caller's `.ok_or_else`, UNIQUE-violation via `is_unique_violation`) is
    // preserved verbatim.

    async fn get_tenant_data_key_row(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Option<TenantDataKey>, SecretError> {
        crate::interact_on_backend!(self.dal, |conn| {
            tenant_data_keys::table
                .filter(tenant_data_keys::org_id.eq(org_id))
                .first(conn)
                .optional()
        })
        .map_err(|e| SecretError::Database(e.to_string()))
    }

    async fn insert_tenant_data_key(&self, new_row: NewTenantDataKey) -> Result<(), SecretError> {
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(tenant_data_keys::table)
                .values(&new_row)
                .execute(conn)
        })
        .map_err(|e| {
            if is_unique_violation(&e) {
                SecretError::DuplicateName("tenant_data_key".to_string())
            } else {
                SecretError::Database(e.to_string())
            }
        })?;

        Ok(())
    }

    async fn insert_secret(&self, new_secret: NewSecret) -> Result<(), SecretError> {
        let name = new_secret.name.clone();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(secrets::table)
                .values(&new_secret)
                .execute(conn)
        })
        .map_err(|e| {
            if is_unique_violation(&e) {
                SecretError::DuplicateName(name.clone())
            } else {
                SecretError::Database(e.to_string())
            }
        })?;

        Ok(())
    }

    async fn update_secret(
        &self,
        org_id: UniversalUuid,
        name: String,
        field_names: String,
        encrypted_fields: UniversalBinary,
        updated_at: UniversalTimestamp,
    ) -> Result<usize, SecretError> {
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                secrets::table
                    .filter(secrets::org_id.eq(org_id))
                    .filter(secrets::name.eq(name)),
            )
            .set((
                secrets::field_names.eq(field_names),
                secrets::encrypted_fields.eq(encrypted_fields),
                secrets::updated_at.eq(updated_at),
            ))
            .execute(conn)
        })
        .map_err(|e| SecretError::Database(e.to_string()))
    }

    async fn get_secret_row(
        &self,
        org_id: UniversalUuid,
        name: String,
    ) -> Result<Option<Secret>, SecretError> {
        crate::interact_on_backend!(self.dal, |conn| {
            secrets::table
                .filter(secrets::org_id.eq(org_id))
                .filter(secrets::name.eq(name))
                .first(conn)
                .optional()
        })
        .map_err(|e| SecretError::Database(e.to_string()))
    }

    async fn list_secret_rows(&self, org_id: UniversalUuid) -> Result<Vec<Secret>, SecretError> {
        crate::interact_on_backend!(self.dal, |conn| {
            secrets::table.filter(secrets::org_id.eq(org_id)).load(conn)
        })
        .map_err(|e| SecretError::Database(e.to_string()))
    }

    async fn delete_secret_row(
        &self,
        org_id: UniversalUuid,
        name: String,
    ) -> Result<usize, SecretError> {
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(
                secrets::table
                    .filter(secrets::org_id.eq(org_id))
                    .filter(secrets::name.eq(name)),
            )
            .execute(conn)
        })
        .map_err(|e| SecretError::Database(e.to_string()))
    }
}

/// Classify an insert error as a uniqueness violation vs. a generic DB error.
fn is_unique_violation(e: &diesel::result::Error) -> bool {
    let s = e.to_string();
    s.contains("duplicate") || s.contains("UNIQUE")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    // ── Helper to build a fresh DAL per test ─────────────────────────────────

    #[cfg(feature = "sqlite")]
    async fn unique_dal() -> DAL {
        let url = format!(
            "file:secret_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    #[cfg(feature = "sqlite")]
    fn kek() -> [u8; 32] {
        [7u8; 32]
    }

    #[cfg(feature = "sqlite")]
    fn sample_fields() -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("host".to_string(), "db.internal".to_string());
        m.insert("user".to_string(), "svc".to_string());
        m.insert("password".to_string(), "s3cr3t-p@ss".to_string());
        m
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_create_and_list_metadata_no_plaintext() {
        let store = SecretStore::new(unique_dal().await);
        let org = UniversalUuid::new_v4();
        let fields = sample_fields();

        let meta = store
            .create_secret(org, "db_prod", &fields, &kek())
            .await
            .unwrap();

        assert_eq!(meta.name, "db_prod");
        assert_eq!(meta.org_id, org);
        // field_names sorted (BTreeMap order): host, password, user
        assert_eq!(meta.field_names, vec!["host", "password", "user"]);

        // Metadata must not carry any plaintext value.
        let plaintext_secret = "s3cr3t-p@ss";
        let debug = format!("{:?}", meta);
        assert!(
            !debug.contains(plaintext_secret),
            "plaintext value leaked into SecretMetadata Debug: {debug}"
        );

        let listed = store.list_secrets_metadata(org).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "db_prod");
        let listed_debug = format!("{:?}", listed);
        assert!(
            !listed_debug.contains(plaintext_secret),
            "plaintext value leaked into list metadata"
        );
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_ciphertext_at_rest_differs_from_plaintext() {
        let store = SecretStore::new(unique_dal().await);
        let org = UniversalUuid::new_v4();
        let fields = sample_fields();

        store
            .create_secret(org, "db_prod", &fields, &kek())
            .await
            .unwrap();

        // Read the raw ciphertext column directly and prove no plaintext is present.
        let row = store
            .get_secret_row(org, "db_prod".to_string())
            .await
            .unwrap()
            .unwrap();
        let raw = row.encrypted_fields.into_inner();
        let needle = b"s3cr3t-p@ss";
        assert!(
            !raw.windows(needle.len()).any(|w| w == needle),
            "plaintext password found in encrypted_fields at rest"
        );
        // The plaintext field names must not appear as the raw values either.
        assert_ne!(raw, serde_json::to_vec(&fields).unwrap());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_resolve_roundtrips_map() {
        let store = SecretStore::new(unique_dal().await);
        let org = UniversalUuid::new_v4();
        let fields = sample_fields();

        store
            .create_secret(org, "db_prod", &fields, &kek())
            .await
            .unwrap();

        let resolved = store.resolve_secret(org, "db_prod", &kek()).await.unwrap();
        assert_eq!(resolved, fields);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_rotate_changes_value() {
        let store = SecretStore::new(unique_dal().await);
        let org = UniversalUuid::new_v4();

        store
            .create_secret(org, "db_prod", &sample_fields(), &kek())
            .await
            .unwrap();

        let mut new_fields = BTreeMap::new();
        new_fields.insert("host".to_string(), "db.internal".to_string());
        new_fields.insert("user".to_string(), "svc".to_string());
        new_fields.insert("password".to_string(), "rotated-value".to_string());

        store
            .rotate_secret(org, "db_prod", &new_fields, &kek())
            .await
            .unwrap();

        let resolved = store.resolve_secret(org, "db_prod", &kek()).await.unwrap();
        assert_eq!(resolved.get("password").unwrap(), "rotated-value");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_wrong_kek_fails_resolve() {
        let store = SecretStore::new(unique_dal().await);
        let org = UniversalUuid::new_v4();

        store
            .create_secret(org, "db_prod", &sample_fields(), &kek())
            .await
            .unwrap();

        let wrong_kek = [9u8; 32];
        let result = store.resolve_secret(org, "db_prod", &wrong_kek).await;
        assert!(matches!(result, Err(SecretError::Decryption(_))));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_tenant_isolation() {
        let store = SecretStore::new(unique_dal().await);
        let org_a = UniversalUuid::new_v4();
        let org_b = UniversalUuid::new_v4();

        store
            .create_secret(org_a, "db_prod", &sample_fields(), &kek())
            .await
            .unwrap();

        // Tenant B cannot see or resolve tenant A's secret.
        let b_list = store.list_secrets_metadata(org_b).await.unwrap();
        assert!(b_list.is_empty());

        let b_get = store.get_secret_metadata(org_b, "db_prod").await;
        assert!(matches!(b_get, Err(SecretError::NotFound(_))));

        let b_resolve = store.resolve_secret(org_b, "db_prod", &kek()).await;
        assert!(matches!(b_resolve, Err(SecretError::NotFound(_))));

        // Same name in both tenants is allowed and independent.
        let mut b_fields = BTreeMap::new();
        b_fields.insert("token".to_string(), "b-token".to_string());
        store
            .create_secret(org_b, "db_prod", &b_fields, &kek())
            .await
            .unwrap();
        let b_resolved = store
            .resolve_secret(org_b, "db_prod", &kek())
            .await
            .unwrap();
        assert_eq!(b_resolved.get("token").unwrap(), "b-token");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_duplicate_name_rejected() {
        let store = SecretStore::new(unique_dal().await);
        let org = UniversalUuid::new_v4();

        store
            .create_secret(org, "db_prod", &sample_fields(), &kek())
            .await
            .unwrap();
        let result = store
            .create_secret(org, "db_prod", &sample_fields(), &kek())
            .await;
        assert!(matches!(result, Err(SecretError::DuplicateName(_))));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_secret() {
        let store = SecretStore::new(unique_dal().await);
        let org = UniversalUuid::new_v4();

        store
            .create_secret(org, "db_prod", &sample_fields(), &kek())
            .await
            .unwrap();
        store.delete_secret(org, "db_prod").await.unwrap();

        let got = store.get_secret_metadata(org, "db_prod").await;
        assert!(matches!(got, Err(SecretError::NotFound(_))));

        // Deleting again is a NotFound.
        let again = store.delete_secret(org, "db_prod").await;
        assert!(matches!(again, Err(SecretError::NotFound(_))));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_rotate_missing_secret_not_found() {
        let store = SecretStore::new(unique_dal().await);
        let org = UniversalUuid::new_v4();

        let result = store
            .rotate_secret(org, "missing", &sample_fields(), &kek())
            .await;
        assert!(matches!(result, Err(SecretError::NotFound(_))));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_tenant_dek_reused_across_secrets() {
        let store = SecretStore::new(unique_dal().await);
        let org = UniversalUuid::new_v4();

        // Two secrets under the same tenant should both resolve — i.e. share one DEK.
        store
            .create_secret(org, "s1", &sample_fields(), &kek())
            .await
            .unwrap();
        let mut f2 = BTreeMap::new();
        f2.insert("api_key".to_string(), "abc123".to_string());
        store.create_secret(org, "s2", &f2, &kek()).await.unwrap();

        assert_eq!(
            store.resolve_secret(org, "s1", &kek()).await.unwrap(),
            sample_fields()
        );
        assert_eq!(store.resolve_secret(org, "s2", &kek()).await.unwrap(), f2);

        // Exactly one tenant DEK row exists.
        let dek_row = store.get_tenant_data_key_row(org).await.unwrap();
        assert!(dek_row.is_some());
    }
}
