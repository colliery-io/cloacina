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

//! Fleet secret resolution — server-side wrap + agent-side unwrap
//! (CLOACI-T-0861, I-0133 D-2/D-5/D-6, NFR-003).
//!
//! This module bridges the [`crate::crypto::envelope`] HPKE primitive and the
//! [`crate::fleet::protocol`] wire types into the two halves of the fleet path:
//!
//! - **Server:** [`resolve_and_wrap_secrets`] grant-checks + decrypts each named
//!   secret through a [`SecretResolver`] (e.g. the gated
//!   [`crate::security::SecretStoreResolver`]) and HPKE-wraps the plaintext to
//!   the agent's advertised ephemeral public key, producing
//!   [`WrappedSecret`](crate::fleet::protocol::WrappedSecret) blobs. Only
//!   ciphertext leaves the server.
//! - **Agent:** [`InMemorySecretResolver::from_wrapped`] unwraps those blobs with
//!   the agent's ephemeral private key into an in-memory map, then serves them to
//!   task bodies via [`Context::secret`](cloacina_workflow::Context::secret). The
//!   agent never holds the at-rest KEK and never persists the plaintext.
//!
//! ## Binding (replay/isolation)
//!
//! Each wrap's AEAD associated data is `"{execution_id}/{name}"` ([`secret_aad`]).
//! Both sides derive it identically, so a ciphertext for one execution+secret
//! cannot be opened as a different one — on top of the per-keypair binding the
//! HPKE KEM already provides.
//!
//! ## Granularity caveat (honest seam)
//!
//! D-5 specifies a fresh keypair *per task claim*. The live fleet protocol is
//! **push**, not claim: the server pushes a `WorkPacket` and the agent never
//! sends a per-execution "claim" it could attach a fresh key to. This module
//! therefore takes the agent's ephemeral key from **registration**
//! ([`AgentRegisterRequest::ephemeral_public_key`](crate::fleet::protocol::AgentRegisterRequest)),
//! binding it to one agent *connection*. True per-execution forward secrecy
//! requires a pre-dispatch handshake (agent replies to a "prepare" with a fresh
//! pubkey) or a pre-advertised pool of ephemeral keys — a protocol change left as
//! a documented seam. The crypto, wrap/unwrap, and resolver here are unchanged by
//! that decision; only *when* the key is minted moves.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;

use cloacina_workflow::secret::{SecretResolver, SecretResolverError};

use crate::crypto::envelope::{self, EnvelopeError, EphemeralPrivateKey};
use crate::fleet::protocol::WrappedSecret;

/// Errors from the fleet wrap/unwrap path.
#[derive(Debug, thiserror::Error)]
pub enum FleetSecretError {
    /// A secret failed to resolve on the server before wrapping.
    #[error("resolve failed for '{name}': {source}")]
    Resolve {
        name: String,
        #[source]
        source: SecretResolverError,
    },

    /// HPKE wrap/unwrap failed.
    #[error(transparent)]
    Envelope(#[from] EnvelopeError),

    /// A base64 field on a [`WrappedSecret`] was malformed.
    #[error("malformed wrapped-secret encoding for '{0}'")]
    Encoding(String),

    /// The unwrapped bytes were not a valid JSON `{field: value}` map.
    #[error("unwrapped payload for '{0}' was not a valid field map")]
    Payload(String),
}

/// Derive the AEAD associated data binding a wrap to one execution + secret.
///
/// Both the server (wrap) and agent (unwrap) MUST derive this identically.
pub fn secret_aad(execution_id: &str, name: &str) -> Vec<u8> {
    format!("{execution_id}/{name}").into_bytes()
}

/// Server side: resolve each named secret (grant-checked by `resolver`) and
/// HPKE-wrap it to `recipient_public_key`, bound to `execution_id`.
///
/// `resolver` should be the tenant + grant scoped resolver from T-0858/T-0860
/// (the wrap step adds no authorization of its own — it trusts the resolver's
/// gate). Returns one [`WrappedSecret`] per name; the plaintext exists only
/// transiently on the stack here and is never logged or persisted (NFR-001).
pub async fn resolve_and_wrap_secrets(
    resolver: &dyn SecretResolver,
    names: &[String],
    execution_id: &str,
    recipient_public_key: &[u8],
) -> Result<Vec<WrappedSecret>, FleetSecretError> {
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let fields = resolver
            .resolve(name)
            .await
            .map_err(|source| FleetSecretError::Resolve {
                name: name.clone(),
                source,
            })?;
        out.push(wrap_field_map(
            name,
            &fields,
            execution_id,
            recipient_public_key,
        )?);
    }
    Ok(out)
}

