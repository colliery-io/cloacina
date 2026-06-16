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
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use tracing::{info, warn};

use cloacina::executor::WorkflowExecutor;
use cloacina::Context;
use cloacina_api_types::{
    ExecuteRequest, ExecuteResponse, ExecutionDetail, ExecutionEvent, ExecutionEventsResponse,
    ExecutionSummary, ExecutionTasksResponse, ListExecutionsQuery, TaskExecutionDetail,
    TenantListResponse,
};

use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::AppState;

/// POST /tenants/:tenant_id/workflows/:name/execute — execute a workflow.
///
/// CLOACI-T-0580: execution is routed through `TenantRunnerCache`, which
/// returns (or constructs) a `DefaultRunner` bound to the tenant's
/// `Database`. The execution row + every event lands in the tenant's
/// schema, never the admin schema.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/execute",
    tag = "executions",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Workflow name"),
    ),
    request_body = ExecuteRequest,
    responses(
        (status = 202, description = "Execution scheduled", body = ExecuteResponse),
        (status = 400, description = "Execution failed", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or role denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
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

    // CLOACI-T-0580: resolve the tenant-scoped Database, then look up
    // (or construct) the per-tenant DefaultRunner from the LRU cache.
    let tenant_db = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            warn!(
                "Failed to resolve tenant database for '{}': {}",
                tenant_id, e
            );
            return ApiError::internal(format!("tenant database unavailable: {}", e))
                .into_response();
        }
    };
    // "public" maps to the admin DB/schema, which the GLOBAL runner already
    // operates on (fleet executor registered, reconciler populating the shared
    // Runtime). Reuse it rather than building a redundant per-tenant runner: a
    // separate "public" runner means two scheduler loops polling the same
    // public-schema rows and double-dispatching every task (deduped by the
    // executor claim since T-0639, but wasteful). Non-public tenants get their
    // own schema-scoped runner from the cache. CLOACI-T-0639 follow-up.
    let tenant_runner = if tenant_id == "public" {
        state.runner.clone()
    } else {
        match state
            .tenant_runners
            .get_or_create(&tenant_id, tenant_db)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to acquire tenant runner for '{}': {}", tenant_id, e);
                return ApiError::internal(format!("tenant runner unavailable: {}", e))
                    .into_response();
            }
        }
    };

    match tenant_runner.execute_async(&name, context).await {
        Ok(execution) => {
            info!(
                "Executed workflow '{}' for tenant '{}': {}",
                name, tenant_id, execution.execution_id
            );
            (
                StatusCode::ACCEPTED,
                Json(ExecuteResponse {
                    execution_id: execution.execution_id.to_string(),
                    workflow_name: name,
                    tenant_id,
                    status: "scheduled".to_string(),
                }),
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

/// Default page size for `list_executions` when the client doesn't
/// specify `?limit=`. Bounded so the response stays small enough for
/// CLI rendering.
const DEFAULT_EXECUTIONS_LIMIT: i64 = 100;
/// Hard ceiling on `?limit=` to keep a single response from pulling
/// the entire `workflow_executions` table.
const MAX_EXECUTIONS_LIMIT: i64 = 1000;

/// GET /tenants/:tenant_id/executions — list workflow executions.
///
/// **CLOACI-T-0594 / API-02:** accepts `?status=Failed` and
/// `?workflow_name=foo` and `?limit=N&offset=M`. Previously these
/// query params were silently discarded.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/executions",
    tag = "executions",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ListExecutionsQuery,
    ),
    responses(
        (status = 200, description = "Executions page", body = TenantListResponse<ExecutionSummary>),
        (status = 400, description = "Invalid pagination", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_executions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
    Query(q): Query<ListExecutionsQuery>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }

    let limit = q.limit.unwrap_or(DEFAULT_EXECUTIONS_LIMIT);
    if !(1..=MAX_EXECUTIONS_LIMIT).contains(&limit) {
        return ApiError::bad_request(
            "invalid_pagination",
            format!("limit must be 1..={}", MAX_EXECUTIONS_LIMIT),
        )
        .into_response();
    }
    let offset = q.offset.unwrap_or(0);
    if offset < 0 {
        return ApiError::bad_request("invalid_pagination", "offset must be >= 0".to_string())
            .into_response();
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

    let filter = cloacina::dal::unified::workflow_execution::ExecutionListFilter {
        status: q.status,
        workflow_name: q.workflow,
        limit,
        offset,
    };

    match dal.workflow_execution().list_filtered(filter).await {
        Ok(executions) => {
            let items: Vec<ExecutionSummary> = executions
                .into_iter()
                .map(|e| ExecutionSummary {
                    id: e.id.0.to_string(),
                    workflow_name: e.workflow_name,
                    status: e.status,
                    started_at: e.started_at.0.to_rfc3339(),
                    completed_at: e.completed_at.map(|t| t.0.to_rfc3339()),
                })
                .collect();
            // CLOACI-T-0594 / API-03: unified `{items, total}` envelope.
            // `total` is best-effort — equals the returned page size when
            // we don't run a separate COUNT (high-cardinality table).
            Json(TenantListResponse::new(tenant_id, items)).into_response()
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
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/executions/{exec_id}",
    tag = "executions",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("exec_id" = String, Path, description = "Execution UUID"),
    ),
    responses(
        (status = 200, description = "Execution detail", body = ExecutionDetail),
        (status = 400, description = "Invalid execution ID", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Execution not found", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
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
        Ok(execution) => {
            // Pass through the stored status string verbatim. Earlier code
            // had a per-variant match that returned the same value, which
            // was redundant. Trust the producer to write a valid status.
            Json(ExecutionDetail {
                tenant_id,
                execution_id: exec_id,
                status: execution.status.as_str().to_string(),
            })
            .into_response()
        }
        Err(e) => ApiError::not_found("execution_not_found", format!("{}", e)).into_response(),
    }
}

/// GET /tenants/:tenant_id/executions/:id/events — execution event log.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/executions/{exec_id}/events",
    tag = "executions",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("exec_id" = String, Path, description = "Execution UUID"),
    ),
    responses(
        (status = 200, description = "Execution event log", body = ExecutionEventsResponse),
        (status = 400, description = "Invalid execution ID", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
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

    match dal.execution_event().list_by_workflow(universal_id).await {
        Ok(events) => {
            let items: Vec<ExecutionEvent> = events
                .into_iter()
                .map(|e| ExecutionEvent {
                    id: e.id.0.to_string(),
                    event_type: e.event_type,
                    event_data: e.event_data,
                    created_at: e.created_at.0.to_rfc3339(),
                    sequence_num: e.sequence_num,
                })
                .collect();
            Json(ExecutionEventsResponse {
                tenant_id,
                execution_id: exec_id,
                events: items,
            })
            .into_response()
        }
        Err(e) => ApiError::internal(format!("{}", e)).into_response(),
    }
}

/// GET /tenants/:tenant_id/executions/:id/tasks — per-task rows for an execution.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/executions/{exec_id}/tasks",
    tag = "executions",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("exec_id" = String, Path, description = "Execution UUID"),
    ),
    responses(
        (status = 200, description = "Per-task execution rows", body = ExecutionTasksResponse),
        (status = 400, description = "Invalid execution ID", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_execution_tasks(
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

    match dal
        .task_execution()
        .get_all_tasks_for_workflow(universal_id)
        .await
    {
        Ok(tasks) => {
            let items: Vec<TaskExecutionDetail> = tasks
                .into_iter()
                .map(|t| TaskExecutionDetail {
                    id: t.id.0.to_string(),
                    task_name: t.task_name,
                    status: t.status,
                    started_at: t.started_at.map(|ts| ts.0.to_rfc3339()),
                    completed_at: t.completed_at.map(|ts| ts.0.to_rfc3339()),
                    attempt: t.attempt,
                    max_attempts: t.max_attempts,
                    created_at: t.created_at.0.to_rfc3339(),
                    updated_at: t.updated_at.0.to_rfc3339(),
                    sub_status: t.sub_status,
                    last_error: t.last_error,
                    error_details: t.error_details,
                })
                .collect();
            Json(ExecutionTasksResponse {
                tenant_id,
                execution_id: exec_id,
                tasks: items,
            })
            .into_response()
        }
        Err(e) => ApiError::internal(format!("{}", e)).into_response(),
    }
}
