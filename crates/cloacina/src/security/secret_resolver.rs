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
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use cloacina_workflow::secret::{SecretResolver, SecretResolverError};

use crate::database::universal_types::UniversalUuid;
use crate::security::{SecretError, SecretStore};

/// Environment variable holding the server KEK (base64 or hex of 32 bytes).
pub const KEK_ENV_VAR: &str = "CLOACINA_SECRET_KEK";

/// Which secret names a [`SecretStoreResolver`] is permitted to resolve
/// (CLOACI-I-0133 / T-0860, design D-3) — the enforced trust boundary.
///
/// The resolver already scopes every lookup to its `org_id` (tenant is the outer
/// boundary), so this is the *inner* gate: within the tenant, which named secrets
/// may this holder resolve.
///
/// - [`SecretAllow::All`] — the **trusted embedded-host** path. The host owns the
///   KEK and wires the resolver in directly (T-0858), so it may resolve any secret
///   in its own tenant. Reachable ONLY via the explicitly-named ungated
///   constructor [`SecretStoreResolver::new`] / [`SecretStoreResolver::from_env`].
/// - [`SecretAllow::List`] — the **fail-closed gated** path for untrusted
///   packaged-workflow / WASM-constructor code. The set is the constructor's
///   granted `ResolvedGrants.secrets`; a name not in it is denied before any
///   decrypt. An empty set denies everything. Reachable via the gated
///   constructor [`SecretStoreResolver::new_gated`] (and, under the
///   `constructors-wasm` feature, the `from_grants` conveniences).
///
/// The two paths are separate constructors on purpose: "forgot to set the list"
/// cannot silently ungate the packaged path, because ungating (`All`) requires
/// calling the explicitly-named trusted constructor.
#[derive(Debug, Clone)]
pub enum SecretAllow {
    /// Trusted: any secret in the resolver's tenant may be resolved.
    All,
    /// Fail-closed: only names in this set may be resolved.
    List(HashSet<String>),
}

impl SecretAllow {
    /// Build the fail-closed allow-list from a constructor's resolved grants —
    /// the packaged/WASM path's gate (`ResolvedGrants.secrets` → allow-list).
    ///
    /// Available only under `constructors-wasm`, the feature that defines the
    /// grant model; the embedded host builds a [`SecretAllow`] directly.
    #[cfg(feature = "constructors-wasm")]
    pub fn from_grants(grants: &crate::registry::loader::grants::ResolvedGrants) -> Self {
        SecretAllow::List(grants.secrets.iter().cloned().collect())
    }

    /// Whether `name` may be resolved under this policy.
    fn permits(&self, name: &str) -> bool {
        match self {
            SecretAllow::All => true,
            SecretAllow::List(set) => set.contains(name),
        }
    }
}

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
    /// The enforced allow-list gate (CLOACI-T-0860, D-3). `All` for the trusted
    /// embedded host; `List` (fail-closed) for the gated packaged path.
    allow: SecretAllow,
}

impl std::fmt::Debug for SecretStoreResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never render the KEK. `allow` holds secret NAMES only (no values), so it
        // is safe to render — and useful when auditing what a resolver may reach.
        f.debug_struct("SecretStoreResolver")
            .field("org_id", &self.org_id)
            .field("kek", &"<redacted>")
            .field("allow", &self.allow)
            .finish()
    }
}

impl SecretStoreResolver {
    /// Construct a **trusted, ungated** resolver from an explicit KEK (32 bytes).
    ///
    /// This is the embedded-host wiring seam (T-0858): the host owns the KEK and
    /// is trusted, so the resolver may resolve any secret in `org_id`
    /// ([`SecretAllow::All`]). Untrusted packaged/WASM code must NOT be handed a
    /// resolver built this way — use [`new_gated`](Self::new_gated) /
    /// [`from_grants`](Self::from_grants) so its granted allow-list is enforced.
    ///
    /// `kek` must be 32 bytes; a wrong length surfaces later as a backend error at
    /// resolve time (mirroring the store's own contract).
    pub fn new(store: SecretStore, org_id: UniversalUuid, kek: Vec<u8>) -> Self {
        Self {
            store,
            org_id,
            kek,
            allow: SecretAllow::All,
        }
    }

