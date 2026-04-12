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

//! Execution API — trigger workflows and query execution status.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;
use tracing::{info, warn};

use cloacina::executor::WorkflowExecutor;
use cloacina::Context;

use crate::commands::serve::AppState;
use crate::server::auth::AuthenticatedKey;
use crate::server::error::ApiError;

/// Request body for executing a workflow.
#[derive(Deserialize)]
pub struct ExecuteRequest {
    /// Optional JSON context to pass to the workflow.
    #[serde(default)]
    pub context: Option<serde_json::Value>,
}

/// POST /tenants/:tenant_id/workflows/:name/execute — execute a workflow.
///
/// NOTE: Execution is scheduled through the shared DefaultRunner, which uses
/// its own database connection. In per-tenant deployments (recommended), the
/// runner IS scoped to the tenant. In multi-tenant deployments, executions
/// land in the runner's schema. Full multi-tenant execute_workflow requires
/// per-tenant runners or a runner that accepts a Database override.
pub async fn execute_workflow(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
    Json(body): Json<ExecuteRequest>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }
    if !auth.can_write() {
        return AuthenticatedKey::insufficient_role_response().into_response();
    }

    let mut context = Context::new();

    // Merge provided context if any
    if let Some(ctx_value) = body.context {
        if let Some(obj) = ctx_value.as_object() {
            for (k, v) in obj {
                context.insert(k.clone(), v.clone()).unwrap_or_default();
            }
        }
    }

    match state.runner.execute_async(&name, context).await {
        Ok(execution) => {
            info!(
                "Executed workflow '{}' for tenant '{}': {}",
                name, tenant_id, execution.execution_id
            );
            (
                StatusCode::ACCEPTED,
                Json(serde_json::json!({
                    "execution_id": execution.execution_id.to_string(),
                    "workflow_name": name,
                    "tenant_id": tenant_id,
                    "status": "scheduled",
                })),
            )
                .into_response()
        }
        Err(e) => {
            warn!(
                "Failed to execute workflow '{}' for tenant '{}': {}",
                name, tenant_id, e
            );
            ApiError::bad_request("execution_failed", format!("{}", e)).into_response()
        }
    }
}

/// GET /tenants/:tenant_id/executions — list pipeline executions.
pub async fn list_executions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }

    let tenant_db = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal(format!("tenant database error: {}", e)).into_response()
        }
    };
    let dal = cloacina::dal::DAL::new(tenant_db);

    match dal.workflow_execution().get_active_executions().await {
        Ok(executions) => {
            let items: Vec<_> = executions
                .into_iter()
                .map(|e| {
                    serde_json::json!({
                        "id": e.id.0.to_string(),
                        "workflow_name": e.pipeline_name,
                        "status": e.status,
                        "started_at": e.started_at.0.to_rfc3339(),
                        "completed_at": e.completed_at.map(|t| t.0.to_rfc3339()),
                    })
                })
                .collect();
            Json(serde_json::json!({
                "tenant_id": tenant_id,
                "executions": items,
            }))
            .into_response()
        }
        Err(e) => {
            warn!(
                "Failed to list executions for tenant '{}': {}",
                tenant_id, e
            );
            ApiError::internal(format!("{}", e)).into_response()
        }
    }
}

/// GET /tenants/:tenant_id/executions/:id — get execution details.
pub async fn get_execution(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, exec_id)): Path<(String, String)>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }

    let id = match uuid::Uuid::parse_str(&exec_id) {
        Ok(id) => id,
        Err(_) => {
            return ApiError::bad_request("invalid_request", "invalid execution ID").into_response()
        }
    };

    let tenant_db = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal(format!("tenant database error: {}", e)).into_response()
        }
    };
    let dal = cloacina::dal::DAL::new(tenant_db);
    let universal_id = cloacina::database::universal_types::UniversalUuid(id);

    match dal.workflow_execution().get_by_id(universal_id).await {
        Ok(pipeline) => {
            let status = match pipeline.status.as_str() {
                "Pending" => "Pending",
                "Running" => "Running",
                "Completed" => "Completed",
                "Failed" => "Failed",
                "Cancelled" => "Cancelled",
                "Paused" => "Paused",
                other => other,
            };
            Json(serde_json::json!({
                "tenant_id": tenant_id,
                "execution_id": exec_id,
                "status": status,
            }))
            .into_response()
        }
        Err(e) => ApiError::not_found("execution_not_found", format!("{}", e)).into_response(),
    }
}

/// GET /tenants/:tenant_id/executions/:id/events — execution event log.
pub async fn get_execution_events(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, exec_id)): Path<(String, String)>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }

    let id = match uuid::Uuid::parse_str(&exec_id) {
        Ok(id) => id,
        Err(_) => {
            return ApiError::bad_request("invalid_request", "invalid execution ID").into_response()
        }
    };

    let tenant_db = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal(format!("tenant database error: {}", e)).into_response()
        }
    };
    let dal = cloacina::dal::DAL::new(tenant_db);
    let universal_id = cloacina::database::universal_types::UniversalUuid(id);

    match dal.execution_event().list_by_pipeline(universal_id).await {
        Ok(events) => {
            let items: Vec<_> = events
                .into_iter()
                .map(|e| {
                    serde_json::json!({
                        "id": e.id.0.to_string(),
                        "event_type": e.event_type,
                        "event_data": e.event_data,
                        "created_at": e.created_at.0.to_rfc3339(),
                        "sequence_num": e.sequence_num,
                    })
                })
                .collect();
            Json(serde_json::json!({
                "tenant_id": tenant_id,
                "execution_id": exec_id,
                "events": items,
            }))
            .into_response()
        }
        Err(e) => ApiError::internal(format!("{}", e)).into_response(),
    }
}
