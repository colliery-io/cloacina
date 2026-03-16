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

//! Execution management endpoints.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use cloacina::executor::pipeline_executor::PipelineExecutor;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::error::ApiError;
use super::health::AppState;

fn require_runner(state: &AppState) -> Result<&cloacina::runner::DefaultRunner, ApiError> {
    state.runner.as_ref().ok_or_else(|| {
        ApiError::service_unavailable("Server running in API-only mode without backend services")
    })
}

/// Request body for triggering an execution.
#[derive(Deserialize)]
pub struct ExecutionRequest {
    pub workflow_name: String,
    #[serde(default)]
    pub context: serde_json::Value,
}

/// Response for execution creation.
#[derive(Serialize)]
pub struct ExecutionCreatedResponse {
    pub execution_id: String,
    pub status: String,
}

/// Response for execution status.
#[derive(Serialize)]
pub struct ExecutionStatusResponse {
    pub execution_id: String,
    pub workflow_name: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub task_results: Vec<TaskResultResponse>,
}

#[derive(Serialize)]
pub struct TaskResultResponse {
    pub task_name: String,
    pub status: String,
    pub error_message: Option<String>,
}

/// POST /executions — trigger a workflow execution.
pub async fn create_execution(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ExecutionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;

    // Build context from request body
    let mut context = cloacina::Context::new();
    if let serde_json::Value::Object(map) = body.context {
        for (k, v) in map {
            let _ = context.insert(&k, v);
        }
    }

    let execution = runner
        .execute_async(&body.workflow_name, context)
        .await
        .map_err(ApiError::from)?;

    Ok((
        StatusCode::ACCEPTED,
        Json(ExecutionCreatedResponse {
            execution_id: format!("{}", execution.execution_id),
            status: "accepted".to_string(),
        }),
    ))
}

/// GET /executions — list recent executions.
pub async fn list_executions(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;

    let results = runner.list_executions().await.map_err(ApiError::from)?;

    let response: Vec<ExecutionStatusResponse> = results
        .iter()
        .map(|r| ExecutionStatusResponse {
            execution_id: r.execution_id.to_string(),
            workflow_name: r.workflow_name.clone(),
            status: format!("{:?}", r.status),
            started_at: Some(r.start_time.to_rfc3339()),
            completed_at: r.end_time.map(|t| t.to_rfc3339()),
            error_message: r.error_message.clone(),
            task_results: r
                .task_results
                .iter()
                .map(|t| TaskResultResponse {
                    task_name: t.task_name.clone(),
                    status: format!("{:?}", t.status),
                    error_message: t.error_message.clone(),
                })
                .collect(),
        })
        .collect();

    Ok(Json(response))
}

/// GET /executions/{id} — get execution status and result.
pub async fn get_execution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;
    let uuid = uuid::Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request(format!("Invalid UUID: {}", id)))?;

    let result = runner
        .get_execution_result(uuid)
        .await
        .map_err(ApiError::from)?;

    let response = ExecutionStatusResponse {
        execution_id: result.execution_id.to_string(),
        workflow_name: result.workflow_name.clone(),
        status: format!("{:?}", result.status),
        started_at: Some(result.start_time.to_rfc3339()),
        completed_at: result.end_time.map(|t| t.to_rfc3339()),
        error_message: result.error_message.clone(),
        task_results: result
            .task_results
            .iter()
            .map(|t| TaskResultResponse {
                task_name: t.task_name.clone(),
                status: format!("{:?}", t.status),
                error_message: t.error_message.clone(),
            })
            .collect(),
    };

    Ok(Json(response))
}

/// Request body for pause (optional reason).
#[derive(Deserialize)]
pub struct PauseRequest {
    pub reason: Option<String>,
}

/// Simple status response for control operations.
#[derive(Serialize)]
pub struct ControlResponse {
    pub execution_id: String,
    pub status: String,
}

/// POST /executions/{id}/pause
pub async fn pause_execution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: Option<Json<PauseRequest>>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;
    let uuid = uuid::Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request(format!("Invalid UUID: {}", id)))?;

    let reason = body.and_then(|b| b.reason.clone());

    runner
        .pause_execution(uuid, reason.as_deref())
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ControlResponse {
        execution_id: id,
        status: "paused".to_string(),
    }))
}

/// POST /executions/{id}/resume
pub async fn resume_execution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;
    let uuid = uuid::Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request(format!("Invalid UUID: {}", id)))?;

    runner
        .resume_execution(uuid)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ControlResponse {
        execution_id: id,
        status: "resumed".to_string(),
    }))
}

/// DELETE /executions/{id}
pub async fn cancel_execution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let runner = require_runner(&state)?;
    let uuid = uuid::Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request(format!("Invalid UUID: {}", id)))?;

    runner
        .cancel_execution(uuid)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ControlResponse {
        execution_id: id,
        status: "cancelled".to_string(),
    }))
}
