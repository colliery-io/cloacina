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

use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::AppState;

/// GET /tenants/:tenant_id/triggers — list all schedules (cron + trigger).
pub async fn list_triggers(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }

    let dal = cloacina::dal::DAL::new(state.database.clone());

    match dal.schedule().list(None, false, 100, 0).await {
        Ok(schedules) => {
            let items: Vec<_> = schedules
                .into_iter()
                .map(|s| {
                    serde_json::json!({
                        "id": s.id.0.to_string(),
                        "schedule_type": s.schedule_type,
                        "workflow_name": s.workflow_name,
                        "enabled": s.enabled.is_true(),
                        "cron_expression": s.cron_expression,
                        "trigger_name": s.trigger_name,
                        "poll_interval_ms": s.poll_interval_ms,
                        "next_run_at": s.next_run_at.map(|t| t.0.to_rfc3339()),
                        "last_run_at": s.last_run_at.map(|t| t.0.to_rfc3339()),
                        "created_at": s.created_at.0.to_rfc3339(),
                    })
                })
                .collect();
            Json(serde_json::json!({
                "tenant_id": tenant_id,
                "schedules": items,
            }))
            .into_response()
        }
        Err(e) => {
            warn!("Failed to list triggers for tenant '{}': {}", tenant_id, e);
            ApiError::internal(format!("{}", e)).into_response()
        }
    }
}

/// GET /tenants/:tenant_id/triggers/:name — trigger details + recent executions.
pub async fn get_trigger(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }

    let dal = cloacina::dal::DAL::new(state.database.clone());

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

            let exec_items: Vec<_> = executions
                .into_iter()
                .map(|e| {
                    serde_json::json!({
                        "id": e.id.0.to_string(),
                        "scheduled_time": e.scheduled_time.map(|t| t.0.to_rfc3339()),
                        "started_at": e.started_at.0.to_rfc3339(),
                        "completed_at": e.completed_at.map(|t| t.0.to_rfc3339()),
                    })
                })
                .collect();

            Json(serde_json::json!({
                "tenant_id": tenant_id,
                "schedule": {
                    "id": schedule.id.0.to_string(),
                    "schedule_type": schedule.schedule_type,
                    "workflow_name": schedule.workflow_name,
                    "enabled": schedule.enabled.is_true(),
                    "cron_expression": schedule.cron_expression,
                    "trigger_name": schedule.trigger_name,
                },
                "recent_executions": exec_items,
            }))
            .into_response()
        }
        None => ApiError::not_found("trigger_not_found", format!("trigger '{}' not found", name))
            .into_response(),
    }
}
