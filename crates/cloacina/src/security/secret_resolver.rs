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

//! Concrete secret resolver over the tenant-scoped [`SecretStore`]
//! (CLOACI-I-0133 / T-0858, design D-1).
//!
//! [`SecretStoreResolver`] is the embedded / in-process backend for the
//! [`Context::secret`](cloacina_workflow::Context::secret) accessor: it bundles
//! the three things resolution needs — a [`SecretStore`] handle, the tenant
//! `org_id`, and the server **KEK** — and decrypts a named secret into its
//! `{field: value}` map at fire time via [`SecretStore::resolve_secret`]. The
//! resolved plaintext is returned to the task and never persisted or logged
//! (NFR-001).
//!
//! ## Server KEK sourcing (D-7)
//!
//! T-0857 left the server KEK unwired. This task sources it from the
//! `CLOACINA_SECRET_KEK` environment variable — the same env-driven pattern the
//! codebase already uses for secret material (e.g. `CLOACINA_VAR_API_KEY` in
//! [`crate::var`]). The value is a base64 (standard) **or** hex encoding of
//! exactly 32 bytes (AES-256). Callers that manage the KEK themselves construct
//! the resolver directly via [`SecretStoreResolver::new`].

use async_trait::async_trait;
use base64::Engine;
use std::collections::BTreeMap;
use std::sync::Arc;

use cloacina_workflow::secret::{SecretResolver, SecretResolverError};

use crate::database::universal_types::UniversalUuid;
use crate::security::{SecretError, SecretStore};

/// Environment variable holding the server KEK (base64 or hex of 32 bytes).
pub const KEK_ENV_VAR: &str = "CLOACINA_SECRET_KEK";

/// Errors constructing a [`SecretStoreResolver`] from configuration/environment.
#[derive(Debug, thiserror::Error)]
pub enum SecretResolverConfigError {
    #[error("environment variable {0} is not set")]
    MissingEnv(&'static str),

    #[error("{0} must be base64 or hex encoding of exactly 32 bytes")]
    InvalidKek(&'static str),
}

/// Resolves secrets by decrypting against a tenant-scoped [`SecretStore`].
///
/// Holds the server KEK in memory for the life of the resolver; it is never
/// serialized, logged, or exposed through [`Debug`] (see the manual impl below).
#[derive(Clone)]
pub struct SecretStoreResolver {
    store: SecretStore,
    org_id: UniversalUuid,
    kek: Vec<u8>,
}

impl std::fmt::Debug for SecretStoreResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never render the KEK.
        f.debug_struct("SecretStoreResolver")
            .field("org_id", &self.org_id)
            .field("kek", &"<redacted>")
            .finish()
    }
}

impl SecretStoreResolver {
    /// Construct a resolver from an explicit KEK (32 bytes).
    ///
    /// This is the wiring seam for hosts/runners that manage the KEK
    /// themselves. `kek` must be 32 bytes; a wrong length surfaces later as a
    /// backend error at resolve time (mirroring the store's own contract).
    pub fn new(store: SecretStore, org_id: UniversalUuid, kek: Vec<u8>) -> Self {
        Self { store, org_id, kek }
    }

    /// Construct + box as a trait object ready to attach to a `Context`.
    pub fn into_arc(self) -> Arc<dyn SecretResolver> {
        Arc::new(self)
    }

    /// Parse a KEK from a base64 (standard) or hex string; requires 32 bytes.
    pub fn parse_kek(raw: &str) -> Result<Vec<u8>, SecretResolverConfigError> {
        let raw = raw.trim();
        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(raw) {
            if bytes.len() == 32 {
                return Ok(bytes);
            }
        }
        if let Ok(bytes) = hex::decode(raw) {
            if bytes.len() == 32 {
                return Ok(bytes);
            }
        }
        Err(SecretResolverConfigError::InvalidKek(KEK_ENV_VAR))
    }

    /// Read + parse the server KEK from `CLOACINA_SECRET_KEK`.
    pub fn kek_from_env() -> Result<Vec<u8>, SecretResolverConfigError> {
        let raw = std::env::var(KEK_ENV_VAR)
            .map_err(|_| SecretResolverConfigError::MissingEnv(KEK_ENV_VAR))?;
        Self::parse_kek(&raw)
    }

