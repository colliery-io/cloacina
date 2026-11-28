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
//! # The OIDC session (CLOACI-T-0923, CLOACINA-T-1082)
//!
//! The callback opens an `oidc_sessions` row per tenant the login maps to,
//! whose `expires_at` is the session's **absolute** deadline
//! (`CLOACINA_OIDC_SESSION_MAX_AGE_S`, default 8h). Refresh re-mints a fresh
//! 15-minute key inside that window, rotates the session onto it, and revokes
//! the old key.
//!
//! The rows of a login carry its claims. Each refresh runs the **current**
//! allowlist on them: a lower role applies to the new key, and a tenant no
//! longer mapped ends that row. The map is read at start, so a map change is a
//! rolling restart, then live sessions follow at their next refresh.
//!
//! The IdP is not on every refresh. When the login gave a refresh token (and a
//! KEK is set) the rows store it, bound to the login, and once per
//! `CLOACINA_OIDC_GROUP_CHECK_INTERVAL_S` (default 1h) one refresh of the login
//! spends it and reads the groups again. An IdP that refuses the token ends
//! the login; an IdP that cannot be reached leaves the session on its last
//! claims until its deadline, with a warning. Rows opened before
//! CLOACINA-T-1082 have no claims and keep their role to their deadline.
//!
//! # Scope
//!
//! The tenant comes from the **persisted key row**; the role from the stored
//! key row (local, and OIDC rows with no claims) or the current allowlist —
//! never from anything the caller sends. [`mint_for_principal`] has no
//! `is_admin` parameter at all, so a refreshed key is structurally incapable
//! of gaining god mode however the request is shaped.

use std::time::Duration;

use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use tracing::{info, warn};
use utoipa::ToSchema;

use crate::AppState;
use crate::identity::{DEFAULT_MINTED_KEY_TTL, ResolvedPrincipal, mint_for_principal};
use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::routes::local_auth::LocalLoginResponse;

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

/// Default interval of the IdP group check of an OIDC login (CLOACINA-T-1082).
pub const DEFAULT_OIDC_GROUP_CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// How often a refresh reads the groups again at the IdP, from
/// `CLOACINA_OIDC_GROUP_CHECK_INTERVAL_S`.
pub fn oidc_group_check_interval() -> Duration {
    std::env::var("CLOACINA_OIDC_GROUP_CHECK_INTERVAL_S")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|s| *s > 0)
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_OIDC_GROUP_CHECK_INTERVAL)
}

/// Seal an IdP refresh token for the rows of `login_id` with the server KEK
/// (`CLOACINA_SECRET_KEK`), bound to the login. `None` when there is no
/// token, no KEK or an encryption error: the groups are then read at login
/// only. Never logs the token.
pub(crate) fn seal_refresh_token(token: Option<&str>, login_id: uuid::Uuid) -> Option<Vec<u8>> {
    use cloacina::dal::unified::oidc_sessions::{encrypt_token_bound, refresh_token_aad};
    let token = token?;
    let kek = match cloacina::security::SecretStoreResolver::kek_from_env() {
        Ok(kek) => kek,
        Err(_) => {
            warn!(
                "oidc: the IdP gave a refresh token but CLOACINA_SECRET_KEK is not set: it is not \
                 stored, and this login's groups are read at login only"
            );
            return None;
        }
    };
    match encrypt_token_bound(
        token.as_bytes(),
        &kek,
        refresh_token_aad(login_id).as_bytes(),
    ) {
        Ok(sealed) => Some(sealed),
        Err(e) => {
            warn!("oidc: the IdP refresh token could not be encrypted, so it is not stored: {e}");
            None
        }
    }
}

fn open_refresh_token(sealed: &[u8], login_id: uuid::Uuid) -> Result<String, String> {
    use cloacina::dal::unified::oidc_sessions::{decrypt_token_bound, refresh_token_aad};
    let kek = cloacina::security::SecretStoreResolver::kek_from_env().map_err(|e| e.to_string())?;
    let plain = decrypt_token_bound(sealed, &kek, refresh_token_aad(login_id).as_bytes())
        .map_err(|e| e.to_string())?;
    String::from_utf8(plain).map_err(|e| e.to_string())
}

