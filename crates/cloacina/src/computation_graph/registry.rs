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

//! Endpoint registry — maps accumulator/reactor names to their channel senders.
//!
//! The WebSocket handlers look up names in this registry to route messages to
//! the correct process. Supports broadcast: multiple accumulators registered
//! under the same name all receive the message.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use serde::{Deserialize, Serialize};

use super::accumulator::{AccumulatorHealth, FreshnessHandle};
use super::reactor::{ManualCommand, ReactorHandle};
use tokio::sync::watch;

/// Errors from registry operations.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("no accumulator registered for '{0}'")]
    AccumulatorNotFound(String),

    #[error("no reactor registered for '{0}'")]
    ReactorNotFound(String),

    #[error("failed to send to accumulator '{name}': channel closed")]
    AccumulatorSendFailed { name: String },

    #[error("failed to send to reactor '{name}': channel closed")]
    ReactorSendFailed { name: String },

    #[error("not authorized for accumulator '{0}'")]
    AccumulatorUnauthorized(String),

    #[error("not authorized for reactor '{0}'")]
    ReactorUnauthorized(String),

    #[error("operation '{op}' not permitted on reactor '{name}'")]
    OperationNotPermitted { name: String, op: String },

    /// CLOACI-T-0921: two different owners (packages/reactors) inside the SAME
    /// tenant tried to claim the same endpoint name. This is a load-time
    /// rejection, not a silent cross-wire — the second package must rename.
    #[error(
        "{kind} '{name}' is already registered in tenant '{tenant}' by {existing}; \
         {incoming} cannot claim the same name — rename the {kind} in one of the two packages"
    )]
    EndpointOwnershipConflict {
        kind: &'static str,
        name: String,
        tenant: String,
        existing: String,
        incoming: String,
    },

    /// CLOACI-T-0921: an admin (tenant-less) caller named an endpoint that
    /// exists in more than one tenant. Refuse rather than guess.
    #[error(
        "{kind} '{name}' is ambiguous across tenants {tenants:?}; scope the request to a tenant"
    )]
    AmbiguousEndpoint {
        kind: &'static str,
        name: String,
        tenants: Vec<String>,
    },
}

/// Operations that can be performed on a reactor via WebSocket.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactorOp {
    ForceFire,
    FireWith,
    GetState,
    Pause,
    Resume,
    GetHealth,
}

/// Caller identity for authorization checks.
pub struct KeyContext<'a> {
    pub key_id: &'a uuid::Uuid,
    pub tenant_id: Option<&'a str>,
    pub is_admin: bool,
}

/// Authorization policy for an accumulator endpoint.
#[derive(Debug, Clone, Default)]
pub struct AccumulatorAuthPolicy {
    /// If true, any authenticated key is authorized (single-tenant default).
    pub allow_all_authenticated: bool,
    /// Tenant IDs whose keys are authorized. Checked when allow_all is false.
    pub allowed_tenants: Vec<String>,
    /// PAK key IDs authorized to push to this accumulator (explicit override).
    pub allowed_producers: Vec<uuid::Uuid>,
}

/// Authorization policy for a reactor endpoint.
#[derive(Debug, Clone, Default)]
pub struct ReactorAuthPolicy {
    /// If true, any authenticated key is authorized (single-tenant default).
    pub allow_all_authenticated: bool,
    /// Tenant IDs whose keys are authorized. Checked when allow_all is false.
    pub allowed_tenants: Vec<String>,
    /// PAK key IDs authorized to connect (explicit override).
    pub allowed_operators: Vec<uuid::Uuid>,
    /// Per-key operation restrictions. If a key is in allowed_operators
    /// but not in this map, all operations are permitted.
    pub operation_permissions: HashMap<uuid::Uuid, Vec<ReactorOp>>,
}

impl AccumulatorAuthPolicy {
    /// Create a policy that allows any authenticated key (global/single-tenant).
    pub fn allow_all() -> Self {
        Self {
            allow_all_authenticated: true,
            allowed_tenants: Vec::new(),
            allowed_producers: Vec::new(),
        }
    }

    /// Create a policy scoped to a specific tenant.
    pub fn for_tenant(tenant_id: &str) -> Self {
        Self {
            allow_all_authenticated: false,
            allowed_tenants: vec![tenant_id.to_string()],
            allowed_producers: Vec::new(),
        }
    }

    /// Check if a key is authorized.
    pub fn is_authorized(&self, ctx: &KeyContext) -> bool {
        if self.allow_all_authenticated || ctx.is_admin {
            return true;
        }
        if self.allowed_producers.contains(ctx.key_id) {
            return true;
        }
        if let Some(key_tenant) = ctx.tenant_id {
            return self.allowed_tenants.iter().any(|t| t == key_tenant);
        }
        false
    }
}

impl ReactorAuthPolicy {
    /// Create a policy that allows any authenticated key (global/single-tenant).
    pub fn allow_all() -> Self {
        Self {
            allow_all_authenticated: true,
            allowed_tenants: Vec::new(),
            allowed_operators: Vec::new(),
            operation_permissions: HashMap::new(),
        }
    }

    /// Create a policy scoped to a specific tenant.
    pub fn for_tenant(tenant_id: &str) -> Self {
        Self {
            allow_all_authenticated: false,
            allowed_tenants: vec![tenant_id.to_string()],
            allowed_operators: Vec::new(),
            operation_permissions: HashMap::new(),
        }
    }

    /// Check if a key is authorized to connect.
    pub fn is_authorized(&self, ctx: &KeyContext) -> bool {
        if self.allow_all_authenticated || ctx.is_admin {
            return true;
        }
        if self.allowed_operators.contains(ctx.key_id) {
            return true;
        }
        if let Some(key_tenant) = ctx.tenant_id {
            return self.allowed_tenants.iter().any(|t| t == key_tenant);
        }
        false
    }

    /// Check if a key is authorized for a specific operation.
    pub fn is_operation_permitted(&self, ctx: &KeyContext, op: &ReactorOp) -> bool {
        if self.allow_all_authenticated || ctx.is_admin {
            return true;
        }
        if !self.is_authorized(ctx) {
            return false;
        }
        // If no per-key restrictions, all ops are allowed
        match self.operation_permissions.get(ctx.key_id) {
            None => true,
            Some(permitted) => permitted.contains(op),
        }
    }
}

/// Discoverability metadata an accumulator self-registers at graph load
/// (CLOACI-I-0128 follow-up). The runtime channel registration
/// ([`EndpointRegistry::register_accumulator`]) only carries the socket sender;
/// this descriptor adds the structural context the discovery API needs — chiefly
/// the reactor/graph the accumulator feeds — so an operator can tell what
/// connecting to `/v1/ws/accumulator/{name}` actually drives.
#[derive(Debug, Clone)]
pub struct AccumulatorDescriptor {
    /// The reactor (graph) this accumulator feeds.
    pub reactor: String,
    /// Owning tenant, or `None` for untagged single-tenant graphs.
    pub tenant_id: Option<String>,
}

