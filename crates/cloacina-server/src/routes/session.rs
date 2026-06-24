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
//! - `oidc:<issuer>:<sub>` — refresh = exchange the stored IdP refresh token;
//!   lands with the OIDC phase (T-0790). Returns 501 until then.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde::Serialize;
use tracing::{info, warn};
use utoipa::ToSchema;

use crate::identity::{mint_for_principal, ResolvedPrincipal, DEFAULT_MINTED_KEY_TTL};
use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::routes::local_auth::LocalLoginResponse;
use crate::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct LogoutResponse {
    pub status: String,
}

/// `POST /v1/auth/refresh` — silently re-mint the caller's short-TTL key.
#[utoipa::path(
    post,
    path = "/v1/auth/refresh",
    tag = "auth",
    responses(
        (status = 200, description = "Re-minted key returned once", body = LocalLoginResponse),
        (status = 400, description = "Key is not a refreshable login key", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Invalid key, or the login is no longer valid", body = cloacina_api_types::ErrorBody),
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

    // ---- other providers (oidc:…): the IdP refresh exchange lands in T-0790 ----
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
