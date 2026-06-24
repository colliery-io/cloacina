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

//! Compiler / build-pipeline status (CLOACI-I-0124 / WS-0b).
//!
//! The server is DB-coupled to the compiler (the compiler writes
//! `build_status` + a build-claim heartbeat onto `workflow_packages`), so this
//! reads the same rows the compiler's own `/v1/status` reports — no HTTP call
//! or compiler-URL config required.

use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Extension, Json};

use cloacina_api_types::CompilerStatus;

use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::AppState;

/// `GET /v1/compiler/status` — build-pipeline status (admin only).
#[utoipa::path(
    get,
    path = "/v1/compiler/status",
    tag = "compiler",
    responses(
        (status = 200, description = "Build-pipeline status", body = CompilerStatus),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Admin required", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn compiler_status(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    match cloacina::registry::workflow_registry::build_queue_stats(&state.database).await {
        Ok(s) => {
            let seconds_since_heartbeat = s
                .heartbeat_at
                .map(|h| (chrono::Utc::now() - h).num_seconds().max(0) as u64);
            let status = if s.building > 0 {
                "building"
            } else if s.pending > 0 {
                "backlogged"
            } else {
                "idle"
            };
            Json(CompilerStatus {
                status: status.to_string(),
                pending: s.pending,
                building: s.building,
                seconds_since_heartbeat,
                last_success_at: s.last_success_at.map(|t| t.to_rfc3339()),
                last_failure_at: s.last_failure_at.map(|t| t.to_rfc3339()),
            })
            .into_response()
        }
        Err(e) => ApiError::internal(format!("{}", e)).into_response(),
    }
}
