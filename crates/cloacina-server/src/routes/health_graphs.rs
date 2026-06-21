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

//! Health endpoints for the computation graph system.
//!
//! - `GET /v1/health/accumulators` — list all accumulators with status
//! - `GET /v1/health/graphs` — list loaded computation graphs with status
//! - `GET /v1/health/graphs/{name}` — single graph health
//!
//! Endpoint semantic per CLOACI-S-0011: "currently running graph
//! instances with their operational state" — not a catalog of
//! registered-but-not-running graphs.

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};

use tracing::warn;

use cloacina::computation_graph::reactor::ManualCommand;
use cloacina::computation_graph::registry::{KeyContext, ReactorOp};
use cloacina::computation_graph::types::{serialize as serialize_boundary, InputCache, SourceName};
use cloacina::dal::UnifiedRegistryStorage;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use cloacina::security::audit;
use cloacina_api_types::{
    AccumulatorStatus, DeclaredSurface, FireMode, FireReactorRequest, FireReactorResponse,
    GraphStatus, InjectAccumulatorRequest, InjectAccumulatorResponse, ListResponse, ReactorStatus,
};

use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::routes::executions::{validate_declared_params, validate_value_against_schema};
use crate::AppState;

/// Build a `KeyContext` from the `AuthenticatedKey` for policy
/// authorization checks. CLOACI-T-0579.
fn key_context<'a>(auth: &'a AuthenticatedKey) -> KeyContext<'a> {
    KeyContext {
        key_id: &auth.key_id,
        tenant_id: auth.tenant_id.as_deref(),
        is_admin: auth.is_admin,
    }
}

/// Decide whether the caller may see a graph based on its tenant scope.
/// Admin keys see everything. Tenant-scoped keys see graphs in their own
/// tenant and untagged (single-tenant / admin-owned) graphs.
/// CLOACI-T-0579.
fn graph_visible(auth: &AuthenticatedKey, graph_tenant: Option<&str>) -> bool {
    if auth.is_admin {
        return true;
    }
    match (graph_tenant, auth.tenant_id.as_deref()) {
        // Untagged graphs (no tenant) are visible to any authenticated
        // caller — matches the single-tenant default before multi-tenant
        // CG packages existed.
        (None, _) => true,
        (Some(g), Some(t)) => g == t,
        (Some(_), None) => false,
    }
}

