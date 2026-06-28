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

//! Agent capacity limits — admin default + per-tenant exceptions (CLOACI-T-0808,
//! CLOACI-I-0127). The global default is server config (`CLOACINA_DEFAULT_MAX_AGENTS`,
//! on `AppState`); this surface manages the per-tenant *exceptions* an admin grants
//! (e.g. default 4, acme 6) and reports the *effective* limit (override if set, else
//! the default) — the hard ceiling the provision API (T-0809) and the autoscaler
//! (T-0811) clamp to.
//!
//! AuthZ (NFR-004, enforced by the authz table — no client-only trust):
//! - `POST` / `DELETE` set/clear an exception → **god-only** (`platform + admin`).
//! - `GET` reads the effective limit → **tenant-scoped read**; a caller may read
//!   only its own tenant's limit (cross-tenant is denied server-side).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::warn;
use utoipa::ToSchema;

use crate::routes::error::ApiError;
use crate::AppState;

/// Set (or replace) a tenant's agent-capacity exception. God-only.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SetAgentLimitRequest {
    pub max_agents: u32,
}

/// A tenant's effective agent-capacity limit and how it was derived.
#[derive(Debug, Serialize, ToSchema)]
pub struct TenantAgentLimitInfo {
    pub tenant_id: String,
    /// Platform-wide default (`CLOACINA_DEFAULT_MAX_AGENTS`).
    pub default_max_agents: u32,
    /// Per-tenant exception, if one is set (else the default applies).
    pub tenant_override: Option<u32>,
    /// The limit actually enforced: the override if set, else the default.
    pub effective_limit: u32,
}

/// Resolve the full limit view for a tenant (default + override + effective).
async fn limit_info(state: &AppState, tenant_id: &str) -> Result<TenantAgentLimitInfo, ApiError> {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let tenant_override = dal
        .agent_limits()
        .get_tenant_limit(tenant_id)
        .await
        .map_err(|e| {
            warn!("agent limit lookup failed: {}", e);
            ApiError::internal("failed to read agent limit")
        })?;
    let default = state.default_max_agents;
    Ok(TenantAgentLimitInfo {
        tenant_id: tenant_id.to_string(),
        default_max_agents: default,
        tenant_override,
        effective_limit: tenant_override.unwrap_or(default),
    })
}

/// `POST /v1/tenants/{tenant_id}/limits` — set a tenant's agent-capacity
/// exception. God-only (the authz table requires `platform + admin`).
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/limits",
    tag = "fleet",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    request_body = SetAgentLimitRequest,
    responses(
        (status = 200, description = "Exception set; returns the effective limit", body = TenantAgentLimitInfo),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Platform-admin required", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn set_tenant_limit(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Json(body): Json<SetAgentLimitRequest>,
) -> impl IntoResponse {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    if let Err(e) = dal
        .agent_limits()
        .set_tenant_limit(&tenant_id, body.max_agents)
        .await
    {
        warn!("set agent limit failed: {}", e);
        return ApiError::internal("failed to set agent limit").into_response();
    }
    match limit_info(&state, &tenant_id).await {
        Ok(info) => (StatusCode::OK, Json(info)).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `GET /v1/tenants/{tenant_id}/limits` — the tenant's effective agent-capacity
/// limit. Tenant-scoped: a caller may read only its own tenant's limit.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/limits",
    tag = "fleet",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    responses(
        (status = 200, description = "The tenant's effective agent-capacity limit", body = TenantAgentLimitInfo),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_tenant_limit(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    match limit_info(&state, &tenant_id).await {
        Ok(info) => Json(info).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `DELETE /v1/tenants/{tenant_id}/limits` — remove a tenant's exception (revert
/// to the default). God-only.
#[utoipa::path(
    delete,
    path = "/v1/tenants/{tenant_id}/limits",
    tag = "fleet",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    responses(
        (status = 200, description = "Exception cleared; returns the effective (default) limit", body = TenantAgentLimitInfo),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Platform-admin required", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn clear_tenant_limit(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    if let Err(e) = dal.agent_limits().clear_tenant_limit(&tenant_id).await {
        warn!("clear agent limit failed: {}", e);
        return ApiError::internal("failed to clear agent limit").into_response();
    }
    match limit_info(&state, &tenant_id).await {
        Ok(info) => (StatusCode::OK, Json(info)).into_response(),
        Err(e) => e.into_response(),
    }
}
