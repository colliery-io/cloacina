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
use serde::Deserialize;
use tracing::{info, warn};

use crate::commands::serve::AppState;
use crate::server::auth::AuthenticatedKey;

/// Request body for creating a new API key.
#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
    /// Role for the key: "admin", "write", or "read". Defaults to "admin".
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "admin".to_string()
}

/// POST /auth/keys — create a new API key.
///
/// Returns the plaintext key exactly once. It cannot be retrieved again.
pub async fn create_key(
    State(state): State<AppState>,
    Json(body): Json<CreateKeyRequest>,
) -> impl IntoResponse {
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();

    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal
        .api_keys()
        .create_key(&hash, &body.name, None, false, &body.role)
        .await
    {
        Ok(info) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "id": info.id.to_string(),
                "name": info.name,
                "key": plaintext,
                "permissions": info.permissions,
                "tenant_id": info.tenant_id,
                "is_admin": info.is_admin,
                "created_at": info.created_at.to_rfc3339(),
            })),
        )
            .into_response(),
        Err(e) => {
            warn!("Failed to create API key: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "failed to create API key"})),
            )
                .into_response()
        }
    }
}

/// GET /auth/keys — list all API keys (no hashes or plaintext).
pub async fn list_keys(State(state): State<AppState>) -> impl IntoResponse {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal.api_keys().list_keys().await {
        Ok(keys) => {
            let items: Vec<_> = keys
                .into_iter()
                .map(|k| {
                    serde_json::json!({
                        "id": k.id.to_string(),
                        "name": k.name,
                        "permissions": k.permissions,
                        "tenant_id": k.tenant_id,
                        "is_admin": k.is_admin,
                        "created_at": k.created_at.to_rfc3339(),
                        "revoked": k.revoked,
                    })
                })
                .collect();
            Json(serde_json::json!({"keys": items})).into_response()
        }
        Err(e) => {
            warn!("Failed to list API keys: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "failed to list API keys"})),
            )
                .into_response()
        }
    }
}

/// DELETE /auth/keys/:key_id — revoke an API key.
/// Requires admin role within tenant (or god mode).
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
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid key ID format"})),
            )
                .into_response()
        }
    };

    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal.api_keys().revoke_key(id).await {
        Ok(true) => {
            // Clear cache so revoked key is rejected immediately
            state.key_cache.clear().await;
            Json(serde_json::json!({"status": "revoked", "id": key_id})).into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "key not found or already revoked"})),
        )
            .into_response(),
        Err(e) => {
            warn!("Failed to revoke API key: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "failed to revoke API key"})),
            )
                .into_response()
        }
    }
}

/// POST /tenants/:tenant_id/keys — create a key scoped to a tenant.
/// Admin-only: only is_admin keys can create tenant-scoped keys.
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
        .create_key(&hash, &body.name, Some(&tenant_id), false, &body.role)
        .await
    {
        Ok(info) => {
            info!(
                "Created tenant-scoped key '{}' for tenant '{}'",
                info.name, tenant_id
            );
            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "id": info.id.to_string(),
                    "name": info.name,
                    "key": plaintext,
                    "permissions": info.permissions,
                    "tenant_id": info.tenant_id,
                    "is_admin": info.is_admin,
                    "created_at": info.created_at.to_rfc3339(),
                })),
            )
                .into_response()
        }
        Err(e) => {
            warn!("Failed to create tenant key: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "failed to create API key"})),
            )
                .into_response()
        }
    }
}
