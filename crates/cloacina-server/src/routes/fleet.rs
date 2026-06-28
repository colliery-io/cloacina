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

//! Per-tenant fleet scaling — desired-count provisioning (CLOACI-T-0809,
//! CLOACI-I-0127). Tenant self-service provisioning, bounded by the god-set
//! `effective_limit` from T-0808 (`agent_capacity_limits`):
//!
//! - `desired_count` is the tenant's requested agent count (the operational
//!   target the actuator (T-0810) and autoscaler (T-0811) reconcile/clamp to).
//! - `provision` is `+1` while under the effective limit (else 409 "at
//!   capacity"); `deprovision` is `−1` with a floor of 0.
//! - `actual_count` is the number of agents currently registered for the tenant
//!   in *this* server replica's roster (the registry is per-replica, T-0631).
//!
//! AuthZ (NFR-004, enforced by the authz table — no client-only trust):
//! - `POST .../fleet/provision` / `.../fleet/deprovision` → **tenant-admin**
//!   (a tenant self-services its OWN fleet; god bypasses; cross-tenant denied).
//! - `GET .../fleet` → **tenant-scoped read** (own tenant only).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use tracing::warn;
use utoipa::ToSchema;

use crate::routes::error::ApiError;
use crate::AppState;

/// A tenant's fleet-scaling view: requested count, live count, and the ceiling.
#[derive(Debug, Serialize, ToSchema)]
pub struct FleetScaleInfo {
    pub tenant_id: String,
    /// The tenant's requested agent count (self-service provisioning target).
    pub desired_count: u32,
    /// Agents currently registered for this tenant in this replica's roster
    /// (per-replica local view — the registry is not shared across replicas).
    pub actual_count: u32,
    /// The hard ceiling provisioning clamps to: the per-tenant override if set,
    /// else the platform default (T-0808 `effective_limit`).
    pub effective_limit: u32,
    /// Platform-wide default (`CLOACINA_DEFAULT_MAX_AGENTS`).
    pub default_max_agents: u32,
}

/// Count the agents registered for `tenant_id` in this replica's roster.
///
/// The `AgentRegistry` (T-0631) is an in-memory, per-replica roster; this is the
/// local-replica view of how many agents are live for the tenant.
fn actual_count_for(state: &AppState, tenant_id: &str) -> u32 {
    state
        .agent_registry
        .snapshot()
        .iter()
        .filter(|r| r.tenant_id.as_deref() == Some(tenant_id))
        .count() as u32
}

/// Resolve the full fleet-scaling view for a tenant (desired + actual +
/// effective limit + default).
async fn fleet_info(state: &AppState, tenant_id: &str) -> Result<FleetScaleInfo, ApiError> {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let default = state.default_max_agents;
    let desired_count = dal
        .agent_desired()
        .get_desired(tenant_id)
        .await
        .map_err(|e| {
            warn!("desired count lookup failed: {}", e);
            ApiError::internal("failed to read desired count")
        })?;
    let effective_limit = dal
        .agent_limits()
        .effective_limit(tenant_id, default)
        .await
        .map_err(|e| {
            warn!("effective limit lookup failed: {}", e);
            ApiError::internal("failed to read effective limit")
        })?;
    Ok(FleetScaleInfo {
        tenant_id: tenant_id.to_string(),
        desired_count,
        actual_count: actual_count_for(state, tenant_id),
        effective_limit,
        default_max_agents: default,
    })
}

/// `GET /v1/tenants/{tenant_id}/fleet` — the tenant's fleet-scaling view.
/// Tenant-scoped: a caller may read only its own tenant's fleet.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/fleet",
    tag = "fleet",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    responses(
        (status = 200, description = "The tenant's fleet-scaling view", body = FleetScaleInfo),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_fleet(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    match fleet_info(&state, &tenant_id).await {
        Ok(info) => Json(info).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `POST /v1/tenants/{tenant_id}/fleet/provision` — request one more agent.
/// Tenant self-service (tenant-admin); increments `desired_count` by 1 while
/// under the effective limit, else 409 "at capacity".
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/fleet/provision",
    tag = "fleet",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    responses(
        (status = 200, description = "Provisioned; returns the updated fleet view", body = FleetScaleInfo),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 409, description = "At capacity (desired count is at the effective limit)", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn provision(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    let info = match fleet_info(&state, &tenant_id).await {
        Ok(info) => info,
        Err(e) => return e.into_response(),
    };
    if info.desired_count >= info.effective_limit {
        return ApiError::new(
            StatusCode::CONFLICT,
            "at_capacity",
            "desired agent count is at the effective limit",
        )
        .into_response();
    }
    let dal = cloacina::dal::DAL::new(state.database.clone());
    if let Err(e) = dal
        .agent_desired()
        .set_desired(&tenant_id, info.desired_count + 1)
        .await
    {
        warn!("provision (set desired) failed: {}", e);
        return ApiError::internal("failed to provision agent").into_response();
    }
    match fleet_info(&state, &tenant_id).await {
        Ok(info) => (StatusCode::OK, Json(info)).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `POST /v1/tenants/{tenant_id}/fleet/deprovision` — release one agent.
/// Tenant self-service (tenant-admin); decrements `desired_count` by 1 with a
/// floor of 0.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/fleet/deprovision",
    tag = "fleet",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    responses(
        (status = 200, description = "Deprovisioned; returns the updated fleet view", body = FleetScaleInfo),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn deprovision(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    let info = match fleet_info(&state, &tenant_id).await {
        Ok(info) => info,
        Err(e) => return e.into_response(),
    };
    let dal = cloacina::dal::DAL::new(state.database.clone());
    if let Err(e) = dal
        .agent_desired()
        .set_desired(&tenant_id, info.desired_count.saturating_sub(1))
        .await
    {
        warn!("deprovision (set desired) failed: {}", e);
        return ApiError::internal("failed to deprovision agent").into_response();
    }
    match fleet_info(&state, &tenant_id).await {
        Ok(info) => (StatusCode::OK, Json(info)).into_response(),
        Err(e) => e.into_response(),
    }
}
