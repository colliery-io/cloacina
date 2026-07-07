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