/// What a refresh of an OIDC session does (CLOACINA-T-1082).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OidcScope {
    /// Mint the new key with this role.
    Role(String),
    /// End this row: the tenant is no longer mapped.
    EndRow(&'static str),
    /// End every row of the login: the IdP ended it.
    EndLogin(&'static str),
}

/// The role of the next key of an OIDC session row: the current allowlist run
/// on the login's claims, after the hourly IdP group check when it is due
/// (CLOACINA-T-1082). A row with no claims keeps the role of its key.
pub(crate) async fn oidc_scope_at_refresh(
    state: &AppState,
    checker: Option<&dyn crate::oidc::IdpClaimsCheck>,
    session: &cloacina::dal::unified::oidc_sessions::LoginSession,
    key: &cloacina::dal::unified::api_keys::ApiKeyInfo,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<OidcScope, ApiError> {
    use crate::oidc::{ClaimsCheck, IdentityClaims};

    // Opened before CLOACINA-T-1082: the role of the key, to the deadline.
    let Some(raw) = session.claims.as_deref() else {
        return Ok(OidcScope::Role(key.permissions.clone()));
    };
    let mut claims: IdentityClaims = match serde_json::from_str(raw) {
        Ok(c) => c,
        Err(e) => {
            warn!(key_id = %key.id, "oidc refresh: the stored login claims cannot be read: {e}");
            return Ok(OidcScope::EndRow(
                "the login session cannot be read — sign in again",
            ));
        }
    };

    if let (Some(login_id), Some(sealed), Some(checker)) =
        (session.login_id, session.refresh_enc.as_deref(), checker)
    {
        let interval = chrono::Duration::from_std(oidc_group_check_interval())
            .unwrap_or_else(|_| chrono::Duration::hours(1));
        let due = session.checked_at.is_none_or(|t| now - t >= interval);
        let dal = cloacina::dal::DAL::new(state.database.clone());
        let taken = due
            && dal
                .oidc_sessions()
                .claim_group_check(login_id, session.checked_at, now)
                .await
                .map_err(|e| ApiError::internal_logged("refresh failed", &e))?;
        if taken {
            match open_refresh_token(sealed, login_id) {
                Err(e) => warn!(
                    login_id = %login_id,
                    "oidc refresh: the stored IdP refresh token cannot be decrypted, so the \
                     groups are not checked at the IdP: {e}"
                ),
                Ok(token) => match checker.check(&token).await {
                    ClaimsCheck::Current {
                        claims: fresh,
                        refresh_token,
                    } => {
                        if fresh.subject != claims.subject {
                            warn!(login_id = %login_id, "oidc refresh: the IdP answered for another subject");
                            return Ok(OidcScope::EndLogin(
                                "the identity provider answered for another identity — sign in again",
                            ));
                        }
                        claims = fresh;
                        let stored = serde_json::to_string(&claims).unwrap_or_default();
                        let resealed = seal_refresh_token(refresh_token.as_deref(), login_id);
                        dal.oidc_sessions()
                            .store_login_claims(login_id, &stored, resealed)
                            .await
                            .map_err(|e| ApiError::internal_logged("refresh failed", &e))?;
                        info!(login_id = %login_id, "oidc refresh: groups checked at the IdP");
                    }
                    ClaimsCheck::Refused(reason) => {
                        info!(login_id = %login_id, "oidc refresh: the IdP refused the refresh token: {reason}");
                        return Ok(OidcScope::EndLogin(
                            "the identity provider ended the login — sign in again",
                        ));
                    }
                    ClaimsCheck::Unreachable(reason) => warn!(
                        login_id = %login_id,
                        "oidc refresh: the IdP could not be reached for the group check; the \
                         session keeps its last groups until its deadline: {reason}"
                    ),
                },
            }
        }
    }

    let principals = state.oidc_policy.resolve_all(&claims, &session.provider);
    match principals.into_iter().find(|p| p.tenant == key.tenant_id) {
        Some(p) => Ok(OidcScope::Role(p.role)),
        None => Ok(OidcScope::EndRow(
            "your identity is no longer mapped to this tenant — sign in again",
        )),
    }
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
    let provider = state.oidc.clone();
    let checker = provider
        .as_deref()
        .map(|p| p as &dyn crate::oidc::IdpClaimsCheck);
    refresh_with(&state, auth, checker).await
}

/// The refresh, with the IdP group check given (CLOACINA-T-1082): the
/// configured provider in the route, a mock in tests.
pub(crate) async fn refresh_with(
    state: &AppState,
    auth: AuthenticatedKey,
    checker: Option<&dyn crate::oidc::IdpClaimsCheck>,
) -> axum::response::Response {
    let state = state.clone();
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
            .into_response();
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
            crate::routes::auth::announce_key_revocation_or_warn(&state).await;
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
                crate::routes::auth::announce_key_revocation_or_warn(&state).await;
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
        let session = match dal.oidc_sessions().get_login_session(auth.key_id).await {
            Ok(s) => s,
            Err(e) => {
                warn!("refresh: oidc session lookup failed: {}", e);
                return ApiError::internal("refresh failed").into_response();
            }
        };
        let now = chrono::Utc::now();
        let session = match session {
            Some(s) if s.expires_at.is_some_and(|deadline| deadline > now) => s,
            _ => {
                // Session over: kill the key so the browser cannot keep using it
                // for the remainder of its TTL, and force a full re-auth.
                let _ = dal.api_keys().revoke_key(auth.key_id).await;
                let _ = dal.oidc_sessions().delete(auth.key_id).await;
                crate::routes::auth::announce_key_revocation_or_warn(&state).await;
                return ApiError::unauthorized("login session has expired — sign in again")
                    .into_response();
            }
        };

        // The tenant comes from the persisted key row; the role from the
        // current allowlist run on the login's claims (CLOACINA-T-1082), never
        // from the request. `is_admin` is not a parameter of the mint path.
        let role = match oidc_scope_at_refresh(&state, checker, &session, &info, now).await {
            Ok(OidcScope::Role(role)) => role,
            Ok(OidcScope::EndRow(reason)) => {
                let _ = dal.api_keys().revoke_key(auth.key_id).await;
                let _ = dal.oidc_sessions().delete(auth.key_id).await;
                crate::routes::auth::announce_key_revocation_or_warn(&state).await;
                info!(provenance = %provenance, "refresh: OIDC session ended: {reason}");
                return ApiError::unauthorized(reason).into_response();
            }
            Ok(OidcScope::EndLogin(reason)) => {
                let mut keys = vec![auth.key_id];
                if let Some(login_id) = session.login_id {
                    keys.extend(
                        dal.oidc_sessions()
                            .delete_login(login_id)
                            .await
                            .unwrap_or_default(),
                    );
                }
                let _ = dal.oidc_sessions().delete(auth.key_id).await;
                for key in keys {
                    let _ = dal.api_keys().revoke_key(key).await;
                }
                crate::routes::auth::announce_key_revocation_or_warn(&state).await;
                info!(provenance = %provenance, "refresh: OIDC login ended: {reason}");
                return ApiError::unauthorized(reason).into_response();
            }
            Err(e) => return e.into_response(),
        };
        let principal = ResolvedPrincipal {
            tenant: info.tenant_id.clone(),
            role,
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
                crate::routes::auth::announce_key_revocation_or_warn(&state).await;
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
    crate::routes::auth::announce_key_revocation_or_warn(&state).await;
    info!(key_id = %auth.key_id, "logout — key revoked + refresh forgotten");
    Json(LogoutResponse {
        status: "logged_out".to_string(),
    })
    .into_response()
}
