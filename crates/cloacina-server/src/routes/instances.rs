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

//! Named workflow-instance API (CLOACI-T-0894).
//!
//! I-0116 shipped named, param-bound, scheduled workflow instances, but
//! registration existed only on the embedded runner
//! (`DefaultRunner::register_cron_workflow_instance`). On the server — the
//! deployment mode the docs lead with — the feature's headline capability was
//! unreachable: users could bind params per RUN via `workflow run --context`
//! but could not create a persistent named instance at all.
//!
//! An instance is stored as a `schedules` row carrying `instance_name` and the
//! resolved `params` JSON. The fire-time merge already delivers those params as
//! top-level context keys, so nothing in the execution path needed changing.
//!
//! Tenant scoping follows the same rule as every other route here: all reads
//! and writes go through the tenant-scoped `Database` from
//! `TenantDatabaseCache`, so an instance in tenant A is simply not present in
//! tenant B's schema — a cross-tenant request 404s naturally rather than
//! leaking existence through a distinct error code.

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Extension, Json,
};
use tracing::warn;

use cloacina_api_types::{
    CreateInstanceRequest, DeleteInstanceResponse, ListInstancesQuery, TenantListResponse,
    WorkflowInstanceSummary,
};

use cloacina::dal::UnifiedRegistryStorage;
use cloacina::database::universal_types::UniversalTimestamp;
use cloacina::models::schedule::{NewSchedule, Schedule};
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use cloacina::CronEvaluator;

use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::routes::executions::validate_declared_params;
use crate::AppState;

const DEFAULT_INSTANCES_LIMIT: i64 = 100;
const MAX_INSTANCES_LIMIT: i64 = 1000;

/// Map a stored `schedules` row onto the wire type.
///
/// `params` is stored as a JSON *string*; a row whose params fail to parse is
/// surfaced as `None` rather than failing the whole listing — the binding is
/// still visible to an operator who needs to delete it.
fn to_summary(s: Schedule) -> WorkflowInstanceSummary {
    WorkflowInstanceSummary {
        id: s.id.0.to_string(),
        workflow_name: s.workflow_name,
        instance_name: s.instance_name.unwrap_or_default(),
        params: s
            .params
            .as_deref()
            .and_then(|p| serde_json::from_str::<serde_json::Value>(p).ok()),
        cron_expression: s.cron_expression,
        timezone: s.timezone,
        enabled: s.enabled.is_true(),
        paused: s.paused.is_true(),
        next_run_at: s.next_run_at.map(|t| t.0.to_rfc3339()),
        last_run_at: s.last_run_at.map(|t| t.0.to_rfc3339()),
        created_at: s.created_at.0.to_rfc3339(),
    }
}

