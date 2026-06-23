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

//! Trigger schedule API — read-only listing of cron and trigger schedules.

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};
use tracing::warn;

use axum::extract::Query;

use cloacina_api_types::{
    DeclaredSurface, FireTriggerRequest, FireTriggerResponse, FiredExecution, InputSlot,
    ListTriggersQuery, TenantListResponse, TriggerDetailResponse, TriggerExecution,
    TriggerPauseResponse, TriggerScheduleInfo, TriggerScheduleSummary,
};

use cloacina::dal::UnifiedRegistryStorage;
use cloacina::executor::WorkflowExecutor;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;

use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::routes::executions::validate_declared_params;
use crate::AppState;

const DEFAULT_TRIGGERS_LIMIT: i64 = 100;
const MAX_TRIGGERS_LIMIT: i64 = 1000;

/// GET /tenants/:tenant_id/triggers — list all schedules (cron + trigger).
///
/// CLOACI-T-0579: routed through the tenant-scoped `Database` from
/// `TenantDatabaseCache` so the underlying `SELECT FROM schedules`
/// hits the tenant's schema, not the admin schema. Closes SEC-02.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/triggers",
    tag = "triggers",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ListTriggersQuery,
    ),
    responses(
        (status = 200, description = "Schedules page (cron + trigger)", body = TenantListResponse<TriggerScheduleSummary>),
        (status = 400, description = "Invalid pagination", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_triggers(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
    Query(q): Query<ListTriggersQuery>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }

    // CLOACI-T-0596 / API-10: client-bounded pagination. Defaults match
    // the historical hardcoded `LIMIT 100`; explicit caps prevent a
    // pathological `?limit=1000000` from pulling the entire table.
    let limit = q.limit.unwrap_or(DEFAULT_TRIGGERS_LIMIT);
    if !(1..=MAX_TRIGGERS_LIMIT).contains(&limit) {
        return ApiError::bad_request(
            "invalid_pagination",
            format!("limit must be 1..={}", MAX_TRIGGERS_LIMIT),
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
            warn!(
                "Failed to resolve tenant database for '{}': {}",
                tenant_id, e
            );
            return ApiError::internal(format!("tenant database unavailable: {}", e))
                .into_response();
        }
    };
    let dal = cloacina::dal::DAL::new(tenant_db);

    match dal.schedule().list(None, false, limit, offset).await {
        Ok(schedules) => {
            let items: Vec<TriggerScheduleSummary> = schedules
                .into_iter()
                .map(|s| TriggerScheduleSummary {
                    id: s.id.0.to_string(),
                    schedule_type: s.schedule_type,
                    workflow_name: s.workflow_name,
                    enabled: s.enabled.is_true(),
                    cron_expression: s.cron_expression,
                    trigger_name: s.trigger_name,
                    poll_interval_ms: s.poll_interval_ms.map(i64::from),
                    next_run_at: s.next_run_at.map(|t| t.0.to_rfc3339()),
                    last_run_at: s.last_run_at.map(|t| t.0.to_rfc3339()),
                    created_at: s.created_at.0.to_rfc3339(),
                    paused: s.paused.is_true(),
                    paused_at: s.paused_at.map(|t| t.0.to_rfc3339()),
                })
                .collect();
            // CLOACI-T-0594 / API-03: unified `{items, total}` envelope.
            // tenant_id retained at the top level for backward compatibility
            // with operator dashboards that key off it.
            Json(TenantListResponse::new(tenant_id, items)).into_response()
        }
        Err(e) => {
            warn!("Failed to list triggers for tenant '{}': {}", tenant_id, e);
            ApiError::internal(format!("{}", e)).into_response()
        }
    }
}

/// GET /tenants/:tenant_id/triggers/:name — trigger details + recent executions.
///
/// CLOACI-T-0579: routed through the tenant-scoped `Database`. A request
/// for tenant B with a trigger id that belongs to tenant A naturally
/// 404s — the row simply doesn't exist in tenant B's schema. No
/// info-disclosure via "not in your tenant" error code. Closes SEC-02.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/triggers/{name}",
    tag = "triggers",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Trigger or workflow name"),
    ),
    responses(
        (status = 200, description = "Trigger detail + recent executions", body = TriggerDetailResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Trigger not found", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_trigger(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
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
            warn!(
                "Failed to resolve tenant database for '{}': {}",
                tenant_id, e
            );
            return ApiError::internal(format!("tenant database unavailable: {}", e))
                .into_response();
        }
    };
    let dal = cloacina::dal::DAL::new(tenant_db);

    // Find schedule by workflow name or trigger name
    let schedules = match dal.schedule().list(None, false, 100, 0).await {
        Ok(s) => s,
        Err(e) => return ApiError::internal(format!("{}", e)).into_response(),
    };

    let found = schedules
        .into_iter()
        .find(|s| s.trigger_name.as_deref() == Some(&name) || s.workflow_name == name);

    match found {
        Some(schedule) => {
            // Get recent executions for this schedule
            let executions = dal
                .schedule_execution()
                .list_by_schedule(schedule.id, 10, 0)
                .await
                .unwrap_or_default();

            let exec_items: Vec<TriggerExecution> = executions
                .into_iter()
                .map(|e| TriggerExecution {
                    id: e.id.0.to_string(),
                    scheduled_time: e.scheduled_time.map(|t| t.0.to_rfc3339()),
                    started_at: e.started_at.0.to_rfc3339(),
                    completed_at: e.completed_at.map(|t| t.0.to_rfc3339()),
                })
                .collect();

            Json(TriggerDetailResponse {
                tenant_id,
                schedule: TriggerScheduleInfo {
                    id: schedule.id.0.to_string(),
                    schedule_type: schedule.schedule_type,
                    workflow_name: schedule.workflow_name,
                    enabled: schedule.enabled.is_true(),
                    cron_expression: schedule.cron_expression,
                    trigger_name: schedule.trigger_name,
                    poll_interval_ms: schedule.poll_interval_ms.map(i64::from),
                    paused: schedule.paused.is_true(),
                    paused_at: schedule.paused_at.map(|t| t.0.to_rfc3339()),
                },
                recent_executions: exec_items,
            })
            .into_response()
        }
        None => ApiError::not_found("trigger_not_found", format!("trigger '{}' not found", name))
            .into_response(),
    }
}

