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
//! ## Per-execution forward secrecy via a one-time key POOL (D-5, refined)
//!
//! The live fleet protocol is **push**, not claim — the server pushes a
//! `WorkPacket` and there is no per-dispatch round-trip on which the agent could
//! attach a fresh key. D-5 therefore uses a **one-time key pool**: the agent
//! mints a pool of ephemeral keypairs ([`AgentKeyPool`]), advertises the public
//! halves (each with a `key_id`) at registration + replenish; the server persists
//! the unused public keys ([`ServerKeyPool`]) and **consumes exactly one** per
//! secret-bearing dispatch, stamping its `key_id` on the packet. The agent looks
//! up the matching private key, unwraps ONCE, and discards it. Each execution
//! thus gets a fresh single-use key = true per-execution forward secrecy, with no
//! dispatch latency. Pool exhaustion fails the dispatch cleanly (never plaintext,
//! never key reuse); the server signals the agent to top up before that happens.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;

use cloacina_workflow::secret::{SecretResolver, SecretResolverError};

use crate::crypto::envelope::{
    self, generate_ephemeral_keypair, EnvelopeError, EphemeralPrivateKey,
};
use crate::fleet::protocol::{EphemeralKeyEntry, WrappedSecret};

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

/// Extract the concrete secret NAMES a fire context references via the T-0859
/// `{"$secret": name}` alias map (stored under
/// [`SECRET_REFS_KEY`](cloacina_workflow::secret::SECRET_REFS_KEY)).
///
/// Returns the deduped, sorted set of secret names a fleet dispatch must resolve
/// + wrap. NAMES only — this reads the alias map, which by construction carries
/// no secret values (NFR-001). Used by the server's `FleetExecutor` to learn
/// which secrets a task needs from the merged context it already builds.
pub fn secret_ref_names(context: &serde_json::Value) -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();
    if let Some(serde_json::Value::Object(map)) =
        context.get(cloacina_workflow::secret::SECRET_REFS_KEY)
    {
        for v in map.values() {
            if let serde_json::Value::String(name) = v {
                names.insert(name.clone());
            }
        }
    }
    names.into_iter().collect()
}

/// Decode an [`EphemeralKeyEntry`]'s base64 public key into raw X25519 bytes.
pub fn decode_pool_public_key(entry: &EphemeralKeyEntry) -> Result<Vec<u8>, FleetSecretError> {
    base64::engine::general_purpose::STANDARD
        .decode(&entry.public_key_b64)
        .map_err(|_| FleetSecretError::Encoding(entry.key_id.clone()))
}

/// The **agent's** one-time ephemeral key pool (CLOACI-T-0861 / D-5).
///
/// Mints ephemeral X25519 keypairs, retains the private halves keyed by an opaque
/// `key_id`, and hands the public halves out as [`EphemeralKeyEntry`]s to
/// advertise to the server. A private key is used **at most once**: on a dispatch
/// stamped with a `key_id`, the agent [`take`](AgentKeyPool::take)s (removes) that
/// key, unwraps, and the key is gone forever. The private key material never
/// leaves the process and is never serialized.
#[derive(Default)]
pub struct AgentKeyPool {
    keys: HashMap<String, EphemeralPrivateKey>,
}

impl std::fmt::Debug for AgentKeyPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // key_ids only — never private key material.
        f.debug_struct("AgentKeyPool")
            .field("available", &self.keys.len())
            .finish()
    }
}

impl AgentKeyPool {
    /// An empty pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mint `n` fresh one-time keypairs: retain each private key under a fresh
    /// `key_id` and return the public entries to advertise to the server.
    pub fn mint(&mut self, n: usize) -> Vec<EphemeralKeyEntry> {
        let b64 = base64::engine::general_purpose::STANDARD;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let kp = generate_ephemeral_keypair();
            let key_id = uuid::Uuid::new_v4().to_string();
            out.push(EphemeralKeyEntry {
                key_id: key_id.clone(),
                public_key_b64: b64.encode(&kp.public_key_bytes),
            });
            self.keys.insert(key_id, kp.private);
        }
        out
    }

    /// Number of unused private keys held.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether the pool holds no unused keys.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Take (remove) the private key for `key_id` for **one-time** use. `None`
    /// when the `key_id` is unknown or already consumed — the caller MUST then
    /// fail the execution cleanly rather than run with missing secrets.
    pub fn take(&mut self, key_id: &str) -> Option<EphemeralPrivateKey> {
        self.keys.remove(key_id)
    }
}

/// The **server's** view of one agent's UNUSED one-time public keys
/// (CLOACI-T-0861 / D-5).
///
/// Consume-once: [`consume`](ServerKeyPool::consume) hands out each key exactly
/// once and removes it, so a key is NEVER reused across dispatches. Exhaustion
/// (`consume` → `None`) MUST fail the dispatch cleanly — never send plaintext,
/// never reuse a key. FIFO so the oldest advertised key is spent first.
#[derive(Debug, Default, Clone)]
pub struct ServerKeyPool {
    unused: VecDeque<EphemeralKeyEntry>,
}

