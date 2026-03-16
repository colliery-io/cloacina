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

//! Authentication and authorization middleware for axum.

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use std::sync::Arc;

use super::cache::{AuthCache, CachedKey};
use super::context::AuthContext;

/// Auth middleware state, shared across requests.
#[derive(Clone)]
pub struct AuthState {
    pub cache: AuthCache,
    pub dal: Arc<cloacina::dal::DAL>,
}

/// Middleware function for authentication.
/// Use with `axum::middleware::from_fn_with_state`.
pub async fn auth_middleware(
    axum::extract::State(auth_state): axum::extract::State<AuthState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract Bearer token
    let token = match extract_bearer_token(&request) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "missing or invalid Authorization header"})),
            )
                .into_response();
        }
    };

    // Extract prefix for cache lookup
    let prefix = cloacina::security::api_keys::extract_prefix(&token);

    // Look up in cache (or fall through to DB)
    let cached_keys = match auth_state.cache.lookup(&prefix) {
        Some(keys) => keys,
        None => {
            // Cache miss — query DB
            let dal_ref = &auth_state.dal;
            let api_key_dal = cloacina::dal::unified::ApiKeyDAL::new(dal_ref);
            match api_key_dal.load_by_prefix(&prefix).await {
                Ok(results) => {
                    if results.is_empty() {
                        auth_state.cache.insert_not_found(prefix.clone());
                        return (
                            StatusCode::UNAUTHORIZED,
                            Json(json!({"error": "invalid API key"})),
                        )
                            .into_response();
                    }
                    let cached: Vec<CachedKey> = results
                        .into_iter()
                        .map(|(key_row, patterns)| CachedKey {
                            key_hash: key_row.key_hash,
                            key_id: key_row.id.0,
                            tenant_id: key_row.tenant_id.map(|t| t.0),
                            can_read: key_row.can_read.into(),
                            can_write: key_row.can_write.into(),
                            can_execute: key_row.can_execute.into(),
                            can_admin: key_row.can_admin.into(),
                            expires_at: key_row.expires_at.map(|t| t.into()),
                            revoked_at: key_row.revoked_at.map(|t| t.into()),
                            workflow_patterns: patterns.into_iter().map(|p| p.pattern).collect(),
                        })
                        .collect();
                    auth_state.cache.insert(prefix.clone(), cached.clone());
                    cached
                }
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "authentication service unavailable"})),
                    )
                        .into_response();
                }
            }
        }
    };

    // Find matching key by hash verification
    let matched_key = cached_keys
        .iter()
        .find(|k| cloacina::security::api_keys::verify_key(&token, &k.key_hash));

    let key = match matched_key {
        Some(k) => k,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "invalid API key"})),
            )
                .into_response();
        }
    };

    // Check revocation
    if key.revoked_at.is_some() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "API key has been revoked"})),
        )
            .into_response();
    }

    // Check expiry
    if let Some(expires) = key.expires_at {
        if expires < chrono::Utc::now() {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "API key has expired"})),
            )
                .into_response();
        }
    }

    // Inject AuthContext into request extensions
    let auth_context = AuthContext {
        key_id: key.key_id,
        tenant_id: key.tenant_id,
        can_read: key.can_read,
        can_write: key.can_write,
        can_execute: key.can_execute,
        can_admin: key.can_admin,
        workflow_patterns: key.workflow_patterns.clone(),
    };

    request.extensions_mut().insert(auth_context);
    next.run(request).await
}

fn extract_bearer_token(request: &Request) -> Option<String> {
    let header = request.headers().get("authorization")?.to_str().ok()?;
    if header.starts_with("Bearer ") {
        Some(header[7..].to_string())
    } else {
        None
    }
}

// ============================================================================
// Permission Guard (T-0190)
// ============================================================================

/// Permission types for route-level authorization.
#[derive(Debug, Clone, Copy)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Admin,
}

/// Middleware function that checks the Read permission.
pub async fn require_read(request: Request, next: Next) -> Response {
    check_permission(request, next, Permission::Read).await
}

/// Middleware function that checks the Write permission.
pub async fn require_write(request: Request, next: Next) -> Response {
    check_permission(request, next, Permission::Write).await
}

/// Middleware function that checks the Execute permission.
pub async fn require_execute(request: Request, next: Next) -> Response {
    check_permission(request, next, Permission::Execute).await
}

/// Middleware function that checks the Admin permission.
pub async fn require_admin(request: Request, next: Next) -> Response {
    check_permission(request, next, Permission::Admin).await
}

async fn check_permission(request: Request, next: Next, required: Permission) -> Response {
    let auth = match request.extensions().get::<AuthContext>() {
        Some(ctx) => ctx.clone(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "not authenticated"})),
            )
                .into_response();
        }
    };

    let has_permission = match required {
        Permission::Read => auth.can_read,
        Permission::Write => auth.can_write,
        Permission::Execute => auth.can_execute,
        Permission::Admin => auth.can_admin,
    };

    if !has_permission {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": format!("insufficient permissions: {:?} required", required)})),
        )
            .into_response();
    }

    next.run(request).await
}
