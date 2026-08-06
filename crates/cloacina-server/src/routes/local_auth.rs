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

//! Local (self-managed) username/password login — CLOACI-T-0796.
//!
//! `POST /v1/auth/local/login` is **public** (the caller has no bearer key
//! yet). It verifies a password against [`local_accounts`], and on success
//! mints a short-TTL bearer key tagged `issued_via = local:<account_id>` via the
//! provider-agnostic [`crate::identity::mint_for_principal`]. The key flows
//! through `require_auth` + the Phase 0 authZ matcher exactly like any other.
//!
//! No refresh-token row is stored for local accounts: the key's `local:<id>`
//! provenance + the account's `status` column ARE the refresh validity, which
//! `/auth/refresh` (T-0794) re-checks.
//!
//! ## Brute-force throttle (CLOACI-T-0923, closes I-0118 OQ-13)
//!
//! Argon2id makes each guess expensive for the *server*; it does nothing to
//! bound how many guesses an attacker gets. Every attempt is therefore gated on
//! (and every failure recorded against) the DB-backed
//! [`cloacina::dal::unified::login_throttle`] counters — persisted, not a
//! per-replica map, so a lockout holds across replicas. See that module for the
//! dual-key (username + source-IP) rationale.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, State},
    http::{header, Extensions, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use utoipa::ToSchema;

use cloacina::dal::unified::login_throttle::{
    ip_key, username_key, ThrottlePolicy, SCOPE_IP, SCOPE_USERNAME,
};
use cloacina::dal::unified::{LocalAccount, LoginOutcome};
use cloacina::security::audit;
use cloacina_api_types::common::ListResponse;

use crate::identity::{mint_for_principal, ResolvedPrincipal, DEFAULT_MINTED_KEY_TTL};
use crate::routes::error::ApiError;
use crate::AppState;

/// Operator-tunable brute-force throttle settings, resolved once at startup and
/// carried on [`AppState`] (CLOACI-T-0923).
#[derive(Debug, Clone, Copy)]
pub struct LoginThrottleConfig {
    /// Username-scoped policy — the primary defense.
    pub username: ThrottlePolicy,
    /// Source-IP-scoped policy, or `None` when the IP scope is disabled.
    pub ip: Option<ThrottlePolicy>,
    /// Whether `X-Forwarded-For` may be believed when deriving the client IP.
    pub trust_proxy_headers: bool,
}

impl Default for LoginThrottleConfig {
    fn default() -> Self {
        Self {
            username: ThrottlePolicy::username_default(),
            ip: Some(ThrottlePolicy::ip_default()),
            trust_proxy_headers: false,
        }
    }
}

impl LoginThrottleConfig {
    /// Read the throttle knobs from the environment.
    ///
    /// - `CLOACINA_LOGIN_THROTTLE_USER_THRESHOLD` (default 5) — consecutive
    ///   failures per username before it locks.
    /// - `CLOACINA_LOGIN_THROTTLE_IP_THRESHOLD` (default 50; **`0` disables the
    ///   IP scope**) — consecutive failures from one source address.
    /// - `CLOACINA_LOGIN_THROTTLE_BASE_LOCK_S` (default 30) — first lock
    ///   window; each further failure doubles it.
    /// - `CLOACINA_LOGIN_THROTTLE_MAX_LOCK_S` (default 900) — backoff ceiling,
    ///   and also the idle window after which a counter is forgotten.
    /// - `CLOACINA_TRUST_PROXY_HEADERS` (default off) — see [`client_ip`].
    ///
    /// **Deploying behind a reverse proxy:** the socket peer is then the proxy
    /// for *every* user, so one shared IP counter would cover the whole
    /// deployment. Either set `CLOACINA_TRUST_PROXY_HEADERS=1` (only safe when
    /// the proxy overwrites `X-Forwarded-For`) so real client addresses are
    /// used, or set `CLOACINA_LOGIN_THROTTLE_IP_THRESHOLD=0` to turn the IP
    /// scope off and rely on the username scope alone.
    pub fn from_env() -> Self {
        fn num(var: &str, default: i64) -> i64 {
            std::env::var(var)
                .ok()
                .and_then(|v| v.trim().parse::<i64>().ok())
                .filter(|v| *v >= 0)
                .unwrap_or(default)
        }
        let base_lock = chrono::Duration::seconds(num("CLOACINA_LOGIN_THROTTLE_BASE_LOCK_S", 30));
        let max_lock = chrono::Duration::seconds(num("CLOACINA_LOGIN_THROTTLE_MAX_LOCK_S", 900));
        let decay = max_lock;

        let user_threshold = num("CLOACINA_LOGIN_THROTTLE_USER_THRESHOLD", 5).max(1) as i32;
        let ip_threshold = num("CLOACINA_LOGIN_THROTTLE_IP_THRESHOLD", 50) as i32;

        Self {
            username: ThrottlePolicy {
                threshold: user_threshold,
                base_lock,
                max_lock,
                decay,
            },
            // The IP scope is a blunt instrument: cap its backoff at the
            // ceiling immediately rather than doubling, so a shared-NAT office
            // never compounds its way into an hours-long block.
            ip: (ip_threshold > 0).then_some(ThrottlePolicy {
                threshold: ip_threshold,
                base_lock: max_lock,
                max_lock,
                decay,
            }),
            trust_proxy_headers: matches!(
                std::env::var("CLOACINA_TRUST_PROXY_HEADERS")
                    .unwrap_or_default()
                    .trim(),
                "1" | "true" | "TRUE" | "yes"
            ),
        }
    }
}

/// Derive the client address used for the IP-scoped counter.
///
/// Defaults to the **socket peer**, which cannot be forged. `X-Forwarded-For`
/// is consulted only when the operator opts in, because an unauthenticated
/// caller controls that header outright: believing it by default would let an
/// attacker mint a fresh IP identity per request (evading the counter entirely)
/// *and* forge failures against someone else's address. When trusted, the
/// left-most entry is taken — the original client as recorded by the first
/// proxy — which is only meaningful if that proxy overwrites rather than
/// appends.
fn client_ip(headers: &HeaderMap, peer: Option<SocketAddr>, trust_proxy: bool) -> String {
    if trust_proxy {
        if let Some(first) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return first.to_string();
        }
    }
    peer.map(|p| p.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// 429 for a throttled attempt, carrying `Retry-After`.
///
/// Deliberately the *same* response whether or not the username exists: an
/// unknown username accumulates failures exactly like a real one, so this
/// status is never an enumeration oracle. It is a distinct status from the
/// 401 so a legitimate locked-out human (and the SPA) can tell "wrong password"
/// from "wait and try again".
fn throttled_response(retry_after_secs: i64) -> Response {
    let mut resp = ApiError::new(
        StatusCode::TOO_MANY_REQUESTS,
        "login_throttled",
        "too many failed login attempts — try again later",
    )
    .into_response();
    if let Ok(v) = HeaderValue::from_str(&retry_after_secs.max(1).to_string()) {
        resp.headers_mut().insert(header::RETRY_AFTER, v);
    }
    resp
}

/// A local-login attempt. `tenant` selects which tenant's account namespace to
/// authenticate against (`None` = a global account).
#[derive(Debug, Deserialize, ToSchema)]
pub struct LocalLoginRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub tenant: Option<String>,
}

/// A successful local login. `key` is the minted bearer key — shown exactly
/// once; the SPA stores it (sessionStorage) and presents it as `Bearer`.
#[derive(Debug, Serialize, ToSchema)]
pub struct LocalLoginResponse {
    pub key: String,
    pub tenant_id: Option<String>,
    pub role: String,
    pub expires_at: Option<String>,
}

/// `POST /v1/auth/local/login` — verify a password, mint a short-TTL key.
#[utoipa::path(
    post,
    path = "/v1/auth/local/login",
    tag = "auth",
    request_body = LocalLoginRequest,
    responses(
        (status = 200, description = "Logged in — minted key returned once", body = LocalLoginResponse),
        (status = 401, description = "Invalid username or password", body = cloacina_api_types::ErrorBody),
        (status = 429, description = "Too many failed attempts — throttled; see Retry-After", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    )
)]
pub async fn local_login(
    State(state): State<AppState>,
    // `ConnectInfo` is read out of the extensions rather than extracted
    // directly: axum 0.8 has no optional impl for it, and this route must not
    // 500 when the router is driven without connect-info (tests, `oneshot`).
    // Absent peer just means the IP scope degrades to a single `unknown` key.
    extensions: Extensions,
    headers: HeaderMap,
    Json(body): Json<LocalLoginRequest>,
) -> impl IntoResponse {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let cfg = state.login_throttle;

    let peer = extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(a)| *a);
    let user_key = username_key(body.tenant.as_deref(), &body.username);
    let source_ip = client_ip(&headers, peer, cfg.trust_proxy_headers);
    let source_key = ip_key(&source_ip);

    // ---- gate: refuse locked keys BEFORE paying for an argon2 verify --------
    // Both scopes are checked; either one locked refuses the attempt.
    let mut scopes: Vec<(&str, &str, ThrottlePolicy)> =
        vec![(user_key.as_str(), SCOPE_USERNAME, cfg.username)];
    if let Some(p) = cfg.ip {
        scopes.push((source_key.as_str(), SCOPE_IP, p));
    }
    for (key, scope, _) in &scopes {
        match dal.login_throttle().locked_until(key).await {
            Ok(Some(until)) => {
                let secs = (until - chrono::Utc::now()).num_seconds();
                metrics::counter!("cloacina_auth_login_attempts_total", "outcome" => "throttled")
                    .increment(1);
                warn!(scope = %scope, retry_after_s = secs, "local login refused — throttled");
                return throttled_response(secs);
            }
            Ok(None) => {}
            // Fail OPEN on a throttle-store error, deliberately: the throttle
            // is a mitigation, not the authentication decision, and a DB blip
            // must not lock every user out of the platform. The failure is
            // logged, and the password check below still has to pass.
            Err(e) => warn!("login throttle lookup failed (allowing attempt): {}", e),
        }
    }

    let outcome = match dal
        .local_accounts()
        .authenticate(&body.username, &body.password, body.tenant.as_deref())
        .await
    {
        Ok(o) => o,
        Err(e) => {
            warn!("local login DB error: {}", e);
            return ApiError::internal("login failed").into_response();
        }
    };

    let account = match outcome {
        LoginOutcome::Ok(a) => a,
        // Same opaque error for unknown user / wrong password / disabled — no
        // enumeration. `authenticate` also burns an equivalent argon2 verify
        // for an unknown username so the two are indistinguishable by timing.
        LoginOutcome::Denied => {
            // Record against BOTH scopes. Note this counts failures for
            // usernames that do not exist exactly like real ones — that is
            // what keeps the eventual 429 from leaking account existence.
            let now = chrono::Utc::now();
            let mut lock_expiry: Option<i64> = None;
            for (key, scope, policy) in &scopes {
                match dal
                    .login_throttle()
                    .record_failure(key, scope, *policy)
                    .await
                {
                    Ok(st) => {
                        if let Some(until) = st.locked_until.filter(|t| *t > now) {
                            let secs = (until - now).num_seconds();
                            // Report the longest live lock, so `Retry-After` is
                            // never optimistic about when the caller may return.
                            lock_expiry = Some(lock_expiry.map_or(secs, |c: i64| c.max(secs)));
                            // Audit + count the lock EDGE only: while a lock
                            // holds, every further attempt would otherwise
                            // re-emit the same incident.
                            if st.newly_locked {
                                audit::log_login_lockout(key, scope, st.failure_count, secs);
                                metrics::counter!("cloacina_auth_login_lockouts_total", "scope" => (*scope).to_string())
                                    .increment(1);
                            }
                        }
                    }
                    Err(e) => warn!("login throttle record failed: {}", e),
                }
            }
            // The attempt that crosses the threshold is answered with the 429,
            // not a 401 that would leave the caller unaware they are now
            // locked. This is not an enumeration leak: an unknown username
            // reaches the identical 429 on the identical attempt number.
            if let Some(secs) = lock_expiry {
                metrics::counter!("cloacina_auth_login_attempts_total", "outcome" => "throttled")
                    .increment(1);
                return throttled_response(secs);
            }
            metrics::counter!("cloacina_auth_login_attempts_total", "outcome" => "denied")
                .increment(1);
            return ApiError::unauthorized("invalid username or password").into_response();
        }
    };

    // ---- success: the legitimate user must not inherit their own failures ---
    // Only the username counter is cleared. The IP counter is left to decay:
    // clearing it on success would let an attacker who holds one valid account
    // reset the spray counter at will (see the DAL module docs).
    if let Err(e) = dal.login_throttle().clear(&user_key).await {
        warn!("login throttle clear failed: {}", e);
    }
    // Opportunistic hygiene — keeps the table bounded without a sweeper task.
    let _ = dal.login_throttle().prune_idle(cfg.username.decay).await;

    let principal = ResolvedPrincipal {
        tenant: account.tenant_id.clone(),
        role: account.role.clone(),
        provenance: format!("local:{}", account.id),
    };

    match mint_for_principal(&state, &principal, DEFAULT_MINTED_KEY_TTL).await {
        Ok((plaintext, info)) => {
            metrics::counter!("cloacina_auth_login_attempts_total", "outcome" => "ok").increment(1);
            info!(
                account_id = %account.id,
                tenant = ?account.tenant_id,
                role = %account.role,
                "local login succeeded — minted short-TTL key"
            );
            (
                StatusCode::OK,
                Json(LocalLoginResponse {
                    key: plaintext,
                    tenant_id: info.tenant_id,
                    role: info.permissions,
                    expires_at: info.expires_at.map(|t| t.to_rfc3339()),
                }),
            )
                .into_response()
        }
        Err(e) => e.into_response(),
    }
}