/// Composite lookup key for every endpoint map (CLOACI-T-0921).
///
/// Tenant is THE isolation boundary, so it is part of the *key*, not payload
/// hanging off the value. `tenant_id: None` is the untenanted/embedded
/// single-tenant case (and keeps the pre-multi-tenancy allow-all behaviour).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EndpointKey {
    pub tenant_id: Option<String>,
    pub name: String,
}

impl EndpointKey {
    pub fn new(tenant_id: Option<&str>, name: &str) -> Self {
        Self {
            tenant_id: tenant_id.map(|t| t.to_string()),
            name: name.to_string(),
        }
    }

    fn tenant_label(&self) -> &str {
        self.tenant_id.as_deref().unwrap_or("<untenanted>")
    }
}

/// Full ownership identity recorded on each registration (CLOACI-T-0921).
///
/// The key is `(tenant, name)`; this is the rest of the provenance. It exists
/// so that (a) a same-tenant same-name claim by a *different* package/reactor
/// is rejected loudly at load time instead of silently cross-wiring, and
/// (b) deregistration only ever removes the entry its own owner installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointOwner {
    /// Owning tenant, or `None` for untenanted/embedded graphs.
    pub tenant_id: Option<String>,
    /// Owning package, when the loader knows it. `None` for hand-wired and
    /// embedded graphs.
    pub package: Option<String>,
    /// Owning reactor (the graph this endpoint belongs to).
    pub reactor: String,
}

impl EndpointOwner {
    pub fn new(
        tenant_id: Option<String>,
        package: Option<String>,
        reactor: impl Into<String>,
    ) -> Self {
        Self {
            tenant_id,
            package,
            reactor: reactor.into(),
        }
    }

    /// An owner with no tenant and no package — embedded/hand-wired graphs.
    pub fn embedded(reactor: impl Into<String>) -> Self {
        Self::new(None, None, reactor)
    }

    /// The map key this owner registers `name` under.
    pub fn key(&self, name: &str) -> EndpointKey {
        EndpointKey {
            tenant_id: self.tenant_id.clone(),
            name: name.to_string(),
        }
    }

    /// Operator-facing provenance string for conflict errors.
    fn describe(&self) -> String {
        match &self.package {
            Some(pkg) => format!("package '{}' (reactor '{}')", pkg, self.reactor),
            None => format!("reactor '{}'", self.reactor),
        }
    }
}

/// Tenant scope of a *caller* performing an endpoint lookup (CLOACI-T-0921).
///
/// The WebSocket and REST routes build this from the authenticated key so a
/// bare `{name}` path segment resolves inside the caller's own tenant.
#[derive(Debug, Clone, Copy, Default)]
pub struct EndpointScope<'a> {
    pub tenant_id: Option<&'a str>,
    pub is_admin: bool,
}

impl<'a> EndpointScope<'a> {
    /// A non-admin caller in `tenant_id` (`None` = untenanted deployment).
    pub fn of(tenant_id: Option<&'a str>) -> Self {
        Self {
            tenant_id,
            is_admin: false,
        }
    }

    /// A non-admin caller in a specific tenant.
    pub fn tenant(tenant_id: &'a str) -> Self {
        Self::of(Some(tenant_id))
    }

    /// The untenanted (embedded / single-tenant) caller.
    pub fn untenanted() -> Self {
        Self::of(None)
    }

    /// An admin caller: may reach any tenant's endpoint, but only when the
    /// name is unambiguous across tenants.
    pub fn admin() -> Self {
        Self {
            tenant_id: None,
            is_admin: true,
        }
    }

    /// Derive the lookup scope from an authenticated key's context.
    pub fn from_key_context(ctx: &KeyContext<'a>) -> Self {
        Self {
            tenant_id: ctx.tenant_id,
            is_admin: ctx.is_admin,
        }
    }
}

/// Why a scoped lookup failed to land on exactly one key.
enum ResolveMiss {
    NotFound,
    Ambiguous(Vec<String>),
}

/// Resolve a bare `name` to a concrete [`EndpointKey`] within `scope`.
///
/// Order:
/// 1. the caller's own tenant — the normal multi-tenant path;
/// 2. the untenanted entry (`tenant_id: None`) — embedded/pre-tenancy graphs,
///    which carry an allow-all policy and were always globally addressable;
/// 3. admins only: a unique cross-tenant match. Two tenants owning the same
///    name is [`ResolveMiss::Ambiguous`], never a guess.
///
/// A non-admin caller can therefore never resolve another tenant's endpoint,
/// which is the whole point of CLOACI-T-0921.
fn resolve_key<V>(
    map: &HashMap<EndpointKey, V>,
    scope: EndpointScope<'_>,
    name: &str,
) -> Result<EndpointKey, ResolveMiss> {
    if let Some(tenant) = scope.tenant_id {
        let key = EndpointKey::new(Some(tenant), name);
        if map.contains_key(&key) {
            return Ok(key);
        }
    }

    let untenanted = EndpointKey::new(None, name);
    if map.contains_key(&untenanted) {
        return Ok(untenanted);
    }

    if scope.is_admin {
        let matches: Vec<&EndpointKey> = map.keys().filter(|k| k.name == name).collect();
        match matches.len() {
            0 => return Err(ResolveMiss::NotFound),
            1 => return Ok(matches[0].clone()),
            _ => {
                return Err(ResolveMiss::Ambiguous(
                    matches
                        .iter()
                        .map(|k| k.tenant_label().to_string())
                        .collect(),
                ))
            }
        }
    }

    Err(ResolveMiss::NotFound)
}

/// An accumulator registration: its owner plus the socket senders fanned out
/// under this `(tenant, name)` key.
struct AccumulatorEntry {
    owner: EndpointOwner,
    senders: Vec<mpsc::Sender<Vec<u8>>>,
}

/// A reactor registration: its owner, manual command channel, and shared handle.
struct ReactorEntry {
    owner: EndpointOwner,
    sender: mpsc::Sender<ManualCommand>,
    handle: ReactorHandle,
}

/// Registry mapping `(tenant, endpoint name)` to channel senders.
///
/// Shared between the computation graph scheduler (registers on spawn) and
/// WebSocket handlers (look up on message receipt). One instance serves the
/// whole server process, so every map is keyed by [`EndpointKey`] —
/// CLOACI-T-0921: keying by bare name let same-named accumulators from two
/// tenants broadcast into each other's boundary channels.
#[derive(Clone)]
pub struct EndpointRegistry {
    inner: Arc<RwLock<RegistryInner>>,
}

