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
use cloacina::registry::{WorkflowRegistry, WorkflowRegistryImpl};
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