impl ServerKeyPool {
    /// An empty pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed the pool from an agent's advertised entries (e.g. at registration).
    pub fn from_entries(entries: Vec<EphemeralKeyEntry>) -> Self {
        Self {
            unused: entries.into(),
        }
    }

    /// Append fresh entries (a replenish top-up). De-dupes by `key_id` so a
    /// retried top-up can't double-insert the same key.
    pub fn replenish(&mut self, entries: impl IntoIterator<Item = EphemeralKeyEntry>) -> usize {
        let mut added = 0;
        for e in entries {
            if !self.unused.iter().any(|x| x.key_id == e.key_id) {
                self.unused.push_back(e);
                added += 1;
            }
        }
        added
    }

    /// Consume one unused key (FIFO), removing it so it can never be handed out
    /// again (one-time). `None` when the pool is exhausted.
    pub fn consume(&mut self) -> Option<EphemeralKeyEntry> {
        self.unused.pop_front()
    }

    /// Number of unused keys remaining.
    pub fn len(&self) -> usize {
        self.unused.len()
    }

    /// Whether the pool is exhausted.
    pub fn is_empty(&self) -> bool {
        self.unused.is_empty()
    }

    /// How many more keys are needed to reach `target` (0 if at/above it) — the
    /// replenish signal the server returns to the agent.
    pub fn replenish_deficit(&self, target: usize) -> usize {
        target.saturating_sub(self.unused.len())
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

    // ── D-5 one-time key pool ───────────────────────────────────────────────

    fn decode_pk(entry: &EphemeralKeyEntry) -> Vec<u8> {
        decode_pool_public_key(entry).unwrap()
    }

    /// Two dispatches consume two DIFFERENT pooled keys, and each execution's
    /// wrapped blob unwraps ONLY with its own key — execution-2's blob does not
    /// unwrap with execution-1's key and vice-versa (per-execution isolation).
    /// A consumed key is gone from the agent pool (one-time use).
    #[test]
    fn two_dispatches_use_two_different_one_time_keys() {
        let mut agent = AgentKeyPool::new();
        let mut server = ServerKeyPool::from_entries(agent.mint(2));

        // Dispatch 1 consumes a key; wrap a secret to it.
        let k1 = server.consume().expect("key for exec-1");
        let w1 = wrap_field_map(
            "db",
            &field_map(&[("password", "value-1")]),
            "exec-1",
            &decode_pk(&k1),
        )
        .unwrap();

        // Dispatch 2 consumes a DIFFERENT key.
        let k2 = server.consume().expect("key for exec-2");
        assert_ne!(k1.key_id, k2.key_id, "each dispatch must spend a fresh key");
        let w2 = wrap_field_map(
            "db",
            &field_map(&[("password", "value-2")]),
            "exec-2",
            &decode_pk(&k2),
        )
        .unwrap();

        // Agent unwraps exec-1 with exec-1's key (take = one-time removal).
        let priv1 = agent.take(&k1.key_id).expect("exec-1 private key");
        InMemorySecretResolver::from_wrapped(&priv1, std::slice::from_ref(&w1), "exec-1")
            .expect("exec-1 unwraps with its own key");
        // exec-2's blob must NOT unwrap with exec-1's key.
        assert!(
            InMemorySecretResolver::from_wrapped(&priv1, std::slice::from_ref(&w2), "exec-2")
                .is_err(),
            "exec-2 blob must not open with exec-1 key"
        );

        // Agent unwraps exec-2 with exec-2's key; exec-1's blob must NOT open with it.
        let priv2 = agent.take(&k2.key_id).expect("exec-2 private key");
        assert!(
            InMemorySecretResolver::from_wrapped(&priv2, std::slice::from_ref(&w1), "exec-1")
                .is_err(),
            "exec-1 blob must not open with exec-2 key"
        );
        InMemorySecretResolver::from_wrapped(&priv2, std::slice::from_ref(&w2), "exec-2")
            .expect("exec-2 unwraps with its own key");

        // One-time: both keys are now gone from the agent pool.
        assert!(agent.take(&k1.key_id).is_none());
        assert!(agent.take(&k2.key_id).is_none());
        assert!(agent.is_empty());
    }

    /// A key_id cannot be consumed twice server-side (consume removes it).
    #[test]
    fn a_key_cannot_be_consumed_twice() {
        let mut agent = AgentKeyPool::new();
        let entries = agent.mint(1);
        let key_id = entries[0].key_id.clone();
        let mut server = ServerKeyPool::from_entries(entries);

        let first = server.consume().expect("first consume");
        assert_eq!(first.key_id, key_id);
        assert!(
            server.consume().is_none(),
            "the same key must not be consumable again"
        );
    }

    /// Pool exhaustion fails cleanly (consume → None, no reuse); replenish
    /// restores capacity.
    #[test]
    fn pool_exhaustion_then_replenish() {
        let mut agent = AgentKeyPool::new();
        let mut server = ServerKeyPool::from_entries(agent.mint(1));

        assert!(server.consume().is_some());
        assert!(server.consume().is_none(), "exhausted pool yields no key");
        assert_eq!(
            server.replenish_deficit(3),
            3,
            "server should ask for 3 keys"
        );

        // Agent tops up; consumption works again.
        let added = server.replenish(agent.mint(2));
        assert_eq!(added, 2);
        assert!(server.consume().is_some());
        assert_eq!(server.replenish_deficit(3), 2);
    }

    /// Replenish de-dupes a retried top-up by key_id (never double-inserts).
    #[test]
    fn replenish_dedupes_by_key_id() {
        let mut agent = AgentKeyPool::new();
        let entries = agent.mint(2);
        let mut server = ServerKeyPool::new();
        assert_eq!(server.replenish(entries.clone()), 2);
        assert_eq!(server.replenish(entries), 0, "retry adds nothing");
        assert_eq!(server.len(), 2);
    }

    /// End-to-end over the actual `WorkPacket` wire shape: server consumes a
    /// pooled key, wraps, stamps `secret_key_id`; the agent looks the key up,
    /// unwraps ONCE, and `Context::secret` returns the value. Only ciphertext +
    /// key_id cross the wire — no plaintext in the serialized packet.
    #[tokio::test]
    async fn pool_end_to_end_only_ciphertext_and_key_id_on_wire() {
        use crate::fleet::{ArtifactRef, WorkPacket, AGENT_PROTOCOL_VERSION};

        let mut agent = AgentKeyPool::new();
        let mut server = ServerKeyPool::from_entries(agent.mint(1));

        // Server side: consume a key, wrap the secret to it, build the packet.
        let key = server.consume().expect("a pooled key");
        let wrapped = wrap_field_map(
            "db_prod",
            &field_map(&[("password", "hunter2")]),
            "exec-9",
            &decode_pk(&key),
        )
        .unwrap();
        let packet = WorkPacket {
            protocol_version: AGENT_PROTOCOL_VERSION,
            task_execution_id: "exec-9".into(),
            workflow_execution_id: "w1".into(),
            task_name: "ns::task".into(),
            attempt: 1,
            context: serde_json::json!({}),
            artifact: ArtifactRef {
                digest: "d".into(),
                fetch_url: "/x".into(),
                build_target_triple: "aarch64-apple-darwin".into(),
            },
            timeout_seconds: 60,
            tenant_id: None,
            language: None,
            wrapped_secrets: vec![wrapped],
            secret_key_id: Some(key.key_id.clone()),
        };

        // Wire assertion: the plaintext value must not appear anywhere in the
        // serialized packet (NFR-001/NFR-003).
        let json = serde_json::to_string(&packet).unwrap();
        assert!(!json.contains("hunter2"), "plaintext leaked onto the wire");
        assert!(json.contains(&key.key_id), "packet must carry the key_id");

        // Agent side: look up the key by the packet's `secret_key_id`, take it
        // (one-time), unwrap into the resolver, serve through Context.
        let key_id = packet.secret_key_id.as_deref().unwrap();
        let priv_key = agent
            .take(key_id)
            .expect("agent holds the pooled private key");
        let resolver = InMemorySecretResolver::from_wrapped(
            &priv_key,
            &packet.wrapped_secrets,
            &packet.task_execution_id,
        )
        .unwrap();
        let ctx = Context::<serde_json::Value>::new().with_secret_resolver(resolver.into_arc());
        assert_eq!(
            ctx.secret("db_prod")
                .await
                .unwrap()
                .get("password")
                .unwrap(),
            "hunter2"
        );

        // The key is now spent — a replay of the same packet can't re-unwrap.
        assert!(agent.take(key_id).is_none());
    }

    /// The server extracts the concrete secret names from the T-0859 `$secret`
    /// alias map (values), deduped + sorted; absent/empty map ⇒ no names.
    #[test]
    fn secret_ref_names_reads_the_alias_map() {
        let ctx = serde_json::json!({
            "some_param": 1,
            cloacina_workflow::secret::SECRET_REFS_KEY: {
                "db": "db_prod",
                "also_db": "db_prod",
                "api": "stripe",
            }
        });
        assert_eq!(
            secret_ref_names(&ctx),
            vec!["db_prod".to_string(), "stripe".to_string()]
        );

        // No alias map → no names.
        assert!(secret_ref_names(&serde_json::json!({"x": 1})).is_empty());
    }

    /// An unknown key_id (server consumed a key the agent no longer has, e.g.
    /// after a restart) is a clean lookup miss — never a silent run with missing
    /// secrets.
    #[test]
    fn unknown_key_id_is_a_clean_miss() {
        let mut agent = AgentKeyPool::new();
        let _ = agent.mint(1);
        assert!(agent.take("never-minted-this").is_none());
    }

    #[tokio::test]
    async fn in_memory_resolver_reports_missing() {
        let resolver = InMemorySecretResolver::empty();
        assert!(matches!(
            resolver.resolve("absent").await.unwrap_err(),
            SecretResolverError::NotFound(_)
        ));
    }

    // DB-backed: uses a sqlite in-memory store, so gate to the sqlite backend
    // (matches the db_key_manager.rs convention — the postgres-only lane has no
    // sqlite backend compiled in and would panic in Database::new).
    #[cfg(feature = "sqlite")]
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
