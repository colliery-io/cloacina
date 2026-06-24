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

//! OIDC browser login — `/auth/oidc/login` + `/auth/callback` (CLOACI-T-0790).
//!
//! **Public** (the user has no bearer key yet). `login` redirects to the IdP
//! with PKCE + state + nonce; `callback` exchanges the code, validates the ID
//! token (signature via JWKS, `iss`/`aud`/`exp`, nonce — all in
//! `OidcProvider::complete_login`), maps the identity to a principal via the
//! god-owned allowlist, and mints a short-TTL key. Both 501 when OIDC is not
//! configured.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::{info, warn};

use crate::identity::{mint_for_principal, DEFAULT_MINTED_KEY_TTL};
use crate::routes::error::ApiError;
use crate::routes::local_auth::LocalLoginResponse;
use crate::AppState;

fn oidc_disabled() -> axum::response::Response {
    ApiError::new(
        StatusCode::NOT_IMPLEMENTED,
        "oidc_disabled",
        "OIDC login is not configured on this server",
    )
    .into_response()
}

/// `GET /v1/auth/oidc/login` — start the authorization-code + PKCE flow.
pub async fn oidc_login(State(state): State<AppState>) -> impl IntoResponse {
    let Some(provider) = state.oidc.as_ref() else {
        return oidc_disabled();
    };
    let start = match provider.begin_login() {
        Ok(s) => s,
        Err(e) => {
            warn!("oidc begin_login failed: {e}");
            return ApiError::internal("oidc login failed").into_response();
        }
    };
    state
        .oidc_login
        .put(start.state.clone(), start.nonce, start.pkce_verifier)
        .await;
    axum::response::Redirect::to(&start.auth_url).into_response()
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

/// `GET /v1/auth/callback` — the IdP redirects here with `code` + `state`.
pub async fn oidc_callback(
    State(state): State<AppState>,
    Query(q): Query<CallbackQuery>,
) -> impl IntoResponse {
    let Some(provider) = state.oidc.as_ref() else {
        return oidc_disabled();
    };
    if let Some(err) = q.error {
        warn!("oidc callback error from IdP: {err}");
        return ApiError::unauthorized("identity provider returned an error").into_response();
    }
    let (Some(code), Some(flow_state)) = (q.code, q.state) else {
        return ApiError::bad_request("invalid_callback", "missing code or state").into_response();
    };

    // Single-use lookup of the in-flight login (state -> nonce + PKCE verifier).
    // A missing/expired/replayed state fails closed (CSRF / replay defense).
    let Some((nonce, pkce_verifier)) = state.oidc_login.take(&flow_state).await else {
        return ApiError::unauthorized("invalid or expired login state").into_response();
    };

    let claims = match provider.complete_login(code, nonce, pkce_verifier).await {
        Ok(c) => c,
        Err(e) => {
            warn!("oidc complete_login failed: {e}");
            return ApiError::unauthorized("OIDC login failed").into_response();
        }
    };

    let principal = match state.oidc_policy.resolve(&claims, &provider.config.issuer_url) {
        Some(p) => p,
        None => {
            return ApiError::forbidden(
                "identity_not_mapped",
                "your identity is not mapped to any tenant",
            )
            .into_response()
        }
    };

    match mint_for_principal(&state, &principal, DEFAULT_MINTED_KEY_TTL).await {
        Ok((plaintext, key_info)) => {
            info!(subject = %claims.subject, "OIDC login succeeded — minted key");
            Json(LocalLoginResponse {
                key: plaintext,
                tenant_id: key_info.tenant_id,
                role: key_info.permissions,
                expires_at: key_info.expires_at.map(|t| t.to_rfc3339()),
            })
            .into_response()
        }
        Err(e) => e.into_response(),
    }
}
