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
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use tracing::{info, warn};

use cloacina::Context;
use cloacina::dal::UnifiedRegistryStorage;
use cloacina::executor::WorkflowExecutor;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use cloacina_api_types::{
    ExecuteRequest, ExecuteResponse, ExecutionDetail, ExecutionEvent, ExecutionEventsResponse,
    ExecutionSummary, ExecutionTasksResponse, ListExecutionsQuery, TaskExecutionDetail,
    TenantListResponse,
};

use crate::AppState;
use crate::routes::audit::{self as audit_record, AuditEvent};
use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;

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
        (status = 404, description = "Workflow not registered for this tenant (I-0138 cross-tenant gate; `public` is exempt)", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn execute_workflow(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Extension(grant): Extension<crate::routes::authz::Grant>,
    Path((tenant_id, name)): Path<(String, String)>,
    Json(body): Json<ExecuteRequest>,
) -> impl IntoResponse {
    let mut context = Context::new();

    // CLOACINA-T-1045: a priority outside the scale is a bad request. One
    // inside the scale is never refused: below, it is held to the highest
    // priority this principal may set.
    if let Some(asked) = body.priority
        && let Err(message) = cloacina_api_types::validate_priority(i64::from(asked))
    {
        return ApiError::bad_request("invalid_priority", message).into_response();
    }
    let run_priority = run_priority_for(body.priority, grant.limits);

    // CLOACI-T-0757: capture the provided context object for declared-param
    // validation before it's merged/consumed below.
    let provided_ctx: Option<serde_json::Map<String, serde_json::Value>> =
        body.context.as_ref().and_then(|v| v.as_object()).cloned();

    // CLOACINA-T-1020: a manual run sets the interval of the run in the same
    // keys as a scheduled run. They are checked here: two timestamps, the
    // end after the start.
    if let Some(ctx) = provided_ctx.as_ref()
        && let Err(message) = cloacina::workflow_instance::validate_interval_keys(ctx)
    {
        return ApiError::bad_request("invalid_interval", message).into_response();
    }

    // Merge provided context if any. Route through the SAME merge the
    // instance fire path uses (CLOACI-T-0859): a `{"$secret": "name"}` value
    // is routed AWAY from the plaintext context into the reserved alias map
    // (names only — the resolved value never enters the durable context), so
    // a direct `workflow run` can bind declared secrets exactly like a
    // scheduled instance does. Reserved scheduler keys are protected too.
    if let Some(ctx_value) = body.context {
        if ctx_value.as_object().is_some() {
            let params_json = ctx_value.to_string();
            if let Err(e) =
                cloacina::workflow_instance::merge_instance_params(&mut context, &params_json)
            {
                return ApiError::bad_request(
                    "workflow_input_invalid",
                    format!("invalid execution context: {}", e),
                )
                .into_response();
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
            return ApiError::internal_logged("tenant database unavailable", &e).into_response();
        }
    };
    // CLOACI-T-0749: refuse to start a new execution for a paused workflow.
    // Pause is a deliberate operator hold; in-flight runs are unaffected, only
    // new ones are blocked. Registry-init / lookup failures fail open (proceed)
    // so a transient registry error never wedges execution.
    {
        let storage = UnifiedRegistryStorage::new(tenant_db.clone());
        if let Ok(registry) = WorkflowRegistryImpl::new(storage, tenant_db.clone()) {
            // CLOACI-T-0901: a non-public tenant may only execute workflows in
            // its OWN registry. `execute_async` resolves the name against the
            // process-shared `Runtime` (populated by every tenant's reconciler),
            // so without this gate a tenant could run a workflow it never
            // installed — a cross-tenant isolation leak. A definitive "not
            // found" fails CLOSED (404); a registry ERROR fails open (proceed)
            // so a transient DB fault never wedges execution. `public` maps to
            // the admin/global catalog and is intentionally exempt (it may serve
            // inventory/global workflows not tracked as tenant packages).
            if tenant_id != "public" {
                match registry.workflow_exists(&name).await {
                    Ok(false) => {
                        return ApiError::new(
                            StatusCode::NOT_FOUND,
                            "workflow_not_found",
                            format!(
                                "workflow '{}' is not registered for tenant '{}'",
                                name, tenant_id
                            ),
                        )
                        .into_response();
                    }
                    Ok(true) => {}
                    Err(e) => {
                        warn!("existence-check failed for workflow '{}': {}", name, e)
                    }
                }
            }

            match registry.is_workflow_paused(&name).await {
                Ok(true) => {
                    return ApiError::new(
                        StatusCode::CONFLICT,
                        "workflow_paused",
                        format!("workflow '{}' is paused; resume it before executing", name),
                    )
                    .into_response();
                }
                Ok(false) => {}
                Err(e) => warn!("paused-check failed for workflow '{}': {}", name, e),
            }

            // CLOACI-T-0757: validate the provided context against the workflow's
            // declared params (I-0128). Undeclared workflows accept free-form
            // context (empty params → no validation). Registry errors fail open.
            match registry.get_workflow_declared_params(&name).await {
                Ok(params) if !params.is_empty() => {
                    let errors = validate_declared_params(&params, provided_ctx.as_ref());
                    if !errors.is_empty() {
                        return ApiError::bad_request(
                            "workflow_input_invalid",
                            format!("invalid execution context: {}", errors.join("; ")),
                        )
                        .into_response();
                    }
                }
                Ok(_) => {}
                Err(e) => warn!("declared-params lookup failed for '{}': {}", name, e),
            }
        }
    }

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
                return ApiError::internal_logged("tenant runner unavailable", &e).into_response();
            }
        }
    };

    match tenant_runner
        .execute_async_with_priority(&name, context, run_priority)
        .await
    {
        Ok((execution, priority)) => {
            info!(
                "Executed workflow '{}' for tenant '{}': {}",
                name, tenant_id, execution.execution_id
            );
            // CLOACINA-T-1045: a run that asked for more than its principal
            // may set is lowered, and that is said loudly: here in the log
            // and the audit record, and in the answer.
            let priority_notice = if body.priority.is_some() {
                priority.notice()
            } else {
                None
            };
            if let Some(notice) = &priority_notice {
                warn!(
                    tenant = %tenant_id,
                    workflow = %name,
                    key_id = %auth.key_id,
                    key_name = %auth.name,
                    priority_asked = priority.asked,
                    priority_given = priority.given,
                    "priority lowered: {notice}"
                );
            }
            let mut detail = serde_json::json!({
                "execution_id": execution.execution_id.to_string(),
                "priority_asked": priority.asked,
                "priority_given": priority.given,
            });
            if priority_notice.is_some() {
                detail["priority_lowered"] = serde_json::Value::Bool(true);
            }
            audit_record::note(
                Some(&tenant_id),
                AuditEvent::ok(audit_record::actions::WORKFLOW_RUN, name.as_str()).detail(detail),
            );
            // CLOACI-T-0776: mark this as a manual operator run so the UI can
            // distinguish it from cron/trigger/reactor-driven runs. Best-effort —
            // a tagging failure must not fail the (already-started) execution.
            if let Ok(db) = state
                .tenant_databases
                .resolve(&tenant_id, &state.database)
                .await
            {
                if let Err(e) = cloacina::dal::DAL::new(db)
                    .workflow_execution()
                    .set_trigger_origin(
                        cloacina::database::universal_types::UniversalUuid::from(
                            execution.execution_id,
                        ),
                        "manual",
                    )
                    .await
                {
                    warn!(
                        "failed to mark execution {} as manual: {}",
                        execution.execution_id, e
                    );
                }
            }
            (
                StatusCode::ACCEPTED,
                Json(ExecuteResponse {
                    execution_id: execution.execution_id.to_string(),
                    workflow_name: name,
                    tenant_id,
                    status: "scheduled".to_string(),
                    priority: Some(priority),
                    priority_notice,
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

/// The priority a run request asks for, with the limit of its principal
/// (CLOACINA-T-1045). A request that names no priority asks for nothing: the
/// workflow's declared priority applies. A principal whose role carries no
/// limit may set no priority: the lowest is the highest it gets.
pub(crate) fn run_priority_for(
    asked: Option<u8>,
    limits: crate::routes::authz::Limits,
) -> cloacina::execution_planner::RunPriority {
    match asked {
        Some(asked) => cloacina::execution_planner::RunPriority::asked_by(
            asked,
            Some(
                limits
                    .get(crate::routes::authz::LimitKind::MaxPriority)
                    .unwrap_or(cloacina_api_types::LOWEST_PRIORITY),
            ),
        ),
        None => cloacina::execution_planner::RunPriority::default(),
    }
}

/// The priority of a stored execution: asked and given (CLOACINA-T-1045).
/// `None` for a row from before priorities, which has no asked value.
fn priority_of(asked: Option<i32>, given: i32) -> Option<cloacina_api_types::PriorityGrant> {
    asked.map(|asked| cloacina_api_types::PriorityGrant {
        asked: cloacina_api_types::clamp_priority(asked),
        given: cloacina_api_types::clamp_priority(given),
    })
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
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
    Query(q): Query<ListExecutionsQuery>,
) -> impl IntoResponse {
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
            return ApiError::internal_logged("tenant database error", &e).into_response();
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
                    trigger_origin: e.trigger_origin,
                    priority: priority_of(e.priority_asked, e.priority),
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
            ApiError::internal_logged("list executions failed", &e).into_response()
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
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, exec_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let id = match uuid::Uuid::parse_str(&exec_id) {
        Ok(id) => id,
        Err(_) => {
            return ApiError::bad_request("invalid_request", "invalid execution ID")
                .into_response();
        }
    };

    let tenant_db = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal_logged("tenant database error", &e).into_response();
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
                priority: priority_of(execution.priority_asked, execution.priority),
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
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, exec_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let id = match uuid::Uuid::parse_str(&exec_id) {
        Ok(id) => id,
        Err(_) => {
            return ApiError::bad_request("invalid_request", "invalid execution ID")
                .into_response();
        }
    };

    let tenant_db = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal_logged("tenant database error", &e).into_response();
        }
    };
    let dal = cloacina::dal::DAL::new(tenant_db);
    let universal_id = cloacina::database::universal_types::UniversalUuid(id);

    match dal.execution_event().list_by_workflow(universal_id).await {
        Ok(events) => {
            // Resolve each event's `task_execution_id` to a task name so the log
            // can say *which* task an event is about (CLOACI-I-0124 / WS-9).
            // Workflow-scoped events (no task_execution_id) stay unnamed.
            let task_names: std::collections::HashMap<String, String> = dal
                .task_execution()
                .get_all_tasks_for_workflow(universal_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|t| (t.id.0.to_string(), t.task_name))
                .collect();

            let items: Vec<ExecutionEvent> = events
                .into_iter()
                .map(|e| ExecutionEvent {
                    id: e.id.0.to_string(),
                    event_type: e.event_type,
                    event_data: e.event_data,
                    task_name: e
                        .task_execution_id
                        .and_then(|tid| task_names.get(&tid.0.to_string()).cloned()),
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
        Err(e) => ApiError::internal_logged("get execution events failed", &e).into_response(),
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
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, exec_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let id = match uuid::Uuid::parse_str(&exec_id) {
        Ok(id) => id,
        Err(_) => {
            return ApiError::bad_request("invalid_request", "invalid execution ID")
                .into_response();
        }
    };

    let tenant_db = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal_logged("tenant database error", &e).into_response();
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
            // CLOACINA-T-1047: the size of the context each task returned.
            // The task rows are served without sizes if this read fails.
            let context_sizes = match dal
                .task_execution_metadata()
                .context_sizes_for_workflow(universal_id)
                .await
            {
                Ok(sizes) => sizes,
                Err(e) => {
                    tracing::warn!(
                        execution_id = %exec_id,
                        error = %e,
                        "could not read task context sizes"
                    );
                    Default::default()
                }
            };
            // CLOACINA-T-1100: the error of each failed attempt. The task
            // rows are served without them if this read fails.
            let mut attempt_errors = match dal
                .task_execution()
                .attempt_errors_for_workflow(universal_id)
                .await
            {
                Ok(errors) => errors,
                Err(e) => {
                    tracing::warn!(
                        execution_id = %exec_id,
                        error = %e,
                        "could not read the errors of failed task attempts"
                    );
                    Default::default()
                }
            };
            // CLOACINA-T-1024: the link to the task's logs, when the server
            // has a log URL template.
            let log_urls = crate::task_logs::url_template();
            let items: Vec<TaskExecutionDetail> = tasks
                .into_iter()
                .map(|t| TaskExecutionDetail {
                    attempt_errors: attempt_errors
                        .remove(&t.id)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|e| cloacina_api_types::TaskAttemptErrorDetail {
                            attempt: e.attempt,
                            message: e.message,
                            details: e.details,
                            failed_at: e.failed_at.0.to_rfc3339(),
                        })
                        .collect(),
                    context_size_bytes: context_sizes.get(&t.id).copied(),
                    id: t.id.0.to_string(),
                    log_url: log_urls.map(|template| {
                        template.render(&tenant_id, &id.to_string(), &t.task_name, t.attempt)
                    }),
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
        Err(e) => ApiError::internal_logged("get execution tasks failed", &e).into_response(),
    }
}

/// Validate a provided execution context against a workflow's declared input
/// params (CLOACI-T-0757 / I-0128). v1 checks required-presence and a top-level
/// JSON-Schema `type` match; returns human-readable error strings (empty =
/// valid). Full nested JSON-Schema validation is a follow-up — when a slot's
/// schema has no simple top-level `type` (enums/oneOf/etc.) the value is
/// accepted rather than rejected.
pub(crate) fn validate_declared_params(
    params: &[cloacina_api_types::InputSlot],
    provided: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Vec<String> {
    let mut errors = Vec::new();
    for p in params {
        match provided.and_then(|m| m.get(&p.name)) {
            None => {
                if p.encrypted {
                    // CLOACINA-T-1035: a declared secret with no reference.
                    // Say which secret and show the reference to add.
                    errors.push(cloacina::workflow_instance::missing_secret_ref_message(
                        &p.name,
                    ));
                } else if p.required && p.default.is_none() {
                    errors.push(format!("missing required param '{}'", p.name));
                }
            }
            Some(v) => {
                // CLOACI-T-0859: an encrypted slot (declared via
                // `secrets(...)`) is bound with a `{"$secret": "name"}`
                // reference — never a literal value (which would land the
                // plaintext in the durable context).
                if p.encrypted {
                    let is_secret_ref = v
                        .as_object()
                        .map(|o| o.len() == 1 && o.get("$secret").is_some_and(|s| s.is_string()))
                        .unwrap_or(false);
                    if !is_secret_ref {
                        errors.push(format!(
                            "secret '{}' must be bound via {{\"$secret\": \"name\"}}, \
                             not a literal value",
                            p.name
                        ));
                    }
                    continue;
                }
                if let Some(expected) = p.schema.get("type").and_then(|t| t.as_str()) {
                    if !json_value_matches_type(v, expected) {
                        errors.push(format!(
                            "param '{}' expects type '{}', got {}",
                            p.name,
                            expected,
                            json_type_name(v)
                        ));
                    }
                }
            }
        }
    }
    errors
}

/// Validate a single JSON value against one slot's schema (CLOACI-I-0128 T-0759).
/// Used for surfaces that accept a single value (accumulator inject) rather than
/// a named map. Returns an error message, or `None` when valid / untyped.
pub(crate) fn validate_value_against_schema(
    value: &serde_json::Value,
    schema: &serde_json::Value,
) -> Option<String> {
    if let Some(expected) = schema.get("type").and_then(|t| t.as_str()) {
        if !json_value_matches_type(value, expected) {
            return Some(format!(
                "expects type '{}', got {}",
                expected,
                json_type_name(value)
            ));
        }
    }
    None
}

pub(crate) fn json_value_matches_type(v: &serde_json::Value, expected: &str) -> bool {
    match expected {
        "string" => v.is_string(),
        "integer" => v.is_i64() || v.is_u64(),
        "number" => v.is_number(),
        "boolean" => v.is_boolean(),
        "array" => v.is_array(),
        "object" => v.is_object(),
        "null" => v.is_null(),
        // Unknown/compound schema type — don't reject (v1 limitation).
        _ => true,
    }
}

pub(crate) fn json_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod input_validation_tests {
    use super::*;
    use cloacina::input_interface::schema_for;
    use cloacina_api_types::InputSlot;

    fn obj(pairs: &[(&str, serde_json::Value)]) -> serde_json::Map<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    // ----- CLOACINA-T-1045: the priority of a run request -----

    use crate::routes::authz::Role;
    use cloacina_api_types::PriorityGrant;

    /// What a principal of `role` gets when its run asks for `asked`.
    fn granted(role: Role, asked: u8) -> PriorityGrant {
        let run = run_priority_for(Some(asked), role.default_limits());
        match run.limit {
            Some(limit) => PriorityGrant::held_to(run.asked.unwrap(), Some(limit)),
            None => PriorityGrant::unlimited(run.asked.unwrap()),
        }
    }

    #[test]
    fn a_write_key_that_asks_for_more_than_5_gets_5_and_a_notice() {
        let grant = granted(Role::Write, 2);
        assert_eq!((grant.asked, grant.given), (2, 5));
        let notice = grant.notice().expect("a notice");
        assert!(notice.contains("priority 2 was asked"), "{notice}");
        assert!(notice.contains("priority 5 was given"), "{notice}");
        // Inside its limit it gets what it asks for, with no notice.
        let grant = granted(Role::Write, 8);
        assert_eq!((grant.asked, grant.given), (8, 8));
        assert!(grant.notice().is_none());
    }

    #[test]
    fn an_admin_key_gets_the_priority_it_asks_for() {
        let grant = granted(Role::Admin, 0);
        assert_eq!((grant.asked, grant.given), (0, 0));
        assert!(grant.notice().is_none());
    }

    #[test]
    fn a_role_with_no_priority_limit_gets_the_lowest_priority() {
        for role in [Role::Read, Role::Agent] {
            let grant = granted(role, 5);
            assert_eq!(grant.given, 10, "{role:?}");
            assert!(grant.notice().is_some());
        }
    }

    #[test]
    fn a_request_with_no_priority_asks_for_nothing() {
        let run = run_priority_for(None, Role::Write.default_limits());
        assert_eq!(run, cloacina::execution_planner::RunPriority::default());
    }

    #[test]
    fn a_stored_execution_shows_asked_and_given() {
        assert_eq!(
            priority_of(Some(2), 5),
            Some(PriorityGrant { asked: 2, given: 5 })
        );
        // A row from before priorities has no asked value.
        assert_eq!(priority_of(None, 5), None);
    }

    #[test]
    fn missing_required_param_errors() {
        let params = vec![InputSlot::required("order_id", schema_for::<String>())];
        let errors = validate_declared_params(&params, Some(&obj(&[])));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("order_id"));
    }

    /// CLOACINA-T-1035: a declared secret with no reference in the context.
    /// The error names the secret and shows the form of the reference.
    #[test]
    fn unbound_declared_secret_names_the_secret_and_shows_the_reference() {
        let params = vec![InputSlot::secret("report_key")];
        for provided in [None, Some(obj(&[]))] {
            let errors = validate_declared_params(&params, provided.as_ref());
            assert_eq!(errors.len(), 1, "{errors:?}");
            assert!(errors[0].contains("secret 'report_key'"), "{errors:?}");
            assert!(
                errors[0].contains(r#"{"report_key": {"$secret": "<name of a stored secret>"}}"#),
                "{errors:?}"
            );
            assert!(!errors[0].contains("missing required param"), "{errors:?}");
        }
    }

    #[test]
    fn optional_with_default_omitted_is_ok() {
        let params = vec![InputSlot::optional(
            "limit",
            schema_for::<u32>(),
            Some(serde_json::json!(100)),
        )];
        assert!(validate_declared_params(&params, Some(&obj(&[]))).is_empty());
    }

    #[test]
    fn wrong_type_errors() {
        let params = vec![InputSlot::required("order_id", schema_for::<String>())];
        let provided = obj(&[("order_id", serde_json::json!(42))]);
        let errors = validate_declared_params(&params, Some(&provided));
        assert_eq!(errors.len(), 1, "{:?}", errors);
        assert!(errors[0].contains("string"));
    }

    #[test]
    fn correct_input_passes() {
        let params = vec![
            InputSlot::required("order_id", schema_for::<String>()),
            InputSlot::optional("limit", schema_for::<u32>(), Some(serde_json::json!(100))),
        ];
        let provided = obj(&[
            ("order_id", serde_json::json!("A-1")),
            ("limit", serde_json::json!(5)),
        ]);
        assert!(validate_declared_params(&params, Some(&provided)).is_empty());
    }

    #[test]
    fn undeclared_workflow_skips_validation() {
        assert!(validate_declared_params(&[], Some(&obj(&[]))).is_empty());
    }

    #[test]
    fn value_against_schema_typed_ok_and_mismatch() {
        // CLOACI-T-0759: single-value validator (accumulator inject path).
        let string_schema = schema_for::<String>();
        assert!(validate_value_against_schema(&serde_json::json!("hi"), &string_schema).is_none());
        let err = validate_value_against_schema(&serde_json::json!(42), &string_schema);
        assert!(err.is_some());
        assert!(err.unwrap().contains("string"));
    }

    #[test]
    fn value_against_permissive_schema_accepts_anything() {
        // Untyped boundary → permissive {} schema → accept any event.
        let any = serde_json::json!({});
        assert!(validate_value_against_schema(&serde_json::json!(42), &any).is_none());
        assert!(validate_value_against_schema(&serde_json::json!("x"), &any).is_none());
    }
}
