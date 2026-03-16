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

use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
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

/// Response for package upload.
#[derive(Serialize)]
pub struct PackageUploadResponse {
    pub id: String,
    pub message: String,
}

/// POST /workflows/packages — upload a workflow package.
pub async fn upload_package(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let _runner = require_runner(&state)?;

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

    // TODO: DefaultRunner needs a method to access workflow_registry for package upload
    Ok((
        StatusCode::CREATED,
        Json(PackageUploadResponse {
            id: "placeholder".to_string(),
            message: format!(
                "Package upload received ({} bytes). Registry integration pending.",
                data.len()
            ),
        }),
    ))
}

/// Response for workflow list.
#[derive(Serialize)]
pub struct WorkflowListItem {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub tasks: Vec<String>,
}

/// GET /workflows — list registered workflows.
pub async fn list_workflows(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let _runner = require_runner(&state)?;

    // TODO: Access workflow registry through DefaultRunner
    // runner.workflow_registry() returns Arc<RwLock<Option<Arc<dyn WorkflowRegistry>>>>
    // Need to read lock, check if Some, then call list_workflows()

    let workflows: Vec<WorkflowListItem> = vec![];

    Ok(Json(workflows))
}
