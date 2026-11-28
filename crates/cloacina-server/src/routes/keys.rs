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
    Extension, Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use tracing::{info, warn};

use cloacina_api_types::{
    CreateKeyRequest, KeyCreatedResponse, KeyInfo, KeyRevokedResponse, ListResponse,
    UpdateKeyRequest, WsTicketResponse,
};

use crate::AppState;
use crate::routes::audit::{self as audit_record, AuditEvent};
use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;

/// The body of a key request, or a 400 that says what is wrong with it.
///
/// A request with no `role` is refused (CLOACINA-T-1055): the server does not
/// choose a role for the caller. An unknown role is refused the same way.
fn key_request(
    body: Result<Json<CreateKeyRequest>, JsonRejection>,
) -> Result<CreateKeyRequest, ApiError> {
    match body {
        Ok(Json(body)) => Ok(body),
        Err(rejection) => Err(ApiError::bad_request(
            "invalid_request",
            format!(
                "a key request needs a name and a role (read, write, admin or agent): {}",
                rejection.body_text()
            ),
        )),
    }
}

/// The optional expiry of a key request (CLOACINA-T-1083): an RFC 3339
/// timestamp in the future, or none. Else a 400 that says why.
fn key_expiry(
    raw: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, ApiError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let at = chrono::DateTime::parse_from_rfc3339(raw.trim())
        .map_err(|e| {
            ApiError::bad_request(
                "invalid_expires_at",
                format!("expires_at must be an RFC 3339 timestamp: {e}"),
            )
        })?
        .with_timezone(&chrono::Utc);
    if at <= now {
        return Err(ApiError::bad_request(
            "invalid_expires_at",
            "expires_at must be in the future",
        ));
    }
    Ok(Some(at))
}

/// The audit facts of a created key: name, role, and the expiry and the
/// priority limit when set.
fn key_create_detail(info: &cloacina::dal::unified::api_keys::ApiKeyInfo) -> serde_json::Value {
    let mut detail = serde_json::json!({ "name": info.name, "role": info.permissions });
    if let Some(at) = info.expires_at {
        detail["expires_at"] = serde_json::Value::String(at.to_rfc3339());
    }
    if let Some(limit) = info.max_priority {
        detail["max_priority"] = limit.into();
    }
    detail
}

/// The highest priority a key may be given (CLOACINA-T-1123): 0 to 10, on a
/// role that may set a priority (not `read` or `agent`: 400), and not better
/// than the caller's own limit (403).
pub(crate) fn checked_max_priority(
    asked: Option<u8>,
    role: &str,
    caller: &AuthenticatedKey,
) -> Result<Option<u8>, ApiError> {
    let Some(asked) = asked else {
        return Ok(None);
    };
    if let Err(message) = cloacina_api_types::validate_priority(i64::from(asked)) {
        return Err(ApiError::bad_request("invalid_priority", message));
    }
    let role_may_set = crate::routes::authz::Role::parse(role)
        .and_then(|r| {
            r.default_limits()
                .get(crate::routes::authz::LimitKind::MaxPriority)
        })
        .is_some();
    if !role_may_set {
        return Err(ApiError::bad_request(
            "invalid_request",
            format!("a key of role `{role}` sets no priority, so it cannot have max_priority"),
        ));
    }
    let own = crate::routes::authz::Principal::from_key(caller)
        .limits
        .get(crate::routes::authz::LimitKind::MaxPriority)
        .unwrap_or(cloacina_api_types::LOWEST_PRIORITY);
    if asked < own {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "priority_above_limit",
            format!(
                "max_priority {asked} is above your own limit: the highest priority you may \
                 set is {own}"
            ),
        ));
    }
    Ok(Some(asked))
}

