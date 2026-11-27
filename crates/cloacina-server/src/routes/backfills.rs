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

//! Backfill API (CLOACINA-T-1020).
//!
//! A backfill is a bounded second schedule for one workflow: one run for
//! each fire time of a cron expression from `start` to `end`, oldest first,
//! with its own limit on active runs. It is stored as a `schedules` row of
//! type `backfill` (see `cloacina::models::backfill`), and the scheduler of
//! the tenant runs it. These routes create, list, inspect, cancel and retry.
//!
//! Tenant scoping is the same as on the other routes: every read and write
//! goes through the tenant's database, so a backfill of tenant A is not
//! found in tenant B.

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use tracing::warn;

use cloacina_api_types::{
    BackfillDetail, BackfillFailedInterval, BackfillProgress, BackfillSummary,
    CreateBackfillRequest, ListBackfillsQuery, RetryBackfillRequest, TenantListResponse,
};

use cloacina::CronEvaluator;
use cloacina::dal::{DAL, UnifiedRegistryStorage};
use cloacina::database::universal_types::UniversalUuid;
use cloacina::models::backfill::{BackfillCounts, BackfillSpec, BackfillStatus, NewBackfill};
use cloacina::models::schedule::Schedule;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use cloacina::workflow_instance::{BACKFILL_ID_KEY, INTERVAL_END_KEY, INTERVAL_START_KEY};

use crate::AppState;
use crate::routes::audit::{self as audit_record, AuditEvent};
use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::routes::executions::validate_declared_params;
use crate::routes::instances::wake_cron_scheduler;

const DEFAULT_BACKFILLS_LIMIT: i64 = 100;
const MAX_BACKFILLS_LIMIT: i64 = 1000;
/// The most failed intervals in one inspect response.
const MAX_FAILED_INTERVALS: i64 = 1000;

/// Reads an RFC 3339 timestamp of a request body.
fn parse_time(field: &str, value: &str) -> Result<DateTime<Utc>, ApiError> {
    DateTime::parse_from_rfc3339(value)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| {
            ApiError::bad_request(
                "invalid_backfill",
                format!(
                    "{field} must be an RFC 3339 timestamp such as 2026-01-31T06:00:00Z: '{value}' is not ({e})"
                ),
            )
        })
}

fn internal(context: &'static str, e: impl std::fmt::Display) -> ApiError {
    ApiError::internal_logged(context, &e)
}

fn not_found(id: &str, workflow: &str) -> ApiError {
    ApiError::not_found(
        "backfill_not_found",
        format!("backfill '{}' of workflow '{}' not found", id, workflow),
    )
}

/// Maps a backfill row and its counts onto the wire type.
///
/// `remaining` and the status come from the position of the row;
/// `completed`, `failed` and `running` are counted from its runs.
fn to_summary(schedule: &Schedule, counts: &BackfillCounts) -> Result<BackfillSummary, ApiError> {
    let unreadable = |e: String| internal("backfill row is not readable", e);
    let spec = BackfillSpec::parse(schedule.backfill.as_deref().unwrap_or_default())
        .map_err(unreadable)?;
    let cron_expression = schedule.cron_expression.clone().unwrap_or_default();
    let timezone = schedule
        .timezone
        .clone()
        .unwrap_or_else(|| "UTC".to_string());
    let last_run_at = schedule.last_run_at.map(|t| t.0);
    let remaining = match last_run_at {
        Some(last) => {
            let evaluator = CronEvaluator::new(&cron_expression, &timezone)
                .map_err(|e| unreadable(e.to_string()))?;
            spec.remaining_after(&evaluator, last).map_err(unreadable)?
        }
        None => spec.total,
    };
    let status = BackfillStatus::derive(
        schedule.is_enabled(),
        schedule.is_paused(),
        remaining,
        counts.running,
    );
    Ok(BackfillSummary {
        id: schedule.id.0.to_string(),
        workflow_name: schedule.workflow_name.clone(),
        cron_expression,
        timezone,
        start: spec.start.to_rfc3339(),
        end: spec.end.to_rfc3339(),
        max_active_runs: schedule
            .max_active_runs
            .and_then(|limit| u32::try_from(limit).ok())
            .unwrap_or(1),
        params: schedule
            .params
            .as_deref()
            .and_then(|p| serde_json::from_str::<serde_json::Value>(p).ok()),
        status: status.as_str().to_string(),
        progress: BackfillProgress {
            total: spec.total,
            completed: counts.completed,
            failed: counts.failed,
            running: counts.running,
            remaining,
            last_interval_end: (remaining < spec.total)
                .then(|| last_run_at.map(|t| t.to_rfc3339()))
                .flatten(),
            next_interval_end: (remaining > 0)
                .then(|| schedule.next_run_at.map(|t| t.0.to_rfc3339()))
                .flatten(),
        },
        retry_of: spec.retry_of,
        created_at: schedule.created_at.0.to_rfc3339(),
    })
}

