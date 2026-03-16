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

//! Health check endpoint.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;

/// Shared application state available to all handlers.
#[derive(Clone)]
pub struct AppState {
    /// When the server started (for uptime calculation).
    pub startup_instant: Instant,
    /// The configured server mode.
    pub mode: String,
    /// Auth middleware state. None when running in api-only mode without a database.
    pub auth_state: Option<crate::auth::middleware::AuthState>,
    /// The DefaultRunner for backend services. None in api-only mode.
    pub runner: Option<cloacina::runner::DefaultRunner>,
}

/// Health check response body.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    /// Server health status ("ok" or "degraded").
    pub status: String,
    /// Server version from Cargo.toml.
    pub version: String,
    /// Configured operational mode.
    pub mode: String,
    /// Seconds since server start.
    pub uptime_seconds: u64,
}

/// GET /health — returns server health status.
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Server is healthy", body = HealthResponse),
        (status = 503, description = "Server is degraded", body = HealthResponse),
    ),
    tag = "system"
)]
pub async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let uptime = state.startup_instant.elapsed().as_secs();

    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        mode: state.mode.clone(),
        uptime_seconds: uptime,
    };

    (StatusCode::OK, Json(response))
}
