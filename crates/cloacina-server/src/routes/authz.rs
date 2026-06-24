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

use std::collections::HashMap;

use axum::extract::{FromRequestParts, MatchedPath, RawPathParams, Request, State};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use tracing::warn;

use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::AppState;

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

// ---------------------------------------------------------------------------
// Route → Access table + the authorization middleware (CLOACI-T-0783).
// ---------------------------------------------------------------------------

/// The declarative authorization table: `(method, registration-path) -> Access`
/// for every route behind the `authz_mw` layer. Keys use the **relative** paths
/// the routes are registered with (no `/v1` prefix); the middleware strips the
/// nest prefix from `MatchedPath` before lookup.
pub type AuthzTable = HashMap<(Method, String), Access>;

/// Build the authorization table for every `/v1` route behind `authz_mw`.
///
/// **Behavior-preserving (T-0783):** each entry reproduces the route's gate as
/// it exists *today*. In particular `/auth/keys*` is `Any + Admin` (today's
/// unscoped `can_admin`) and `/tenants/{tenant_id}/keys` + `/agents` are
/// `Platform` (today's `is_admin`) — the leak fix and the tenant-admin lowering
/// land in T-0784 / T-0785, which edit these entries.
///
/// The `/ws/*` routes are intentionally absent: they authenticate + authorize
/// in-handler (pre-upgrade) and are not behind `authz_mw`.
pub fn build_authz_table() -> AuthzTable {
    let mut t: AuthzTable = HashMap::new();
    let mut add = |m: Method, p: &str, a: Access| {
        t.insert((m, p.to_string()), a);
    };

    // ----- Platform + Admin (today: is_admin) -----
    add(Method::POST, "/tenants", Access::platform(Level::Admin));
    add(Method::GET, "/tenants", Access::platform(Level::Admin));
    add(Method::DELETE, "/tenants/{schema_name}", Access::platform(Level::Admin));
    add(Method::GET, "/compiler/status", Access::platform(Level::Admin));
    // CLOACI-T-0785: tenant-admin (was Platform); the handler filters the
    // roster to the caller's tenant (god sees all).
    add(Method::GET, "/agents", Access::any(Level::Admin));

    // ----- Tenant + Admin: tenant-admin key self-service (CLOACI-T-0784).
    //       POST lowered from Platform; GET/DELETE are new. The DELETE handler
    //       additionally verifies the target key belongs to {tenant_id}. -----
    add(Method::POST, "/tenants/{tenant_id}/keys", Access::tenant(Level::Admin));
    add(Method::GET, "/tenants/{tenant_id}/keys", Access::tenant(Level::Admin));
    add(
        Method::DELETE,
        "/tenants/{tenant_id}/keys/{key_id}",
        Access::tenant(Level::Admin),
    );

    // CLOACI-T-0797: tenant-admin local-account management.
    add(Method::POST, "/tenants/{tenant_id}/accounts", Access::tenant(Level::Admin));
    add(Method::GET, "/tenants/{tenant_id}/accounts", Access::tenant(Level::Admin));
    add(
        Method::DELETE,
        "/tenants/{tenant_id}/accounts/{account_id}",
        Access::tenant(Level::Admin),
    );
    add(
        Method::POST,
        "/tenants/{tenant_id}/accounts/{account_id}/password",
        Access::tenant(Level::Admin),
    );

    // ----- Platform + Admin: the global key surface (CLOACI-T-0784 leak fix).
    //       Was Any+Admin (today's unscoped can_admin), which let a tenant
    //       role=admin key list/revoke ANY tenant's keys. God-only now;
    //       tenant-admins manage their own keys under /tenants/{t}/keys. -----
    add(Method::POST, "/auth/keys", Access::platform(Level::Admin));
    add(Method::GET, "/auth/keys", Access::platform(Level::Admin));
    add(Method::DELETE, "/auth/keys/{key_id}", Access::platform(Level::Admin));

    // ----- Tenant + Read -----
    add(Method::GET, "/tenants/{tenant_id}/workflows", Access::tenant(Level::Read));
    add(Method::GET, "/tenants/{tenant_id}/workflows/{name}", Access::tenant(Level::Read));
    add(Method::GET, "/tenants/{tenant_id}/workflows/{name}/source", Access::tenant(Level::Read));
    add(Method::GET, "/tenants/{tenant_id}/triggers", Access::tenant(Level::Read));
    add(Method::GET, "/tenants/{tenant_id}/triggers/{name}", Access::tenant(Level::Read));
    add(Method::GET, "/tenants/{tenant_id}/triggers/{name}/interface", Access::tenant(Level::Read));
    add(Method::GET, "/tenants/{tenant_id}/executions", Access::tenant(Level::Read));
    add(Method::GET, "/tenants/{tenant_id}/executions/{exec_id}", Access::tenant(Level::Read));
    add(Method::GET, "/tenants/{tenant_id}/executions/{exec_id}/events", Access::tenant(Level::Read));
    add(Method::GET, "/tenants/{tenant_id}/executions/{exec_id}/tasks", Access::tenant(Level::Read));

    // ----- Tenant + Write -----
    add(Method::POST, "/tenants/{tenant_id}/workflows", Access::tenant(Level::Write));
    add(Method::POST, "/tenants/{tenant_id}/workflows/{name}/pause", Access::tenant(Level::Write));
    add(Method::POST, "/tenants/{tenant_id}/workflows/{name}/resume", Access::tenant(Level::Write));
    add(Method::DELETE, "/tenants/{tenant_id}/workflows/{name}/{version}", Access::tenant(Level::Write));
    add(Method::POST, "/tenants/{tenant_id}/workflows/{name}/execute", Access::tenant(Level::Write));
    add(Method::POST, "/tenants/{tenant_id}/triggers/{name}/pause", Access::tenant(Level::Write));
    add(Method::POST, "/tenants/{tenant_id}/triggers/{name}/resume", Access::tenant(Level::Write));
    add(Method::POST, "/tenants/{tenant_id}/triggers/{name}/fire", Access::tenant(Level::Write));

    // ----- Any + Read (today: authenticated; data-scoping stays in handler) -----
    add(Method::POST, "/auth/ws-ticket", Access::any(Level::Read));
    add(Method::POST, "/agent/register", Access::any(Level::Read));
    add(Method::POST, "/agent/heartbeat", Access::any(Level::Read));
    add(Method::POST, "/agent/result", Access::any(Level::Read));
    add(Method::GET, "/agent/artifact/{digest}", Access::any(Level::Read));
    add(Method::GET, "/agent/source/{digest}", Access::any(Level::Read));
    add(Method::GET, "/health/accumulators", Access::any(Level::Read));
    add(Method::GET, "/health/reactors", Access::any(Level::Read));
    add(Method::POST, "/health/reactors/{name}/fire", Access::any(Level::Read));
    add(Method::GET, "/health/reactors/{name}/fires", Access::any(Level::Read));
    add(Method::GET, "/health/reactors/{name}/fires/timeseries", Access::any(Level::Read));
    add(Method::POST, "/health/accumulators/{name}/inject", Access::any(Level::Read));
    add(Method::GET, "/health/reactors/{name}/interface", Access::any(Level::Read));
    add(Method::GET, "/health/accumulators/{name}/interface", Access::any(Level::Read));
    add(Method::GET, "/health/graphs", Access::any(Level::Read));
    add(Method::GET, "/health/graphs/{name}", Access::any(Level::Read));

    t
}