    /// Construct a **gated, fail-closed** resolver whose `resolve` is restricted to
    /// `allow` (CLOACI-T-0860, D-3).
    ///
    /// This is the constructor the untrusted packaged-workflow / WASM path takes:
    /// pass a [`SecretAllow::List`] (e.g. built by `SecretAllow::from_grants` from
    /// the constructor's `ResolvedGrants`) so a name the tenant did not grant is
    /// denied before any decrypt. An empty [`SecretAllow::List`] denies everything.
    pub fn new_gated(
        store: SecretStore,
        org_id: UniversalUuid,
        kek: Vec<u8>,
        allow: SecretAllow,
    ) -> Self {
        Self {
            store,
            org_id,
            kek,
            allow,
        }
    }

    /// Construct a gated resolver whose allow-list is the constructor's granted
    /// secrets (`ResolvedGrants.secrets`) — the one-call packaged-path seam.
    ///
    /// Available only under `constructors-wasm` (the feature that defines the
    /// grant model). This is where the packaged/WASM trust boundary is realized:
    /// the resolver handed to untrusted constructor code is built from *its*
    /// grants, fail-closed.
    #[cfg(feature = "constructors-wasm")]
    pub fn from_grants(
        store: SecretStore,
        org_id: UniversalUuid,
        kek: Vec<u8>,
        grants: &crate::registry::loader::grants::ResolvedGrants,
    ) -> Self {
        Self::new_gated(store, org_id, kek, SecretAllow::from_grants(grants))
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
        // Enforce the granted allow-list BEFORE any decrypt (CLOACI-T-0860, D-3).
        // The audit line records the secret NAME only — never a value (NFR-001).
        if !self.allow.permits(name) {
            tracing::warn!(
                secret.name = %name,
                org_id = %self.org_id,
                "secret resolution DENIED: name not in the holder's secrets grant"
            );
            return Err(SecretResolverError::NotGranted(name.to_string()));
        }
        tracing::debug!(
            secret.name = %name,
            org_id = %self.org_id,
            "secret resolution allowed by grant; resolving"
        );
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

    // ── Grant enforcement (CLOACI-T-0860, D-3) ──────────────────────────────

    /// A gated resolver resolves a GRANTED name but DENIES an un-granted one —
    /// even though the un-granted secret exists in the store (proving the denial
    /// is the grant, not a missing secret) and without ever decrypting it.
    #[tokio::test]
    async fn test_gated_resolver_allows_granted_denies_ungranted() {
        let dal = unique_dal().await;
        let store = SecretStore::new(dal);
        let org = UniversalUuid::new_v4();

        let mut db_fields = BTreeMap::new();
        db_fields.insert("password".to_string(), "granted-value".to_string());
        store
            .create_secret(org, "db_prod", &db_fields, &kek())
            .await
            .unwrap();
        // `other` also EXISTS in the store, but is not granted.
        let mut other_fields = BTreeMap::new();
        other_fields.insert("token".to_string(), "ungranted-value".to_string());
        store
            .create_secret(org, "other", &other_fields, &kek())
            .await
            .unwrap();

        let allow = SecretAllow::List(HashSet::from(["db_prod".to_string()]));
        let resolver = SecretStoreResolver::new_gated(store, org, kek(), allow);

        // Granted name resolves.
        let resolved = resolver.resolve("db_prod").await.unwrap();
        assert_eq!(resolved.get("password").unwrap(), "granted-value");

        // Un-granted name is denied with NotGranted (not NotFound), before decrypt.
        assert!(matches!(
            resolver.resolve("other").await.unwrap_err(),
            SecretResolverError::NotGranted(n) if n == "other"
        ));
    }

    /// Fail-closed: an EMPTY gated allow-list denies every name.
    #[tokio::test]
    async fn test_empty_gated_allow_list_denies_everything() {
        let dal = unique_dal().await;
        let store = SecretStore::new(dal);
        let org = UniversalUuid::new_v4();

        let mut fields = BTreeMap::new();
        fields.insert("password".to_string(), "v".to_string());
        store
            .create_secret(org, "db_prod", &fields, &kek())
            .await
            .unwrap();

        let resolver =
            SecretStoreResolver::new_gated(store, org, kek(), SecretAllow::List(HashSet::new()));

        assert!(matches!(
            resolver.resolve("db_prod").await.unwrap_err(),
            SecretResolverError::NotGranted(_)
        ));
    }

    /// `from_grants` lowers a `ResolvedGrants.secrets` list into the enforced gate.
    #[cfg(feature = "constructors-wasm")]
    #[tokio::test]
    async fn test_from_grants_builds_gate_from_resolved_grants() {
        use crate::registry::loader::grants::{translate, GrantSpec};

        let dal = unique_dal().await;
        let store = SecretStore::new(dal);
        let org = UniversalUuid::new_v4();

        let mut fields = BTreeMap::new();
        fields.insert("password".to_string(), "granted-value".to_string());
        store
            .create_secret(org, "db_prod", &fields, &kek())
            .await
            .unwrap();

        let grants = translate(&GrantSpec::from_lists(
            vec![],
            vec![],
            vec![],
            vec![],
            vec!["db_prod".into()],
        ))
        .unwrap();
        let resolver = SecretStoreResolver::from_grants(store, org, kek(), &grants);

        assert_eq!(
            resolver.resolve("db_prod").await.unwrap().get("password"),
            Some(&"granted-value".to_string())
        );
        assert!(matches!(
            resolver.resolve("nope").await.unwrap_err(),
            SecretResolverError::NotGranted(_)
        ));
    }

    /// The trusted (ungated) resolver resolves ANY name in its tenant — the
    /// embedded-host path where the host owns the KEK.
    #[tokio::test]
    async fn test_trusted_resolver_resolves_any_name_in_tenant() {
        let dal = unique_dal().await;
        let store = SecretStore::new(dal);
        let org = UniversalUuid::new_v4();

        for name in ["db_prod", "stripe", "anything_else"] {
            let mut fields = BTreeMap::new();
            fields.insert("k".to_string(), format!("v-{name}"));
            store
                .create_secret(org, name, &fields, &kek())
                .await
                .unwrap();
        }

        // `new` ⇒ SecretAllow::All (trusted).
        let resolver = SecretStoreResolver::new(store, org, kek());
        for name in ["db_prod", "stripe", "anything_else"] {
            assert_eq!(
                resolver.resolve(name).await.unwrap().get("k"),
                Some(&format!("v-{name}"))
            );
        }
    }

    /// Tenant-scope is the OUTER boundary: an allow-listed name still resolves only
    /// within the resolver's `org_id`. A secret created under org A is NotFound for
    /// a resolver bound to org B even though the name is granted.
    #[tokio::test]
    async fn test_grant_does_not_cross_tenant_scope() {
        let dal = unique_dal().await;
        let store = SecretStore::new(dal);
        let org_a = UniversalUuid::new_v4();
        let org_b = UniversalUuid::new_v4();

        let mut fields = BTreeMap::new();
        fields.insert("password".to_string(), "a-only".to_string());
        store
            .create_secret(org_a, "db_prod", &fields, &kek())
            .await
            .unwrap();

        // Grant names `db_prod`, but the resolver is bound to org_b.
        let allow = SecretAllow::List(HashSet::from(["db_prod".to_string()]));
        let resolver = SecretStoreResolver::new_gated(store, org_b, kek(), allow);

        // Passes the grant gate (name is allowed) but is NotFound in org_b's scope.
        assert!(matches!(
            resolver.resolve("db_prod").await.unwrap_err(),
            SecretResolverError::NotFound(_)
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
