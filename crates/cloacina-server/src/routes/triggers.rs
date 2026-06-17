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
    ListTriggersQuery, TenantListResponse, TriggerDetailResponse, TriggerExecution,
    TriggerScheduleInfo, TriggerScheduleSummary,
};

use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
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
                },
                recent_executions: exec_items,
            })
            .into_response()
        }
        None => ApiError::not_found("trigger_not_found", format!("trigger '{}' not found", name))
            .into_response(),
    }
}