/// Wrap an already-resolved `{field: value}` map into a [`WrappedSecret`].
///
/// Split out from [`resolve_and_wrap_secrets`] so callers holding pre-resolved
/// values (and tests) can wrap directly.
pub fn wrap_field_map(
    name: &str,
    fields: &BTreeMap<String, String>,
    execution_id: &str,
    recipient_public_key: &[u8],
) -> Result<WrappedSecret, FleetSecretError> {
    // Canonical JSON of the field map is the plaintext we seal.
    let plaintext =
        serde_json::to_vec(fields).map_err(|_| FleetSecretError::Payload(name.to_string()))?;
    let aad = secret_aad(execution_id, name);
    let (enc, ciphertext) = envelope::wrap(recipient_public_key, &plaintext, &aad)?;
    let b64 = base64::engine::general_purpose::STANDARD;
    Ok(WrappedSecret {
        name: name.to_string(),
        enc_b64: b64.encode(enc),
        ciphertext_b64: b64.encode(ciphertext),
    })
}

/// A [`SecretResolver`] that serves already-resolved values from an in-memory
/// map — the fleet **agent's** resolver (CLOACI-T-0861).
///
/// Built by unwrapping the dispatch's [`WrappedSecret`] blobs. Holds the
/// decrypted `{field: value}` maps in memory for the run only; there is no DB,
/// no KEK, and nothing is persisted. Its [`Debug`] renders names only.
#[derive(Clone, Default)]
pub struct InMemorySecretResolver {
    secrets: HashMap<String, BTreeMap<String, String>>,
}

impl std::fmt::Debug for InMemorySecretResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Names only — never values.
        let names: Vec<&String> = self.secrets.keys().collect();
        f.debug_struct("InMemorySecretResolver")
            .field("names", &names)
            .finish()
    }
}

impl InMemorySecretResolver {
    /// Build directly from a name → field-map table (mainly for tests / the
    /// embedded caller that already holds plaintext).
    pub fn new(secrets: HashMap<String, BTreeMap<String, String>>) -> Self {
        Self { secrets }
    }

    /// An empty resolver — serves nothing (every lookup is `NotFound`).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Agent side: unwrap a set of [`WrappedSecret`] blobs with the ephemeral
    /// private key into an in-memory resolver.
    ///
    /// `execution_id` MUST be the same value the server used to wrap (the
    /// dispatch's `task_execution_id` / `firing_id`) so the per-message AAD
    /// matches; otherwise the AEAD open fails closed.
    pub fn from_wrapped(
        private_key: &EphemeralPrivateKey,
        wrapped: &[WrappedSecret],
        execution_id: &str,
    ) -> Result<Self, FleetSecretError> {
        let b64 = base64::engine::general_purpose::STANDARD;
        let mut secrets = HashMap::with_capacity(wrapped.len());
        for w in wrapped {
            let enc = b64
                .decode(&w.enc_b64)
                .map_err(|_| FleetSecretError::Encoding(w.name.clone()))?;
            let ciphertext = b64
                .decode(&w.ciphertext_b64)
                .map_err(|_| FleetSecretError::Encoding(w.name.clone()))?;
            let aad = secret_aad(execution_id, &w.name);
            let plaintext = envelope::unwrap(private_key, &enc, &ciphertext, &aad)?;
            let fields: BTreeMap<String, String> = serde_json::from_slice(&plaintext)
                .map_err(|_| FleetSecretError::Payload(w.name.clone()))?;
            secrets.insert(w.name.clone(), fields);
        }
        Ok(Self { secrets })
    }

    /// Number of secrets held.
    pub fn len(&self) -> usize {
        self.secrets.len()
    }

    /// Whether the resolver holds no secrets.
    pub fn is_empty(&self) -> bool {
        self.secrets.is_empty()
    }

    /// Box as a trait object ready to attach to a `Context`.
    pub fn into_arc(self) -> Arc<dyn SecretResolver> {
        Arc::new(self)
    }
}

