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

//! Workflow package API — upload/list/get/delete .cloacina packages per tenant.

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use tracing::{info, warn};

use cloacina::dal::UnifiedRegistryStorage;
use cloacina::registry::traits::WorkflowRegistry;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;

use crate::commands::serve::AppState;
use crate::server::auth::AuthenticatedKey;

/// POST /tenants/:tenant_id/workflows — multipart upload of .cloacina source package.
pub async fn upload_workflow(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }
    if !auth.can_write() {
        return AuthenticatedKey::insufficient_role_response().into_response();
    }

    // Extract file from multipart
    let package_data = match extract_file_field(&mut multipart).await {
        Ok(data) => data,
        Err(msg) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": msg})),
            )
                .into_response()
        }
    };

    if package_data.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "empty package file"})),
        )
            .into_response();
    }

    // Register via WorkflowRegistry
    let storage = UnifiedRegistryStorage::new(state.database.clone());
    let mut registry = match WorkflowRegistryImpl::new(storage, state.database.clone()) {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to create registry: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal registry error"})),
            )
                .into_response();
        }
    };

    match registry.register_workflow_package(package_data).await {
        Ok(package_id) => {
            info!(
                "Uploaded workflow package for tenant '{}': {}",
                tenant_id, package_id
            );
            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "package_id": package_id.to_string(),
                    "tenant_id": tenant_id,
                })),
            )
                .into_response()
        }
        Err(e) => {
            warn!(
                "Failed to register workflow for tenant '{}': {}",
                tenant_id, e
            );
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response()
        }
    }
}

/// GET /tenants/:tenant_id/workflows — list registered workflows.
pub async fn list_workflows(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }

    let storage = UnifiedRegistryStorage::new(state.database.clone());
    let registry = match WorkflowRegistryImpl::new(storage, state.database.clone()) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response()
        }
    };

    match registry.list_workflows().await {
        Ok(workflows) => {
            let items: Vec<_> = workflows
                .into_iter()
                .map(|w| {
                    serde_json::json!({
                        "id": w.id.to_string(),
                        "package_name": w.package_name,
                        "version": w.version,
                        "description": w.description,
                        "tasks": w.tasks,
                        "created_at": w.created_at.to_rfc3339(),
                    })
                })
                .collect();
            Json(serde_json::json!({
                "tenant_id": tenant_id,
                "workflows": items,
            }))
            .into_response()
        }
        Err(e) => {
            warn!("Failed to list workflows for tenant '{}': {}", tenant_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response()
        }
    }
}

/// GET /tenants/:tenant_id/workflows/:name — get workflow details.
pub async fn get_workflow(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }

    let storage = UnifiedRegistryStorage::new(state.database.clone());
    let registry = match WorkflowRegistryImpl::new(storage, state.database.clone()) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response()
        }
    };

    match registry.list_workflows().await {
        Ok(workflows) => {
            let found = workflows.into_iter().find(|w| w.package_name == name);
            match found {
                Some(w) => Json(serde_json::json!({
                    "tenant_id": tenant_id,
                    "id": w.id.to_string(),
                    "package_name": w.package_name,
                    "version": w.version,
                    "description": w.description,
                    "tasks": w.tasks,
                    "created_at": w.created_at.to_rfc3339(),
                }))
                .into_response(),
                None => (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": format!("workflow '{}' not found", name)})),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("{}", e)})),
        )
            .into_response(),
    }
}

/// DELETE /tenants/:tenant_id/workflows/:name/:version — unregister workflow.
pub async fn delete_workflow(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name, version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }
    if !auth.can_write() {
        return AuthenticatedKey::insufficient_role_response().into_response();
    }

    let storage = UnifiedRegistryStorage::new(state.database.clone());
    let mut registry = match WorkflowRegistryImpl::new(storage, state.database.clone()) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response()
        }
    };

    match registry
        .unregister_workflow_package_by_name(&name, &version)
        .await
    {
        Ok(()) => {
            info!(
                "Deleted workflow '{}' v{} for tenant '{}'",
                name, version, tenant_id
            );
            Json(serde_json::json!({
                "status": "deleted",
                "package_name": name,
                "version": version,
            }))
            .into_response()
        }
        Err(e) => {
            warn!(
                "Failed to delete workflow '{}' v{} for tenant '{}': {}",
                name, version, tenant_id, e
            );
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response()
        }
    }
}

/// Extract the first file field from a multipart request.
async fn extract_file_field(multipart: &mut Multipart) -> Result<Vec<u8>, String> {
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            let data = field
                .bytes()
                .await
                .map_err(|e| format!("failed to read file: {}", e))?;
            return Ok(data.to_vec());
        }
    }
    Err("no 'file' field in multipart request".to_string())
}
