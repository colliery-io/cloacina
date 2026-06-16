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

//! Execution-agent fleet HTTP endpoints (CLOACI-I-0114 / T-0631).
//!
//! - `POST /v1/agent/register`  — agents register their target triple + capacity.
//! - `POST /v1/agent/heartbeat` — periodic liveness + capacity update.
//! - `POST /v1/agent/result`    — agent reports outcome of a work packet.
//! - `GET  /v1/agent/artifact/{digest}` — content-addressed artifact fetch.
//!
//! The substrate handles server→agent work-packet push (recipient =
//! `agent:<agent_id>`). Result reconciliation through the shared
//! [`cloacina::executor::TaskResultHandler`] (T-0630) lands in T-0633 — for
//! T-0631 the result endpoint accepts + logs.

use std::time::Instant;

use axum::body::Body;
use axum::extract::{Extension, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use cloacina::fleet::{
    host_target_triple, AgentHeartbeatRequest, AgentHeartbeatResponse, AgentRegisterRequest,
    AgentRegisterResponse, AgentResultRequest, AgentResultResponse, AGENT_PROTOCOL_VERSION,
};
use tracing::info;
use uuid::Uuid;

use crate::agent_registry::AgentRecord;
use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::AppState;

// `server_host_target_triple` moved to `cloacina::fleet::host_target_triple`
// in T-0632 so server + agent compute it from the same code (the OQ-6
// fail-closed comparison is exact-string and depends on this).

fn require_protocol_version(version: u32) -> Result<(), ApiError> {
    if version == AGENT_PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "protocol_version_mismatch",
            format!(
                "agent protocol_version {} not supported (server speaks {})",
                version, AGENT_PROTOCOL_VERSION
            ),
        ))
    }
}

/// `POST /v1/agent/register`
pub async fn register_agent(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Json(req): Json<AgentRegisterRequest>,
) -> Result<Json<AgentRegisterResponse>, ApiError> {
    require_protocol_version(req.protocol_version)?;

    let agent_id = req
        .agent_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    state.agent_registry.register(AgentRecord {
        agent_id: agent_id.clone(),
        max_concurrency: req.max_concurrency,
        in_flight: 0,
        available_capacity: req.max_concurrency,
        target_triple: req.target_triple.clone(),
        capabilities: req.capabilities.clone(),
        last_heartbeat: Instant::now(),
        tenant_id: auth.tenant_id.clone(),
    });

    info!(
        agent_id = %agent_id,
        api_key = %auth.name,
        tenant = ?auth.tenant_id,
        target_triple = %req.target_triple,
        max_concurrency = req.max_concurrency,
        "agent registered"
    );

    Ok(Json(AgentRegisterResponse {
        protocol_version: AGENT_PROTOCOL_VERSION,
        agent_id,
        // Operator-configured (CLOACI-T-0639); defaults to
        // DEFAULT_HEARTBEAT_INTERVAL_SECONDS. The agent heartbeats at this rate
        // and the server's liveness sweeper uses the same basis.
        heartbeat_interval_seconds: state.agent_heartbeat_interval_seconds,
    }))
}

/// `POST /v1/agent/heartbeat`
pub async fn heartbeat_agent(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Json(req): Json<AgentHeartbeatRequest>,
) -> Result<Json<AgentHeartbeatResponse>, ApiError> {
    require_protocol_version(req.protocol_version)?;

    if !state
        .agent_registry
        .record_heartbeat(&req.agent_id, req.in_flight, req.available_capacity)
    {
        return Err(ApiError::not_found(
            "agent_not_registered",
            format!("agent not registered: {} (re-register)", req.agent_id),
        ));
    }
    Ok(Json(AgentHeartbeatResponse {
        protocol_version: AGENT_PROTOCOL_VERSION,
    }))
}

