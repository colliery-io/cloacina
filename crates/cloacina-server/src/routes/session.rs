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

//! Login-session lifecycle — `/auth/refresh` + `/auth/logout` (CLOACI-T-0794).
//!
//! Both are **authenticated** (the caller presents its current minted key).
//! `refresh` silently re-mints a fresh short-TTL key before the current one
//! expires; `logout` revokes the key and forgets any server-side refresh state.
//!
//! Provider dispatch keys off the minted key's `issued_via` provenance:
//! - `local:<account_id>` — refresh = re-check the account is still `active`
//!   (no external call; the local-accounts strand, T-0795/0796).
//! - `oidc:<issuer>:<sub>` — refresh = re-mint inside the server-side login
//!   session opened by the callback (CLOACI-T-0923; was a 501 before that).
//!
//! # Why a session record and not the IdP's refresh token (CLOACI-T-0923)
//!
//! T-0793 built an encrypted store for the IdP refresh token, and the obvious
//! implementation of refresh is to spend that token at the IdP's token endpoint
//! and re-derive everything from the fresh ID token. We deliberately did not:
//!
//! * **It needs the IdP to cooperate.** Refresh tokens require `offline_access`
//!   (or an issuer-specific equivalent) and are not universally issued; the
//!   token endpoint is only *recommended* to return a new `id_token` on the
//!   refresh grant. A design that silently degrades to "no refresh" on half of
//!   the IdPs it meets does not close UC2.
//! * **It puts the IdP on the hot path.** Every session in the deployment
//!   re-contacts the issuer at least every 15 minutes; an IdP outage becomes a
//!   platform-wide forced logout.
//! * **It buys custody of a long-lived credential** — precisely the thing the
//!   short-TTL minted-key design exists to avoid — plus refresh-token rotation
//!   bookkeeping. That is full token-lifecycle management, not "stay signed in".
//!
//! Instead the callback opens an `oidc_sessions` row whose `expires_at` is the
//! session's **absolute** deadline. Refresh re-mints a fresh 15-minute key
//! inside that window, rotates the session onto it, and revokes the old key.
//! The one thing we give up — noticing that the IdP deactivated the user
//! mid-session — is bounded by that deadline
//! (`CLOACINA_OIDC_SESSION_MAX_AGE_S`, default 8h), after which a full re-auth
//! re-checks the IdP *and* re-resolves the current allowlist. That is a
//! deliberate, documented, tunable trade rather than an unbounded one.
//!
//! # Scope preservation (both providers)
//!
//! The re-mint reads `tenant_id` + `permissions` from the **persisted key row**,
//! which is the durable record of the original allowlist resolution — not from
//! anything the caller sends. [`mint_for_principal`] has no `is_admin`
//! parameter at all, so a refreshed key is structurally incapable of gaining
//! god mode however the request is shaped.

use std::time::Duration;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde::Serialize;
use tracing::{info, warn};
use utoipa::ToSchema;

use crate::identity::{mint_for_principal, ResolvedPrincipal, DEFAULT_MINTED_KEY_TTL};
use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::routes::local_auth::LocalLoginResponse;
use crate::AppState;

/// Default absolute lifetime of an OIDC login session: the wall past which
/// refreshing stops working and the user must sign in through the IdP again.
pub const DEFAULT_OIDC_SESSION_MAX_AGE: Duration = Duration::from_secs(8 * 60 * 60);

/// Absolute OIDC session lifetime, from `CLOACINA_OIDC_SESSION_MAX_AGE_S`.
///
/// This is the deployment's dial on the one thing the session-record design
/// trades away: how long an IdP-side deactivation can go unnoticed. Lower it
/// (e.g. 3600) when the IdP is the authority on employment status and you want
/// same-hour offboarding; raise it for kiosk-style long sessions.
pub fn oidc_session_max_age() -> Duration {
    std::env::var("CLOACINA_OIDC_SESSION_MAX_AGE_S")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|s| *s > 0)
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_OIDC_SESSION_MAX_AGE)
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LogoutResponse {
    pub status: String,
}