struct RegistryInner {
    /// `(tenant, accumulator name)` → owner + socket senders (Vec for broadcast).
    accumulators: HashMap<EndpointKey, AccumulatorEntry>,
    /// `(tenant, reactor name)` → owner + manual command sender + shared handle.
    reactors: HashMap<EndpointKey, ReactorEntry>,
    /// `(tenant, accumulator name)` → auth policy.
    accumulator_policies: HashMap<EndpointKey, AccumulatorAuthPolicy>,
    /// `(tenant, reactor name)` → auth policy.
    reactor_policies: HashMap<EndpointKey, ReactorAuthPolicy>,
    /// `(tenant, accumulator name)` → health watch receiver.
    accumulator_health: HashMap<EndpointKey, watch::Receiver<AccumulatorHealth>>,
    /// `(tenant, accumulator name)` → freshness probe (events_total + last-event), CLOACI-T-0765.
    accumulator_freshness: HashMap<EndpointKey, FreshnessHandle>,
    /// `(tenant, accumulator name)` → discoverability descriptor (CLOACI-I-0128 follow-up).
    accumulator_meta: HashMap<EndpointKey, AccumulatorDescriptor>,
    /// `(tenant, accumulator name)` → `(operator-inject count, last-inject epoch ms)`
    /// (CLOACI-T-0776). Every `send_to_accumulator` is an operator inject — real
    /// source events arrive on the accumulator's own socket, not here — so this
    /// counts manual interventions for the UI to mark.
    accumulator_injects: HashMap<EndpointKey, (u64, i64)>,
}

