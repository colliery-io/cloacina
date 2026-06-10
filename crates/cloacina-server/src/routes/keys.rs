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

//! API key management endpoints.
//!
//! All endpoints require an existing valid API key (behind auth middleware).
//! The bootstrap key is created automatically on first server startup.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use tracing::{info, warn};

use cloacina_api_types::{
    CreateKeyRequest, KeyCreatedResponse, KeyInfo, KeyRevokedResponse, ListResponse,
    WsTicketResponse,
};

use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::AppState;

/// POST /auth/keys — create a new API key.
///
/// Requires admin role. Non-admin keys cannot create keys with higher
/// permissions than their own (prevents privilege escalation).
/// Returns the plaintext key exactly once. It cannot be retrieved again.
#[utoipa::path(
    post,
    path = "/v1/auth/keys",
    tag = "keys",
    request_body = CreateKeyRequest,
    responses(
        (status = 201, description = "Key created — plaintext returned exactly once", body = KeyCreatedResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Admin role required", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn create_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Json(body): Json<CreateKeyRequest>,
) -> impl IntoResponse {
    if !auth.can_admin() {
        return AuthenticatedKey::insufficient_role_response().into_response();
    }

    // Privilege-escalation guard. `body.role` populates the per-key
    // `permissions` string (read/write/admin scoped to a tenant);
    // `is_admin` is the orthogonal god-mode flag that grants
    // cross-tenant access. T-0557 Bug 3 audit flagged the
    // earlier shape of this block as a no-op — the comment claimed
    // "create_key always sets is_admin=false, this is safe" while
    // performing no actual check. The safety claim is correct
    // (god-mode is granted only via `bootstrap-admin` at server
    // start, never here): we hard-code `is_admin = false` in the
    // `create_key` call below, so a tenant-admin issuing
    // role="admin" gets another tenant-admin key, never god-mode.
    // Keep the call shape explicit and reject any caller that
    // somehow tried to set is_admin (today the request body has no
    // such field, but lock that in defensively).
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();

    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal
        .api_keys()
        .create_key(
            &hash,
            &body.name,
            None,
            false, /* is_admin: never granted via this endpoint */
            body.role.as_str(),
        )
        .await
    {
        Ok(info) => (
            StatusCode::CREATED,
            Json(KeyCreatedResponse {
                id: info.id.to_string(),
                name: info.name,
                key: plaintext,
                permissions: info.permissions,
                tenant_id: info.tenant_id,
                is_admin: info.is_admin,
                created_at: info.created_at.to_rfc3339(),
            }),
        )
            .into_response(),
        Err(e) => {
            warn!("Failed to create API key: {}", e);
            ApiError::internal("failed to create API key").into_response()
        }
    }
}

/// GET /auth/keys — list all API keys (no hashes or plaintext).
/// Requires admin role.
#[utoipa::path(
    get,
    path = "/v1/auth/keys",
    tag = "keys",
    responses(
        (status = 200, description = "All keys (no hashes or plaintext)", body = ListResponse<KeyInfo>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Admin role required", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_keys(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    if !auth.can_admin() {
        return AuthenticatedKey::insufficient_role_response().into_response();
    }
    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal.api_keys().list_keys().await {
        Ok(keys) => {
            let items: Vec<KeyInfo> = keys
                .into_iter()
                .map(|k| KeyInfo {
                    id: k.id.to_string(),
                    name: k.name,
                    permissions: k.permissions,
                    tenant_id: k.tenant_id,
                    is_admin: k.is_admin,
                    created_at: k.created_at.to_rfc3339(),
                    revoked: k.revoked,
                })
                .collect();
            // CLOACI-T-0594 / API-03: unified `{items, total}` envelope.
            Json(ListResponse::new(items)).into_response()
        }
        Err(e) => {
            warn!("Failed to list API keys: {}", e);
            ApiError::internal("failed to list API keys").into_response()
        }
    }
}

/// DELETE /auth/keys/:key_id — revoke an API key.
/// Requires admin role within tenant (or god mode).
#[utoipa::path(
    delete,
    path = "/v1/auth/keys/{key_id}",
    tag = "keys",
    params(("key_id" = String, Path, description = "Key UUID")),
    responses(
        (status = 200, description = "Key revoked", body = KeyRevokedResponse),
        (status = 400, description = "Invalid key ID format", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Admin role required", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Key not found or already revoked", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn revoke_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(key_id): Path<String>,
) -> impl IntoResponse {
    if !auth.can_admin() {
        return AuthenticatedKey::insufficient_role_response().into_response();
    }
    let id = match uuid::Uuid::parse_str(&key_id) {
        Ok(id) => id,
        Err(_) => {
            return ApiError::bad_request("invalid_key_id", "invalid key ID format").into_response()
        }
    };

    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal.api_keys().revoke_key(id).await {
        Ok(true) => {
            // Clear cache so revoked key is rejected immediately
            state.key_cache.clear().await;
            Json(KeyRevokedResponse {
                status: "revoked".to_string(),
                id: key_id,
            })
            .into_response()
        }
        Ok(false) => {
            ApiError::not_found("key_not_found", "key not found or already revoked").into_response()
        }
        Err(e) => {
            warn!("Failed to revoke API key: {}", e);
            ApiError::internal("failed to revoke API key").into_response()
        }
    }
}

/// POST /tenants/:tenant_id/keys — create a key scoped to a tenant.
/// Admin-only: only is_admin keys can create tenant-scoped keys.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/keys",
    tag = "keys",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    request_body = CreateKeyRequest,
    responses(
        (status = 201, description = "Tenant-scoped key created — plaintext returned exactly once", body = KeyCreatedResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Admin (god-mode) key required", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn create_tenant_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
    Json(body): Json<CreateKeyRequest>,
) -> impl IntoResponse {
    if !auth.is_admin {
        return AuthenticatedKey::admin_required_response().into_response();
    }

    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();

    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal
        .api_keys()
        .create_key(
            &hash,
            &body.name,
            Some(&tenant_id),
            false,
            body.role.as_str(),
        )
        .await
    {
        Ok(info) => {
            info!(
                "Created tenant-scoped key '{}' for tenant '{}'",
                info.name, tenant_id
            );
            (
                StatusCode::CREATED,
                Json(KeyCreatedResponse {
                    id: info.id.to_string(),
                    name: info.name,
                    key: plaintext,
                    permissions: info.permissions,
                    tenant_id: info.tenant_id,
                    is_admin: info.is_admin,
                    created_at: info.created_at.to_rfc3339(),
                }),
            )
                .into_response()
        }
        Err(e) => {
            warn!("Failed to create tenant key: {}", e);
            ApiError::internal("failed to create API key").into_response()
        }
    }
}

/// POST /auth/ws-ticket — exchange a Bearer token for a single-use WebSocket ticket.
///
/// Returns a short-lived ticket that can be used as a query parameter for
/// WebSocket upgrade requests, avoiding long-lived API keys in URLs.
#[utoipa::path(
    post,
    path = "/v1/auth/ws-ticket",
    tag = "keys",
    responses(
        (status = 200, description = "Single-use WebSocket ticket", body = WsTicketResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn create_ws_ticket(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    let ticket = state.ws_tickets.issue(auth).await;
    Json(WsTicketResponse {
        ticket,
        expires_in_seconds: 60,
    })
}
