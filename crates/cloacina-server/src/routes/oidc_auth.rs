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

use base64::Engine as _;

use crate::identity::{mint_for_principal, DEFAULT_MINTED_KEY_TTL};
use crate::routes::error::ApiError;
use crate::AppState;

/// One minted tenant membership handed back to the SPA after an OIDC login.
#[derive(serde::Serialize)]
struct Membership {
    key: String,
    tenant: String,
    role: String,
}

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

    // A single sign-in may grant several tenant memberships (one IdP identity →
    // {tenant, role} set). Mint a scoped key for each; the SPA lets the user
    // pick which to enter. Unmapped → denied.
    let principals = state
        .oidc_policy
        .resolve_all(&claims, &provider.config.issuer_url);
    if principals.is_empty() {
        return ApiError::forbidden(
            "identity_not_mapped",
            "your identity is not mapped to any tenant",
        )
        .into_response();
    }

    let mut memberships = Vec::with_capacity(principals.len());
    for p in &principals {
        match mint_for_principal(&state, p, DEFAULT_MINTED_KEY_TTL).await {
            Ok((plaintext, info)) => memberships.push(Membership {
                key: plaintext,
                tenant: info.tenant_id.unwrap_or_default(),
                role: info.permissions,
            }),
            Err(e) => return e.into_response(),
        }
    }
    info!(subject = %claims.subject, memberships = memberships.len(), "OIDC login succeeded — minted key(s)");

    // When a UI success-redirect is configured (the browser flow), hand the
    // membership set to the SPA via the URL fragment (base64url of the JSON — a
    // fragment is never sent to a server or logged). Otherwise return JSON.
    let payload = serde_json::to_string(&memberships).unwrap_or_else(|_| "[]".to_string());
    match std::env::var("CLOACINA_OIDC_SUCCESS_REDIRECT") {
        Ok(redirect) if !redirect.is_empty() => {
            let frag = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.as_bytes());
            axum::response::Redirect::to(&format!("{redirect}#memberships={frag}")).into_response()
        }
        _ => Json(memberships).into_response(),
    }
}
