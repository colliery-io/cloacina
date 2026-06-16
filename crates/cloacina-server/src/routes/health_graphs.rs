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

use cloacina::computation_graph::registry::KeyContext;
use cloacina_api_types::{AccumulatorStatus, GraphStatus, ListResponse};

use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
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

    let accumulators: Vec<AccumulatorStatus> = accumulators_with_health
        .into_iter()
        .map(|(name, health)| AccumulatorStatus {
            name,
            status: serde_json::to_value(health).unwrap_or(serde_json::Value::Null),
        })
        .collect();

    // CLOACI-T-0594 / API-03: unified `{items, total}` envelope.
    Json(ListResponse::new(accumulators))
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
}