impl EndpointRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RegistryInner {
                accumulators: HashMap::new(),
                reactors: HashMap::new(),
                accumulator_policies: HashMap::new(),
                reactor_policies: HashMap::new(),
                accumulator_health: HashMap::new(),
                accumulator_freshness: HashMap::new(),
                accumulator_meta: HashMap::new(),
                accumulator_injects: HashMap::new(),
            })),
        }
    }

    /// Record an operator inject into an accumulator (CLOACI-T-0776). Called only
    /// from the REST inject endpoint — the true human-operator path. (The WS
    /// accumulator-push path also goes through `send_to_accumulator` but is the
    /// data-source feed, e.g. the demo producer, so it must NOT count here.)
    pub async fn note_accumulator_operator_inject(&self, name: &str, scope: EndpointScope<'_>) {
        let now = chrono::Utc::now().timestamp_millis();
        let mut inner = self.inner.write().await;
        let Ok(key) = resolve_key(&inner.accumulators, scope, name) else {
            return;
        };
        let entry = inner.accumulator_injects.entry(key).or_insert((0, 0));
        entry.0 += 1;
        entry.1 = now;
    }

    /// `(operator-inject count, last-inject epoch ms)` for an accumulator, or
    /// `None` if it has never been operator-injected (CLOACI-T-0776).
    pub async fn accumulator_inject_stat(
        &self,
        name: &str,
        scope: EndpointScope<'_>,
    ) -> Option<(u64, i64)> {
        let inner = self.inner.read().await;
        let key = resolve_key(&inner.accumulators, scope, name).ok()?;
        inner.accumulator_injects.get(&key).copied()
    }

    /// Register an accumulator's socket sender under `(owner.tenant, name)`.
    ///
    /// Several accumulator *instances* belonging to the same owner may share a
    /// name — messages are broadcast to all of them, and the restart path
    /// re-registers this way. A *different* owner claiming an already-taken
    /// `(tenant, name)` is rejected with
    /// [`RegistryError::EndpointOwnershipConflict`] (CLOACI-T-0921): silently
    /// appending is what let two packages cross-wire each other's boundary
    /// channels.
    pub async fn register_accumulator(
        &self,
        owner: &EndpointOwner,
        name: String,
        sender: mpsc::Sender<Vec<u8>>,
    ) -> Result<(), RegistryError> {
        let key = owner.key(&name);
        let mut inner = self.inner.write().await;
        match inner.accumulators.get_mut(&key) {
            Some(entry) => {
                if &entry.owner != owner {
                    return Err(RegistryError::EndpointOwnershipConflict {
                        kind: "accumulator",
                        name,
                        tenant: key.tenant_label().to_string(),
                        existing: entry.owner.describe(),
                        incoming: owner.describe(),
                    });
                }
                entry.senders.push(sender);
            }
            None => {
                inner.accumulators.insert(
                    key,
                    AccumulatorEntry {
                        owner: owner.clone(),
                        senders: vec![sender],
                    },
                );
            }
        }
        Ok(())
    }

    /// Register a reactor's manual command sender and shared handle under
    /// `(owner.tenant, name)`. Same ownership rule as
    /// [`register_accumulator`](Self::register_accumulator): a different owner
    /// claiming a live `(tenant, name)` is a load-time error.
    pub async fn register_reactor(
        &self,
        owner: &EndpointOwner,
        name: String,
        sender: mpsc::Sender<ManualCommand>,
        handle: ReactorHandle,
    ) -> Result<(), RegistryError> {
        let key = owner.key(&name);
        let mut inner = self.inner.write().await;
        if let Some(existing) = inner.reactors.get(&key) {
            if &existing.owner != owner {
                return Err(RegistryError::EndpointOwnershipConflict {
                    kind: "reactor",
                    name,
                    tenant: key.tenant_label().to_string(),
                    existing: existing.owner.describe(),
                    incoming: owner.describe(),
                });
            }
        }
        inner.reactors.insert(
            key,
            ReactorEntry {
                owner: owner.clone(),
                sender,
                handle,
            },
        );
        Ok(())
    }

    /// Register an accumulator's discoverability descriptor (its parent reactor
    /// + tenant), self-supplied by the graph at load. Keyed by
    /// `(tenant, name)`; a re-load by the same owner overwrites.
    /// (CLOACI-I-0128 follow-up.)
    pub async fn register_accumulator_meta(
        &self,
        owner: &EndpointOwner,
        name: String,
        descriptor: AccumulatorDescriptor,
    ) {
        let mut inner = self.inner.write().await;
        inner.accumulator_meta.insert(owner.key(&name), descriptor);
    }

    /// The discoverability descriptor for an accumulator visible to `scope`.
    pub async fn accumulator_descriptor(
        &self,
        name: &str,
        scope: EndpointScope<'_>,
    ) -> Option<AccumulatorDescriptor> {
        let inner = self.inner.read().await;
        let key = resolve_key(&inner.accumulator_meta, scope, name).ok()?;
        inner.accumulator_meta.get(&key).cloned()
    }

    /// Deregister the accumulator `owner` registered under `name`, with all of
    /// its side tables. A no-op when the live entry belongs to somebody else —
    /// CLOACI-T-0921: unloading tenant A's package must not tear down tenant
    /// B's same-named accumulator.
    pub async fn deregister_accumulator(&self, owner: &EndpointOwner, name: &str) {
        let key = owner.key(name);
        let mut inner = self.inner.write().await;
        match inner.accumulators.get(&key) {
            Some(entry) if &entry.owner == owner => {}
            Some(entry) => {
                tracing::warn!(
                    accumulator = %name,
                    tenant = %key.tenant_label(),
                    owner = %entry.owner.describe(),
                    requester = %owner.describe(),
                    "refusing to deregister an accumulator owned by another package"
                );
                return;
            }
            None => return,
        }
        inner.accumulators.remove(&key);
        inner.accumulator_meta.remove(&key);
        inner.accumulator_health.remove(&key);
        inner.accumulator_freshness.remove(&key);
        inner.accumulator_policies.remove(&key);
        inner.accumulator_injects.remove(&key);
    }

    /// Deregister the reactor `owner` registered under `name`. A no-op when the
    /// live entry belongs to somebody else (CLOACI-T-0921).
    pub async fn deregister_reactor(&self, owner: &EndpointOwner, name: &str) {
        let key = owner.key(name);
        let mut inner = self.inner.write().await;
        match inner.reactors.get(&key) {
            Some(entry) if &entry.owner == owner => {}
            Some(entry) => {
                tracing::warn!(
                    reactor = %name,
                    tenant = %key.tenant_label(),
                    owner = %entry.owner.describe(),
                    requester = %owner.describe(),
                    "refusing to deregister a reactor owned by another package"
                );
                return;
            }
            None => return,
        }
        inner.reactors.remove(&key);
        inner.reactor_policies.remove(&key);
    }

    /// Get a reactor's shared handle (for GetState/Pause/Resume), resolved
    /// within the caller's tenant scope.
    pub async fn get_reactor_handle(
        &self,
        name: &str,
        scope: EndpointScope<'_>,
    ) -> Option<ReactorHandle> {
        let inner = self.inner.read().await;
        let key = resolve_key(&inner.reactors, scope, name).ok()?;
        inner.reactors.get(&key).map(|e| e.handle.clone())
    }

    /// Set the auth policy for an accumulator endpoint.
    pub async fn set_accumulator_policy(
        &self,
        owner: &EndpointOwner,
        name: String,
        policy: AccumulatorAuthPolicy,
    ) {
        let mut inner = self.inner.write().await;
        inner.accumulator_policies.insert(owner.key(&name), policy);
    }

    /// Set the auth policy for a reactor endpoint.
    pub async fn set_reactor_policy(
        &self,
        owner: &EndpointOwner,
        name: String,
        policy: ReactorAuthPolicy,
    ) {
        let mut inner = self.inner.write().await;
        inner.reactor_policies.insert(owner.key(&name), policy);
    }

    /// Check if a key is authorized for an accumulator endpoint.
    ///
    /// Returns Ok(()) if authorized, Err if denied.
    /// Deny by default: no policy = no access.
    pub async fn check_accumulator_auth(
        &self,
        name: &str,
        ctx: &KeyContext<'_>,
    ) -> Result<(), RegistryError> {
        let inner = self.inner.read().await;
        let scope = EndpointScope::from_key_context(ctx);
        // CLOACI-T-0921: an unresolvable name (including another tenant's) is
        // indistinguishable from "no policy" — deny by default either way, and
        // never leak that the name exists in a tenant the caller can't see.
        let Ok(key) = resolve_key(&inner.accumulator_policies, scope, name) else {
            return Err(RegistryError::AccumulatorUnauthorized(name.to_string()));
        };
        match inner.accumulator_policies.get(&key) {
            None => Err(RegistryError::AccumulatorUnauthorized(name.to_string())),
            Some(policy) => {
                if policy.is_authorized(ctx) {
                    Ok(())
                } else {
                    Err(RegistryError::AccumulatorUnauthorized(name.to_string()))
                }
            }
        }
    }

    /// Check if a key is authorized for a reactor endpoint.
    pub async fn check_reactor_auth(
        &self,
        name: &str,
        ctx: &KeyContext<'_>,
    ) -> Result<(), RegistryError> {
        let inner = self.inner.read().await;
        let scope = EndpointScope::from_key_context(ctx);
        let Ok(key) = resolve_key(&inner.reactor_policies, scope, name) else {
            return Err(RegistryError::ReactorUnauthorized(name.to_string()));
        };
        match inner.reactor_policies.get(&key) {
            None => Err(RegistryError::ReactorUnauthorized(name.to_string())),
            Some(policy) => {
                if policy.is_authorized(ctx) {
                    Ok(())
                } else {
                    Err(RegistryError::ReactorUnauthorized(name.to_string()))
                }
            }
        }
    }

    /// Check if a key is authorized for a specific reactor operation.
    pub async fn check_reactor_op_auth(
        &self,
        name: &str,
        ctx: &KeyContext<'_>,
        op: &ReactorOp,
    ) -> Result<(), RegistryError> {
        let inner = self.inner.read().await;
        let scope = EndpointScope::from_key_context(ctx);
        let Ok(key) = resolve_key(&inner.reactor_policies, scope, name) else {
            return Err(RegistryError::ReactorUnauthorized(name.to_string()));
        };
        match inner.reactor_policies.get(&key) {
            None => Err(RegistryError::ReactorUnauthorized(name.to_string())),
            Some(policy) => {
                if policy.is_operation_permitted(ctx, op) {
                    Ok(())
                } else {
                    Err(RegistryError::OperationNotPermitted {
                        name: name.to_string(),
                        op: format!("{:?}", op),
                    })
                }
            }
        }
    }

    /// Send bytes to all accumulators registered under `name`.
    ///
    /// Returns error if no accumulators are registered, or if all channels
    /// are closed. Channels that are closed are pruned on send.
    pub async fn send_to_accumulator(
        &self,
        name: &str,
        scope: EndpointScope<'_>,
        bytes: Vec<u8>,
    ) -> Result<usize, RegistryError> {
        let mut inner = self.inner.write().await;
        let key = match resolve_key(&inner.accumulators, scope, name) {
            Ok(k) => k,
            Err(ResolveMiss::NotFound) => {
                return Err(RegistryError::AccumulatorNotFound(name.to_string()))
            }
            Err(ResolveMiss::Ambiguous(tenants)) => {
                return Err(RegistryError::AmbiguousEndpoint {
                    kind: "accumulator",
                    name: name.to_string(),
                    tenants,
                })
            }
        };
        let senders = &mut inner
            .accumulators
            .get_mut(&key)
            .ok_or_else(|| RegistryError::AccumulatorNotFound(name.to_string()))?
            .senders;

        if senders.is_empty() {
            return Err(RegistryError::AccumulatorNotFound(name.to_string()));
        }

        let mut sent = 0;
        let mut closed = Vec::new();

        for (i, sender) in senders.iter().enumerate() {
            match sender.try_send(bytes.clone()) {
                Ok(()) => sent += 1,
                Err(mpsc::error::TrySendError::Closed(_)) => closed.push(i),
                Err(mpsc::error::TrySendError::Full(_)) => {
                    // Channel full — log but count as sent (data will be dropped)
                    tracing::warn!(
                        accumulator = %name,
                        "accumulator channel full, dropping message"
                    );
                }
            }
        }

        // Prune closed channels (reverse order to preserve indices)
        for i in closed.into_iter().rev() {
            senders.remove(i);
        }

        if sent == 0 {
            return Err(RegistryError::AccumulatorSendFailed {
                name: name.to_string(),
            });
        }

        Ok(sent)
    }

    /// Send a manual command to a reactor.
    pub async fn send_to_reactor(
        &self,
        name: &str,
        scope: EndpointScope<'_>,
        command: ManualCommand,
    ) -> Result<(), RegistryError> {
        let inner = self.inner.read().await;
        let key = match resolve_key(&inner.reactors, scope, name) {
            Ok(k) => k,
            Err(ResolveMiss::NotFound) => {
                return Err(RegistryError::ReactorNotFound(name.to_string()))
            }
            Err(ResolveMiss::Ambiguous(tenants)) => {
                return Err(RegistryError::AmbiguousEndpoint {
                    kind: "reactor",
                    name: name.to_string(),
                    tenants,
                })
            }
        };
        let sender = &inner
            .reactors
            .get(&key)
            .ok_or_else(|| RegistryError::ReactorNotFound(name.to_string()))?
            .sender;

        sender
            .send(command)
            .await
            .map_err(|_| RegistryError::ReactorSendFailed {
                name: name.to_string(),
            })
    }

    /// List all registered accumulator names, across every tenant. Operator/
    /// diagnostic surface only — the tenant-filtered listing is
    /// [`list_accumulators_with_health_for_key`](Self::list_accumulators_with_health_for_key).
    pub async fn list_accumulators(&self) -> Vec<String> {
        let inner = self.inner.read().await;
        inner.accumulators.keys().map(|k| k.name.clone()).collect()
    }

    /// List all registered reactor names, across every tenant.
    pub async fn list_reactors(&self) -> Vec<String> {
        let inner = self.inner.read().await;
        inner.reactors.keys().map(|k| k.name.clone()).collect()
    }

    /// List every registered accumulator as its full `(tenant, name)` key.
    pub async fn list_accumulator_keys(&self) -> Vec<EndpointKey> {
        let inner = self.inner.read().await;
        inner.accumulators.keys().cloned().collect()
    }

    /// List every registered reactor as its full `(tenant, name)` key.
    pub async fn list_reactor_keys(&self) -> Vec<EndpointKey> {
        let inner = self.inner.read().await;
        inner.reactors.keys().cloned().collect()
    }

    /// Get the number of accumulator senders registered under a name, as
    /// visible to `scope`.
    pub async fn accumulator_count(&self, name: &str, scope: EndpointScope<'_>) -> usize {
        let inner = self.inner.read().await;
        let Ok(key) = resolve_key(&inner.accumulators, scope, name) else {
            return 0;
        };
        inner
            .accumulators
            .get(&key)
            .map(|e| e.senders.len())
            .unwrap_or(0)
    }

    /// Register a health watch receiver for an accumulator.
    pub async fn register_accumulator_health(
        &self,
        owner: &EndpointOwner,
        name: String,
        health_rx: watch::Receiver<AccumulatorHealth>,
    ) {
        let mut inner = self.inner.write().await;
        inner.accumulator_health.insert(owner.key(&name), health_rx);
    }

    /// Register a freshness probe for an accumulator (CLOACI-T-0765): the shared
    /// events_total + last-event handle off its `BoundarySender`.
    pub async fn register_accumulator_freshness(
        &self,
        owner: &EndpointOwner,
        name: String,
        handle: FreshnessHandle,
    ) {
        let mut inner = self.inner.write().await;
        inner.accumulator_freshness.insert(owner.key(&name), handle);
    }

    /// Get the current health of an accumulator visible to `scope`.
    pub async fn get_accumulator_health(
        &self,
        name: &str,
        scope: EndpointScope<'_>,
    ) -> Option<AccumulatorHealth> {
        let inner = self.inner.read().await;
        let key = resolve_key(&inner.accumulator_health, scope, name).ok()?;
        inner
            .accumulator_health
            .get(&key)
            .map(|rx| rx.borrow().clone())
    }

    /// List all accumulators with their current health + freshness (CLOACI-T-0765).
    pub async fn list_accumulators_with_health(
        &self,
    ) -> Vec<(String, AccumulatorHealth, Option<FreshnessHandle>)> {
        let inner = self.inner.read().await;
        inner
            .accumulators
            .keys()
            .map(|key| {
                let health = inner
                    .accumulator_health
                    .get(key)
                    .map(|rx| rx.borrow().clone())
                    .unwrap_or(AccumulatorHealth::Live); // default for accumulators without health tracking
                let fresh = inner.accumulator_freshness.get(key).cloned();
                (key.name.clone(), health, fresh)
            })
            .collect()
    }

    /// CLOACI-T-0579: list accumulators authorized for the given caller.
    /// Filters by each accumulator's `AccumulatorAuthPolicy::is_authorized`
    /// against the caller's `KeyContext`. Admin keys see everything;
    /// tenant-scoped keys see only their own accumulators; producer-pin
    /// keys see whichever they're explicitly allowed on.
    ///
    /// Closes SEC-05 (cross-tenant health enumeration). The route handler
    /// at `/v1/health/accumulators` uses this instead of the unfiltered
    /// `list_accumulators_with_health` so tenant B can't enumerate tenant
    /// A's accumulator names.
    pub async fn list_accumulators_with_health_for_key(
        &self,
        ctx: &KeyContext<'_>,
    ) -> Vec<(String, AccumulatorHealth, Option<FreshnessHandle>)> {
        let inner = self.inner.read().await;
        inner
            .accumulators
            .keys()
            .filter(|key| {
                // CLOACI-T-0921: the key's own tenant is the first gate — an
                // entry owned by another tenant is invisible regardless of
                // policy. Untenanted entries stay globally visible (that is the
                // pre-tenancy single-tenant deployment).
                let tenant_visible = match (&key.tenant_id, ctx.tenant_id) {
                    (None, _) => true,
                    (Some(_), _) if ctx.is_admin => true,
                    (Some(owner), Some(caller)) => owner == caller,
                    (Some(_), None) => false,
                };
                if !tenant_visible {
                    return false;
                }
                // Then apply the per-accumulator policy. Accumulators without a
                // policy entry default to `allow_all_authenticated` so the
                // pre-tenancy single-tenant deployments aren't suddenly
                // empty after this change.
                match inner.accumulator_policies.get(*key) {
                    Some(policy) => policy.is_authorized(ctx),
                    None => AccumulatorAuthPolicy::allow_all().is_authorized(ctx),
                }
            })
            .map(|key| {
                let health = inner
                    .accumulator_health
                    .get(key)
                    .map(|rx| rx.borrow().clone())
                    .unwrap_or(AccumulatorHealth::Live);
                let fresh = inner.accumulator_freshness.get(key).cloned();
                (key.name.clone(), health, fresh)
            })
            .collect()
    }
}