/// Reads one backfill of the workflow in the path. A row of another
/// workflow is not found.
async fn load_backfill(dal: &DAL, workflow: &str, id: &str) -> Result<Schedule, ApiError> {
    let Ok(uuid) = uuid::Uuid::parse_str(id) else {
        return Err(ApiError::bad_request(
            "invalid_request",
            format!("invalid backfill id '{}'", id),
        ));
    };
    match dal.schedule().get_backfill(UniversalUuid(uuid)).await {
        Ok(Some(schedule)) if schedule.workflow_name == workflow => Ok(schedule),
        Ok(_) => Err(not_found(id, workflow)),
        Err(e) => Err(internal("read backfill failed", e)),
    }
}

async fn summary_of(dal: &DAL, schedule: &Schedule) -> Result<BackfillSummary, ApiError> {
    let counts = dal
        .schedule()
        .backfill_counts(schedule.id)
        .await
        .map_err(|e| internal("read backfill progress failed", e))?;
    to_summary(schedule, &counts)
}

async fn tenant_dal(
    state: &AppState,
    tenant_id: &str,
) -> Result<(DAL, cloacina::Database), ApiError> {
    match state
        .tenant_databases
        .resolve(tenant_id, &state.database)
        .await
    {
        Ok(db) => Ok((DAL::new(db.clone()), db)),
        Err(e) => {
            warn!(
                "Failed to resolve tenant database for '{}': {}",
                tenant_id, e
            );
            Err(internal("tenant database unavailable", e))
        }
    }
}

/// The cron expression and timezone of a new backfill: those of the body,
/// or else those of the one cron schedule of the workflow.
async fn resolve_cron(
    dal: &DAL,
    workflow: &str,
    body: &CreateBackfillRequest,
) -> Result<(String, String), ApiError> {
    if let Some(cron) = body.cron.as_deref() {
        let timezone = body.timezone.clone().unwrap_or_else(|| "UTC".to_string());
        return Ok((cron.to_string(), timezone));
    }
    let live: Vec<Schedule> = dal
        .schedule()
        .find_by_workflow(workflow)
        .await
        .map_err(|e| internal("read schedules failed", e))?
        .into_iter()
        .filter(|s| s.is_cron() && s.cron_expression.as_deref().is_some_and(|c| !c.is_empty()))
        .collect();
    match live.as_slice() {
        [one] => Ok((
            one.cron_expression.clone().unwrap_or_default(),
            body.timezone
                .clone()
                .or_else(|| one.timezone.clone())
                .unwrap_or_else(|| "UTC".to_string()),
        )),
        [] => Err(ApiError::bad_request(
            "invalid_backfill",
            format!(
                "cron is required: workflow '{}' has no cron schedule to take it from",
                workflow
            ),
        )),
        many => Err(ApiError::bad_request(
            "invalid_backfill",
            format!(
                "cron is required: workflow '{}' has {} cron schedules",
                workflow,
                many.len()
            ),
        )),
    }
}