/// `POST /v1/agent/result`
///
/// Routes the agent's outcome to the `FleetCoordinator` so the
/// `FleetExecutor::execute` call that's awaiting this `task_execution_id`
/// can wake up and reconcile through `TaskResultHandler::handle_outcome`.
/// If no executor is waiting (orphan — most likely from a server restart
/// between dispatch and report, or a stale duplicate after a retry), the
/// request is accepted + logged; the substrate's at-least-once retry path
/// is what keeps the system convergent.
pub async fn report_result(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Json(req): Json<AgentResultRequest>,
) -> Result<Json<AgentResultResponse>, ApiError> {
    require_protocol_version(req.protocol_version)?;

    info!(
        agent_id = %req.agent_id,
        task_id = %req.task_execution_id,
        attempt = req.attempt,
        duration_ms = req.duration_ms,
        outcome = %serde_json::to_string(&req.outcome).unwrap_or_else(|_| "<unserializable>".into()),
        "agent reported result"
    );

    // The task_execution_id arrives from the wire as a string; the
    // coordinator's key is a `UniversalUuid` to match what the FleetExecutor
    // registered with. Parse defensively.
    let task_id = match uuid::Uuid::parse_str(&req.task_execution_id) {
        Ok(u) => cloacina::database::universal_types::UniversalUuid(u),
        Err(e) => {
            return Err(ApiError::bad_request(
                "invalid_task_execution_id",
                format!(
                    "task_execution_id is not a valid UUID: {} ({})",
                    req.task_execution_id, e
                ),
            ));
        }
    };

    match state.fleet_coordinator.forward(task_id, req) {
        Ok(()) => {}
        Err(orphan) => {
            tracing::debug!(
                task_id = %task_id,
                agent_id = %orphan.agent_id,
                attempt = orphan.attempt,
                "agent result has no pending FleetExecutor (orphan; ack + drop — \
                 substrate at-least-once + sweeper retry keeps the system convergent)"
            );
        }
    }

    Ok(Json(AgentResultResponse {
        protocol_version: AGENT_PROTOCOL_VERSION,
    }))
}

/// `GET /v1/agent/artifact/{digest}` — content-addressed cdylib fetch.
///
/// Returns the raw cdylib bytes for the `workflow_packages` row whose
/// `content_hash == digest` and `build_status == 'success'`. The digest IS
/// the secret in a content-addressed scheme, so blind probing is safe; an
/// authenticated session is still required to keep this endpoint behind
/// the same access fence as the rest of the API.
pub async fn fetch_artifact(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(digest): Path<String>,
) -> Result<Response, ApiError> {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let bytes = dal
        .workflow_packages()
        .get_compiled_data_by_content_hash(&digest)
        .await
        .map_err(|e| ApiError::internal(format!("artifact lookup failed: {}", e)))?;

    let Some(bytes) = bytes else {
        return Err(ApiError::not_found(
            "artifact_not_found",
            format!("no successfully-compiled artifact with digest {}", digest),
        ));
    };

    // Long-cacheable: content-addressed, so the bytes for a given digest
    // never change. Agents typically cache locally by digest as well.
    let resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .header("x-build-target-triple", host_target_triple())
        .body(Body::from(bytes))
        .map_err(|e| ApiError::internal(format!("build artifact response: {}", e)))?;
    Ok(resp)
}

/// `GET /v1/agents` — operator-facing snapshot of the execution-agent fleet
/// roster (admin only). CLOACI-I-0124 / WS-0b. Per-replica: reflects the agents
/// registered against *this* server instance.
#[utoipa::path(
    get,
    path = "/v1/agents",
    tag = "fleet",
    responses(
        (status = 200, description = "Fleet roster", body = cloacina_api_types::common::ListResponse<cloacina_api_types::AgentInfo>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Admin required", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_agents(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    if !auth.is_admin {
        return AuthenticatedKey::admin_required_response().into_response();
    }
    let now = Instant::now();
    let agents: Vec<cloacina_api_types::AgentInfo> = state
        .agent_registry
        .snapshot()
        .into_iter()
        .map(|r| cloacina_api_types::AgentInfo {
            agent_id: r.agent_id,
            target_triple: r.target_triple,
            max_concurrency: r.max_concurrency,
            in_flight: r.in_flight,
            available_capacity: r.available_capacity,
            seconds_since_heartbeat: now.duration_since(r.last_heartbeat).as_secs(),
            capabilities: r.capabilities,
            tenant_id: r.tenant_id,
        })
        .collect();
    Json(cloacina_api_types::common::ListResponse::new(agents)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_target_triple_is_arch_os_format() {
        let t = host_target_triple();
        assert!(t.contains('-'));
        let parts: Vec<&str> = t.split('-').collect();
        assert!(!parts[0].is_empty());
        assert!(!parts[1].is_empty());
    }

    #[test]
    fn require_protocol_version_accepts_current() {
        assert!(require_protocol_version(AGENT_PROTOCOL_VERSION).is_ok());
        assert!(require_protocol_version(AGENT_PROTOCOL_VERSION + 1).is_err());
    }
}
