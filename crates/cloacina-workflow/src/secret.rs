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

//! Secret resolution side channel (CLOACI-I-0133 / T-0858, design D-1).
//!
//! A task/constructor reads a resolved secret through [`Context::secret`] — a
//! dedicated accessor on the execution scope that is **structurally distinct**
//! from the durable [`Context`](crate::Context) data. The resolved plaintext is
//! *returned* to the task; it is never inserted into the context's serialized
//! `data` map, so it can never land in `schedules.params`, the fires log, audit
//! rows, or execution history (NFR-001).
//!
//! This module defines only the trait + error types that live in the authoring
//! crate. The concrete backend (which decrypts against the tenant-scoped
//! `SecretStore`) lives in the `cloacina` runtime crate as `SecretStoreResolver`
//! and is threaded onto the `Context` by the executor at fire time.

use async_trait::async_trait;
use std::collections::BTreeMap;
use thiserror::Error;

/// Reserved `Context` data key holding the instance's `{"$secret": name}` binding
/// map (CLOACI-I-0133 / T-0859, design D-4).
///
/// At fire time `merge_instance_params` recognizes a `{"$secret": "name"}` param
/// value, keeps the **resolved value** out of the context entirely, and records
/// only the non-sensitive `local_binding_name -> secret_name` alias here. The map
/// carries NAMES ONLY (never values), so it is safe to serialize into the durable
/// context; it survives the fire → persist → execute boundary and lets
/// [`Context::secret`](crate::Context::secret) resolve a task's declared local
/// binding name to the concrete secret the instance chose.
pub const SECRET_REFS_KEY: &str = "__cloacina_secret_refs__";

/// Error returned by a [`SecretResolver`] backend implementation.
#[derive(Debug, Error)]
pub enum SecretResolverError {
    /// No secret of that name is visible to this tenant/scope.
    #[error("secret not found: {0}")]
    NotFound(String),

    /// The name is not in this scope's granted secret allow-list
    /// (CLOACI-I-0133 / T-0860, design D-3). Returned **before** any decryption —
    /// the holder was never authorized to resolve this secret, regardless of
    /// whether it exists. Distinct from [`NotFound`](Self::NotFound) so a denial
    /// is not confusable with a missing secret in audit/logs.
    #[error("secret not granted: {0}")]
    NotGranted(String),

    /// The backend failed to resolve (decrypt failure, DB error, misconfigured
    /// KEK, …). The message is a redacted, non-plaintext description.
    #[error("secret backend error: {0}")]
    Backend(String),
}

/// Error surfaced to a task body by the [`Context`](crate::Context) secret
/// accessor.
#[derive(Debug, Error)]
pub enum SecretAccessError {
    /// No resolver was configured on this execution scope. On the embedded /
    /// in-process path the host/runner wires one in; when it is absent,
    /// `context.secret(...)` fails clearly instead of silently returning empty.
    #[error("secrets backend not configured for this execution scope")]
    NotConfigured,

    /// The named secret does not exist (or is not visible to this tenant).
    #[error("secret not found: {0}")]
    NotFound(String),

    /// The execution scope's grant does not include this secret name
    /// (CLOACI-I-0133 / T-0860, D-3). The resolver denied it **before** any
    /// decrypt; add the name to the constructor's `secrets` grant to allow it.
    #[error("secret not granted: {0}")]
    NotGranted(String),

    /// The secret exists but has no field of that name.
    #[error("secret '{secret}' has no field '{field}'")]
    FieldNotFound { secret: String, field: String },

    /// The backend failed to resolve the secret.
    #[error("secret backend error: {0}")]
    Backend(String),
}

/// A backend that resolves a named secret into its plaintext `{field: value}`
/// map at fire time.
///
/// Implementations decrypt at the last possible moment and return the fields to
/// the caller; they MUST NOT persist or log the plaintext. The runtime attaches
/// a resolver to the [`Context`](crate::Context) via a non-serialized handle
/// (see [`Context::set_secret_resolver`](crate::Context::set_secret_resolver)),
/// which is what keeps resolution structurally separate from the durable
/// context.
#[async_trait]
pub trait SecretResolver: Send + Sync {
    /// Resolve `name` to its decrypted `{field: value}` map.
    async fn resolve(&self, name: &str) -> Result<BTreeMap<String, String>, SecretResolverError>;
}

/// In-memory resolver over already-resolved secret values, keyed by concrete
/// secret name (CLOACI-T-0895).
///
/// The packaged-task bridge uses this on the PLUGIN side: the host resolves
/// every `{"$secret"}`-referenced secret through its real backend before the
/// plugin call and ships the values across the boundary in the
/// `TaskExecutionRequest`; the plugin shell rebuilds the execution scope with
/// this resolver so `context.secret(...)` works identically inside the
/// package. Values live only in this object for the duration of one task
/// invocation — never serialized into the durable context (NFR-001).
pub struct MapSecretResolver {
    secrets: BTreeMap<String, BTreeMap<String, String>>,
}

impl MapSecretResolver {
    /// Wrap a `{secret_name: {field: value}}` map.
    pub fn new(secrets: BTreeMap<String, BTreeMap<String, String>>) -> Self {
        Self { secrets }
    }
}

// Values must never appear in logs; a manual Debug keeps names only.
impl std::fmt::Debug for MapSecretResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapSecretResolver")
            .field("names", &self.secrets.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[async_trait]
impl SecretResolver for MapSecretResolver {
    async fn resolve(&self, name: &str) -> Result<BTreeMap<String, String>, SecretResolverError> {
        self.secrets
            .get(name)
            .cloned()
            .ok_or_else(|| SecretResolverError::NotFound(name.to_string()))
    }
}
