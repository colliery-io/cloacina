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

use axum::Json;
use axum::body::Body;
use axum::extract::{Extension, Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use cloacina::fleet::{
    AGENT_PROTOCOL_VERSION, AgentHeartbeatRequest, AgentHeartbeatResponse,
    AgentKeyReplenishRequest, AgentKeyReplenishResponse, AgentOutcome, AgentRegisterRequest,
    AgentRegisterResponse, AgentResultRequest, AgentResultResponse, AgentUndecodableRequest,
    AgentUndecodableResponse, EphemeralKeyEntry, host_target_triple,
};
use cloacina::security::ServerKeyPool;
use cloacina::security::ownership::{AgentOwner, Principal, Refusal};
use tracing::{debug, info, warn};
use uuid::Uuid;

use cloacina::dal::unified::FleetAgentRegistration;

use crate::AppState;
use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;

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

/// The caller key as the ownership rules see it
/// ([`cloacina::security::ownership`]).
pub(crate) fn principal_of(auth: &AuthenticatedKey) -> Principal {
    Principal {
        key_id: auth.key_id,
        tenant_id: auth.tenant_id.clone(),
        is_admin: auth.is_admin,
    }
}

/// Refuse an agent-protocol call unless the caller key holds the agent
/// (CLOACINA-T-1050, CLOACINA-T-1051; the rule is
/// [`Principal::may_act_as_agent`]). Returns the roster row's owner.
///
/// - No row has the id: 404 `agent_not_registered`. The agent registers
///   again.
/// - The row is in another tenant, or another key holds it: 403
///   `agent_access_denied`, one answer for both.
///
/// A platform admin key gets no bypass. The row is read from the DB-backed
/// roster (CLOACI-T-0916), so the answer is the same on every replica.
async fn require_agent_owner(
    state: &AppState,
    auth: &AuthenticatedKey,
    agent_id: &str,
) -> Result<AgentOwner, ApiError> {
    let row = state
        .agent_registry
        .agent_owner(agent_id)
        .await
        .map_err(|e| ApiError::internal_logged("agent roster lookup failed", &e))?;
    match (principal_of(auth).may_act_as_agent(row.as_ref()), row) {
        (Ok(()), Some(row)) => Ok(row),
        (Err(Refusal::UnknownAgent), _) | (Ok(()), None) => Err(ApiError::not_found(
            "agent_not_registered",
            format!("agent not registered: {} (re-register)", agent_id),
        )),
        (Err(why), _) => {
            warn!(
                agent_id = %agent_id,
                api_key = %auth.name,
                tenant = ?auth.tenant_id,
                reason = %why,
                "agent call refused: the key does not hold this agent"
            );
            Err(ApiError::forbidden(
                "agent_access_denied",
                "this key does not hold the agent",
            ))
        }
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

    // CLOACI-T-0861 / D-5 — seed the server-side one-time key pool from the
    // agent's advertised entries. A pre-pool agent that sent only the single
    // `ephemeral_public_key` is accommodated by synthesizing one pool entry so
    // one secret-bearing dispatch can still be served; a modern agent sends a
    // full pool. A register by the key that holds the id overwrites the
    // record (and its pool) wholesale.
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

    // CLOACI-T-0916: the roster row is DB-backed (visible to every replica);
    // the one-time key pool is seeded on THIS replica (see agent_registry.rs).
    //
    // CLOACINA-T-1051: the row gets the tenant of the key and the key as its
    // owner. An id the server assigned is new. An id the caller chose is
    // checked in the same transaction as the write: a register for an id
    // that another tenant or another key holds is refused, and the holder's
    // row and key pool stay as they are.
    let registered = state
        .agent_registry
        .register(
            FleetAgentRegistration {
                agent_id: agent_id.clone(),
                target_triple: req.target_triple.clone(),
                capabilities: req.capabilities.clone(),
                max_concurrency: req.max_concurrency,
                // CLOACINA-T-1027: the agent's totals, when it reports them.
                cpu_millis: req.cpu_millis,
                memory_bytes: req.memory_bytes,
            },
            ServerKeyPool::from_entries(pool_entries),
            &principal_of(&auth),
        )
        .await
        .map_err(|e| ApiError::internal_logged("agent registration persist failed", &e))?;
    if let Err(why) = registered {
        warn!(
            agent_id = %agent_id,
            api_key = %auth.name,
            tenant = ?auth.tenant_id,
            reason = %why,
            "agent register refused: another key holds this agent id"
        );
        return Err(ApiError::conflict(
            "agent_id_taken",
            format!(
                "agent id {agent_id} is held by another key; register with the key that holds \
                 it, or with no agent_id to get a new one"
            ),
        ));
    }

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

/// Add an agent's worker counts to the server's metrics (CLOACINA-T-1126). A
/// retire reason outside the known list is counted as `other`, so an agent
/// cannot make new label values.
fn record_worker_counts(counts: &cloacina::fleet::AgentWorkerCounts) {
    if counts.cold_starts > 0 {
        metrics::counter!("cloacina_fleet_worker_starts_total", "start" => "cold")
            .increment(counts.cold_starts);
    }
    if counts.warm_starts > 0 {
        metrics::counter!("cloacina_fleet_worker_starts_total", "start" => "warm")
            .increment(counts.warm_starts);
    }
    for (reason, n) in &counts.retired {
        if *n == 0 {
            continue;
        }
        metrics::counter!(
            "cloacina_fleet_warm_workers_retired_total",
            "reason" => retire_reason_label(reason)
        )
        .increment(*n);
    }
}

/// The metric label of a warm-worker retire reason: a known reason, else
/// `other`.
fn retire_reason_label(reason: &str) -> &'static str {
    cloacina::fleet::WARM_RETIRE_REASONS
        .iter()
        .copied()
        .find(|known| *known == reason)
        .unwrap_or("other")
}

/// `POST /v1/agent/heartbeat`
pub async fn heartbeat_agent(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Json(req): Json<AgentHeartbeatRequest>,
) -> Result<Json<AgentHeartbeatResponse>, ApiError> {
    require_protocol_version(req.protocol_version)?;
    require_agent_owner(&state, &auth, &req.agent_id).await?;

    // CLOACI-T-0916: heartbeat lands in the DB-backed roster, so it succeeds
    // no matter which replica handled the registration (no more "not
    // registered" flapping behind a non-affine load balancer).
    let known = state
        .agent_registry
        .record_heartbeat(&req.agent_id, req.in_flight, req.available_capacity)
        .await
        .map_err(|e| ApiError::internal_logged("agent heartbeat persist failed", &e))?;
    if !known {
        return Err(ApiError::not_found(
            "agent_not_registered",
            format!("agent not registered: {} (re-register)", req.agent_id),
        ));
    }
    // CLOACINA-T-1126 — the agent's worker starts and warm-worker retirements
    // since its last heartbeat.
    if let Some(counts) = &req.worker_counts {
        record_worker_counts(counts);
    }
    // CLOACINA-T-1011 — renew the lease of the work packets the agent says it
    // still works on, so the delivery sweeper does not push them again. Best
    // effort: on a failure the sweeper pushes again and the agent ignores the
    // re-push of a packet it has in work. The renewal names the rows by id;
    // `renew_delivered` touches only the rows addressed to this agent, and
    // the caller holds this agent (checked above), so one agent cannot renew
    // the rows of another (CLOACINA-T-1052).
    if !req.in_flight_push_ids.is_empty() {
        let recipient = format!(
            "{}{}",
            cloacina::fleet::AGENT_RECIPIENT_PREFIX,
            req.agent_id
        );
        let dal = cloacina::dal::DAL::new(state.database.clone());
        if let Err(e) = dal
            .delivery_outbox()
            .renew_delivered(&recipient, &req.in_flight_push_ids)
            .await
        {
            tracing::warn!(agent_id = %req.agent_id, error = %e, "work packet lease renewal failed");
        }
    }
    // CLOACI-T-0861 / D-5 — tell the agent how many one-time keys to top up if
    // secret-bearing dispatches have drawn the pool below its target. The pool
    // is replica-local; a replica the agent has not topped up yet reports the
    // full target, which is how each replica's pool gets established
    // (CLOACI-T-0916 residual — see agent_registry.rs).
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
/// `key_id`. An unknown agent is rejected (it should re-register), and so is
/// a key that does not hold the agent: the keys are what the server wraps
/// the agent's secrets to.
pub async fn replenish_keys(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Json(req): Json<AgentKeyReplenishRequest>,
) -> Result<Json<AgentKeyReplenishResponse>, ApiError> {
    require_protocol_version(req.protocol_version)?;
    // CLOACI-T-0916: registration is checked against the DB-backed roster (any
    // replica); the keys themselves land in THIS replica's local pool.
    require_agent_owner(&state, &auth, &req.agent_id).await?;
    let accepted = state
        .agent_registry
        .replenish_secret_keys(&req.agent_id, req.keys);
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
/// If no executor waits on this replica, the report is stored in the fleet
/// result inbox and the replica that dispatched the task takes it
/// (CLOACINA-T-1015). A report nobody waits for (a stale duplicate after a
/// retry, or a report whose replica stopped) changes no state. A report that
/// can be neither forwarded nor stored is answered with a 5xx, so the agent
/// sends it again.
///
/// CLOACINA-T-1050: the caller key must hold the agent the report names (an
/// unknown agent id is a 404, another key's agent a 403). The wait then takes
/// the report only if it comes from the tenant and the agent its packet went
/// to, with the dispatch id of the packet
/// ([`cloacina::security::ownership::DispatchTarget::check_report`]).
pub async fn report_result(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Json(req): Json<AgentResultRequest>,
) -> Result<Json<AgentResultResponse>, ApiError> {
    require_protocol_version(req.protocol_version)?;
    let agent_row = require_agent_owner(&state, &auth, &req.agent_id).await?;

    // CLOACI-T-0780: a refusal is an EXPECTED fail-closed outcome — and now rare,
    // since the fleet only dispatches to agents with a runnable arch. Log it at
    // debug so the backstop stays quiet; real results (success/failure) stay at info.
    log_reported_result(&req);

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

    // CLOACINA-T-1015: a wait on this replica gets the report at once. With
    // no wait here the report is stored for the replica that dispatched the
    // task: the agent posts to the Service, so any replica can get it. The
    // rules of CLOACINA-A-0017 are the same on both paths: a report resolves
    // only its own dispatch, and one that no replica waits for changes no
    // state (it is deleted by age and counted in
    // `cloacina_fleet_orphan_results_total`).
    let agent_id = req.agent_id.clone();
    let attempt = req.attempt;
    let dispatch_id = req.dispatch_id.clone();
    let duration_ms = req.duration_ms;
    let inbox = cloacina::dal::DAL::new(state.database.clone());
    // The report goes on with the tenant of the agent row the key was checked
    // against. A stored report keeps it, so the replica that takes it runs
    // the same check.
    let checked = crate::fleet_coordinator::CheckedReport::new(&agent_row, req);
    match state
        .fleet_coordinator
        .accept(&inbox, task_id, checked)
        .await
    {
        Ok(crate::fleet_coordinator::Accepted::Local) => {}
        // A wait here is for this dispatch and the sender is not the agent
        // it went to. The coordinator logged and counted it; no state
        // changed. The answer is the one an orphan gets.
        Ok(crate::fleet_coordinator::Accepted::Refused(_)) => {}
        Ok(crate::fleet_coordinator::Accepted::Stored) => {
            tracing::info!(
                task_id = %task_id,
                agent_id = %agent_id,
                attempt,
                dispatch_id = ?dispatch_id,
                duration_ms,
                "agent result is for no dispatch this replica waits for: stored for the replica \
                 that does"
            );
        }
        Err(e) => {
            // Neither forwarded nor stored. A 5xx makes the agent send the
            // report again (it retries with backoff).
            return Err(ApiError::internal_logged("agent result not accepted", &e));
        }
    }

    Ok(Json(AgentResultResponse {
        protocol_version: AGENT_PROTOCOL_VERSION,
    }))
}

/// `POST /v1/agent/undecodable` (CLOACINA-T-1058)
///
/// An agent got a push that it cannot decode. It has no task id, so it names
/// the push by its delivery-outbox id. The server reads its own row, and the
/// dispatch of that packet ends with a failure that names a likely version
/// mismatch: the task's retry policy decides the next run, a graph firing
/// ends with the error
/// ([`crate::fleet_coordinator::fail_undecodable_packet`]).
///
/// The caller key must hold the agent (as for a result), and the row must be
/// addressed to that agent in its tenant (as for an ack). A row of another
/// recipient or tenant and an unknown row id get the same 404
/// `delivery_row_not_found`, and nothing changes. A 5xx makes the agent send
/// the message again.
pub async fn report_undecodable(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Json(req): Json<AgentUndecodableRequest>,
) -> Result<Json<AgentUndecodableResponse>, ApiError> {
    use crate::fleet_coordinator::Undecodable;

    require_protocol_version(req.protocol_version)?;
    let agent_row = require_agent_owner(&state, &auth, &req.agent_id).await?;
    let db = cloacina::dal::DAL::new(state.database.clone());
    match crate::fleet_coordinator::fail_undecodable_packet(
        &state.fleet_coordinator,
        &db,
        &agent_row,
        &req,
    )
    .await
    {
        Ok(Undecodable::Failed(_)) | Ok(Undecodable::AlreadyAcked) => {}
        Ok(Undecodable::NoRow) | Ok(Undecodable::NotOwned) => {
            return Err(ApiError::not_found(
                "delivery_row_not_found",
                format!("no delivery row {} of this agent", req.push_id),
            ));
        }
        Ok(Undecodable::NotAPacket) => {
            return Err(ApiError::bad_request(
                "not_a_work_packet",
                format!("delivery row {} holds no work packet", req.push_id),
            ));
        }
        Err(e) => {
            return Err(ApiError::internal_logged(
                "undecodable-packet message not accepted",
                &e,
            ));
        }
    }
    Ok(Json(AgentUndecodableResponse {
        protocol_version: AGENT_PROTOCOL_VERSION,
    }))
}

/// The database that holds the packages the caller may fetch
/// (CLOACI-T-0781, CLOACINA-T-1016). A tenant's packages live in ITS schema,
/// so the three content-addressed fetch routes below must all read there:
///
/// - a tenant key reads its own tenant's database (`resolve("public")` is the
///   admin schema, so a key of the `public` tenant reads public);
/// - a platform admin key (no tenant) reads the admin schema;
/// - a key with no tenant that is not a platform admin reads nothing.
///
/// A tenant database that does not resolve is an error. It is never replaced
/// by the admin schema: that would hand a tenant key the public packages.
async fn caller_package_db(
    state: &AppState,
    auth: &AuthenticatedKey,
) -> Result<cloacina::database::Database, ApiError> {
    match &auth.tenant_id {
        Some(tenant) => state
            .tenant_databases
            .resolve(tenant, &state.database)
            .await
            .map_err(|e| ApiError::internal_logged("tenant database error", &e)),
        None if auth.is_admin => Ok(state.database.clone()),
        None => Err(ApiError::forbidden(
            "tenant_access_denied",
            "key has no tenant",
        )),
    }
}

/// `GET /v1/agent/artifact/{digest}` — content-addressed cdylib fetch.
///
/// Returns the raw cdylib bytes for the `workflow_packages` row whose
/// `content_hash == digest` and `build_status == 'success'`, read from the
/// caller's tenant (see [`caller_package_db`]). A digest of another tenant is
/// a 404, the same as an unknown digest.
pub async fn fetch_artifact(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(digest): Path<String>,
) -> Result<Response, ApiError> {
    let dal = cloacina::dal::DAL::new(caller_package_db(&state, &auth).await?);
    let bytes = dal
        .workflow_packages()
        .get_compiled_data_by_content_hash(&digest)
        .await
        .map_err(|e| ApiError::internal_logged("artifact lookup failed", &e))?;

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
        .map_err(|e| ApiError::internal_logged("build artifact response", &e))?;
    Ok(resp)
}

/// `GET /v1/agent/source/{digest}` — content-addressed package SOURCE fetch
/// (CLOACI-T-0716). Returns the uploaded `.cloacina` archive for the package
/// whose active build has `content_hash == digest`. Used for Python packages:
/// they have no cdylib (`/artifact/{digest}` is empty), so the agent fetches
/// the archive and imports the `workflow/` + `vendor/` tree via PyO3. Reads
/// the caller's tenant, as `fetch_artifact` does.
pub async fn fetch_source(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(digest): Path<String>,
) -> Result<Response, ApiError> {
    let dal = cloacina::dal::DAL::new(caller_package_db(&state, &auth).await?);
    let bytes = dal
        .workflow_packages()
        .get_package_archive_by_content_hash(&digest)
        .await
        .map_err(|e| ApiError::internal_logged("source lookup failed", &e))?;

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
        .map_err(|e| ApiError::internal_logged("build source response", &e))?;
    Ok(resp)
}

/// `GET /v1/agent/python-vendor/{digest}?platform=<arch>-<os>` — the
/// dependencies the compiler vendored for the Python package whose active
/// build has `content_hash == digest`, for the agent's platform
/// (CLOACINA-T-1073). The body is the archive the agent's loader unpacks into
/// `vendor/<platform>/`.
///
/// 204: the package has no vendored set for this platform (it ships its own
/// `vendor/` folder, has no dependencies, or the compiler did not vendor for
/// this platform). 404: no such package for the caller's tenant. Reads the
/// caller's tenant, as `fetch_artifact` does.
pub async fn fetch_python_vendor(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(digest): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Response, ApiError> {
    let Some(platform) = params.get("platform").filter(|p| !p.is_empty()) else {
        return Err(ApiError::bad_request(
            "platform_required",
            "give the agent's platform: ?platform=<arch>-<os>",
        ));
    };

    let dal = cloacina::dal::DAL::new(caller_package_db(&state, &auth).await?);
    let Some((package_name, version)) = dal
        .workflow_packages()
        .get_package_name_version_by_content_hash(&digest)
        .await
        .map_err(|e| ApiError::internal_logged("python vendor lookup failed", &e))?
    else {
        return Err(ApiError::not_found(
            "package_not_found",
            format!("no successfully-built package with digest {}", digest),
        ));
    };

    let bytes = dal
        .workflow_packages()
        .get_python_vendor_artifact(&package_name, &version, platform)
        .await
        .map_err(|e| ApiError::internal_logged("python vendor lookup failed", &e))?;

    let resp = match bytes {
        Some(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .body(Body::from(bytes)),
        None => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty()),
    }
    .map_err(|e| ApiError::internal_logged("build python vendor response", &e))?;
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
/// Empty list = the package uses no constructors. Reads the caller's tenant,
/// as `fetch_artifact` does.
pub async fn fetch_providers(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(digest): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<AgentProvidersResponse>, ApiError> {
    use base64::Engine as _;

    let dal = cloacina::dal::DAL::new(caller_package_db(&state, &auth).await?);
    let Some((package_name, version)) = dal
        .workflow_packages()
        .get_package_name_version_by_content_hash(&digest)
        .await
        .map_err(|e| ApiError::internal_logged("provider lookup failed", &e))?
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
        .map_err(|e| ApiError::internal_logged("provider fetch failed", &e))?;

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
/// roster (admin only). CLOACI-I-0124 / WS-0b. CLOACI-T-0916: read from the
/// DB-backed roster (heartbeat-recency-filtered), so the listing reflects the
/// WHOLE fleet regardless of which replica each agent registered against.
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
    let live = match state.agent_registry.live_agents().await {
        Ok(live) => live,
        Err(e) => {
            return ApiError::internal_logged("agent roster read failed", &e).into_response();
        }
    };
    let now = chrono::Utc::now();
    let agents: Vec<cloacina_api_types::AgentInfo> = live
        .into_iter()
        // CLOACI-T-0785: tenant-admins see only their own tenant's agents; god sees all.
        .filter(|r| auth.is_admin || r.tenant_id == auth.tenant_id)
        .map(|r| cloacina_api_types::AgentInfo {
            agent_id: r.agent_id,
            target_triple: r.target_triple,
            max_concurrency: r.max_concurrency,
            in_flight: r.in_flight,
            available_capacity: r.available_capacity,
            seconds_since_heartbeat: (now - r.last_heartbeat_at.0).num_seconds().max(0) as u64,
            capabilities: r.capabilities,
            tenant_id: r.tenant_id,
        })
        .collect();
    Json(cloacina_api_types::common::ListResponse::new(agents)).into_response()
}

/// The log lines of a reported result (CLOACINA-T-1072).
///
/// Info (debug for a refusal) names the report: the agent, the task, the
/// attempt, the kind of outcome and the size of its payload. The context of a
/// success and the message of a failure or a refusal are task data: they are
/// in the trace line only, which an operator must turn on.
fn log_reported_result(req: &AgentResultRequest) {
    let (kind, payload_bytes) = match &req.outcome {
        AgentOutcome::Success { context } => (
            "success".to_string(),
            serde_json::to_string(context).map(|s| s.len()).unwrap_or(0),
        ),
        AgentOutcome::Failure {
            message,
            classification,
        } => (format!("failure({classification:?})"), message.len()),
        AgentOutcome::Refused { reason, message } => {
            (format!("refused({reason:?})"), message.len())
        }
    };
    if matches!(req.outcome, AgentOutcome::Refused { .. }) {
        debug!(
            agent_id = %req.agent_id,
            task_id = %req.task_execution_id,
            attempt = req.attempt,
            outcome = %kind,
            payload_bytes,
            "agent reported result (refused — fail-closed guard)"
        );
    } else {
        info!(
            agent_id = %req.agent_id,
            task_id = %req.task_execution_id,
            attempt = req.attempt,
            duration_ms = req.duration_ms,
            outcome = %kind,
            payload_bytes,
            "agent reported result"
        );
    }
    tracing::trace!(
        agent_id = %req.agent_id,
        task_id = %req.task_execution_id,
        attempt = req.attempt,
        outcome = %serde_json::to_string(&req.outcome).unwrap_or_else(|_| "<unserializable>".into()),
        "agent reported result: the full outcome (task data)"
    );
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

    #[test]
    fn a_warm_worker_retire_reason_outside_the_list_is_other() {
        assert_eq!(retire_reason_label("idle"), "idle");
        assert_eq!(retire_reason_label("max_jobs"), "max_jobs");
        assert_eq!(retire_reason_label("made-up"), "other");
    }

    // -----------------------------------------------------------------------
    // CLOACINA-T-1072: the log of a reported result holds no task data below
    // trace.
    // -----------------------------------------------------------------------

    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone)]
    struct StringWriter(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for StringWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for StringWriter {
        type Writer = StringWriter;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    /// What `log_reported_result` writes for `req` with the log at `level`.
    fn logged(level: tracing::Level, req: &AgentResultRequest) -> String {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::fmt()
            .with_writer(StringWriter(buffer.clone()))
            .with_ansi(false)
            .with_max_level(level)
            .finish();
        tracing::subscriber::with_default(subscriber, || log_reported_result(req));
        let bytes = buffer.lock().unwrap().clone();
        String::from_utf8(bytes).unwrap()
    }

    fn report(outcome: AgentOutcome) -> AgentResultRequest {
        AgentResultRequest {
            protocol_version: AGENT_PROTOCOL_VERSION,
            agent_id: "agent-t1072".into(),
            task_execution_id: "7f0c1a52-0000-4000-8000-000000001072".into(),
            attempt: 2,
            dispatch_id: None,
            duration_ms: 41,
            outcome,
        }
    }

    fn success() -> AgentResultRequest {
        report(AgentOutcome::Success {
            context: serde_json::json!({ "customer_note": "value-from-the-task-context" }),
        })
    }

    fn failure() -> AgentResultRequest {
        report(AgentOutcome::Failure {
            message: "row for robin has text-from-the-task-error".into(),
            classification: cloacina::fleet::FailureClassification::TaskError,
        })
    }

    fn refusal() -> AgentResultRequest {
        report(AgentOutcome::Refused {
            reason: cloacina::fleet::RefusalReason::ArtifactFetchFailed,
            message: "text-from-the-refusal".into(),
        })
    }

    #[test]
    fn the_info_log_of_a_success_names_the_report_and_holds_no_context() {
        let out = logged(tracing::Level::INFO, &success());
        assert!(out.contains("agent reported result"), "{out}");
        assert!(out.contains("agent-t1072"), "{out}");
        assert!(
            out.contains("7f0c1a52-0000-4000-8000-000000001072"),
            "{out}"
        );
        assert!(out.contains("attempt=2"), "{out}");
        assert!(out.contains("duration_ms=41"), "{out}");
        assert!(out.contains("outcome=success"), "{out}");
        // The JSON of the context is 47 bytes.
        assert!(out.contains("payload_bytes=47"), "{out}");
        assert!(
            !out.contains("value-from-the-task-context"),
            "the info log holds a value of the task context: {out}"
        );
        assert!(
            !out.contains("customer_note"),
            "the info log holds a key of the task context: {out}"
        );
    }

    #[test]
    fn the_info_log_of_a_failure_names_the_kind_and_holds_no_message() {
        let out = logged(tracing::Level::INFO, &failure());
        assert!(out.contains("agent reported result"), "{out}");
        assert!(out.contains("outcome=failure(TaskError)"), "{out}");
        assert!(
            !out.contains("text-from-the-task-error"),
            "the info log holds the error message of the task: {out}"
        );
    }

    /// `--verbose` is debug: it shows a refusal, and still no task data.
    #[test]
    fn the_debug_log_holds_no_task_data() {
        for req in [success(), failure(), refusal()] {
            let out = logged(tracing::Level::DEBUG, &req);
            assert!(out.contains("agent reported result"), "{out}");
            for data in [
                "value-from-the-task-context",
                "text-from-the-task-error",
                "text-from-the-refusal",
            ] {
                assert!(!out.contains(data), "the debug log holds {data}: {out}");
            }
        }
        let out = logged(tracing::Level::DEBUG, &refusal());
        assert!(
            out.contains("outcome=refused(ArtifactFetchFailed)"),
            "{out}"
        );
        // A refusal is not in the info log.
        assert_eq!(logged(tracing::Level::INFO, &refusal()), "");
    }

    /// Trace is the one level with the full outcome.
    #[test]
    fn the_trace_log_holds_the_full_outcome() {
        assert!(logged(tracing::Level::TRACE, &success()).contains("value-from-the-task-context"));
        assert!(logged(tracing::Level::TRACE, &failure()).contains("text-from-the-task-error"));
        assert!(logged(tracing::Level::TRACE, &refusal()).contains("text-from-the-refusal"));
    }
}
