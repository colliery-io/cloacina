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

// CLOACI-T-0782: the matcher is pure and not yet mounted; the route table +
// middleware that consume these items land in T-0783. Allow the interim
// dead_code so the crate stays warning-clean until then.
#![allow(dead_code)]

//! ABAC authorization matcher — CLOACI-I-0118 Phase 0 (CLOACI-T-0782).
//!
//! A small, **total**, default-deny authorization function over a coarse,
//! URL-derivable attribute model:
//!
//! - [`Principal`] — the authenticated subject, projected from an
//!   [`AuthenticatedKey`](crate::routes::auth::AuthenticatedKey).
//! - [`Access`] — what a route requires: a [`Scope`] + a minimum [`Level`].
//! - [`evaluate`] — `(&Principal, &ResolvedScope, Level) -> Decision`.
//!
//! This module is **pure**: no router wiring, no middleware, no I/O. T-0783
//! builds the declarative route table and the `authz_mw` middleware on top of
//! it. The matcher reproduces today's `can_access_tenant` / `can_write` /
//! `can_admin` decisions (see the parity truth-table tests below), so the
//! later wire-in is behavior-preserving by construction.

use crate::routes::auth::AuthenticatedKey;

/// Permission level required by — or held by — a principal.
///
/// Ordered `Read < Write < Admin`, so a role satisfies a requirement when
/// `principal.role >= access.level`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Read,
    Write,
    Admin,
}

impl Level {
    /// Project the `permissions` string carried on an API key into a [`Level`].
    ///
    /// Unknown values fall back to the least privilege (`Read`) — fail-safe.
    pub fn from_permissions(permissions: &str) -> Level {
        match permissions {
            "admin" => Level::Admin,
            "write" => Level::Write,
            _ => Level::Read,
        }
    }
}

/// The scope a route requires, *before* the request's tenant is resolved.
///
/// `TenantParam` is resolved to [`ResolvedScope::Tenant`] by the middleware
/// (T-0783) from the `{tenant_id}` path segment at request time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// God-mode (`is_admin`) only — cross-tenant / global control-plane ops.
    Platform,
    /// Scoped to the tenant named in the request path.
    TenantParam,
    /// Any authenticated key; the handler scopes returned *data* to the caller.
    Any,
}

/// A route's access requirement: a [`Scope`] plus the minimum [`Level`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Access {
    pub scope: Scope,
    pub level: Level,
}

impl Access {
    /// God-only access at the given level.
    pub const fn platform(level: Level) -> Access {
        Access {
            scope: Scope::Platform,
            level,
        }
    }

    /// Tenant-scoped access (tenant resolved from the path) at the given level.
    pub const fn tenant(level: Level) -> Access {
        Access {
            scope: Scope::TenantParam,
            level,
        }
    }

    /// Any-authenticated access at the given level (data-scoped in the handler).
    pub const fn any(level: Level) -> Access {
        Access {
            scope: Scope::Any,
            level,
        }
    }
}

/// A [`Scope`] with the request's tenant resolved.
///
/// The middleware turns [`Scope::TenantParam`] into [`ResolvedScope::Tenant`]
/// by reading the `{tenant_id}` path segment; [`Scope::Platform`] and
/// [`Scope::Any`] map across unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedScope {
    Platform,
    Tenant(String),
    Any,
}

/// The authenticated subject, projected into authorization attributes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Principal {
    /// The key's tenant scope. `None` == global/public.
    pub tenant: Option<String>,
    /// The key's role.
    pub role: Level,
    /// God-mode: cross-tenant superuser (`AuthenticatedKey::is_admin`).
    pub platform_admin: bool,
}

impl Principal {
    /// Project an [`AuthenticatedKey`] into the authorization attribute model.
    pub fn from_key(key: &AuthenticatedKey) -> Principal {
        Principal {
            tenant: key.tenant_id.clone(),
            role: Level::from_permissions(&key.permissions),
            platform_admin: key.is_admin,
        }
    }
}

/// Stable deny reason — maps 1:1 to the existing `ApiError::forbidden`
/// `"admin_required"` envelope (god-mode required for a platform op).
pub const DENY_PLATFORM: &str = "platform_admin_required";
/// Stable deny reason — maps to the existing `"tenant_access_denied"` envelope.
pub const DENY_TENANT: &str = "tenant_access_denied";
/// Stable deny reason — maps to the existing `"insufficient_permissions"` envelope.
pub const DENY_ROLE: &str = "insufficient_role";