/// POST /tenants/:tenant_id/triggers/:name/pause — pause a schedule (CLOACI-T-0749).
///
/// Resolves the schedule by trigger name or workflow name (same as
/// `get_trigger`) and sets it paused so the scheduler stops firing it. Works
/// for both `trigger` and `cron` schedules. In-flight executions are
/// unaffected; this only gates new ones.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/triggers/{name}/pause",
    tag = "triggers",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Trigger or workflow name"),
    ),
    responses(
        (status = 200, description = "Schedule paused", body = TriggerPauseResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or role denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Trigger not found", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn pause_trigger(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    set_trigger_paused(state, auth, tenant_id, name, true).await
}

/// POST /tenants/:tenant_id/triggers/:name/resume — resume a paused schedule
/// (CLOACI-T-0749). Re-arms on the normal schedule; missed fires are not
/// caught up (skip policy).
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/triggers/{name}/resume",
    tag = "triggers",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Trigger or workflow name"),
    ),
    responses(
        (status = 200, description = "Schedule resumed", body = TriggerPauseResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or role denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Trigger not found", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn resume_trigger(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    set_trigger_paused(state, auth, tenant_id, name, false).await
}

/// Shared pause/resume implementation for trigger and cron schedules.
async fn set_trigger_paused(
    state: AppState,
    auth: AuthenticatedKey,
    tenant_id: String,
    name: String,
    pause: bool,
) -> axum::response::Response {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }
    if !auth.can_write() {
        return AuthenticatedKey::insufficient_role_response().into_response();
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
    let dal = cloacina::dal::DAL::new(tenant_db);

    // Resolve schedule by trigger name or workflow name (mirrors get_trigger).
    let schedules = match dal.schedule().list(None, false, 1000, 0).await {
        Ok(s) => s,
        Err(e) => return ApiError::internal(format!("{}", e)).into_response(),
    };
    let found = schedules
        .into_iter()
        .find(|s| s.trigger_name.as_deref() == Some(&name) || s.workflow_name == name);

    let schedule = match found {
        Some(s) => s,
        None => {
            return ApiError::not_found(
                "trigger_not_found",
                format!("trigger '{}' not found", name),
            )
            .into_response()
        }
    };

    let result = if pause {
        dal.schedule().pause(schedule.id).await
    } else {
        dal.schedule().resume(schedule.id).await
    };

    match result {
        Ok(()) => Json(TriggerPauseResponse {
            tenant_id,
            id: schedule.id.0.to_string(),
            name,
            status: if pause { "paused" } else { "resumed" }.to_string(),
            paused: pause,
        })
        .into_response(),
        Err(e) => {
            warn!(
                "Failed to {} trigger '{}' for tenant '{}': {}",
                if pause { "pause" } else { "resume" },
                name,
                tenant_id,
                e
            );
            ApiError::internal(format!("{}", e)).into_response()
        }
    }
}

/// POST /tenants/:tenant_id/triggers/:name/fire — manually fire a trigger,
/// fanning out to every subscribed workflow (CLOACI-T-0777). One operator action
/// instead of running each workflow by hand. An optional `event` is merged into
/// each fired workflow's context (alongside trigger metadata). The started
/// executions are marked `manual` (CLOACI-T-0776).
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/triggers/{name}/fire",
    tag = "triggers",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Trigger name"),
    ),
    request_body = FireTriggerRequest,
    responses(
        (status = 200, description = "Trigger fired; fan-out result", body = FireTriggerResponse),
        (status = 404, description = "No enabled subscribers for this trigger", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn fire_trigger(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
    Json(body): Json<FireTriggerRequest>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }
    if !auth.can_write() {
        return AuthenticatedKey::insufficient_role_response().into_response();
    }

    let tenant_db = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal(format!("tenant database unavailable: {}", e)).into_response()
        }
    };

    // Fan-out set (CLOACI-T-0777): every workflow subscribed to this trigger via
    // `#[workflow(triggers = […])]`. The schedules table carries only the
    // trigger's primary `on` workflow, so subscriptions — which may live in other
    // packages — are resolved from the registry's workflow metadata.
    let registry = {
        let storage = UnifiedRegistryStorage::new(tenant_db.clone());
        WorkflowRegistryImpl::new(storage, tenant_db.clone()).ok()
    };
    let subscribers: Vec<String> = match &registry {
        Some(r) => r.find_trigger_subscribers(&name).await.unwrap_or_default(),
        None => Vec::new(),
    };
    if subscribers.is_empty() {
        return ApiError::not_found(
            "trigger_not_found",
            format!("trigger '{}' has no subscribed workflows", name),
        )
        .into_response();
    }

    // CLOACI-T-0777 P2: validate the pushed event against the trigger's effective
    // pass-through schema — the union of its subscribers' declared params. An
    // untyped trigger (no subscriber declares params) accepts a free-form event.
    if body.event.is_some() {
        if let Some(r) = &registry {
            let slots = trigger_declared_slots(r, &subscribers).await;
            if !slots.is_empty() {
                let errors =
                    validate_declared_params(&slots, body.event.as_ref().and_then(|v| v.as_object()));
                if !errors.is_empty() {
                    return ApiError::bad_request(
                        "trigger_event_invalid",
                        format!("invalid trigger event: {}", errors.join("; ")),
                    )
                    .into_response();
                }
            }
        }
    }

    // Resolve the runner (public reuses the global runner; see execute_workflow).
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
                return ApiError::internal(format!("tenant runner unavailable: {}", e))
                    .into_response()
            }
        }
    };

    let triggered_at = chrono::Utc::now().to_rfc3339();
    let mut executions: Vec<FiredExecution> = Vec::with_capacity(subscribers.len());
    for workflow_name in &subscribers {
        let mut context = cloacina::Context::new();
        let _ = context.insert("trigger_name".to_string(), serde_json::json!(name));
        let _ = context.insert("triggered_at".to_string(), serde_json::json!(triggered_at));
        let _ = context.insert("manual_fire".to_string(), serde_json::json!(true));
        if let Some(serde_json::Value::Object(obj)) = &body.event {
            for (k, v) in obj {
                let _ = context.insert(k.clone(), v.clone());
            }
        }
        match tenant_runner.execute_async(workflow_name, context).await {
            Ok(execution) => {
                // Mark the run as a manual operator intervention (CLOACI-T-0776).
                if let Ok(db) = state
                    .tenant_databases
                    .resolve(&tenant_id, &state.database)
                    .await
                {
                    let _ = cloacina::dal::DAL::new(db)
                        .workflow_execution()
                        .set_trigger_origin(
                            cloacina::database::universal_types::UniversalUuid::from(
                                execution.execution_id,
                            ),
                            "manual",
                        )
                        .await;
                }
                executions.push(FiredExecution {
                    workflow_name: workflow_name.clone(),
                    execution_id: execution.execution_id.to_string(),
                });
            }
            Err(e) => {
                warn!(
                    "trigger '{}' fan-out: failed to fire '{}': {}",
                    name, workflow_name, e
                );
            }
        }
    }

    Json(FireTriggerResponse {
        tenant_id,
        trigger: name,
        fired: executions.len() as u32,
        executions,
    })
    .into_response()
}

