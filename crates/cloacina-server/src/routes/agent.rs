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
    host_target_triple, AgentHeartbeatRequest, AgentHeartbeatResponse, AgentKeyReplenishRequest,
    AgentKeyReplenishResponse, AgentOutcome, AgentRegisterRequest, AgentRegisterResponse,
    AgentResultRequest, AgentResultResponse, EphemeralKeyEntry, AGENT_PROTOCOL_VERSION,
};
use cloacina::security::ServerKeyPool;
use tracing::{debug, info};
use uuid::Uuid;

use crate::agent_registry::AgentRecord;
use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::AppState;

/// CLOACI-T-0861 / D-5 — target size of an agent's server-side one-time key pool.
/// The heartbeat response asks the agent to top up toward this whenever
/// consumption has drawn the pool below it. Matches the agent's own target.
pub(crate) const AGENT_KEY_POOL_TARGET: usize = 32;

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

/// CLOACI-T-0785: reject an agent-protocol call whose caller key is not in the
/// agent's tenant. God-mode (`is_admin`) bypasses. Unknown agents fall through
/// (the handler's own not-found / orphan handling applies). Dispatch-time
/// isolation already holds (`fleet_executor` only selects same-tenant agents);
/// this closes the inbound `heartbeat`/`result` endpoints, which previously
/// ignored the caller's tenant entirely.
fn reject_cross_tenant_agent(
    state: &AppState,
    auth: &AuthenticatedKey,
    agent_id: &str,
) -> Result<(), ApiError> {
    if auth.is_admin {
        return Ok(());
    }
    if let Some(agent_tenant) = state.agent_registry.agent_tenant(agent_id) {
        if agent_tenant != auth.tenant_id {
            return Err(ApiError::forbidden(
                "tenant_access_denied",
                "agent belongs to a different tenant",
            ));
        }
    }
    Ok(())
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

    // CLOACI-T-0861 / D-5 — seed the server-side one-time key pool from the
    // agent's advertised entries. A pre-pool agent that sent only the single
    // `ephemeral_public_key` is accommodated by synthesizing one pool entry so
    // one secret-bearing dispatch can still be served; a modern agent sends a
    // full pool. Re-registration overwrites the record (and its pool) wholesale.
    let mut pool_entries = req.ephemeral_key_pool.clone();
    if pool_entries.is_empty() {
        if let Some(pk) = req.ephemeral_public_key.clone() {
            pool_entries.push(EphemeralKeyEntry {
                key_id: format!("legacy-{}", agent_id),
                public_key_b64: pk,
            });
        }
    }
    let pool_size = pool_entries.len();

    state.agent_registry.register(AgentRecord {
        agent_id: agent_id.clone(),
        max_concurrency: req.max_concurrency,
        in_flight: 0,
        available_capacity: req.max_concurrency,
        target_triple: req.target_triple.clone(),
        capabilities: req.capabilities.clone(),
        last_heartbeat: Instant::now(),
        tenant_id: auth.tenant_id.clone(),
        key_pool: ServerKeyPool::from_entries(pool_entries),
    });

    info!(
        agent_id = %agent_id,
        api_key = %auth.name,
        tenant = ?auth.tenant_id,
        target_triple = %req.target_triple,
        max_concurrency = req.max_concurrency,
        key_pool = pool_size,
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
    Extension(auth): Extension<AuthenticatedKey>,
    Json(req): Json<AgentHeartbeatRequest>,
) -> Result<Json<AgentHeartbeatResponse>, ApiError> {
    require_protocol_version(req.protocol_version)?;
    reject_cross_tenant_agent(&state, &auth, &req.agent_id)?;

    if !state
        .agent_registry
        .record_heartbeat(&req.agent_id, req.in_flight, req.available_capacity)
    {
        return Err(ApiError::not_found(
            "agent_not_registered",
            format!("agent not registered: {} (re-register)", req.agent_id),
        ));
    }
    // CLOACI-T-0861 / D-5 — tell the agent how many one-time keys to top up if
    // secret-bearing dispatches have drawn the pool below its target.
    let replenish_keys = state
        .agent_registry
        .key_pool_deficit(&req.agent_id, AGENT_KEY_POOL_TARGET) as u32;
    Ok(Json(AgentHeartbeatResponse {
        protocol_version: AGENT_PROTOCOL_VERSION,
        replenish_keys,
    }))
}

/// `POST /v1/agent/keys` — CLOACI-T-0861 / D-5 one-time key-pool top-up.
///
/// The agent appends fresh [`EphemeralKeyEntry`]s to its server-side pool, either
/// in response to the heartbeat replenish signal or proactively. De-duped by
/// `key_id`; an unknown agent is rejected (it should re-register).
pub async fn replenish_keys(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Json(req): Json<AgentKeyReplenishRequest>,
) -> Result<Json<AgentKeyReplenishResponse>, ApiError> {
    require_protocol_version(req.protocol_version)?;
    reject_cross_tenant_agent(&state, &auth, &req.agent_id)?;

    let accepted = state
        .agent_registry
        .replenish_secret_keys(&req.agent_id, req.keys)
        .ok_or_else(|| {
            ApiError::not_found(
                "agent_not_registered",
                format!("agent not registered: {} (re-register)", req.agent_id),
            )
        })?;
    debug!(agent_id = %req.agent_id, accepted, "agent key pool topped up");
    Ok(Json(AgentKeyReplenishResponse {
        protocol_version: AGENT_PROTOCOL_VERSION,
        accepted: accepted as u32,
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
    Extension(auth): Extension<AuthenticatedKey>,
    Json(req): Json<AgentResultRequest>,
) -> Result<Json<AgentResultResponse>, ApiError> {
    require_protocol_version(req.protocol_version)?;
    reject_cross_tenant_agent(&state, &auth, &req.agent_id)?;

    // CLOACI-T-0780: a refusal is an EXPECTED fail-closed outcome — and now rare,
    // since the fleet only dispatches to agents with a runnable arch. Log it at
    // debug so the backstop stays quiet; real results (success/failure) stay at info.
    if matches!(req.outcome, AgentOutcome::Refused { .. }) {
        debug!(
            agent_id = %req.agent_id,
            task_id = %req.task_execution_id,
            attempt = req.attempt,
            outcome = %serde_json::to_string(&req.outcome).unwrap_or_else(|_| "<unserializable>".into()),
            "agent reported result (refused — fail-closed guard)"
        );
    } else {
        info!(
            agent_id = %req.agent_id,
            task_id = %req.task_execution_id,
            attempt = req.attempt,
            duration_ms = req.duration_ms,
            outcome = %serde_json::to_string(&req.outcome).unwrap_or_else(|_| "<unserializable>".into()),
            "agent reported result"
        );
    }

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
    Extension(auth): Extension<AuthenticatedKey>,
    Path(digest): Path<String>,
) -> Result<Response, ApiError> {
    // CLOACI-T-0781: a tenant's compiled cdylibs live in ITS schema, so resolve
    // the requesting agent's tenant database. A tenant-scoped agent (registered
    // with a tenant key) fetches from its own schema; admin/public agents use the
    // admin schema. `resolve("public")` returns the admin db, so any Some(tenant)
    // routes correctly. Falls back to the admin db if resolution fails.
    let db = match &auth.tenant_id {
        Some(tenant) => state
            .tenant_databases
            .resolve(tenant, &state.database)
            .await
            .unwrap_or_else(|_| state.database.clone()),
        None => state.database.clone(),
    };
    let dal = cloacina::dal::DAL::new(db);
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

/// `GET /v1/agent/source/{digest}` — content-addressed package SOURCE fetch
/// (CLOACI-T-0716). Returns the uploaded `.cloacina` archive for the package
/// whose active build has `content_hash == digest`. Used for Python packages:
/// they have no cdylib (`/artifact/{digest}` is empty), so the agent fetches
/// the archive and imports the `workflow/` + `vendor/` tree via PyO3. Same
/// content-addressed access model as `fetch_artifact`.
pub async fn fetch_source(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(digest): Path<String>,
) -> Result<Response, ApiError> {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let bytes = dal
        .workflow_packages()
        .get_package_archive_by_content_hash(&digest)
        .await
        .map_err(|e| ApiError::internal(format!("source lookup failed: {}", e)))?;

    let Some(bytes) = bytes else {
        return Err(ApiError::not_found(
            "source_not_found",
            format!("no package source archive with digest {}", digest),
        ));
    };

    let resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .body(Body::from(bytes))
        .map_err(|e| ApiError::internal(format!("build source response: {}", e)))?;
    Ok(resp)
}

/// One bundled constructor provider in a [`AgentProvidersResponse`]
/// (CLOACI-T-0838). `data` is the packed provider archive, base64-encoded.
#[derive(serde::Serialize)]
pub struct AgentProviderEntry {
    pub name: String,
    pub data: String,
}

/// Response body for `GET /v1/agent/providers/{digest}`.
#[derive(serde::Serialize)]
pub struct AgentProvidersResponse {
    pub providers: Vec<AgentProviderEntry>,
}

/// `GET /v1/agent/providers/{digest}` — the bundled CONSTRUCTOR PROVIDERS for
/// the package whose active build has `content_hash == digest`
/// (CLOACI-T-0838). Agents fetch these alongside the artifact so their load
/// path can resolve `constructor!` nodes exactly like the server's reconciler
/// Step 5b: unpack the bundles, set the provider search path, load each node.
/// Empty list = the package uses no constructors. Same content-addressed
/// access model as `fetch_artifact`.
pub async fn fetch_providers(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(digest): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<AgentProvidersResponse>, ApiError> {
    use base64::Engine as _;

    let dal = cloacina::dal::DAL::new(state.database.clone());
    let Some((package_name, version)) = dal
        .workflow_packages()
        .get_package_name_version_by_content_hash(&digest)
        .await
        .map_err(|e| ApiError::internal(format!("provider lookup failed: {}", e)))?
    else {
        return Err(ApiError::not_found(
            "package_not_found",
            format!("no successfully-built package with digest {}", digest),
        ));
    };

    let rows = dal
        .workflow_packages()
        .get_providers_for_package(&package_name, &version, None)
        .await
        .map_err(|e| ApiError::internal(format!("provider fetch failed: {}", e)))?;

    // CLOACI-T-0908: select rows for the requesting agent's arch. Agents send
    // `?target_triple=<{arch}-{os}>`; an absent param (a pre-T-0908 agent) gets
    // the primary rows — byte-for-byte the pre-triple behavior.
    let selected = match params.get("target_triple") {
        Some(triple) => {
            cloacina::dal::unified::workflow_packages::select_provider_rows_for_target(rows, triple)
        }
        None => rows
            .into_iter()
            .filter(|r| r.target_triple.is_none())
            .collect(),
    };

    Ok(Json(AgentProvidersResponse {
        providers: selected
            .into_iter()
            .map(|r| AgentProviderEntry {
                name: r.provider_name,
                data: base64::engine::general_purpose::STANDARD
                    .encode(r.provider_data.into_inner()),
            })
            .collect(),
    }))
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
    let now = Instant::now();
    let agents: Vec<cloacina_api_types::AgentInfo> = state
        .agent_registry
        .snapshot()
        .into_iter()
        // CLOACI-T-0785: tenant-admins see only their own tenant's agents; god sees all.
        .filter(|r| auth.is_admin || r.tenant_id == auth.tenant_id)
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