/// POST /auth/keys — create a new API key.
///
/// Platform admin only (the route table). Without `platform_admin` the key
/// belongs to the tenant `public`. With `platform_admin: true` it is a
/// platform admin key: no tenant, role `admin`, the flag (CLOACINA-T-1065).
/// Returns the plaintext key exactly once. It cannot be retrieved again.
#[utoipa::path(
    post,
    path = "/v1/auth/keys",
    tag = "keys",
    request_body = CreateKeyRequest,
    responses(
        (status = 201, description = "Key created — plaintext returned exactly once", body = KeyCreatedResponse),
        (status = 400, description = "No role, an unknown role, `platform_admin` with a role other than admin, an expires_at that is not a future RFC 3339 timestamp, or a body that is not a key request", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Platform admin key required", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn create_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    body: Result<Json<CreateKeyRequest>, JsonRejection>,
) -> impl IntoResponse {
    let body = match key_request(body) {
        Ok(body) => body,
        Err(e) => return e.into_response(),
    };
    let max_priority = match checked_max_priority(body.max_priority, body.role.as_str(), &auth) {
        Ok(limit) => limit,
        Err(e) => return e.into_response(),
    };
    // The route table lets only a platform admin key reach this route, so
    // the flag is granted only by a holder of the flag (CLOACINA-T-1065).
    // A platform admin key has no tenant and role `admin`, like the
    // bootstrap key.
    if body.platform_admin && body.role != cloacina_api_types::KeyRole::Admin {
        return ApiError::bad_request("invalid_request", "a platform admin key has role `admin`")
            .into_response();
    }
    // CLOACI-T-0817: a key with no flag belongs to the "public" tenant (a
    // None-tenant non-admin key would reach nothing). For named tenants use
    // POST /tenants/{tenant_id}/keys.
    let tenant = if body.platform_admin {
        None
    } else {
        Some("public")
    };
    let expires_at = match key_expiry(body.expires_at.as_deref(), chrono::Utc::now()) {
        Ok(at) => at,
        Err(e) => return e.into_response(),
    };
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();

    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal
        .api_keys()
        .create_key_limited(
            &hash,
            &body.name,
            tenant,
            body.platform_admin,
            body.role.as_str(),
            expires_at,
            max_priority,
        )
        .await
    {
        Ok(info) => {
            audit_record::note(
                info.tenant_id.as_deref(),
                AuditEvent::ok(audit_record::actions::KEY_CREATE, info.id.to_string()).detail({
                    let mut detail = key_create_detail(&info);
                    detail["platform_admin"] = serde_json::Value::Bool(info.is_admin);
                    detail
                }),
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
                    expires_at: info.expires_at.map(|t| t.to_rfc3339()),
                    max_priority: info.max_priority,
                }),
            )
                .into_response()
        }
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
    Extension(_auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
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
                    expires_at: k.expires_at.map(|t| t.to_rfc3339()),
                    max_priority: k.max_priority,
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
        (status = 409, description = "The key is the last live platform admin key", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn revoke_key(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(key_id): Path<String>,
) -> impl IntoResponse {
    let id = match uuid::Uuid::parse_str(&key_id) {
        Ok(id) => id,
        Err(_) => {
            return ApiError::bad_request("invalid_key_id", "invalid key ID format")
                .into_response();
        }
    };

    use cloacina::dal::unified::api_keys::RevokeOutcome;
    let dal = cloacina::dal::DAL::new(state.database.clone());
    // The last live platform admin key is kept (CLOACINA-T-1065).
    match dal.api_keys().revoke_key_keeping_platform_admin(id).await {
        Ok(RevokeOutcome::Revoked) => {
            // Every server clears its key cache, so the revoked key is
            // rejected on each replica now (CLOACINA-T-1128).
            crate::routes::auth::announce_key_revocation_or_warn(&state).await;
            audit_record::note(
                None,
                AuditEvent::ok(audit_record::actions::KEY_REVOKE, key_id.as_str()),
            );
            Json(KeyRevokedResponse {
                status: "revoked".to_string(),
                id: key_id,
            })
            .into_response()
        }
        Ok(RevokeOutcome::LastPlatformAdmin) => ApiError::conflict(
            "last_platform_admin_key",
            "this is the last live platform admin key: make another one before you revoke it",
        )
        .into_response(),
        Ok(RevokeOutcome::NotFound) => {
            ApiError::not_found("key_not_found", "key not found or already revoked").into_response()
        }
        Err(e) => {
            warn!("Failed to revoke API key: {}", e);
            ApiError::internal("failed to revoke API key").into_response()
        }
    }
}

/// POST /tenants/:tenant_id/keys — create a key scoped to a tenant.
/// Needs `keys.manage` in the tenant: a tenant admin, or the platform admin.
/// The role is one of `read`, `write`, `admin`, `agent` and must be given.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/keys",
    tag = "keys",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    request_body = CreateKeyRequest,
    responses(
        (status = 201, description = "Tenant-scoped key created — plaintext returned exactly once", body = KeyCreatedResponse),
        (status = 400, description = "No role, an unknown role, `platform_admin: true`, an expires_at that is not a future RFC 3339 timestamp, or a body that is not a key request", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn create_tenant_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
    body: Result<Json<CreateKeyRequest>, JsonRejection>,
) -> impl IntoResponse {
    let body = match key_request(body) {
        Ok(body) => body,
        Err(e) => return e.into_response(),
    };
    let max_priority = match checked_max_priority(body.max_priority, body.role.as_str(), &auth) {
        Ok(limit) => limit,
        Err(e) => return e.into_response(),
    };
    // A tenant key is never a platform admin key (CLOACINA-T-1065).
    if body.platform_admin {
        return ApiError::bad_request(
            "invalid_request",
            "a tenant key cannot be a platform admin key: use POST /v1/auth/keys",
        )
        .into_response();
    }
    let expires_at = match key_expiry(body.expires_at.as_deref(), chrono::Utc::now()) {
        Ok(at) => at,
        Err(e) => return e.into_response(),
    };
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();

    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal
        .api_keys()
        .create_key_limited(
            &hash,
            &body.name,
            Some(&tenant_id),
            false,
            body.role.as_str(),
            expires_at,
            max_priority,
        )
        .await
    {
        Ok(info) => {
            info!(
                "Created tenant-scoped key '{}' for tenant '{}'",
                info.name, tenant_id
            );
            audit_record::note(
                Some(&tenant_id),
                AuditEvent::ok(audit_record::actions::KEY_CREATE, info.id.to_string())
                    .detail(key_create_detail(&info)),
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
                    expires_at: info.expires_at.map(|t| t.to_rfc3339()),
                    max_priority: info.max_priority,
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

/// GET /tenants/:tenant_id/keys — list keys scoped to one tenant (CLOACI-T-0784).
/// Tenant-admin self-service; authZ (`keys.manage` in the tenant of the path) is enforced by the
/// route-table middleware, so the caller is already confined to `tenant_id`.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/keys",
    tag = "keys",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    responses(
        (status = 200, description = "Tenant keys (no hashes or plaintext)", body = ListResponse<KeyInfo>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_tenant_keys(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal.api_keys().list_keys_for_tenant(&tenant_id).await {
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
                    expires_at: k.expires_at.map(|t| t.to_rfc3339()),
                    max_priority: k.max_priority,
                })
                .collect();
            Json(ListResponse::new(items)).into_response()
        }
        Err(e) => {
            warn!("Failed to list keys for tenant '{}': {}", tenant_id, e);
            ApiError::internal("failed to list API keys").into_response()
        }
    }
}

/// DELETE /tenants/:tenant_id/keys/:key_id — revoke a key owned by one tenant
/// (CLOACI-T-0784). The middleware confines the caller to `tenant_id`; this
/// handler additionally verifies the target key belongs to that tenant before
/// revoking. A cross-tenant or unknown id is reported as not-found (never
/// revoked) so a tenant-admin can't probe or touch another tenant's keys.
#[utoipa::path(
    delete,
    path = "/v1/tenants/{tenant_id}/keys/{key_id}",
    tag = "keys",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("key_id" = String, Path, description = "Key UUID"),
    ),
    responses(
        (status = 200, description = "Key revoked", body = KeyRevokedResponse),
        (status = 400, description = "Invalid key ID format", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Key not found in this tenant", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn revoke_tenant_key(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, key_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let id = match uuid::Uuid::parse_str(&key_id) {
        Ok(id) => id,
        Err(_) => {
            return ApiError::bad_request("invalid_key_id", "invalid key ID format")
                .into_response();
        }
    };

    let dal = cloacina::dal::DAL::new(state.database.clone());

    // Ownership gate: the target key must belong to this tenant. A mismatch or
    // missing key reports not-found (never reveal/touch another tenant's keys).
    match dal.api_keys().get_key(id).await {
        Ok(Some(info)) if info.tenant_id.as_deref() == Some(tenant_id.as_str()) => {}
        Ok(_) => {
            return ApiError::not_found("key_not_found", "key not found in this tenant")
                .into_response();
        }
        Err(e) => {
            warn!("Failed to look up key '{}': {}", key_id, e);
            return ApiError::internal("failed to revoke API key").into_response();
        }
    }

    match dal.api_keys().revoke_key(id).await {
        Ok(true) => {
            crate::routes::auth::announce_key_revocation_or_warn(&state).await;
            audit_record::note(
                Some(&tenant_id),
                AuditEvent::ok(audit_record::actions::KEY_REVOKE, key_id.as_str()),
            );
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
            warn!("Failed to revoke API key '{}': {}", key_id, e);
            ApiError::internal("failed to revoke API key").into_response()
        }
    }
}

/// PATCH /tenants/:tenant_id/keys/:key_id — set or clear the highest priority
/// a key of the tenant may set (CLOACINA-T-1123). Needs `keys.manage` in the
/// tenant. The new limit applies to the key's next request, on every replica,
/// and to the next run of each package and schedule the key set a priority
/// for.
#[utoipa::path(
    patch,
    path = "/v1/tenants/{tenant_id}/keys/{key_id}",
    tag = "keys",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("key_id" = String, Path, description = "Key UUID"),
    ),
    request_body = UpdateKeyRequest,
    responses(
        (status = 200, description = "The key, with its new limit", body = KeyInfo),
        (status = 400, description = "Invalid key ID, a priority outside 0 to 10, a `read` or `agent` key, or a body that is not a key update", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied, or a limit above the caller's own", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Key not found in this tenant, or revoked", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn update_tenant_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, key_id)): Path<(String, String)>,
    body: Result<Json<UpdateKeyRequest>, JsonRejection>,
) -> impl IntoResponse {
    let body = match body {
        Ok(Json(body)) => body,
        Err(rejection) => {
            return ApiError::bad_request(
                "invalid_request",
                format!(
                    "a key update needs max_priority (0 to 10, or null): {}",
                    rejection.body_text()
                ),
            )
            .into_response();
        }
    };
    let id = match uuid::Uuid::parse_str(&key_id) {
        Ok(id) => id,
        Err(_) => {
            return ApiError::bad_request("invalid_key_id", "invalid key ID format")
                .into_response();
        }
    };
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let key = match dal.api_keys().get_key(id).await {
        Ok(Some(info))
            if info.tenant_id.as_deref() == Some(tenant_id.as_str()) && !info.revoked =>
        {
            info
        }
        Ok(_) => {
            return ApiError::not_found("key_not_found", "key not found in this tenant")
                .into_response();
        }
        Err(e) => {
            warn!("Failed to look up key '{}': {}", key_id, e);
            return ApiError::internal("failed to update API key").into_response();
        }
    };
    let max_priority = match checked_max_priority(body.max_priority, &key.permissions, &auth) {
        Ok(limit) => limit,
        Err(e) => return e.into_response(),
    };
    match dal.api_keys().set_max_priority(id, max_priority).await {
        Ok(true) => {}
        Ok(false) => {
            return ApiError::not_found("key_not_found", "key not found or already revoked")
                .into_response();
        }
        Err(e) => {
            warn!("Failed to update API key '{}': {}", key_id, e);
            return ApiError::internal("failed to update API key").into_response();
        }
    }
    // Every server clears its key cache, so the next request of the key has
    // its new limit on each replica.
    crate::routes::auth::announce_key_revocation_or_warn(&state).await;
    audit_record::note(
        Some(&tenant_id),
        AuditEvent::ok(audit_record::actions::KEY_UPDATE, key_id.as_str())
            .detail(serde_json::json!({ "max_priority": max_priority })),
    );
    Json(KeyInfo {
        id: key.id.to_string(),
        name: key.name,
        permissions: key.permissions,
        tenant_id: key.tenant_id,
        is_admin: key.is_admin,
        created_at: key.created_at.to_rfc3339(),
        revoked: key.revoked,
        expires_at: key.expires_at.map(|t| t.to_rfc3339()),
        max_priority,
    })
    .into_response()
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
    Extension(_auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    let ticket = state.ws_tickets.issue(_auth).await;
    Json(WsTicketResponse {
        ticket,
        expires_in_seconds: 60,
    })
}