/// Map a matcher [`Decision`] deny reason onto the canonical 403 envelopes
/// (identical to the responses the handlers returned before T-0783).
fn deny_response(reason: &str) -> Response {
    let err = match reason {
        DENY_PLATFORM => ApiError::forbidden("admin_required", "admin access required"),
        DENY_TENANT => {
            ApiError::forbidden("tenant_access_denied", "access denied for this tenant")
        }
        _ => ApiError::forbidden("insufficient_permissions", "insufficient permissions"),
    };
    err.into_response()
}

/// Authorization middleware (CLOACI-T-0783).
///
/// Mounted **after** `require_auth` on the authed sub-routers, so the
/// `AuthenticatedKey` extension is already present. Looks the matched route up
/// in the [`AuthzTable`], resolves `TenantParam` from the `{tenant_id}` path
/// segment, projects the key into a [`Principal`], and runs [`evaluate`].
///
/// **Fail-closed:** a matched route with no table entry (or a missing key /
/// missing tenant param) is denied, never allowed.
pub async fn authz_mw(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let method = request.method().clone();

    // Strip the `/v1` nest prefix so the key matches the relative registration
    // path stored in the table (and tolerate either form).
    let matched_path = match request.extensions().get::<MatchedPath>() {
        Some(mp) => {
            let p = mp.as_str();
            p.strip_prefix("/v1").unwrap_or(p).to_string()
        }
        None => {
            warn!(%method, "authz: no MatchedPath — denying (fail-closed)");
            return ApiError::forbidden("authz_unclassified", "route not authorized")
                .into_response();
        }
    };

    let access = match state.authz_table.get(&(method.clone(), matched_path.clone())) {
        Some(a) => *a,
        None => {
            warn!(%method, path = %matched_path, "authz: unclassified route — denying (fail-closed)");
            return ApiError::forbidden("authz_unclassified", "route not authorized")
                .into_response();
        }
    };

    let principal = match request.extensions().get::<AuthenticatedKey>() {
        Some(key) => Principal::from_key(key),
        None => {
            // require_auth must run first; if the key is absent, fail closed.
            return ApiError::unauthorized("missing authentication").into_response();
        }
    };

    // Resolve the scope (only TenantParam needs the path param).
    match access.scope {
        Scope::Platform => match evaluate(&principal, &ResolvedScope::Platform, access.level) {
            Decision::Permit => next.run(request).await,
            Decision::Deny(reason) => deny_response(reason),
        },
        Scope::Any => match evaluate(&principal, &ResolvedScope::Any, access.level) {
            Decision::Permit => next.run(request).await,
            Decision::Deny(reason) => deny_response(reason),
        },
        Scope::TenantParam => {
            let (mut parts, body) = request.into_parts();
            let tenant = RawPathParams::from_request_parts(&mut parts, &state)
                .await
                .ok()
                .and_then(|params| {
                    params
                        .iter()
                        .find(|(k, _)| *k == "tenant_id")
                        .map(|(_, v)| v.to_string())
                });
            let request = Request::from_parts(parts, body);
            let tenant = match tenant {
                Some(t) => t,
                None => {
                    warn!(path = %matched_path, "authz: TenantParam route missing tenant_id — denying");
                    return deny_response(DENY_TENANT);
                }
            };
            match evaluate(&principal, &ResolvedScope::Tenant(tenant), access.level) {
                Decision::Permit => next.run(request).await,
                Decision::Deny(reason) => deny_response(reason),
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

    /// No-drift / behavior-preservation guard for the route table. The runtime
    /// guarantee is the fail-closed middleware (an unclassified route 403s); this
    /// pins the full set + spot-checks the behavior-preserving classifications.
    #[test]
    fn authz_table_classifies_known_routes() {
        let t = build_authz_table();
        assert_eq!(
            t.len(),
            49,
            "authz table size changed — a route was added/removed without updating the table"
        );

        let get = |m: Method, p: &str| t.get(&(m, p.to_string())).copied();

        // CLOACI-T-0784: the global key surface is god-only (leak fix)...
        assert_eq!(get(Method::POST, "/auth/keys"), Some(Access::platform(Level::Admin)));
        assert_eq!(get(Method::GET, "/auth/keys"), Some(Access::platform(Level::Admin)));
        assert_eq!(
            get(Method::DELETE, "/auth/keys/{key_id}"),
            Some(Access::platform(Level::Admin))
        );
        // ...and tenant-admins manage their own keys.
        assert_eq!(
            get(Method::POST, "/tenants/{tenant_id}/keys"),
            Some(Access::tenant(Level::Admin))
        );
        assert_eq!(
            get(Method::GET, "/tenants/{tenant_id}/keys"),
            Some(Access::tenant(Level::Admin))
        );
        assert_eq!(
            get(Method::DELETE, "/tenants/{tenant_id}/keys/{key_id}"),
            Some(Access::tenant(Level::Admin))
        );
        assert_eq!(get(Method::POST, "/tenants"), Some(Access::platform(Level::Admin)));
        assert_eq!(get(Method::GET, "/agents"), Some(Access::any(Level::Admin))); // T-0785
        assert_eq!(get(Method::GET, "/compiler/status"), Some(Access::platform(Level::Admin)));
        assert_eq!(
            get(Method::POST, "/tenants/{tenant_id}/workflows"),
            Some(Access::tenant(Level::Write))
        );
        assert_eq!(
            get(Method::GET, "/tenants/{tenant_id}/workflows"),
            Some(Access::tenant(Level::Read))
        );
        assert_eq!(
            get(Method::POST, "/tenants/{tenant_id}/workflows/{name}/execute"),
            Some(Access::tenant(Level::Write))
        );
        assert_eq!(get(Method::POST, "/agent/register"), Some(Access::any(Level::Read)));
        assert_eq!(get(Method::GET, "/health/graphs"), Some(Access::any(Level::Read)));

        // WS routes are NOT behind authz_mw (in-handler auth); unknown routes
        // are absent -> the middleware fail-closes them.
        assert_eq!(get(Method::GET, "/ws/delivery/{recipient}"), None);
        assert_eq!(get(Method::GET, "/totally/unknown"), None);
    }
}
