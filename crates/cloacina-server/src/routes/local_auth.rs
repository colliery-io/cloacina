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

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use utoipa::ToSchema;

use cloacina::dal::unified::LoginOutcome;

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
