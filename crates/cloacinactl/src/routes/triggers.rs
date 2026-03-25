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

//! Trigger management endpoints.
//!
//! Triggers are created by packages (not API), but operators can list,
//! enable/disable, and view trigger details.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use cloacina::dal::DAL;
use serde::Serialize;
use std::sync::Arc;

use super::error::ApiError;
use super::health::AppState;

/// Helper to get runner or return 503.
fn require_runner(state: &AppState) -> Result<&cloacina::runner::DefaultRunner, ApiError> {
    state.runner.as_ref().ok_or_else(|| {
        ApiError::service_unavailable("Server running in API-only mode without backend services")
    })
}

/// Trigger summary for list/detail responses.
#[derive(Debug, Serialize)]
pub struct TriggerSummary {
    pub id: String,
    pub trigger_name: String,
    pub workflow_name: String,
    pub poll_interval_ms: i64,
    pub allow_concurrent: bool,
    pub enabled: bool,
    pub last_poll_at: Option<String>,
}

fn schedule_to_summary(s: cloacina::models::trigger_schedule::TriggerSchedule) -> TriggerSummary {
    TriggerSummary {
        id: s.id.to_string(),
        trigger_name: s.trigger_name,
        workflow_name: s.workflow_name,
        poll_interval_ms: s.poll_interval_ms as i64,
        allow_concurrent: s.allow_concurrent.is_true(),
        enabled: s.enabled.is_true(),
        last_poll_at: s.last_poll_at.map(|t| t.0.to_rfc3339()),
    }
}

/// GET /triggers — list all trigger schedules.
pub async fn list_triggers(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;
    let dal = DAL::new(runner.database().clone());

    let schedules = dal
        .trigger_schedule()
        .list(100, 0)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to list triggers: {}", e)))?;

    let triggers: Vec<TriggerSummary> = schedules.into_iter().map(schedule_to_summary).collect();

    Ok((StatusCode::OK, Json(triggers)))
}

/// GET /triggers/:name — get trigger detail.
pub async fn get_trigger(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;
    let dal = DAL::new(runner.database().clone());

    let schedule = dal
        .trigger_schedule()
        .get_by_name(&name)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to get trigger: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Trigger '{}' not found", name)))?;

    Ok((StatusCode::OK, Json(schedule_to_summary(schedule))))
}

/// POST /triggers/:name/enable — enable a trigger.
pub async fn enable_trigger(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;
    let dal = DAL::new(runner.database().clone());

    let schedule = dal
        .trigger_schedule()
        .get_by_name(&name)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to get trigger: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Trigger '{}' not found", name)))?;

    dal.trigger_schedule()
        .enable(schedule.id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to enable trigger: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"status": "enabled", "trigger": name})),
    ))
}

/// POST /triggers/:name/disable — disable a trigger.
pub async fn disable_trigger(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;
    let dal = DAL::new(runner.database().clone());

    let schedule = dal
        .trigger_schedule()
        .get_by_name(&name)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to get trigger: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Trigger '{}' not found", name)))?;

    dal.trigger_schedule()
        .disable(schedule.id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to disable trigger: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"status": "disabled", "trigger": name})),
    ))
}