/// Validates the params of a backfill against the declared params of the
/// workflow, the way the instance and execute routes do. The scheduler sets
/// the interval keys and `backfill_id` on each run, so a workflow that
/// declares one of them does not need it in `params`.
async fn check_params(
    tenant_db: &cloacina::Database,
    workflow: &str,
    params: Option<&serde_json::Value>,
) -> Result<Option<String>, ApiError> {
    let provided = match params {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Object(map)) => Some(map.clone()),
        Some(_) => {
            return Err(ApiError::bad_request(
                "backfill_params_invalid",
                "params must be a JSON object".to_string(),
            ));
        }
    };

    let storage = UnifiedRegistryStorage::new(tenant_db.clone());
    if let Ok(registry) = WorkflowRegistryImpl::new(storage, tenant_db.clone()) {
        match registry.get_workflow_declared_params(workflow).await {
            Ok(slots) if !slots.is_empty() => {
                let mut with_run_keys = provided.clone().unwrap_or_default();
                for key in [INTERVAL_START_KEY, INTERVAL_END_KEY] {
                    with_run_keys
                        .entry(key.to_string())
                        .or_insert_with(|| serde_json::json!("1970-01-01T00:00:00+00:00"));
                }
                with_run_keys
                    .entry(BACKFILL_ID_KEY.to_string())
                    .or_insert_with(|| serde_json::json!(uuid::Uuid::nil().to_string()));
                let errors = validate_declared_params(&slots, Some(&with_run_keys));
                if !errors.is_empty() {
                    return Err(ApiError::bad_request(
                        "backfill_params_invalid",
                        format!("invalid backfill params: {}", errors.join("; ")),
                    ));
                }
            }
            Ok(_) => {}
            Err(e) => warn!("declared-params lookup failed for '{}': {}", workflow, e),
        }
    }

    match provided {
        None => Ok(None),
        Some(map) => serde_json::to_string(&map).map(Some).map_err(|e| {
            ApiError::bad_request(
                "backfill_params_invalid",
                format!("params are not serializable: {}", e),
            )
        }),
    }
}