/// POST /tenants/:tenant_id/workflows/:name/instances — create a named instance.
///
/// Params are validated against the workflow's declared `params(...)` slots
/// using the same `validate_declared_params` the execute route uses, so a
/// scheduled instance cannot be created with a binding that would fail at every
/// fire — the failure surfaces at creation time instead of silently at 3am.
///
/// `cron` is optional. Without it the instance is created **unscheduled**: a
/// durable named param binding with `next_run_at = NULL`, which the scheduler's
/// due-query can never select (`NULL <= now` is never true).
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/instances",
    tag = "instances",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Workflow name"),
    ),
    request_body = CreateInstanceRequest,
    responses(
        (status = 200, description = "Instance created", body = WorkflowInstanceSummary),
        (status = 400, description = "Invalid params, cron expression, or timezone", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 409, description = "Instance name already exists for this workflow", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn create_instance(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
    Json(body): Json<CreateInstanceRequest>,
) -> impl IntoResponse {
    if body.instance_name.trim().is_empty() {
        return ApiError::bad_request(
            "invalid_instance_name",
            "instance_name must not be empty".to_string(),
        )
        .into_response();
    }

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
    let dal = cloacina::dal::DAL::new(tenant_db.clone());

    // Reject a duplicate up front. The read-then-write is not atomic, so a
    // concurrent create of the same name can still slip past here; that is
    // caught below when the insert violates the unique index.
    match dal
        .schedule()
        .find_by_instance_name(&name, &body.instance_name)
        .await
    {
        Ok(Some(_)) => {
            return ApiError::conflict(
                "instance_exists",
                format!(
                    "instance '{}' already exists for workflow '{}'",
                    body.instance_name, name
                ),
            )
            .into_response()
        }
        Ok(None) => {}
        Err(e) => return ApiError::internal(format!("{}", e)).into_response(),
    }

    // Validate the bound params against the workflow's declared slots — the
    // same check the execute route performs on a per-run context. A registry
    // lookup failure fails OPEN (matching execute_workflow) so an instance can
    // still be created for a workflow whose metadata is unavailable.
    let provided = body.params.as_ref().and_then(|v| v.as_object());
    {
        let storage = UnifiedRegistryStorage::new(tenant_db.clone());
        if let Ok(registry) = WorkflowRegistryImpl::new(storage, tenant_db.clone()) {
            match registry.get_workflow_declared_params(&name).await {
                Ok(slots) if !slots.is_empty() => {
                    let errors = validate_declared_params(&slots, provided);
                    if !errors.is_empty() {
                        return ApiError::bad_request(
                            "instance_params_invalid",
                            format!("invalid instance params: {}", errors.join("; ")),
                        )
                        .into_response();
                    }
                }
                Ok(_) => {}
                Err(e) => warn!("declared-params lookup failed for '{}': {}", name, e),
            }
        }
    }

    let timezone = body.timezone.clone().unwrap_or_else(|| "UTC".to_string());

    // Build the row. A cron expression is validated and its first fire computed
    // now, so a bad expression is a 400 rather than a schedule that silently
    // never runs.
    let mut new_schedule = match body.cron.as_deref() {
        Some(cron) => {
            if let Err(e) = CronEvaluator::validate(cron, &timezone) {
                return ApiError::bad_request(
                    "invalid_cron",
                    format!("invalid cron expression or timezone: {}", e),
                )
                .into_response();
            }
            let evaluator = match CronEvaluator::new(cron, &timezone) {
                Ok(ev) => ev,
                Err(e) => {
                    return ApiError::bad_request(
                        "invalid_cron",
                        format!("invalid cron expression or timezone: {}", e),
                    )
                    .into_response()
                }
            };
            let next_run = match evaluator.next_execution(chrono::Utc::now()) {
                Ok(t) => t,
                Err(e) => {
                    return ApiError::bad_request(
                        "invalid_cron",
                        format!("could not compute next execution: {}", e),
                    )
                    .into_response()
                }
            };
            let mut s = NewSchedule::cron(&name, cron, UniversalTimestamp(next_run));
            s.timezone = Some(timezone.clone());
            s
        }
        None => {
            // Unscheduled: a durable named binding. `next_run_at` stays NULL so
            // the due-query can never select it.
            let mut s = NewSchedule::cron(&name, "", UniversalTimestamp(chrono::Utc::now()));
            s.cron_expression = None;
            s.next_run_at = None;
            s.timezone = Some(timezone.clone());
            s
        }
    };

    new_schedule.instance_name = Some(body.instance_name.clone());
    new_schedule.params = match &body.params {
        Some(v) => match serde_json::to_string(v) {
            Ok(s) => Some(s),
            Err(e) => {
                return ApiError::bad_request(
                    "instance_params_invalid",
                    format!("params are not serializable: {}", e),
                )
                .into_response()
            }
        },
        None => None,
    };
    if let Some(enabled) = body.enabled {
        new_schedule.enabled = Some(cloacina::database::universal_types::UniversalBool::new(
            enabled,
        ));
    }

    match dal.schedule().create(new_schedule).await {
        Ok(schedule) => {
            tracing::info!(
                "Created workflow instance '{}' of '{}' for tenant '{}'",
                body.instance_name,
                name,
                tenant_id
            );
            Json(to_summary(schedule)).into_response()
        }
        Err(e) => {
            // The unique index on (workflow_name, instance_name) is the real
            // arbiter when two creates race the pre-check above.
            let msg = format!("{}", e);
            if msg.contains("duplicate") || msg.contains("UNIQUE") || msg.contains("unique") {
                return ApiError::conflict(
                    "instance_exists",
                    format!(
                        "instance '{}' already exists for workflow '{}'",
                        body.instance_name, name
                    ),
                )
                .into_response();
            }
            warn!(
                "Failed to create instance '{}' of '{}' for tenant '{}': {}",
                body.instance_name, name, tenant_id, e
            );
            ApiError::internal(msg).into_response()
        }
    }
}

/// GET /tenants/:tenant_id/workflows/:name/instances — list a workflow's named
/// instances.
///
/// Anonymous schedules (cron rows with no `instance_name`) are excluded — this
/// endpoint is about named bindings, and the trigger endpoints already cover
/// the general schedule listing.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/instances",
    tag = "instances",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Workflow name"),
        ListInstancesQuery,
    ),
    responses(
        (status = 200, description = "Instances for this workflow", body = TenantListResponse<WorkflowInstanceSummary>),
        (status = 400, description = "Invalid pagination", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_instances(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
    Query(q): Query<ListInstancesQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(DEFAULT_INSTANCES_LIMIT);
    if !(1..=MAX_INSTANCES_LIMIT).contains(&limit) {
        return ApiError::bad_request(
            "invalid_pagination",
            format!("limit must be 1..={}", MAX_INSTANCES_LIMIT),
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
            return ApiError::internal(format!("tenant database unavailable: {}", e))
                .into_response()
        }
    };
    let dal = cloacina::dal::DAL::new(tenant_db);

    match dal.schedule().find_by_workflow(&name).await {
        Ok(schedules) => {
            let items: Vec<WorkflowInstanceSummary> = schedules
                .into_iter()
                .filter(|s| s.instance_name.is_some())
                .skip(offset as usize)
                .take(limit as usize)
                .map(to_summary)
                .collect();
            Json(TenantListResponse::new(tenant_id, items)).into_response()
        }
        Err(e) => {
            warn!(
                "Failed to list instances of '{}' for tenant '{}': {}",
                name, tenant_id, e
            );
            ApiError::internal(format!("{}", e)).into_response()
        }
    }
}

/// GET /tenants/:tenant_id/workflows/:name/instances/:instance — one instance.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/instances/{instance}",
    tag = "instances",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Workflow name"),
        ("instance" = String, Path, description = "Instance name"),
    ),
    responses(
        (status = 200, description = "Instance detail", body = WorkflowInstanceSummary),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Instance not found", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_instance(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name, instance)): Path<(String, String, String)>,
) -> impl IntoResponse {
    let tenant_db = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal(format!("tenant database unavailable: {}", e))
                .into_response()
        }
    };
    let dal = cloacina::dal::DAL::new(tenant_db);

    match dal.schedule().find_by_instance_name(&name, &instance).await {
        Ok(Some(s)) => Json(to_summary(s)).into_response(),
        Ok(None) => ApiError::not_found(
            "instance_not_found",
            format!("instance '{}' of workflow '{}' not found", instance, name),
        )
        .into_response(),
        Err(e) => ApiError::internal(format!("{}", e)).into_response(),
    }
}

