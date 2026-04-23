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
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::AppState;

/// GET /v1/health/accumulators — list all registered accumulators with health status.
pub async fn list_accumulators(State(state): State<AppState>) -> impl IntoResponse {
    let accumulators_with_health = state
        .endpoint_registry
        .list_accumulators_with_health()
        .await;

    let accumulators: Vec<serde_json::Value> = accumulators_with_health
        .into_iter()
        .map(|(name, health)| {
            serde_json::json!({
                "name": name,
                "status": health
            })
        })
        .collect();

    Json(serde_json::json!({ "accumulators": accumulators }))
}

/// GET /v1/health/graphs — list loaded computation graphs with status.
pub async fn list_graphs(State(state): State<AppState>) -> impl IntoResponse {
    let loaded = state.graph_scheduler.list_graphs().await;

    let graphs: Vec<serde_json::Value> = loaded
        .into_iter()
        .map(|g| {
            let health = g
                .health
                .as_ref()
                .map(|h| serde_json::to_value(h).unwrap_or(serde_json::json!("unknown")))
                .unwrap_or(
                    serde_json::json!({"state": if !g.running { "stopped" } else { "running" }}),
                );
            serde_json::json!({
                "name": g.name,
                "health": health,
                "accumulators": g.accumulators,
                // `paused` reflects the pause state of the graph's reactor.
                "paused": g.paused,
            })
        })
        .collect();

    Json(serde_json::json!({ "graphs": graphs }))
}

/// GET /v1/health/graphs/{name} — single graph health.
pub async fn get_graph(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let loaded = state.graph_scheduler.list_graphs().await;

    if let Some(g) = loaded.into_iter().find(|g| g.name == name) {
        let health = g
            .health
            .as_ref()
            .map(|h| serde_json::to_value(h).unwrap_or(serde_json::json!("unknown")))
            .unwrap_or(
                serde_json::json!({"state": if !g.running { "stopped" } else { "running" }}),
            );

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "name": g.name,
                "health": health,
                "accumulators": g.accumulators,
                // `paused` reflects the pause state of the graph's reactor.
                "paused": g.paused,
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("graph '{}' not found", name)
            })),
        )
    }
}
