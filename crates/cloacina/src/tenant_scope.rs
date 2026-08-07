/*
 *  Copyright 2026 Colliery Software
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

//! Shared tenant-keying primitives for process-global name registries.
//!
//! Tenant is THE isolation boundary (see CLOACI-S-0011 and the authz design),
//! so any map that lives for the life of the process and is keyed by an entity
//! name authored by a tenant must carry the tenant *in the key*, not as payload
//! hanging off the value.
//!
//! CLOACI-T-0921 established this shape for the `EndpointRegistry`
//! (`EndpointKey` / `EndpointOwner` / `EndpointScope`); CLOACI-T-0924 lifted it
//! here so the [`Runtime`](crate::Runtime) registries and the
//! [`ComputationGraphScheduler`](crate::computation_graph::ComputationGraphScheduler)
//! reuse *one* convention instead of inventing a third.
//!
//! Three ideas:
//!
//! * [`TenantKey`] — the **map key**: `(tenant, name)`. `tenant_id: None` is the
//!   untenanted/embedded single-tenant case and keeps pre-multi-tenancy
//!   behaviour byte-for-byte.
//! * [`TenantOwner`] — the rest of the **provenance** stamped on each entry
//!   (which package claimed the name), used to make a same-tenant collision
//!   between two packages a loud error instead of a silent overwrite.
//! * [`TenantScope`] — the **caller's** scope, used to resolve a bare `name`
//!   into a concrete key via [`resolve_tenant_key`].
//!
//! ## Why not [`TaskNamespace`](crate::task::TaskNamespace)?
//!
//! `RuntimeInner.tasks` is keyed by `TaskNamespace`
//! (`tenant::package::workflow::task`) and is deliberately *not* migrated to
//! this shape: a task's full namespace string is persisted on `task_executions`
//! rows, so every reader genuinely has all four components. The registries this
//! module serves are addressed by **bare name** at their read sites (a workflow
//! name off a `workflow_executions` row, a trigger name off a cron schedule, a
//! reactor name off a WebSocket path), so package belongs in the *owner
//! metadata* used to detect collisions, not in the key.

use std::collections::HashMap;
use std::fmt;

/// Composite `(tenant, name)` lookup key for a process-global registry.
///
/// `tenant_id: None` is the untenanted/embedded entry. The tenant is stored
/// verbatim — there is deliberately no folding of the reserved `"public"`
/// tenant onto `None`, so this key matches CLOACI-T-0921's `EndpointKey` and a
/// single deployment never has two spellings of the same scope.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TenantKey {
    pub tenant_id: Option<String>,
    pub name: String,
}

impl TenantKey {
    pub fn new(tenant_id: Option<&str>, name: &str) -> Self {
        Self {
            tenant_id: tenant_id.map(|t| t.to_string()),
            name: name.to_string(),
        }
    }

    /// Operator-facing tenant label for error messages.
    pub fn tenant_label(&self) -> &str {
        self.tenant_id.as_deref().unwrap_or("<untenanted>")
    }
}

impl fmt::Display for TenantKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.tenant_label(), self.name)
    }
}

/// Provenance stamped on a registry entry beyond its [`TenantKey`].
///
/// The key is `(tenant, name)`; this records *who* claimed it. It exists so a
/// same-tenant claim on a live name by a **different package** is rejected
/// loudly at load time instead of silently replacing the incumbent, and so a
/// package's unload only ever removes entries it installed.
///
/// `package: None` means "provenance unknown" — hand-wired registrations,
/// macro/`inventory` seeding, and the Python decorator paths. An unknown owner
/// never *causes* a conflict (there is nothing to compare), which keeps every
/// pre-existing caller behaving exactly as it did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TenantOwner {
    pub package: Option<String>,
}

impl TenantOwner {
    /// An owner with no package provenance (hand-wired / inventory / embedded).
    pub fn unknown() -> Self {
        Self { package: None }
    }

    /// An owner identified by the package that claimed the name.
    pub fn package(package: impl Into<String>) -> Self {
        Self {
            package: Some(package.into()),
        }
    }

    /// `true` when `other` may replace an entry owned by `self`.
    ///
    /// Same package replaces (a package reload / restart depends on it); an
    /// unknown owner on either side is permissive, because there is no
    /// provenance to contradict. Two *named and different* packages conflict.
    pub fn may_replace(&self, other: &TenantOwner) -> bool {
        match (&self.package, &other.package) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        }
    }

    /// Operator-facing provenance string for conflict errors.
    pub fn label(&self) -> String {
        match &self.package {
            Some(p) => format!("package '{}'", p),
            None => "an unattributed registration".to_string(),
        }
    }
}

/// Tenant scope of a *caller* performing a lookup.
///
/// Routes build this from the authenticated key; in-process callers build it
/// from the tenant their runner/reconciler is bound to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TenantScope<'a> {
    pub tenant_id: Option<&'a str>,
    pub is_admin: bool,
}

impl<'a> TenantScope<'a> {
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

    /// An admin caller: may reach any tenant's entry, but only when the name
    /// is unambiguous across tenants.
    pub fn admin() -> Self {
        Self {
            tenant_id: None,
            is_admin: true,
        }
    }

    /// The key this scope *writes* under.
    pub fn own_key(&self, name: &str) -> TenantKey {
        TenantKey::new(self.tenant_id, name)
    }
}

/// Why a scoped lookup failed to land on exactly one key.
#[derive(Debug, PartialEq, Eq)]
pub enum TenantResolveMiss {
    NotFound,
    /// The name exists in more than one tenant and the caller is an admin —
    /// report both rather than guessing.
    Ambiguous(Vec<String>),
}

impl fmt::Display for TenantResolveMiss {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TenantResolveMiss::NotFound => write!(f, "not found in this tenant scope"),
            TenantResolveMiss::Ambiguous(tenants) => write!(
                f,
                "name is ambiguous across tenants {:?}; address it with an explicit tenant",
                tenants
            ),
        }
    }
}

/// Resolve a bare `name` to a concrete [`TenantKey`] within `scope`.
///
/// Order:
/// 1. the caller's own tenant — the normal multi-tenant path;
/// 2. the untenanted entry (`tenant_id: None`) — embedded/pre-tenancy entries,
///    which were always globally addressable;
/// 3. admins only: a unique cross-tenant match. Two tenants owning the same
///    name is [`TenantResolveMiss::Ambiguous`], never a guess.
///
/// A non-admin caller can therefore never resolve another tenant's entry.
pub fn resolve_tenant_key<V>(
    map: &HashMap<TenantKey, V>,
    scope: TenantScope<'_>,
    name: &str,
) -> Result<TenantKey, TenantResolveMiss> {
    if let Some(tenant) = scope.tenant_id {
        let key = TenantKey::new(Some(tenant), name);
        if map.contains_key(&key) {
            return Ok(key);
        }
    }

    let untenanted = TenantKey::new(None, name);
    if map.contains_key(&untenanted) {
        return Ok(untenanted);
    }

    if scope.is_admin {
        let mut matches: Vec<&TenantKey> = map.keys().filter(|k| k.name == name).collect();
        matches.sort();
        match matches.len() {
            0 => return Err(TenantResolveMiss::NotFound),
            1 => return Ok(matches[0].clone()),
            _ => {
                return Err(TenantResolveMiss::Ambiguous(
                    matches
                        .iter()
                        .map(|k| k.tenant_label().to_string())
                        .collect(),
                ))
            }
        }
    }

    Err(TenantResolveMiss::NotFound)
}

/// Every key in `map` visible to `scope`: the caller's own tenant plus the
/// untenanted entries, or everything for an admin.
pub fn visible_keys<'m, V>(
    map: &'m HashMap<TenantKey, V>,
    scope: TenantScope<'_>,
) -> impl Iterator<Item = &'m TenantKey> {
    let tenant = scope.tenant_id.map(|t| t.to_string());
    let is_admin = scope.is_admin;
    map.keys().filter(move |k| {
        is_admin || k.tenant_id.is_none() || k.tenant_id.as_deref() == tenant.as_deref()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_with(keys: &[(Option<&str>, &str)]) -> HashMap<TenantKey, u32> {
        keys.iter()
            .enumerate()
            .map(|(i, (t, n))| (TenantKey::new(*t, n), i as u32))
            .collect()
    }

    #[test]
    fn own_tenant_wins_over_untenanted() {
        let m = map_with(&[(None, "wf"), (Some("a"), "wf")]);
        let k = resolve_tenant_key(&m, TenantScope::tenant("a"), "wf").unwrap();
        assert_eq!(k.tenant_id.as_deref(), Some("a"));
    }

    #[test]
    fn untenanted_is_the_embedded_fallback() {
        let m = map_with(&[(None, "wf")]);
        let k = resolve_tenant_key(&m, TenantScope::tenant("a"), "wf").unwrap();
        assert_eq!(k.tenant_id, None);
        let k = resolve_tenant_key(&m, TenantScope::untenanted(), "wf").unwrap();
        assert_eq!(k.tenant_id, None);
    }

    #[test]
    fn non_admin_never_reaches_another_tenant() {
        let m = map_with(&[(Some("a"), "wf")]);
        assert_eq!(
            resolve_tenant_key(&m, TenantScope::tenant("b"), "wf").unwrap_err(),
            TenantResolveMiss::NotFound
        );
        assert_eq!(
            resolve_tenant_key(&m, TenantScope::untenanted(), "wf").unwrap_err(),
            TenantResolveMiss::NotFound
        );
    }

    #[test]
    fn admin_resolves_unique_and_refuses_ambiguous() {
        let m = map_with(&[(Some("a"), "wf")]);
        assert!(resolve_tenant_key(&m, TenantScope::admin(), "wf").is_ok());

        let m = map_with(&[(Some("a"), "wf"), (Some("b"), "wf")]);
        match resolve_tenant_key(&m, TenantScope::admin(), "wf") {
            Err(TenantResolveMiss::Ambiguous(tenants)) => assert_eq!(tenants.len(), 2),
            other => panic!("expected ambiguity, got {:?}", other.map(|k| k.to_string())),
        }
    }

    #[test]
    fn visible_keys_hides_other_tenants() {
        let m = map_with(&[(None, "a"), (Some("t1"), "b"), (Some("t2"), "c")]);
        let mut seen: Vec<String> = visible_keys(&m, TenantScope::tenant("t1"))
            .map(|k| k.name.clone())
            .collect();
        seen.sort();
        assert_eq!(seen, vec!["a".to_string(), "b".to_string()]);

        let all: Vec<String> = visible_keys(&m, TenantScope::admin())
            .map(|k| k.name.clone())
            .collect();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn owner_replacement_policy() {
        let pkg_a = TenantOwner::package("a");
        let pkg_b = TenantOwner::package("b");
        let unknown = TenantOwner::unknown();

        assert!(pkg_a.may_replace(&pkg_a));
        assert!(!pkg_a.may_replace(&pkg_b));
        // Unattributed registrations stay permissive in both directions so no
        // pre-existing hand-wired caller starts failing.
        assert!(unknown.may_replace(&pkg_a));
        assert!(pkg_a.may_replace(&unknown));
    }
}
