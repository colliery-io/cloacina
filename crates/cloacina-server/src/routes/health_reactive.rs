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

//! Health endpoints for the reactive computation graph system.
//!
//! - `GET /v1/health/accumulators` — list all accumulators with status
//! - `GET /v1/health/reactors` — list all reactors with status
//! - `GET /v1/health/reactors/{name}` — single reactor health

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

/// GET /v1/health/reactors — list all reactors with status.
pub async fn list_reactors(State(state): State<AppState>) -> impl IntoResponse {
    let graphs = state.reactive_scheduler.list_graphs().await;

    let reactors: Vec<serde_json::Value> = graphs
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
                "paused": g.reactor_paused,
            })
        })
        .collect();

    Json(serde_json::json!({ "reactors": reactors }))
}

/// GET /v1/health/reactors/{name} — single reactor health.
pub async fn get_reactor(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let graphs = state.reactive_scheduler.list_graphs().await;

    if let Some(g) = graphs.into_iter().find(|g| g.name == name) {
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
                "paused": g.reactor_paused,
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("reactor '{}' not found", name)
            })),
        )
    }
}