/// The caller's own identity + role (CLOACI-T-0803) — lets the UI gate
/// write/admin controls to the key's role instead of offering actions that
/// would 403.
#[derive(Debug, Serialize, ToSchema)]
pub struct WhoamiResponse {
    /// Tenant the key is scoped to (`None` = global/public).
    pub tenant_id: Option<String>,
    /// Role within the tenant: `read` | `write` | `admin`.
    pub role: String,
    /// God-mode flag (cross-tenant platform admin).
    pub is_admin: bool,
    /// The key's display name.
    pub name: String,
}

/// `GET /v1/auth/whoami` — return the caller's tenant, role, and admin flag.
/// Any authenticated key (read level); reads only the request's `AuthenticatedKey`.
#[utoipa::path(
    get,
    path = "/v1/auth/whoami",
    tag = "auth",
    responses(
        (status = 200, description = "The caller's identity + role", body = WhoamiResponse),
        (status = 401, description = "Invalid or revoked key", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn whoami(Extension(auth): Extension<AuthenticatedKey>) -> impl IntoResponse {
    Json(WhoamiResponse {
        tenant_id: auth.tenant_id.clone(),
        role: auth.permissions.clone(),
        is_admin: auth.is_admin,
        name: auth.name.clone(),
    })
}

/// `POST /v1/auth/refresh` — silently re-mint the caller's short-TTL key.
#[utoipa::path(
    post,
    path = "/v1/auth/refresh",
    tag = "auth",
    responses(
        (status = 200, description = "Re-minted key returned once", body = LocalLoginResponse),
        (status = 400, description = "Key is not a refreshable login key", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Invalid key, the account is disabled, or the login session has passed its absolute deadline", body = cloacina_api_types::ErrorBody),
        (status = 501, description = "Refresh for this provider not yet supported", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn refresh(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    let dal = cloacina::dal::DAL::new(state.database.clone());

    let info = match dal.api_keys().get_key(auth.key_id).await {
        Ok(Some(i)) => i,
        Ok(None) => return ApiError::unauthorized("key not found").into_response(),
        Err(e) => {
            warn!("refresh: get_key failed: {}", e);
            return ApiError::internal("refresh failed").into_response();
        }
    };

    let provenance = match info.issued_via.as_deref() {
        Some(p) => p.to_string(),
        None => {
            return ApiError::bad_request(
                "not_refreshable",
                "only minted login keys can be refreshed",
            )
            .into_response()
        }
    };

    // ---- local provider: re-check the account is still active ----
    if let Some(account_id) = provenance.strip_prefix("local:") {
        let account_id = match uuid::Uuid::parse_str(account_id) {
            Ok(i) => i,
            Err(_) => return ApiError::internal("malformed key provenance").into_response(),
        };
        let active = matches!(
            dal.local_accounts().get_by_id(account_id).await,
            Ok(Some(ref a)) if a.is_active()
        );
        if !active {
            // Deprovisioned mid-session: kill the current key and deny.
            let _ = dal.api_keys().revoke_key(auth.key_id).await;
            state.key_cache.clear().await;
            return ApiError::unauthorized("account is no longer active").into_response();
        }
        let principal = ResolvedPrincipal {
            tenant: info.tenant_id.clone(),
            role: info.permissions.clone(),
            provenance: provenance.clone(),
        };
        return match mint_for_principal(&state, &principal, DEFAULT_MINTED_KEY_TTL).await {
            Ok((plaintext, new_info)) => {
                // Revoke the old key so only the fresh one is valid.
                let _ = dal.api_keys().revoke_key(auth.key_id).await;
                state.key_cache.clear().await;
                info!(account_id = %account_id, "refresh: re-minted local session key");
                Json(LocalLoginResponse {
                    key: plaintext,
                    tenant_id: new_info.tenant_id,
                    role: new_info.permissions,
                    expires_at: new_info.expires_at.map(|t| t.to_rfc3339()),
                })
                .into_response()
            }
            Err(e) => e.into_response(),
        };
    }

    // ---- oidc provider: re-mint inside the server-side login session --------
    if provenance.starts_with("oidc:") {
        // The session row is the authority on "is this login still alive".
        // Absent (never opened, or logged out) or past its absolute deadline →
        // the user must go through the IdP again.
        let session = match dal.oidc_sessions().get_session(auth.key_id).await {
            Ok(s) => s,
            Err(e) => {
                warn!("refresh: oidc session lookup failed: {}", e);
                return ApiError::internal("refresh failed").into_response();
            }
        };
        let still_open = matches!(
            session,
            Some((_, Some(deadline))) if deadline > chrono::Utc::now()
        );
        if !still_open {
            // Session over: kill the key so the browser cannot keep using it
            // for the remainder of its TTL, and force a full re-auth.
            let _ = dal.api_keys().revoke_key(auth.key_id).await;
            let _ = dal.oidc_sessions().delete(auth.key_id).await;
            state.key_cache.clear().await;
            return ApiError::unauthorized("login session has expired — sign in again")
                .into_response();
        }

        // Scope comes from the persisted key row (the stored result of the
        // original allowlist mapping), never from the request. `is_admin` is
        // not a parameter of the mint path at all.
        let principal = ResolvedPrincipal {
            tenant: info.tenant_id.clone(),
            role: info.permissions.clone(),
            provenance: provenance.clone(),
        };
        return match mint_for_principal(&state, &principal, DEFAULT_MINTED_KEY_TTL).await {
            Ok((plaintext, new_info)) => {
                // Rotate the session onto the new key BEFORE revoking the old
                // one: if the rotation fails, the caller still holds a valid
                // key and can retry, rather than being logged out by our bug.
                // The session's absolute deadline is deliberately not extended.
                match dal.oidc_sessions().rotate(auth.key_id, new_info.id).await {
                    Ok(true) => {}
                    Ok(false) | Err(_) => {
                        warn!("refresh: oidc session rotation failed — leaving old key valid");
                        let _ = dal.api_keys().revoke_key(new_info.id).await;
                        return ApiError::internal("refresh failed").into_response();
                    }
                }
                let _ = dal.api_keys().revoke_key(auth.key_id).await;
                state.key_cache.clear().await;
                info!(provenance = %provenance, "refresh: re-minted OIDC session key");
                Json(LocalLoginResponse {
                    key: plaintext,
                    tenant_id: new_info.tenant_id,
                    role: new_info.permissions,
                    expires_at: new_info.expires_at.map(|t| t.to_rfc3339()),
                })
                .into_response()
            }
            Err(e) => e.into_response(),
        };
    }

    // ---- unknown provenance prefix — not a refreshable login key ----
    ApiError::new(
        StatusCode::NOT_IMPLEMENTED,
        "refresh_unsupported",
        "refresh for this provider is not yet supported",
    )
    .into_response()
}

/// `POST /v1/auth/logout` — revoke the caller's key + forget refresh state.
#[utoipa::path(
    post,
    path = "/v1/auth/logout",
    tag = "auth",
    responses(
        (status = 200, description = "Logged out", body = LogoutResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn logout(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    // Revoke the minted key and forget any server-side refresh session
    // (the latter is a no-op for local accounts, which store none).
    let _ = dal.api_keys().revoke_key(auth.key_id).await;
    let _ = dal.oidc_sessions().delete(auth.key_id).await;
    state.key_cache.clear().await;
    info!(key_id = %auth.key_id, "logout — key revoked + refresh forgotten");
    Json(LogoutResponse {
        status: "logged_out".to_string(),
    })
    .into_response()
}