/// Union of the declared params across a trigger's subscribed workflows — the
/// trigger's effective pass-through schema (CLOACI-T-0777 P2). Deduped by slot
/// name (first subscriber wins). A workflow whose params can't be resolved
/// contributes nothing, so the result is best-effort.
async fn trigger_declared_slots(
    registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
    workflows: &[String],
) -> Vec<InputSlot> {
    let mut seen = std::collections::HashSet::new();
    let mut slots: Vec<InputSlot> = Vec::new();
    for wf in workflows {
        if let Ok(params) = registry.get_workflow_declared_params(wf).await {
            for slot in params {
                if seen.insert(slot.name.clone()) {
                    slots.push(slot);
                }
            }
        }
    }
    slots
}

/// GET /tenants/:tenant_id/triggers/:name/interface — the trigger's declared
/// pass-through schema (CLOACI-T-0777 P2): the union of the declared params of
/// every workflow subscribed to this trigger. Empty `slots` means an untyped
/// trigger (free-form event). Read-only discovery; the same slots back the
/// validation in `fire_trigger`, and the UI builds a typed fire form from them.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/triggers/{name}/interface",
    tag = "triggers",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Trigger name"),
    ),
    responses(
        (status = 200, description = "Trigger pass-through interface", body = DeclaredSurface),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_trigger_interface(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
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
            return ApiError::internal(format!("tenant database unavailable: {}", e)).into_response()
        }
    };

    // Subscribers = workflows that list this trigger in `#[workflow(triggers=[…])]`
    // (CLOACI-T-0777), the same fan-out set `fire_trigger` uses.
    let storage = UnifiedRegistryStorage::new(tenant_db.clone());
    let slots = match WorkflowRegistryImpl::new(storage, tenant_db) {
        Ok(registry) => {
            let workflows = registry
                .find_trigger_subscribers(&name)
                .await
                .unwrap_or_default();
            trigger_declared_slots(&registry, &workflows).await
        }
        Err(_) => Vec::new(),
    };

    Json(DeclaredSurface {
        kind: "trigger".to_string(),
        name,
        slots,
    })
    .into_response()
}