/// POST /tenants/:tenant_id/workflows/:name/backfills — create a backfill.
///
/// The backfill takes each fire time `F` of the cron expression with
/// `start <= F <= end`, oldest first. It may cover fire times that the live
/// schedule of the workflow ran or will run: nothing checks for overlap.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/backfills",
    tag = "backfills",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Workflow name"),
    ),
    request_body = CreateBackfillRequest,
    responses(
        (status = 200, description = "Backfill created", body = BackfillSummary),
        (status = 400, description = "Invalid range, cron expression, timezone, limit or params", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn create_backfill(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
    Json(body): Json<CreateBackfillRequest>,
) -> Response {
    match create_backfill_inner(&state, &auth, &tenant_id, &name, body).await {
        Ok(summary) => Json(summary).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn create_backfill_inner(
    state: &AppState,
    auth: &AuthenticatedKey,
    tenant_id: &str,
    name: &str,
    body: CreateBackfillRequest,
) -> Result<BackfillSummary, ApiError> {
    let start = parse_time("start", &body.start)?;
    let end = parse_time("end", &body.end)?;
    let (dal, tenant_db) = tenant_dal(state, tenant_id).await?;
    let (cron, timezone) = resolve_cron(&dal, name, &body).await?;
    if let Err(e) = CronEvaluator::validate(&cron, &timezone) {
        return Err(ApiError::bad_request(
            "invalid_cron",
            format!("invalid cron expression or timezone: {}", e),
        ));
    }
    let params = check_params(&tenant_db, name, body.params.as_ref()).await?;
    let new = NewBackfill::for_range(
        name,
        &cron,
        &timezone,
        start,
        end,
        body.max_active_runs,
        params,
    )
    .map_err(|e| ApiError::bad_request("invalid_backfill", e))?;

    let schedule = dal
        .schedule()
        .create_backfill(new)
        .await
        .map_err(|e| internal("create backfill failed", e))?;
    let summary = to_summary(&schedule, &BackfillCounts::default())?;
    tracing::info!(
        "Created backfill {} of '{}' for tenant '{}': {} interval(s)",
        summary.id,
        name,
        tenant_id,
        summary.progress.total
    );
    wake_cron_scheduler(state, tenant_id, tenant_db).await;
    let detail = serde_json::json!({
        "workflow": name,
        "start": summary.start,
        "end": summary.end,
        "intervals": summary.progress.total,
    });
    let event = AuditEvent::ok(audit_record::actions::BACKFILL_CREATE, &summary.id);
    audit_record::record(state, auth, Some(tenant_id), event.detail(detail)).await;
    Ok(summary)
}

/// GET /tenants/:tenant_id/workflows/:name/backfills — the backfills of a
/// workflow, newest first, each with its progress.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/backfills",
    tag = "backfills",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Workflow name"),
        ListBackfillsQuery,
    ),
    responses(
        (status = 200, description = "Backfills of this workflow", body = TenantListResponse<BackfillSummary>),
        (status = 400, description = "Invalid pagination", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_backfills(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
    Query(q): Query<ListBackfillsQuery>,
) -> Response {
    let limit = q.limit.unwrap_or(DEFAULT_BACKFILLS_LIMIT);
    if !(1..=MAX_BACKFILLS_LIMIT).contains(&limit) {
        return ApiError::bad_request(
            "invalid_pagination",
            format!("limit must be 1..={}", MAX_BACKFILLS_LIMIT),
        )
        .into_response();
    }
    let offset = q.offset.unwrap_or(0);
    if offset < 0 {
        return ApiError::bad_request("invalid_pagination", "offset must be >= 0".to_string())
            .into_response();
    }

    let result = async {
        let (dal, _) = tenant_dal(&state, &tenant_id).await?;
        let schedules = dal
            .schedule()
            .list_backfills(&name, limit, offset)
            .await
            .map_err(|e| internal("list backfills failed", e))?;
        let mut items = Vec::with_capacity(schedules.len());
        for schedule in &schedules {
            items.push(summary_of(&dal, schedule).await?);
        }
        Ok::<_, ApiError>(items)
    }
    .await;
    match result {
        Ok(items) => Json(TenantListResponse::new(tenant_id, items)).into_response(),
        Err(e) => e.into_response(),
    }
}

/// GET /tenants/:tenant_id/workflows/:name/backfills/:id — one backfill,
/// with its progress and its failed intervals.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/backfills/{id}",
    tag = "backfills",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Workflow name"),
        ("id" = String, Path, description = "Backfill id"),
    ),
    responses(
        (status = 200, description = "Backfill detail", body = BackfillDetail),
        (status = 400, description = "Invalid backfill id", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Backfill not found", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_backfill(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name, id)): Path<(String, String, String)>,
) -> Response {
    let result = async {
        let (dal, _) = tenant_dal(&state, &tenant_id).await?;
        let schedule = load_backfill(&dal, &name, &id).await?;
        let backfill = summary_of(&dal, &schedule).await?;
        // One more than the list holds, to know that it is cut.
        let mut failed = dal
            .schedule()
            .backfill_failed_intervals(schedule.id, MAX_FAILED_INTERVALS + 1)
            .await
            .map_err(|e| internal("read failed intervals failed", e))?;
        let failed_intervals_truncated = failed.len() as i64 > MAX_FAILED_INTERVALS;
        failed.truncate(MAX_FAILED_INTERVALS as usize);

        let evaluator = CronEvaluator::new(&backfill.cron_expression, &backfill.timezone)
            .map_err(|e| internal("backfill row is not readable", e))?;
        let mut failed_intervals = Vec::with_capacity(failed.len());
        for interval in failed {
            let interval_start = evaluator
                .previous_execution(interval.scheduled_time)
                .map_err(|e| internal("backfill row is not readable", e))?;
            failed_intervals.push(BackfillFailedInterval {
                interval_start: interval_start.to_rfc3339(),
                interval_end: interval.scheduled_time.to_rfc3339(),
                execution_id: interval.workflow_execution_id.map(|id| id.0.to_string()),
                status: interval.status,
            });
        }
        Ok::<_, ApiError>(BackfillDetail {
            backfill,
            failed_intervals,
            failed_intervals_truncated,
        })
    }
    .await;
    match result {
        Ok(detail) => Json(detail).into_response(),
        Err(e) => e.into_response(),
    }
}

/// POST /tenants/:tenant_id/workflows/:name/backfills/:id/cancel — stop a
/// backfill.
///
/// No new interval gets a run. The runs that are active end on their own.
/// Cancelling a backfill that is cancelled or completed changes nothing.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/backfills/{id}/cancel",
    tag = "backfills",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Workflow name"),
        ("id" = String, Path, description = "Backfill id"),
    ),
    responses(
        (status = 200, description = "Backfill as it is after the cancel", body = BackfillSummary),
        (status = 400, description = "Invalid backfill id", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Backfill not found", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn cancel_backfill(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name, id)): Path<(String, String, String)>,
) -> Response {
    let result = async {
        let (dal, tenant_db) = tenant_dal(&state, &tenant_id).await?;
        let schedule = load_backfill(&dal, &name, &id).await?;
        let was_enabled = schedule.is_enabled();
        if was_enabled {
            dal.schedule()
                .disable(schedule.id)
                .await
                .map_err(|e| internal("cancel backfill failed", e))?;
            wake_cron_scheduler(&state, &tenant_id, tenant_db).await;
        }
        let schedule = load_backfill(&dal, &name, &id).await?;
        let summary = summary_of(&dal, &schedule).await?;
        tracing::info!(
            "Cancelled backfill {} of '{}' for tenant '{}'",
            summary.id,
            name,
            tenant_id
        );
        let detail = serde_json::json!({
            "workflow": name,
            "was_enabled": was_enabled,
            "remaining": summary.progress.remaining,
        });
        let event = AuditEvent::ok(audit_record::actions::BACKFILL_CANCEL, &summary.id);
        audit_record::record(&state, &auth, Some(&tenant_id), event.detail(detail)).await;
        Ok::<_, ApiError>(summary)
    }
    .await;
    match result {
        Ok(summary) => Json(summary).into_response(),
        Err(e) => e.into_response(),
    }
}

/// POST /tenants/:tenant_id/workflows/:name/backfills/:id/retry — run the
/// failed intervals of a backfill again.
///
/// Creates a new backfill of the intervals that failed in backfill `id`, as
/// they are at the time of the call, with the same cron expression, timezone
/// and params. The backfill `id` is not changed.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/backfills/{id}/retry",
    tag = "backfills",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Workflow name"),
        ("id" = String, Path, description = "Id of the backfill to retry"),
    ),
    request_body = RetryBackfillRequest,
    responses(
        (status = 200, description = "The new backfill", body = BackfillSummary),
        (status = 400, description = "Invalid backfill id or limit", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Backfill not found", body = cloacina_api_types::ErrorBody),
        (status = 409, description = "The backfill has no failed interval", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn retry_backfill(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name, id)): Path<(String, String, String)>,
    Json(body): Json<RetryBackfillRequest>,
) -> Response {
    let result = async {
        let (dal, tenant_db) = tenant_dal(&state, &tenant_id).await?;
        let source = load_backfill(&dal, &name, &id).await?;
        let failed = dal
            .schedule()
            .backfill_failed_intervals(
                source.id,
                cloacina::models::backfill::MAX_BACKFILL_INTERVALS as i64,
            )
            .await
            .map_err(|e| internal("read failed intervals failed", e))?;
        if failed.is_empty() {
            return Err(ApiError::conflict(
                "backfill_has_no_failed_interval",
                format!("backfill '{}' has no failed interval to retry", id),
            ));
        }
        let max_active_runs = body.max_active_runs.or_else(|| {
            source
                .max_active_runs
                .and_then(|limit| u32::try_from(limit).ok())
        });
        let new = NewBackfill::for_fire_times(
            &name,
            source.cron_expression.as_deref().unwrap_or_default(),
            source.timezone.as_deref().unwrap_or("UTC"),
            failed.iter().map(|f| f.scheduled_time).collect(),
            max_active_runs,
            source.params.clone(),
            Some(source.id),
        )
        .map_err(|e| ApiError::bad_request("invalid_backfill", e))?;

        let schedule = dal
            .schedule()
            .create_backfill(new)
            .await
            .map_err(|e| internal("create backfill failed", e))?;
        let summary = to_summary(&schedule, &BackfillCounts::default())?;
        tracing::info!(
            "Created backfill {} of '{}' for tenant '{}': retry of {} interval(s) of {}",
            summary.id,
            name,
            tenant_id,
            summary.progress.total,
            id
        );
        wake_cron_scheduler(&state, &tenant_id, tenant_db).await;
        let detail = serde_json::json!({
            "workflow": name,
            "new_backfill_id": summary.id,
            "intervals": summary.progress.total,
        });
        let event = AuditEvent::ok(audit_record::actions::BACKFILL_RETRY, &id);
        audit_record::record(&state, &auth, Some(&tenant_id), event.detail(detail)).await;
        Ok::<_, ApiError>(summary)
    }
    .await;
    match result {
        Ok(summary) => Json(summary).into_response(),
        Err(e) => e.into_response(),
    }
}