// ---------------------------------------------------------------------------
// Tenant-admin local-account management (CLOACI-T-0797). All routes are
// `TenantParam + Admin` in the authz table, so the caller is already confined
// to `{tenant_id}`; the DAL list/update calls are additionally tenant-scoped.
// ---------------------------------------------------------------------------

/// Create a local account in a tenant.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAccountRequest {
    pub username: String,
    pub password: String,
    pub role: String,
}

/// Reset a local account's password (admin-reset-only, OQ-12).
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub password: String,
}

/// Public view of a local account (never the password hash).
#[derive(Debug, Serialize, ToSchema)]
pub struct AccountInfo {
    pub id: String,
    pub username: String,
    pub role: String,
    pub status: String,
}

/// Outcome of a disable/reset action.
#[derive(Debug, Serialize, ToSchema)]
pub struct AccountActionResponse {
    pub status: String,
    pub id: String,
}

fn to_account_info(a: LocalAccount) -> AccountInfo {
    AccountInfo {
        id: a.id.to_string(),
        username: a.username,
        role: a.role,
        status: a.status,
    }
}

/// `POST /v1/tenants/{tenant_id}/accounts` — create a tenant local account.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/accounts",
    tag = "auth",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    request_body = CreateAccountRequest,
    responses(
        (status = 201, description = "Account created", body = AccountInfo),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn create_account(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Json(body): Json<CreateAccountRequest>,
) -> impl IntoResponse {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal
        .local_accounts()
        .create(&body.username, &body.password, Some(&tenant_id), &body.role)
        .await
    {
        Ok(a) => (StatusCode::CREATED, Json(to_account_info(a))).into_response(),
        Err(e) => {
            warn!("create local account failed: {}", e);
            ApiError::internal("failed to create account").into_response()
        }
    }
}

/// `GET /v1/tenants/{tenant_id}/accounts` — list a tenant's local accounts.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/accounts",
    tag = "auth",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    responses(
        (status = 200, description = "Tenant local accounts (no hashes)", body = ListResponse<AccountInfo>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_accounts(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal.local_accounts().list_for_tenant(Some(&tenant_id)).await {
        Ok(accounts) => {
            let items: Vec<AccountInfo> = accounts.into_iter().map(to_account_info).collect();
            Json(ListResponse::new(items)).into_response()
        }
        Err(e) => {
            warn!("list local accounts failed: {}", e);
            ApiError::internal("failed to list accounts").into_response()
        }
    }
}

/// `DELETE /v1/tenants/{tenant_id}/accounts/{account_id}` — disable an account.
/// Disable (not hard-delete) preserves history; already-minted keys lapse at
/// their TTL (deprovisioning latency bounded by the short TTL).
#[utoipa::path(
    delete,
    path = "/v1/tenants/{tenant_id}/accounts/{account_id}",
    tag = "auth",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("account_id" = String, Path, description = "Account UUID"),
    ),
    responses(
        (status = 200, description = "Account disabled", body = AccountActionResponse),
        (status = 400, description = "Invalid account ID", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Account not found in this tenant", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn disable_account(
    State(state): State<AppState>,
    Path((tenant_id, account_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let id = match uuid::Uuid::parse_str(&account_id) {
        Ok(i) => i,
        Err(_) => {
            return ApiError::bad_request("invalid_account_id", "invalid account ID format")
                .into_response()
        }
    };
    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal
        .local_accounts()
        .set_status(id, &tenant_id, "disabled")
        .await
    {
        Ok(true) => Json(AccountActionResponse {
            status: "disabled".to_string(),
            id: account_id,
        })
        .into_response(),
        Ok(false) => ApiError::not_found("account_not_found", "account not found in this tenant")
            .into_response(),
        Err(e) => {
            warn!("disable local account failed: {}", e);
            ApiError::internal("failed to disable account").into_response()
        }
    }
}

/// `POST /v1/tenants/{tenant_id}/accounts/{account_id}/password` — admin reset.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/accounts/{account_id}/password",
    tag = "auth",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("account_id" = String, Path, description = "Account UUID"),
    ),
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset", body = AccountActionResponse),
        (status = 400, description = "Invalid account ID", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Account not found in this tenant", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn reset_password(
    State(state): State<AppState>,
    Path((tenant_id, account_id)): Path<(String, String)>,
    Json(body): Json<ResetPasswordRequest>,
) -> impl IntoResponse {
    let id = match uuid::Uuid::parse_str(&account_id) {
        Ok(i) => i,
        Err(_) => {
            return ApiError::bad_request("invalid_account_id", "invalid account ID format")
                .into_response()
        }
    };
    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal
        .local_accounts()
        .set_password(id, &tenant_id, &body.password)
        .await
    {
        Ok(true) => Json(AccountActionResponse {
            status: "password_reset".to_string(),
            id: account_id,
        })
        .into_response(),
        Ok(false) => ApiError::not_found("account_not_found", "account not found in this tenant")
            .into_response(),
        Err(e) => {
            warn!("reset local account password failed: {}", e);
            ApiError::internal("failed to reset password").into_response()
        }
    }
}