/// DELETE /tenants/:tenant_id/workflows/:name/instances/:instance — remove a
/// named instance.
///
/// Deletes the binding and its schedule. In-flight executions already started
/// by this instance are unaffected; only future fires are prevented.
#[utoipa::path(
    delete,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/instances/{instance}",
    tag = "instances",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Workflow name"),
        ("instance" = String, Path, description = "Instance name"),
    ),
    responses(
        (status = 200, description = "Instance deleted", body = DeleteInstanceResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Instance not found", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn delete_instance(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name, instance)): Path<(String, String, String)>,
) -> impl IntoResponse {
    let tenant_db = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal(format!("tenant database unavailable: {}", e))
                .into_response()
        }
    };
    let dal = cloacina::dal::DAL::new(tenant_db);

    let schedule = match dal.schedule().find_by_instance_name(&name, &instance).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return ApiError::not_found(
                "instance_not_found",
                format!("instance '{}' of workflow '{}' not found", instance, name),
            )
            .into_response()
        }
        Err(e) => return ApiError::internal(format!("{}", e)).into_response(),
    };

    match dal.schedule().delete(schedule.id).await {
        Ok(()) => {
            tracing::info!(
                "Deleted workflow instance '{}' of '{}' for tenant '{}'",
                instance,
                name,
                tenant_id
            );
            Json(DeleteInstanceResponse {
                tenant_id,
                workflow_name: name,
                instance_name: instance,
                deleted: true,
            })
            .into_response()
        }
        Err(e) => {
            warn!(
                "Failed to delete instance '{}' of '{}' for tenant '{}': {}",
                instance, name, tenant_id, e
            );
            ApiError::internal(format!("{}", e)).into_response()
        }
    }
}