/// Authorization outcome. `Deny` carries a stable reason string that the
/// middleware (T-0783) maps onto the existing 403 `ApiError` envelopes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Permit,
    Deny(&'static str),
}

impl Decision {
    /// True when access is granted.
    pub fn is_permit(&self) -> bool {
        matches!(self, Decision::Permit)
    }
}

/// The authorization matcher. **Total** and **default-deny**.
///
/// - God-mode (`platform_admin`) short-circuits to [`Decision::Permit`].
/// - `Platform` scope is god-only → otherwise denied.
/// - `Tenant(t)` requires the principal to belong to `t` (or be a global key
///   reaching the reserved `"public"` tenant), then `role >= level`.
/// - `Any` requires only `role >= level` (the handler scopes data by tenant).
pub fn evaluate(principal: &Principal, scope: &ResolvedScope, level: Level) -> Decision {
    // God-mode: cross-tenant superuser passes everything.
    if principal.platform_admin {
        return Decision::Permit;
    }

    match scope {
        ResolvedScope::Platform => Decision::Deny(DENY_PLATFORM),
        ResolvedScope::Tenant(tenant) => {
            let in_tenant = match &principal.tenant {
                Some(principal_tenant) => principal_tenant == tenant,
                // A global (non-admin) key may only reach the reserved
                // "public" tenant.
                None => tenant == "public",
            };
            if !in_tenant {
                Decision::Deny(DENY_TENANT)
            } else if principal.role < level {
                Decision::Deny(DENY_ROLE)
            } else {
                Decision::Permit
            }
        }
        ResolvedScope::Any => {
            if principal.role < level {
                Decision::Deny(DENY_ROLE)
            } else {
                Decision::Permit
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----------------------------------------------------------------------
    // Reference implementation of TODAY's authorization primitives, copied
    // from `AuthenticatedKey` (routes/auth.rs:245-282). The parity tests below
    // assert that `evaluate()` composes back into exactly these primitives for
    // every principal — so T-0783's route table (which assigns Access values)
    // preserves current behavior by construction.
    // ----------------------------------------------------------------------

    fn ref_can_access_tenant(tenant: &Option<String>, is_admin: bool, requested: &str) -> bool {
        if is_admin {
            return true;
        }
        match tenant {
            Some(key_tenant) => key_tenant == requested,
            None => requested == "public",
        }
    }

    fn ref_can_write(permissions: &str, is_admin: bool) -> bool {
        is_admin || permissions == "admin" || permissions == "write"
    }

    fn ref_can_admin(permissions: &str, is_admin: bool) -> bool {
        is_admin || permissions == "admin"
    }

    /// Every principal shape we care about: the cross product of god-mode,
    /// the three roles, and the three tenant scopes (none/global, a tenant,
    /// and the reserved "public" tenant).
    fn all_principals() -> Vec<(Principal, &'static str, bool)> {
        let mut out = Vec::new();
        for &is_admin in &[false, true] {
            for &perms in &["read", "write", "admin"] {
                for tenant in &[None, Some("acme".to_string()), Some("public".to_string())] {
                    out.push((
                        Principal {
                            tenant: tenant.clone(),
                            role: Level::from_permissions(perms),
                            platform_admin: is_admin,
                        },
                        perms,
                        is_admin,
                    ));
                }
            }
        }
        out
    }

    #[test]
    fn level_ordering_and_parsing() {
        assert!(Level::Read < Level::Write);
        assert!(Level::Write < Level::Admin);
        assert_eq!(Level::from_permissions("admin"), Level::Admin);
        assert_eq!(Level::from_permissions("write"), Level::Write);
        assert_eq!(Level::from_permissions("read"), Level::Read);
        // Unknown -> least privilege.
        assert_eq!(Level::from_permissions("bogus"), Level::Read);
        assert_eq!(Level::from_permissions(""), Level::Read);
    }

    #[test]
    fn god_mode_permits_everything() {
        let god = Principal {
            tenant: None,
            role: Level::Read, // role is irrelevant under god-mode
            platform_admin: true,
        };
        for scope in [
            ResolvedScope::Platform,
            ResolvedScope::Tenant("acme".into()),
            ResolvedScope::Tenant("beta".into()),
            ResolvedScope::Any,
        ] {
            for level in [Level::Read, Level::Write, Level::Admin] {
                assert_eq!(
                    evaluate(&god, &scope, level),
                    Decision::Permit,
                    "god-mode must permit {scope:?} @ {level:?}"
                );
            }
        }
    }

    #[test]
    fn default_deny_reasons_are_stable() {
        let reader = Principal {
            tenant: Some("acme".into()),
            role: Level::Read,
            platform_admin: false,
        };
        assert_eq!(
            evaluate(&reader, &ResolvedScope::Platform, Level::Admin),
            Decision::Deny(DENY_PLATFORM)
        );
        assert_eq!(
            evaluate(&reader, &ResolvedScope::Tenant("beta".into()), Level::Read),
            Decision::Deny(DENY_TENANT)
        );
        assert_eq!(
            evaluate(&reader, &ResolvedScope::Tenant("acme".into()), Level::Write),
            Decision::Deny(DENY_ROLE)
        );
        assert_eq!(
            evaluate(&reader, &ResolvedScope::Any, Level::Write),
            Decision::Deny(DENY_ROLE)
        );
    }

    /// PARITY: `Tenant` scope reproduces `can_access_tenant` (+ role gate) for
    /// every principal and every requested tenant.
    #[test]
    fn parity_tenant_scope() {
        for (p, perms, is_admin) in all_principals() {
            for requested in ["acme", "beta", "public"] {
                let scope = ResolvedScope::Tenant(requested.to_string());

                // Read: pure tenant access.
                let want_read = ref_can_access_tenant(&p.tenant, is_admin, requested);
                assert_eq!(
                    evaluate(&p, &scope, Level::Read).is_permit(),
                    want_read,
                    "Tenant/Read parity: {p:?} @ {requested}"
                );

                // Write: tenant access AND write role.
                let want_write = ref_can_access_tenant(&p.tenant, is_admin, requested)
                    && ref_can_write(perms, is_admin);
                assert_eq!(
                    evaluate(&p, &scope, Level::Write).is_permit(),
                    want_write,
                    "Tenant/Write parity: {p:?} @ {requested}"
                );

                // Admin: tenant access AND admin role.
                let want_admin = ref_can_access_tenant(&p.tenant, is_admin, requested)
                    && ref_can_admin(perms, is_admin);
                assert_eq!(
                    evaluate(&p, &scope, Level::Admin).is_permit(),
                    want_admin,
                    "Tenant/Admin parity: {p:?} @ {requested}"
                );
            }
        }
    }

    /// PARITY: `Platform` scope reproduces the bare `is_admin` gate that every
    /// god-only handler uses today.
    #[test]
    fn parity_platform_scope() {
        for (p, _perms, is_admin) in all_principals() {
            for level in [Level::Read, Level::Write, Level::Admin] {
                assert_eq!(
                    evaluate(&p, &ResolvedScope::Platform, level).is_permit(),
                    is_admin,
                    "Platform parity: {p:?} @ {level:?}"
                );
            }
        }
    }

    /// PARITY: `Any` scope reproduces "authenticated" (Read), `can_write`
    /// (Write), and `can_admin` (Admin) — the role gate with no tenant check.
    #[test]
    fn parity_any_scope() {
        for (p, perms, is_admin) in all_principals() {
            // Read: any authenticated principal passes.
            assert!(
                evaluate(&p, &ResolvedScope::Any, Level::Read).is_permit(),
                "Any/Read must permit any authenticated principal: {p:?}"
            );
            // Write: can_write.
            assert_eq!(
                evaluate(&p, &ResolvedScope::Any, Level::Write).is_permit(),
                ref_can_write(perms, is_admin),
                "Any/Write parity: {p:?}"
            );
            // Admin: can_admin.
            assert_eq!(
                evaluate(&p, &ResolvedScope::Any, Level::Admin).is_permit(),
                ref_can_admin(perms, is_admin),
                "Any/Admin parity: {p:?}"
            );
        }
    }

    #[test]
    fn principal_projection_from_key() {
        let key = AuthenticatedKey {
            key_id: uuid::Uuid::new_v4(),
            name: "k".into(),
            permissions: "write".into(),
            tenant_id: Some("acme".into()),
            is_admin: false,
        };
        let p = Principal::from_key(&key);
        assert_eq!(p.tenant.as_deref(), Some("acme"));
        assert_eq!(p.role, Level::Write);
        assert!(!p.platform_admin);
    }
}