impl Default for EndpointRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn dummy_handle() -> ReactorHandle {
        ReactorHandle {
            cache: Arc::new(RwLock::new(super::super::types::InputCache::new())),
            paused: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(crate::computation_graph::reactor::ReactorStats::default()),
        }
    }

    /// Untenanted owner — the embedded/pre-tenancy shape most of these tests use.
    fn owner(reactor: &str) -> EndpointOwner {
        EndpointOwner::embedded(reactor)
    }

    /// Owner in a specific tenant/package.
    fn tenant_owner(tenant: &str, package: &str, reactor: &str) -> EndpointOwner {
        EndpointOwner::new(Some(tenant.to_string()), Some(package.to_string()), reactor)
    }

    const GLOBAL: EndpointScope<'static> = EndpointScope {
        tenant_id: None,
        is_admin: false,
    };

    #[tokio::test]
    async fn test_accumulator_descriptor_roundtrip_and_deregister() {
        // CLOACI-I-0128 follow-up: the self-registered discoverability descriptor
        // is readable and cleared on deregister.
        let registry = EndpointRegistry::new();
        let acme = tenant_owner("acme", "mm-pkg", "market_maker");
        let acme_scope = EndpointScope::tenant("acme");
        assert!(registry
            .accumulator_descriptor("alpha", acme_scope)
            .await
            .is_none());

        // The accumulator itself must exist for deregistration to own-match.
        let (tx, _rx) = mpsc::channel(10);
        registry
            .register_accumulator(&acme, "alpha".to_string(), tx)
            .await
            .unwrap();
        registry
            .register_accumulator_meta(
                &acme,
                "alpha".to_string(),
                AccumulatorDescriptor {
                    reactor: "market_maker".to_string(),
                    tenant_id: Some("acme".to_string()),
                },
            )
            .await;

        let d = registry
            .accumulator_descriptor("alpha", acme_scope)
            .await
            .expect("descriptor present");
        assert_eq!(d.reactor, "market_maker");
        assert_eq!(d.tenant_id.as_deref(), Some("acme"));

        registry.deregister_accumulator(&acme, "alpha").await;
        assert!(
            registry
                .accumulator_descriptor("alpha", acme_scope)
                .await
                .is_none(),
            "deregister should clear the descriptor"
        );
    }

    #[tokio::test]
    async fn test_register_send_deregister_accumulator() {
        let registry = EndpointRegistry::new();
        let (tx, mut rx) = mpsc::channel(10);
        let o = owner("g");

        registry
            .register_accumulator(&o, "alpha".to_string(), tx)
            .await
            .unwrap();

        let data = vec![1, 2, 3];
        let sent = registry
            .send_to_accumulator("alpha", GLOBAL, data.clone())
            .await
            .unwrap();
        assert_eq!(sent, 1);

        let received = rx.recv().await.unwrap();
        assert_eq!(received, data);

        registry.deregister_accumulator(&o, "alpha").await;

        let err = registry
            .send_to_accumulator("alpha", GLOBAL, vec![4, 5])
            .await
            .unwrap_err();
        assert!(matches!(err, RegistryError::AccumulatorNotFound(_)));
    }

    #[tokio::test]
    async fn test_broadcast_to_multiple_accumulators() {
        let registry = EndpointRegistry::new();
        let (tx1, mut rx1) = mpsc::channel(10);
        let (tx2, mut rx2) = mpsc::channel(10);
        let o = owner("g");

        registry
            .register_accumulator(&o, "alpha".to_string(), tx1)
            .await
            .unwrap();
        registry
            .register_accumulator(&o, "alpha".to_string(), tx2)
            .await
            .unwrap();

        assert_eq!(registry.accumulator_count("alpha", GLOBAL).await, 2);

        let data = vec![10, 20, 30];
        let sent = registry
            .send_to_accumulator("alpha", GLOBAL, data.clone())
            .await
            .unwrap();
        assert_eq!(sent, 2);

        assert_eq!(rx1.recv().await.unwrap(), data);
        assert_eq!(rx2.recv().await.unwrap(), data);
    }

    #[tokio::test]
    async fn test_send_to_unregistered_accumulator() {
        let registry = EndpointRegistry::new();
        let err = registry
            .send_to_accumulator("nonexistent", GLOBAL, vec![1])
            .await
            .unwrap_err();
        assert!(matches!(err, RegistryError::AccumulatorNotFound(_)));
    }

    #[tokio::test]
    async fn test_register_send_deregister_reactor() {
        let registry = EndpointRegistry::new();
        let (tx, mut rx) = mpsc::channel(10);
        let o = owner("market_maker");

        registry
            .register_reactor(&o, "market_maker".to_string(), tx, dummy_handle())
            .await
            .unwrap();

        registry
            .send_to_reactor("market_maker", GLOBAL, ManualCommand::ForceFire)
            .await
            .unwrap();

        let cmd = rx.recv().await.unwrap();
        assert!(matches!(cmd, ManualCommand::ForceFire));

        registry.deregister_reactor(&o, "market_maker").await;

        let err = registry
            .send_to_reactor("market_maker", GLOBAL, ManualCommand::ForceFire)
            .await
            .unwrap_err();
        assert!(matches!(err, RegistryError::ReactorNotFound(_)));
    }

    #[tokio::test]
    async fn test_send_to_unregistered_reactor() {
        let registry = EndpointRegistry::new();
        let err = registry
            .send_to_reactor("nonexistent", GLOBAL, ManualCommand::ForceFire)
            .await
            .unwrap_err();
        assert!(matches!(err, RegistryError::ReactorNotFound(_)));
    }

    #[tokio::test]
    async fn test_closed_accumulator_channel_pruned() {
        let registry = EndpointRegistry::new();
        let (tx1, rx1) = mpsc::channel(10);
        let (tx2, mut rx2) = mpsc::channel(10);
        let o = owner("g");

        registry
            .register_accumulator(&o, "alpha".to_string(), tx1)
            .await
            .unwrap();
        registry
            .register_accumulator(&o, "alpha".to_string(), tx2)
            .await
            .unwrap();

        // Drop rx1 — its channel is now closed
        drop(rx1);

        let data = vec![42];
        let sent = registry
            .send_to_accumulator("alpha", GLOBAL, data.clone())
            .await
            .unwrap();
        assert_eq!(sent, 1); // only tx2 succeeded

        assert_eq!(rx2.recv().await.unwrap(), data);

        // Closed channel should have been pruned
        assert_eq!(registry.accumulator_count("alpha", GLOBAL).await, 1);
    }

    #[tokio::test]
    async fn test_list_accumulators_and_reactors() {
        let registry = EndpointRegistry::new();
        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel::<ManualCommand>(10);

        registry
            .register_accumulator(&owner("g"), "alpha".to_string(), tx1)
            .await
            .unwrap();
        registry
            .register_reactor(
                &owner("market_maker"),
                "market_maker".to_string(),
                tx2,
                dummy_handle(),
            )
            .await
            .unwrap();

        let accumulators = registry.list_accumulators().await;
        assert_eq!(accumulators, vec!["alpha"]);

        let reactors = registry.list_reactors().await;
        assert_eq!(reactors, vec!["market_maker"]);
    }

    #[tokio::test]
    async fn test_accumulator_auth_deny_by_default() {
        let registry = EndpointRegistry::new();
        let key_id = uuid::Uuid::new_v4();
        let ctx = KeyContext {
            key_id: &key_id,
            tenant_id: None,
            is_admin: false,
        };
        // No policy set → deny
        let err = registry
            .check_accumulator_auth("alpha", &ctx)
            .await
            .unwrap_err();
        assert!(matches!(err, RegistryError::AccumulatorUnauthorized(_)));
    }

    #[tokio::test]
    async fn test_accumulator_auth_authorized_key() {
        let registry = EndpointRegistry::new();
        let key_id = uuid::Uuid::new_v4();

        registry
            .set_accumulator_policy(
                &owner("g"),
                "alpha".to_string(),
                AccumulatorAuthPolicy {
                    allow_all_authenticated: false,
                    allowed_tenants: Vec::new(),
                    allowed_producers: vec![key_id],
                },
            )
            .await;

        // Authorized key succeeds
        let ctx = KeyContext {
            key_id: &key_id,
            tenant_id: None,
            is_admin: false,
        };
        registry
            .check_accumulator_auth("alpha", &ctx)
            .await
            .unwrap();

        // Different key is denied
        let other_key = uuid::Uuid::new_v4();
        let other_ctx = KeyContext {
            key_id: &other_key,
            tenant_id: None,
            is_admin: false,
        };
        let err = registry
            .check_accumulator_auth("alpha", &other_ctx)
            .await
            .unwrap_err();
        assert!(matches!(err, RegistryError::AccumulatorUnauthorized(_)));
    }

    #[tokio::test]
    async fn test_accumulator_auth_tenant_scoped() {
        let registry = EndpointRegistry::new();
        let key_id = uuid::Uuid::new_v4();

        registry
            .set_accumulator_policy(
                &owner("g"),
                "alpha".to_string(),
                AccumulatorAuthPolicy::for_tenant("acme"),
            )
            .await;

        // Acme key → allowed
        let acme_ctx = KeyContext {
            key_id: &key_id,
            tenant_id: Some("acme"),
            is_admin: false,
        };
        registry
            .check_accumulator_auth("alpha", &acme_ctx)
            .await
            .unwrap();

        // Other tenant → denied
        let other_ctx = KeyContext {
            key_id: &key_id,
            tenant_id: Some("other"),
            is_admin: false,
        };
        assert!(registry
            .check_accumulator_auth("alpha", &other_ctx)
            .await
            .is_err());

        // Admin → always allowed
        let admin_ctx = KeyContext {
            key_id: &key_id,
            tenant_id: Some("other"),
            is_admin: true,
        };
        registry
            .check_accumulator_auth("alpha", &admin_ctx)
            .await
            .unwrap();

        // Global key (no tenant) → denied for tenant-scoped endpoint
        let global_ctx = KeyContext {
            key_id: &key_id,
            tenant_id: None,
            is_admin: false,
        };
        assert!(registry
            .check_accumulator_auth("alpha", &global_ctx)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_reactor_auth_with_operation_permissions() {
        let registry = EndpointRegistry::new();
        let key_id = uuid::Uuid::new_v4();

        let mut op_perms = HashMap::new();
        op_perms.insert(key_id, vec![ReactorOp::ForceFire, ReactorOp::GetState]);

        registry
            .set_reactor_policy(
                &owner("mm"),
                "mm".to_string(),
                ReactorAuthPolicy {
                    allow_all_authenticated: false,
                    allowed_tenants: Vec::new(),
                    allowed_operators: vec![key_id],
                    operation_permissions: op_perms,
                },
            )
            .await;

        let ctx = KeyContext {
            key_id: &key_id,
            tenant_id: None,
            is_admin: false,
        };

        // Authorized ops succeed
        registry
            .check_reactor_op_auth("mm", &ctx, &ReactorOp::ForceFire)
            .await
            .unwrap();
        registry
            .check_reactor_op_auth("mm", &ctx, &ReactorOp::GetState)
            .await
            .unwrap();

        // Unauthorized op denied
        let err = registry
            .check_reactor_op_auth("mm", &ctx, &ReactorOp::Pause)
            .await
            .unwrap_err();
        assert!(matches!(err, RegistryError::OperationNotPermitted { .. }));
    }

    // -----------------------------------------------------------------
    // CLOACI-T-0921: tenant is THE isolation boundary
    // -----------------------------------------------------------------

    /// Two tenants each register an accumulator named `ticks`. An inject into
    /// tenant A's must NOT reach tenant B, and unloading A must leave B alive.
    #[tokio::test]
    async fn test_cloaci_t_0921_same_name_accumulators_isolated_across_tenants() {
        let registry = EndpointRegistry::new();
        let acme = tenant_owner("acme", "acme-feed", "market_maker");
        let globex = tenant_owner("globex", "globex-feed", "risk_engine");

        let (tx_a, mut rx_a) = mpsc::channel(10);
        let (tx_b, mut rx_b) = mpsc::channel(10);
        registry
            .register_accumulator(&acme, "ticks".to_string(), tx_a)
            .await
            .expect("tenant A registers");
        registry
            .register_accumulator(&globex, "ticks".to_string(), tx_b)
            .await
            .expect("tenant B registers the same name in its own tenant");

        // Inject as tenant A.
        let payload = vec![7, 7, 7];
        let sent = registry
            .send_to_accumulator("ticks", EndpointScope::tenant("acme"), payload.clone())
            .await
            .expect("A's inject lands");
        assert_eq!(sent, 1, "exactly one recipient — A's own accumulator");
        assert_eq!(rx_a.try_recv().unwrap(), payload);
        assert!(
            rx_b.try_recv().is_err(),
            "tenant B must receive nothing from tenant A's inject"
        );

        // Unloading A leaves B intact and reachable.
        registry.deregister_accumulator(&acme, "ticks").await;
        assert!(
            registry
                .send_to_accumulator("ticks", EndpointScope::tenant("acme"), vec![1])
                .await
                .is_err(),
            "A's accumulator is gone"
        );
        let sent_b = registry
            .send_to_accumulator("ticks", EndpointScope::tenant("globex"), vec![9])
            .await
            .expect("B survives A's unload");
        assert_eq!(sent_b, 1);
        assert_eq!(rx_b.try_recv().unwrap(), vec![9]);
    }

    /// A tenant may not reach another tenant's endpoint by naming it, and a
    /// tenant-less non-admin caller may not either.
    #[tokio::test]
    async fn test_cloaci_t_0921_cross_tenant_lookup_is_not_found() {
        let registry = EndpointRegistry::new();
        let acme = tenant_owner("acme", "acme-feed", "market_maker");
        let (tx, _rx) = mpsc::channel(10);
        registry
            .register_accumulator(&acme, "ticks".to_string(), tx)
            .await
            .unwrap();

        for scope in [EndpointScope::tenant("globex"), EndpointScope::untenanted()] {
            let err = registry
                .send_to_accumulator("ticks", scope, vec![1])
                .await
                .unwrap_err();
            assert!(
                matches!(err, RegistryError::AccumulatorNotFound(_)),
                "cross-tenant/global lookup must not resolve, got {err:?}"
            );
        }

        // An admin reaches it, because the name is unambiguous.
        registry
            .send_to_accumulator("ticks", EndpointScope::admin(), vec![1])
            .await
            .expect("admin resolves the unique cross-tenant match");
    }

    /// An admin naming an endpoint that exists in two tenants gets a refusal,
    /// not a coin flip.
    #[tokio::test]
    async fn test_cloaci_t_0921_admin_ambiguous_name_refused() {
        let registry = EndpointRegistry::new();
        let (tx_a, _ra) = mpsc::channel(10);
        let (tx_b, _rb) = mpsc::channel(10);
        registry
            .register_accumulator(&tenant_owner("acme", "p", "r"), "ticks".to_string(), tx_a)
            .await
            .unwrap();
        registry
            .register_accumulator(&tenant_owner("globex", "p", "r"), "ticks".to_string(), tx_b)
            .await
            .unwrap();

        let err = registry
            .send_to_accumulator("ticks", EndpointScope::admin(), vec![1])
            .await
            .unwrap_err();
        match err {
            RegistryError::AmbiguousEndpoint {
                name, mut tenants, ..
            } => {
                assert_eq!(name, "ticks");
                tenants.sort();
                assert_eq!(tenants, vec!["acme".to_string(), "globex".to_string()]);
            }
            other => panic!("expected AmbiguousEndpoint, got {other:?}"),
        }
    }

    /// Two packages in the SAME tenant claiming one accumulator name is a loud
    /// load-time rejection, not a silent cross-wire.
    #[tokio::test]
    async fn test_cloaci_t_0921_same_tenant_second_claim_rejects_loudly() {
        let registry = EndpointRegistry::new();
        let first = tenant_owner("acme", "pkg-one", "reactor_one");
        let second = tenant_owner("acme", "pkg-two", "reactor_two");

        let (tx1, mut rx1) = mpsc::channel(10);
        let (tx2, mut rx2) = mpsc::channel(10);
        registry
            .register_accumulator(&first, "ticks".to_string(), tx1)
            .await
            .expect("first claim wins");

        let err = registry
            .register_accumulator(&second, "ticks".to_string(), tx2)
            .await
            .expect_err("second package must be rejected");
        let msg = err.to_string();
        assert!(matches!(
            err,
            RegistryError::EndpointOwnershipConflict { .. }
        ));
        assert!(
            msg.contains("pkg-one") && msg.contains("pkg-two") && msg.contains("acme"),
            "error must name both packages and the tenant: {msg}"
        );

        // The incumbent is untouched: it still receives, the rejected one never does.
        registry
            .send_to_accumulator("ticks", EndpointScope::tenant("acme"), vec![5])
            .await
            .unwrap();
        assert_eq!(rx1.try_recv().unwrap(), vec![5]);
        assert!(rx2.try_recv().is_err());
    }

    /// Same rule for reactors.
    #[tokio::test]
    async fn test_cloaci_t_0921_reactor_same_tenant_second_claim_rejects() {
        let registry = EndpointRegistry::new();
        let first = tenant_owner("acme", "pkg-one", "rx");
        let second = tenant_owner("acme", "pkg-two", "rx");
        let (tx1, _r1) = mpsc::channel(10);
        let (tx2, _r2) = mpsc::channel(10);

        registry
            .register_reactor(&first, "rx".to_string(), tx1, dummy_handle())
            .await
            .unwrap();
        let err = registry
            .register_reactor(&second, "rx".to_string(), tx2, dummy_handle())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            RegistryError::EndpointOwnershipConflict { .. }
        ));
    }

    /// Deregistration is owner-scoped: another package's unload is a no-op.
    #[tokio::test]
    async fn test_cloaci_t_0921_deregister_is_owner_scoped() {
        let registry = EndpointRegistry::new();
        let mine = tenant_owner("acme", "pkg-one", "rx-one");
        let theirs = tenant_owner("acme", "pkg-two", "rx-two");
        let (tx, _rx) = mpsc::channel(10);
        registry
            .register_accumulator(&mine, "ticks".to_string(), tx)
            .await
            .unwrap();

        // A different package in the same tenant cannot tear it down.
        registry.deregister_accumulator(&theirs, "ticks").await;
        assert_eq!(
            registry
                .accumulator_count("ticks", EndpointScope::tenant("acme"))
                .await,
            1,
            "another package's deregister must be a no-op"
        );

        registry.deregister_accumulator(&mine, "ticks").await;
        assert_eq!(
            registry
                .accumulator_count("ticks", EndpointScope::tenant("acme"))
                .await,
            0
        );
    }

    /// The health listing filters on the key's tenant, not just the policy.
    #[tokio::test]
    async fn test_cloaci_t_0921_health_listing_filters_by_key_tenant() {
        let registry = EndpointRegistry::new();
        let (tx_a, _ra) = mpsc::channel(10);
        let (tx_b, _rb) = mpsc::channel(10);
        registry
            .register_accumulator(&tenant_owner("acme", "p", "r"), "a_acc".to_string(), tx_a)
            .await
            .unwrap();
        registry
            .register_accumulator(&tenant_owner("globex", "p", "r"), "b_acc".to_string(), tx_b)
            .await
            .unwrap();
        // Deliberately leave both policies unset — the pre-T-0921 code would
        // then fall back to allow_all and expose BOTH names to either tenant.

        let key_id = uuid::Uuid::new_v4();
        let acme_ctx = KeyContext {
            key_id: &key_id,
            tenant_id: Some("acme"),
            is_admin: false,
        };
        let visible: Vec<String> = registry
            .list_accumulators_with_health_for_key(&acme_ctx)
            .await
            .into_iter()
            .map(|(n, _, _)| n)
            .collect();
        assert_eq!(visible, vec!["a_acc".to_string()]);
    }
}
