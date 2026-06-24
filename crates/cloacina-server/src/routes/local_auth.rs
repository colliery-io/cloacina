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

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use utoipa::ToSchema;

use cloacina::dal::unified::{LocalAccount, LoginOutcome};
use cloacina_api_types::common::ListResponse;

use crate::identity::{mint_for_principal, ResolvedPrincipal, DEFAULT_MINTED_KEY_TTL};
use crate::routes::error::ApiError;
use crate::AppState;

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
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    )
)]
pub async fn local_login(
    State(state): State<AppState>,
    Json(body): Json<LocalLoginRequest>,
) -> impl IntoResponse {
    let dal = cloacina::dal::DAL::new(state.database.clone());

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
        // enumeration (OQ-13 throttling is a follow-up).
        LoginOutcome::Denied => {
            return ApiError::unauthorized("invalid username or password").into_response()
        }
    };

    let principal = ResolvedPrincipal {
        tenant: account.tenant_id.clone(),
        role: account.role.clone(),
        provenance: format!("local:{}", account.id),
    };

    match mint_for_principal(&state, &principal, DEFAULT_MINTED_KEY_TTL).await {
        Ok((plaintext, info)) => {
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