#[async_trait]
impl SecretResolver for InMemorySecretResolver {
    async fn resolve(&self, name: &str) -> Result<BTreeMap<String, String>, SecretResolverError> {
        self.secrets
            .get(name)
            .cloned()
            .ok_or_else(|| SecretResolverError::NotFound(name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::envelope::generate_ephemeral_keypair;
    use cloacina_workflow::Context;

    fn field_map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// Full loop: wrap a field map to an agent pubkey, unwrap into the in-memory
    /// resolver, and serve it through `Context::secret()`.
    #[tokio::test]
    async fn wrap_unwrap_serves_through_context() {
        let kp = generate_ephemeral_keypair();
        let fields = field_map(&[("host", "db.internal"), ("password", "hunter2")]);

        let wrapped = wrap_field_map("db_prod", &fields, "exec-1", &kp.public_key_bytes).unwrap();
        // Only ciphertext on the wire — the plaintext must not appear.
        assert!(!wrapped.ciphertext_b64.contains("hunter2"));
        assert!(!wrapped.enc_b64.contains("hunter2"));

        let resolver =
            InMemorySecretResolver::from_wrapped(&kp.private, &[wrapped], "exec-1").unwrap();
        let ctx = Context::<serde_json::Value>::new().with_secret_resolver(resolver.into_arc());

        let resolved = ctx.secret("db_prod").await.unwrap();
        assert_eq!(resolved.get("password").unwrap(), "hunter2");
        assert_eq!(resolved.get("host").unwrap(), "db.internal");

        // Plaintext never entered the serialized context (NFR-001 on the agent).
        let json = ctx.to_json().unwrap();
        assert!(
            !json.contains("hunter2"),
            "secret leaked into context: {json}"
        );
    }

    /// A blob wrapped to agent A cannot be unwrapped with agent B's key — the
    /// wire-isolation guarantee (different keypair/execution can't unwrap).
    #[test]
    fn blob_for_agent_a_not_unwrappable_by_agent_b() {
        let a = generate_ephemeral_keypair();
        let b = generate_ephemeral_keypair();
        let wrapped = wrap_field_map(
            "db_prod",
            &field_map(&[("password", "A-only")]),
            "exec-1",
            &a.public_key_bytes,
        )
        .unwrap();

        let err = InMemorySecretResolver::from_wrapped(&b.private, &[wrapped], "exec-1")
            .expect_err("agent B must not unwrap A's blob");
        assert!(matches!(err, FleetSecretError::Envelope(_)));
    }

    /// The AAD binds the blob to its execution: the right key but the wrong
    /// execution id must fail (replay across executions is rejected).
    #[test]
    fn blob_bound_to_execution_id() {
        let kp = generate_ephemeral_keypair();
        let wrapped = wrap_field_map(
            "db_prod",
            &field_map(&[("password", "bound")]),
            "exec-1",
            &kp.public_key_bytes,
        )
        .unwrap();

        let err = InMemorySecretResolver::from_wrapped(&kp.private, &[wrapped], "exec-2")
            .expect_err("wrong execution id must fail");
        assert!(matches!(err, FleetSecretError::Envelope(_)));
    }

    #[tokio::test]
    async fn in_memory_resolver_reports_missing() {
        let resolver = InMemorySecretResolver::empty();
        assert!(matches!(
            resolver.resolve("absent").await.unwrap_err(),
            SecretResolverError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn resolve_and_wrap_through_store_and_grant_gate() {
        use crate::dal::unified::DAL;
        use crate::database::universal_types::UniversalUuid;
        use crate::database::Database;
        use crate::security::{SecretAllow, SecretStore, SecretStoreResolver};
        use std::collections::HashSet;

        // Real store, real gated resolver (T-0860), real KEK.
        let url = format!(
            "file:fleet_secret_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations().await.unwrap();
        let store = SecretStore::new(DAL::new(db));
        let org = UniversalUuid::new_v4();
        let kek = vec![7u8; 32];

        store
            .create_secret(
                org,
                "db_prod",
                &field_map(&[("password", "at-rest-value")]),
                &kek,
            )
            .await
            .unwrap();
        // Exists but will NOT be granted.
        store
            .create_secret(org, "other", &field_map(&[("token", "nope")]), &kek)
            .await
            .unwrap();

        let allow = SecretAllow::List(HashSet::from(["db_prod".to_string()]));
        let resolver = SecretStoreResolver::new_gated(store, org, kek, allow);

        // Agent side generates the keypair.
        let kp = generate_ephemeral_keypair();

        // Server side resolves (grant-checked) + wraps.
        let wrapped = resolve_and_wrap_secrets(
            &resolver,
            &["db_prod".to_string()],
            "exec-42",
            &kp.public_key_bytes,
        )
        .await
        .unwrap();
        assert_eq!(wrapped.len(), 1);
        assert!(!wrapped[0].ciphertext_b64.contains("at-rest-value"));

        // Agent unwraps and serves through Context.
        let agent_resolver =
            InMemorySecretResolver::from_wrapped(&kp.private, &wrapped, "exec-42").unwrap();
        let ctx =
            Context::<serde_json::Value>::new().with_secret_resolver(agent_resolver.into_arc());
        assert_eq!(
            ctx.secret("db_prod")
                .await
                .unwrap()
                .get("password")
                .unwrap(),
            "at-rest-value"
        );

        // An un-granted secret is denied at wrap time (the gate, before any wire).
        let denied = resolve_and_wrap_secrets(
            &resolver,
            &["other".to_string()],
            "exec-42",
            &kp.public_key_bytes,
        )
        .await;
        assert!(matches!(
            denied,
            Err(FleetSecretError::Resolve {
                source: SecretResolverError::NotGranted(_),
                ..
            })
        ));
    }
}
