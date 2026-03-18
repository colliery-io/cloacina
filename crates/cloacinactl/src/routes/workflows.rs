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

//! Workflow package management endpoints.

use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use cloacina::dal::UnifiedRegistryStorage;
use cloacina::database::universal_types::UniversalUuid;
use cloacina::registry::{WorkflowRegistry, WorkflowRegistryImpl};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::error::ApiError;
use super::health::AppState;

/// Helper to get runner or return 503.
fn require_runner(state: &AppState) -> Result<&cloacina::runner::DefaultRunner, ApiError> {
    state.runner.as_ref().ok_or_else(|| {
        ApiError::service_unavailable("Server running in API-only mode without backend services")
    })
}

/// Build a WorkflowRegistryImpl from the runner's database.
fn build_registry(
    runner: &cloacina::runner::DefaultRunner,
) -> Result<WorkflowRegistryImpl<UnifiedRegistryStorage>, ApiError> {
    let db = runner.database().clone();
    let storage = UnifiedRegistryStorage::new(db.clone());
    WorkflowRegistryImpl::new(storage, db)
        .map_err(|e| ApiError::internal(format!("Failed to initialize workflow registry: {}", e)))
}

/// Response for package upload.
#[derive(Serialize)]
pub struct PackageUploadResponse {
    pub id: String,
    pub package_name: String,
    pub message: String,
}

/// POST /workflows/packages — upload a workflow package.
pub async fn upload_package(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;

    // Read package bytes from multipart
    let mut package_data: Option<Vec<u8>> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "package" || name == "file" {
            package_data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::bad_request(format!("Failed to read upload: {}", e)))?
                    .to_vec(),
            );
            break;
        }
    }

    let data = package_data.ok_or_else(|| {
        ApiError::bad_request("Missing 'package' or 'file' field in multipart upload")
    })?;

    let data_len = data.len();
    let mut registry = build_registry(runner)?;

    let package_id = registry.register_workflow(data).await.map_err(|e| {
        ApiError::bad_request(format!("Failed to register workflow package: {}", e))
    })?;

    Ok((
        StatusCode::CREATED,
        Json(PackageUploadResponse {
            id: package_id.to_string(),
            package_name: format!("registered ({} bytes)", data_len),
            message: "Workflow package registered successfully".to_string(),
        }),
    ))
}

/// Response for workflow list.
#[derive(Serialize)]
pub struct WorkflowListItem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub tasks: Vec<String>,
}

/// GET /workflows — list registered workflows.
pub async fn list_workflows(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;
    let registry = build_registry(runner)?;

    let workflows = registry
        .list_workflows()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to list workflows: {}", e)))?;

    let items: Vec<WorkflowListItem> = workflows
        .into_iter()
        .map(|w| WorkflowListItem {
            id: w.id.to_string(),
            name: w.package_name,
            version: w.version,
            description: w.description,
            tasks: w.tasks,
        })
        .collect();

    Ok(Json(items))
}

/// DELETE /workflows/packages/{id} — unregister a workflow package.
pub async fn delete_package(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;
    let mut registry = build_registry(runner)?;
    let uuid = uuid::Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request(format!("Invalid UUID: {}", id)))?;

    registry
        .unregister_workflow_package_by_id(uuid)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete package: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Schedule management
// ---------------------------------------------------------------------------

/// Request body for creating a cron schedule.
#[derive(Deserialize)]
pub struct CreateScheduleRequest {
    pub cron: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

/// Response for a cron schedule.
#[derive(Serialize)]
pub struct ScheduleResponse {
    pub id: String,
    pub workflow_name: String,
    pub cron_expression: String,
    pub timezone: String,
    pub enabled: bool,
    pub next_run_at: String,
    pub last_run_at: Option<String>,
    pub created_at: String,
}

/// POST /workflows/{name}/schedules — create a cron schedule for a workflow.
pub async fn create_schedule(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<CreateScheduleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;

    let schedule_id = runner
        .register_cron_workflow(&name, &body.cron, &body.timezone)
        .await
        .map_err(ApiError::from)?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": schedule_id.0.to_string(),
            "workflow_name": name,
            "cron_expression": body.cron,
            "timezone": body.timezone,
            "message": "Schedule created successfully"
        })),
    ))
}

/// GET /workflows/{name}/schedules — list cron schedules for a workflow.
pub async fn list_schedules(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;

    let all_schedules = runner
        .list_cron_schedules(false, 1000, 0)
        .await
        .map_err(ApiError::from)?;

    let schedules: Vec<ScheduleResponse> = all_schedules
        .iter()
        .filter(|s| s.workflow_name == name)
        .map(schedule_to_response)
        .collect();

    Ok(Json(schedules))
}

/// GET /workflows/schedules/{id} — get a single schedule by ID.
pub async fn get_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;
    let uuid = parse_uuid(&id)?;

    let schedule = runner
        .get_cron_schedule(uuid)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(schedule_to_response(&schedule)))
}

/// DELETE /workflows/schedules/{id} — delete a cron schedule.
pub async fn delete_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;
    let uuid = parse_uuid(&id)?;

    runner
        .delete_cron_schedule(uuid)
        .await
        .map_err(ApiError::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /workflows/schedules/{id}/history — list recent executions for a schedule.
pub async fn get_schedule_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;
    let uuid = parse_uuid(&id)?;

    let executions = runner
        .get_cron_execution_history(uuid, 50, 0)
        .await
        .map_err(ApiError::from)?;

    let items: Vec<serde_json::Value> = executions
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id.0.to_string(),
                "schedule_id": e.schedule_id.0.to_string(),
                "pipeline_execution_id": e.pipeline_execution_id.as_ref().map(|id| id.0.to_string()),
                "scheduled_time": e.scheduled_time.0.to_rfc3339(),
                "claimed_at": e.claimed_at.0.to_rfc3339(),
                "created_at": e.created_at.0.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(items))
}

fn parse_uuid(s: &str) -> Result<UniversalUuid, ApiError> {
    let uuid = uuid::Uuid::parse_str(s)
        .map_err(|_| ApiError::bad_request(format!("Invalid UUID: {}", s)))?;
    Ok(UniversalUuid(uuid))
}

fn schedule_to_response(s: &cloacina::models::cron_schedule::CronSchedule) -> ScheduleResponse {
    ScheduleResponse {
        id: s.id.0.to_string(),
        workflow_name: s.workflow_name.clone(),
        cron_expression: s.cron_expression.clone(),
        timezone: s.timezone.clone(),
        enabled: s.enabled.0,
        next_run_at: s.next_run_at.0.to_rfc3339(),
        last_run_at: s.last_run_at.as_ref().map(|t| t.0.to_rfc3339()),
        created_at: s.created_at.0.to_rfc3339(),
    }
}