    /// Construct a resolver sourcing the KEK from `CLOACINA_SECRET_KEK`.
    ///
    /// Returns `Ok(None)` when the env var is unset (secrets simply aren't
    /// configured on this deployment); `Err` when it is set but malformed.
    pub fn from_env(
        store: SecretStore,
        org_id: UniversalUuid,
    ) -> Result<Option<Self>, SecretResolverConfigError> {
        match std::env::var(KEK_ENV_VAR) {
            Err(_) => Ok(None),
            Ok(raw) => {
                let kek = Self::parse_kek(&raw)?;
                Ok(Some(Self::new(store, org_id, kek)))
            }
        }
    }
}

#[async_trait]
impl SecretResolver for SecretStoreResolver {
    async fn resolve(&self, name: &str) -> Result<BTreeMap<String, String>, SecretResolverError> {
        self.store
            .resolve_secret(self.org_id, name, &self.kek)
            .await
            .map_err(|e| match e {
                SecretError::NotFound(n) => SecretResolverError::NotFound(n),
                // Everything else (decryption, DB, serialization) is a backend
                // failure; the message is already non-plaintext.
                other => SecretResolverError::Backend(other.to_string()),
            })
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::dal::unified::DAL;
    use crate::database::Database;
    use cloacina_workflow::Context;

    async fn unique_dal() -> DAL {
        let url = format!(
            "file:secret_resolver_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations().await.expect("migrations");
        DAL::new(db)
    }

    fn kek() -> Vec<u8> {
        vec![7u8; 32]
    }

    #[test]
    fn test_parse_kek_accepts_base64_and_hex_32_bytes() {
        let raw = [3u8; 32];
        let b64 = base64::engine::general_purpose::STANDARD.encode(raw);
        assert_eq!(SecretStoreResolver::parse_kek(&b64).unwrap(), raw.to_vec());

        let hexed = hex::encode(raw);
        assert_eq!(
            SecretStoreResolver::parse_kek(&hexed).unwrap(),
            raw.to_vec()
        );
    }

    #[test]
    fn test_parse_kek_rejects_wrong_length() {
        let short = base64::engine::general_purpose::STANDARD.encode([1u8; 16]);
        assert!(SecretStoreResolver::parse_kek(&short).is_err());
    }

    #[tokio::test]
    async fn test_resolver_resolves_stored_secret_through_context() {
        let dal = unique_dal().await;
        let store = SecretStore::new(dal);
        let org = UniversalUuid::new_v4();

        let mut fields = BTreeMap::new();
        fields.insert("password".to_string(), "resolver-secret".to_string());
        store
            .create_secret(org, "db_prod", &fields, &kek())
            .await
            .unwrap();

        let resolver = SecretStoreResolver::new(store, org, kek()).into_arc();
        let ctx = Context::<serde_json::Value>::new().with_secret_resolver(resolver);

        let resolved = ctx.secret("db_prod").await.unwrap();
        assert_eq!(resolved.get("password").unwrap(), "resolver-secret");

        // The resolved plaintext never entered the serialized context.
        let json = ctx.to_json().unwrap();
        assert!(!json.contains("resolver-secret"), "secret leaked: {json}");
    }

    #[tokio::test]
    async fn test_resolver_missing_secret_maps_to_not_found() {
        let dal = unique_dal().await;
        let store = SecretStore::new(dal);
        let org = UniversalUuid::new_v4();
        let resolver = SecretStoreResolver::new(store, org, kek()).into_arc();
        let ctx = Context::<serde_json::Value>::new().with_secret_resolver(resolver);

        assert!(matches!(
            ctx.secret("absent").await.unwrap_err(),
            cloacina_workflow::SecretAccessError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_resolver_wrong_kek_maps_to_backend_error() {
        let dal = unique_dal().await;
        let store = SecretStore::new(dal);
        let org = UniversalUuid::new_v4();

        let mut fields = BTreeMap::new();
        fields.insert("password".to_string(), "resolver-secret".to_string());
        store
            .create_secret(org, "db_prod", &fields, &kek())
            .await
            .unwrap();

        let resolver = SecretStoreResolver::new(store, org, vec![9u8; 32]).into_arc();
        let ctx = Context::<serde_json::Value>::new().with_secret_resolver(resolver);

        assert!(matches!(
            ctx.secret("db_prod").await.unwrap_err(),
            cloacina_workflow::SecretAccessError::Backend(_)
        ));
    }
}