/// GET /v1/health/accumulators — list registered accumulators with health,
/// filtered by the caller's authorization. CLOACI-T-0579 / SEC-05.
#[utoipa::path(
    get,
    path = "/v1/health/accumulators",
    tag = "graph-health",
    responses(
        (status = 200, description = "Registered accumulators with health, filtered by authorization", body = ListResponse<AccumulatorStatus>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_accumulators(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    let ctx = key_context(&auth);
    let accumulators_with_health = state
        .endpoint_registry
        .list_accumulators_with_health_for_key(&ctx)
        .await;

    // CLOACI-I-0128 follow-up: enrich each accumulator with the discoverability
    // descriptor it self-registered at load (parent reactor + tenant), so the
    // list surfaces the relationship, not just name + health.
    let mut accumulators: Vec<AccumulatorStatus> =
        Vec::with_capacity(accumulators_with_health.len());
    for (name, health, freshness) in accumulators_with_health {
        let descriptor = state.endpoint_registry.accumulator_descriptor(&name).await;
        // CLOACI-T-0765: promote freshness from the runtime probe (None until the
        // accumulator has emitted / on runtimes predating the probe).
        let last_event_at = freshness
            .as_ref()
            .and_then(|f| f.last_event_ms())
            .map(|ms| {
                chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            });
        let events_total = freshness.as_ref().map(|f| f.events_total());
        accumulators.push(AccumulatorStatus {
            state: Some(health.to_string()),
            status: serde_json::to_value(health).unwrap_or(serde_json::Value::Null),
            reactor: descriptor.as_ref().map(|d| d.reactor.clone()),
            tenant_id: descriptor.and_then(|d| d.tenant_id),
            last_event_at,
            events_total,
            error: None,
            name,
        });
    }

    // CLOACI-T-0594 / API-03: unified `{items, total}` envelope.
    Json(ListResponse::new(accumulators))
}

/// GET /v1/health/reactors — list loaded reactors visible to the caller
/// (CLOACI-T-0742). Reactor-first: includes reactors with no graph bound, which
/// `list_graphs` omits. Visibility reuses the same tenant gate as graphs.
#[utoipa::path(
    get,
    path = "/v1/health/reactors",
    tag = "graph-health",
    responses(
        (status = 200, description = "Loaded reactors visible to the caller", body = ListResponse<ReactorStatus>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_reactors(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    let loaded = state.graph_scheduler.list_reactors().await;

    let reactors: Vec<ReactorStatus> = loaded
        .into_iter()
        .filter(|r| graph_visible(&auth, r.tenant_id.as_deref()))
        .map(|r| {
            let health = r
                .health
                .as_ref()
                .map(|h| serde_json::to_value(h).unwrap_or(serde_json::json!("unknown")))
                .unwrap_or(
                    serde_json::json!({"state": if !r.running { "stopped" } else { "running" }}),
                );
            ReactorStatus {
                name: r.name,
                health,
                accumulators: r.accumulators,
                reaction_mode: Some(r.reaction_mode),
                input_strategy: Some(r.input_strategy),
                bound_graphs: r.bound_graphs,
                paused: r.paused,
                fires: r.fires,
                last_fired_at: r
                    .last_fire_unix_ms
                    .and_then(chrono::DateTime::from_timestamp_millis)
                    .map(|dt| dt.to_rfc3339()),
            }
        })
        .collect();

    Json(ListResponse::new(reactors))
}

/// GET /v1/health/graphs — list loaded graphs visible to the caller.
/// CLOACI-T-0579 / SEC-05.
#[utoipa::path(
    get,
    path = "/v1/health/graphs",
    tag = "graph-health",
    responses(
        (status = 200, description = "Loaded graphs visible to the caller", body = ListResponse<GraphStatus>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_graphs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    let loaded = state.graph_scheduler.list_graphs().await;

    let graphs: Vec<GraphStatus> = loaded
        .into_iter()
        .filter(|g| graph_visible(&auth, g.tenant_id.as_deref()))
        .map(|g| {
            let health = g
                .health
                .as_ref()
                .map(|h| serde_json::to_value(h).unwrap_or(serde_json::json!("unknown")))
                .unwrap_or(
                    serde_json::json!({"state": if !g.running { "stopped" } else { "running" }}),
                );
            GraphStatus {
                name: g.name,
                health,
                accumulators: g.accumulators,
                // `paused` reflects the pause state of the graph's reactor.
                paused: g.paused,
                topology: g
                    .topology
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok()),
                reactor: g.reactor,
                reaction_mode: Some(g.reaction_mode),
                input_strategy: Some(g.input_strategy),
                fires: g.fires,
                last_fired_at: g
                    .last_fire_unix_ms
                    .and_then(chrono::DateTime::from_timestamp_millis)
                    .map(|dt| dt.to_rfc3339()),
            }
        })
        .collect();

    // CLOACI-T-0594 / API-03: unified `{items, total}` envelope.
    Json(ListResponse::new(graphs))
}

/// GET /v1/health/graphs/{name} — single graph health, gated by caller
/// authorization. Cross-tenant requests 404 rather than 403 so an
/// adversary can't probe for tenant graph names. CLOACI-T-0579.
#[utoipa::path(
    get,
    path = "/v1/health/graphs/{name}",
    tag = "graph-health",
    params(("name" = String, Path, description = "Graph name")),
    responses(
        (status = 200, description = "Single graph health", body = GraphStatus),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Graph not found (or not visible to caller)", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_graph(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let loaded = state.graph_scheduler.list_graphs().await;

    let found = loaded
        .into_iter()
        .find(|g| g.name == name && graph_visible(&auth, g.tenant_id.as_deref()));

    if let Some(g) = found {
        let health = g
            .health
            .as_ref()
            .map(|h| serde_json::to_value(h).unwrap_or(serde_json::json!("unknown")))
            .unwrap_or(
                serde_json::json!({"state": if !g.running { "stopped" } else { "running" }}),
            );

        Json(GraphStatus {
            name: g.name,
            health,
            accumulators: g.accumulators,
            // `paused` reflects the pause state of the graph's reactor.
            paused: g.paused,
            topology: g
                .topology
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            reactor: g.reactor,
            reaction_mode: Some(g.reaction_mode),
            input_strategy: Some(g.input_strategy),
            fires: g.fires,
            last_fired_at: g
                .last_fire_unix_ms
                .and_then(chrono::DateTime::from_timestamp_millis)
                .map(|dt| dt.to_rfc3339()),
        })
        .into_response()
    } else {
        // T-0642: normalized from a bare `{"error": ...}` body to the
        // standard ApiError shape (adds the machine-readable `code`).
        ApiError::not_found("graph_not_found", format!("graph '{}' not found", name))
            .into_response()
    }
}

/// Encode one typed operator input into the boundary wire frame the reactor's
/// `InputCache` holds (CLOACI-T-0751).
///
/// This reproduces the front-door encoding exactly: the passthrough
/// accumulator forwards the raw event bytes (JSON text from the WebSocket) and
/// the `BoundarySender` bincode-serializes them as `Vec<u8>`. The FFI bridge
/// then recovers the original bytes via `bincode::deserialize::<Vec<u8>>` and
/// reads them as UTF-8 JSON (see `packaging_bridge::execute_graph_via_ffi`).
/// So the frame an operator's typed value must become is
/// `bincode(serde_json::to_vec(value))`.
fn encode_boundary_input(value: &serde_json::Value) -> Result<Vec<u8>, String> {
    let json_bytes =
        serde_json::to_vec(value).map_err(|e| format!("failed to encode JSON: {}", e))?;
    serialize_boundary(&json_bytes).map_err(|e| format!("failed to encode boundary: {}", e))
}

/// POST /v1/health/reactors/{name}/fire — manually fire a running reactor
/// (CLOACI-T-0751).
///
/// Operator-facing REST surface over the existing reactor `ForceFire` /
/// `FireWith` mechanics (previously WebSocket-only). Two modes:
///
/// - `force_fire`: fire the graph with the reactor's current cache, untouched.
/// - `fire_with`: replace the reactor's cache with the supplied typed `inputs`
///   then fire. Full-replace only (mirrors the engine's `replace_all`); there
///   is no partial/merge mode in v1.
///
/// Typed JSON `inputs` are serialized to the boundary wire encoding
/// server-side so operators never deal in raw `Vec<u8>`. Each source value is
/// encoded exactly as the front-door accumulator path encodes it: the JSON is
/// rendered to UTF-8 bytes, then those bytes are bincode-wrapped
/// (`bincode(Vec<u8>)`) — the format the passthrough accumulator and the FFI
/// bridge expect.
///
/// Authorization reuses the existing per-op reactor policy
/// (`ReactorOp::ForceFire` / `ReactorOp::FireWith`), the same gate the WS
/// reactor endpoint enforces. Successful fires are audit-logged and marked
/// `operator_injected` since they bypass the real event source.
#[utoipa::path(
    post,
    path = "/v1/health/reactors/{name}/fire",
    tag = "graph-health",
    params(("name" = String, Path, description = "Reactor name")),
    request_body = FireReactorRequest,
    responses(
        (status = 200, description = "Reactor fired", body = FireReactorResponse),
        (status = 400, description = "Invalid input payload", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Operation not permitted on this reactor", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Reactor not found", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn fire_reactor(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(name): Path<String>,
    Json(req): Json<FireReactorRequest>,
) -> impl IntoResponse {
    let ctx = key_context(&auth);

    // Per-op authZ — reuse the same policy the WS reactor endpoint enforces.
    let op = match req.mode {
        FireMode::ForceFire => ReactorOp::ForceFire,
        FireMode::FireWith => ReactorOp::FireWith,
    };
    if state
        .endpoint_registry
        .check_reactor_op_auth(&name, &ctx, &op)
        .await
        .is_err()
    {
        // 403 (not 404) mirrors the WS path, which reports the op as not
        // permitted rather than concealing the reactor's existence. Auth is
        // already required by the route layer, so the caller is known.
        return ApiError::forbidden(
            "reactor_op_denied",
            format!("operation {:?} not permitted on reactor '{}'", op, name),
        )
        .into_response();
    }

    // CLOACI-T-0759: validate fire_with inputs against the reactor's declared
    // per-source boundary slots (when the package declares a typed interface).
    // Undeclared/untyped surfaces accept free-form input; registry errors fail
    // open. The CG-health endpoints operate on the global runtime, so the
    // surfaces come from the admin/public registry.
    if matches!(req.mode, FireMode::FireWith) && !req.inputs.is_empty() {
        let storage = UnifiedRegistryStorage::new(state.database.clone());
        if let Ok(registry) = WorkflowRegistryImpl::new(storage, state.database.clone()) {
            match registry.find_surface_input_slots("reactor", &name).await {
                Ok(slots) if !slots.is_empty() => {
                    let provided: serde_json::Map<String, serde_json::Value> = req
                        .inputs
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    let errors = validate_declared_params(&slots, Some(&provided));
                    if !errors.is_empty() {
                        return ApiError::bad_request(
                            "reactor_input_invalid",
                            format!("invalid reactor inputs: {}", errors.join("; ")),
                        )
                        .into_response();
                    }
                }
                Ok(_) => {}
                Err(e) => warn!("reactor declared-slots lookup failed for '{}': {}", name, e),
            }
        }
    }

    // Build the manual command. For `fire_with`, serialize each typed JSON
    // value to the boundary encoding server-side.
    let (command, sources_injected) = match req.mode {
        FireMode::ForceFire => (ManualCommand::ForceFire, Vec::new()),
        FireMode::FireWith => {
            if req.inputs.is_empty() {
                return ApiError::bad_request(
                    "empty_inputs",
                    "mode 'fire_with' requires a non-empty 'inputs' map",
                )
                .into_response();
            }

            let mut cache = InputCache::new();
            let mut sources: Vec<String> = Vec::with_capacity(req.inputs.len());
            for (source, value) in req.inputs {
                // Render the typed JSON to bytes, then wrap in the bincode
                // boundary frame the accumulator/FFI bridge expects.
                let boundary = match encode_boundary_input(&value) {
                    Ok(b) => b,
                    Err(e) => {
                        return ApiError::bad_request(
                            "invalid_input",
                            format!("source '{}': {}", source, e),
                        )
                        .into_response();
                    }
                };
                cache.update(SourceName::new(&source), boundary);
                sources.push(source);
            }
            sources.sort();
            (ManualCommand::FireWith(cache), sources)
        }
    };

    match state
        .endpoint_registry
        .send_to_reactor(&name, command)
        .await
    {
        Ok(()) => {
            audit::log_reactor_manual_fire(
                &name,
                auth.key_id.into(),
                &auth.name,
                auth.tenant_id.as_deref(),
                if matches!(req.mode, FireMode::ForceFire) {
                    "force_fire"
                } else {
                    "fire_with"
                },
                &sources_injected,
            );
            Json(FireReactorResponse {
                reactor: name,
                mode: req.mode,
                sources_injected,
            })
            .into_response()
        }
        Err(e) => {
            // The registry only fails here if the reactor isn't registered
            // or its command channel is closed — surface as not-found.
            ApiError::not_found(
                "reactor_not_found",
                format!("reactor '{}' not available: {}", name, e),
            )
            .into_response()
        }
    }
}

/// POST /v1/health/accumulators/{name}/inject — push a single typed event into
/// a running accumulator (CLOACI-T-0753).
///
/// Operator-facing REST analogue of the WS accumulator-push path. The typed JSON
/// `event` is serialized to the boundary wire encoding server-side (same framing
/// as the front-door accumulator socket), so operators never craft raw
/// `Vec<u8>`. Authorization reuses the accumulator endpoint policy — the same
/// gate the WS accumulator endpoint enforces. Successful injects are
/// audit-logged and marked `operator_injected` since they bypass the real event
/// source.
#[utoipa::path(
    post,
    path = "/v1/health/accumulators/{name}/inject",
    tag = "graph-health",
    params(("name" = String, Path, description = "Accumulator name")),
    request_body = InjectAccumulatorRequest,
    responses(
        (status = 200, description = "Event injected", body = InjectAccumulatorResponse),
        (status = 400, description = "Invalid event payload", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Not authorized for this accumulator", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Accumulator not found", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn inject_accumulator(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(name): Path<String>,
    Json(req): Json<InjectAccumulatorRequest>,
) -> impl IntoResponse {
    let ctx = key_context(&auth);

    // Endpoint authZ — same policy the WS accumulator endpoint enforces.
    if state
        .endpoint_registry
        .check_accumulator_auth(&name, &ctx)
        .await
        .is_err()
    {
        return ApiError::forbidden(
            "accumulator_access_denied",
            format!("not authorized for accumulator '{}'", name),
        )
        .into_response();
    }

    // CLOACI-T-0759: validate the event against the accumulator's declared
    // boundary type (carried as a per-source slot in its graph/reactor surface).
    // Untyped/unknown accumulators accept free-form events; registry errors fail
    // open.
    {
        let storage = UnifiedRegistryStorage::new(state.database.clone());
        if let Ok(registry) = WorkflowRegistryImpl::new(storage, state.database.clone()) {
            match registry.find_accumulator_input_slot(&name).await {
                Ok(Some(slot)) => {
                    if let Some(err) = validate_value_against_schema(&req.event, &slot.schema) {
                        return ApiError::bad_request(
                            "accumulator_input_invalid",
                            format!("invalid event for accumulator '{}': {}", name, err),
                        )
                        .into_response();
                    }
                }
                Ok(None) => {}
                Err(e) => warn!(
                    "accumulator declared-slot lookup failed for '{}': {}",
                    name, e
                ),
            }
        }
    }

    // Serialize the typed JSON event to the boundary wire encoding server-side.
    let boundary = match encode_boundary_input(&req.event) {
        Ok(b) => b,
        Err(e) => return ApiError::bad_request("invalid_input", e).into_response(),
    };

    match state
        .endpoint_registry
        .send_to_accumulator(&name, boundary)
        .await
    {
        Ok(delivered) => {
            audit::log_accumulator_manual_inject(
                &name,
                auth.key_id.into(),
                &auth.name,
                auth.tenant_id.as_deref(),
                delivered,
            );
            Json(InjectAccumulatorResponse {
                accumulator: name,
                delivered,
            })
            .into_response()
        }
        // The registry only fails here if the accumulator isn't registered or
        // its channels are closed — surface as not-found.
        Err(e) => ApiError::not_found(
            "accumulator_not_found",
            format!("accumulator '{}' not available: {}", name, e),
        )
        .into_response(),
    }
}

/// GET /v1/health/reactors/{name}/interface — the reactor's declared input
/// interface (CLOACI-I-0128 T-0758): the per-source boundary slots an operator
/// supplies to `fire_with`. Empty `slots` means undeclared/untyped (the reactor
/// accepts free-form input). Read-only discovery so a UI can render a typed fire
/// form; the same slots back the server-side validation in `fire_reactor`.
#[utoipa::path(
    get,
    path = "/v1/health/reactors/{name}/interface",
    tag = "graph-health",
    params(("name" = String, Path, description = "Reactor name")),
    responses(
        (status = 200, description = "Declared reactor input interface", body = DeclaredSurface),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_reactor_interface(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let storage = UnifiedRegistryStorage::new(state.database.clone());
    let slots = match WorkflowRegistryImpl::new(storage, state.database.clone()) {
        Ok(registry) => registry
            .find_surface_input_slots("reactor", &name)
            .await
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    Json(DeclaredSurface {
        kind: "reactor".to_string(),
        name,
        slots,
    })
}

/// GET /v1/health/accumulators/{name}/interface — the accumulator's declared
/// input interface (CLOACI-I-0128 T-0758): the single boundary slot an operator
/// supplies to `inject`. Empty `slots` means undeclared/untyped. Read-only
/// discovery; the same slot backs the validation in `inject_accumulator`.
#[utoipa::path(
    get,
    path = "/v1/health/accumulators/{name}/interface",
    tag = "graph-health",
    params(("name" = String, Path, description = "Accumulator name")),
    responses(
        (status = 200, description = "Declared accumulator input interface", body = DeclaredSurface),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_accumulator_interface(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let storage = UnifiedRegistryStorage::new(state.database.clone());
    let slots = match WorkflowRegistryImpl::new(storage, state.database.clone()) {
        Ok(registry) => registry
            .find_accumulator_input_slot(&name)
            .await
            .ok()
            .flatten()
            .map(|s| vec![s])
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    Json(DeclaredSurface {
        kind: "accumulator".to_string(),
        name,
        slots,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth(tenant: Option<&str>, is_admin: bool) -> AuthenticatedKey {
        AuthenticatedKey {
            key_id: uuid::Uuid::new_v4(),
            name: "t".into(),
            permissions: "read".into(),
            tenant_id: tenant.map(str::to_string),
            is_admin,
        }
    }

    #[test]
    fn graph_visible_admin_sees_all() {
        let a = auth(None, true);
        assert!(graph_visible(&a, Some("acme")));
        assert!(graph_visible(&a, Some("other")));
        assert!(graph_visible(&a, None));
    }

    #[test]
    fn graph_visible_tenant_scoped() {
        let a = auth(Some("acme"), false);
        assert!(graph_visible(&a, Some("acme")));
        assert!(!graph_visible(&a, Some("other")));
        // Untagged graphs visible (single-tenant compat).
        assert!(graph_visible(&a, None));
    }

    #[test]
    fn graph_visible_global_key_cannot_see_tenant_graphs() {
        let a = auth(None, false);
        assert!(graph_visible(&a, None));
        assert!(!graph_visible(&a, Some("acme")));
    }

    /// The boundary frame a typed input becomes must round-trip back to the
    /// same JSON the FFI bridge reads: bincode-decode to `Vec<u8>`, then parse
    /// as UTF-8 JSON. CLOACI-T-0751.
    #[test]
    fn boundary_input_roundtrips_like_ffi_bridge() {
        let value = serde_json::json!({"symbol": "ABC", "price": 12.5, "live": true});
        let frame = encode_boundary_input(&value).expect("encode");

        // Mirror packaging_bridge::execute_graph_via_ffi.
        let original_bytes: Vec<u8> = bincode::deserialize(&frame).expect("bincode decode");
        let json_str = String::from_utf8(original_bytes).expect("utf8");
        let decoded: serde_json::Value = serde_json::from_str(&json_str).expect("json parse");

        assert_eq!(decoded, value);
    }
}
