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

//! `/v1/tenants/{tenant_id}/syncs` — the Sync read resource (COLLIERY-I-0635
//! / CLOACINA-T-1004): syncs, their passes, dead-letters and streams, and
//! accepting a stream's new schema. A sync is created, changed and deleted
//! on the instances route.

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use cloacina_api_types::{
    ExecuteResponse, ListResponse, SchemaAcceptedResponse, SyncDeadLetter, SyncDetail,
    SyncPageQuery, SyncPass, SyncStream, SyncSummary, TenantListResponse,
};
use cloacina_sync::views::{self, ViewError};

use super::error::ApiError;
use crate::AppState;
use crate::routes::auth::AuthenticatedKey;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 1000;

async fn tenant_dal(state: &AppState, tenant_id: &str) -> Result<cloacina::dal::DAL, ApiError> {
    let db = state
        .tenant_databases
        .resolve(tenant_id, &state.database)
        .await
        .map_err(|e| ApiError::internal(format!("tenant database error: {e}")))?;
    Ok(cloacina::dal::DAL::new(db))
}

fn page(q: &SyncPageQuery) -> Result<(i64, i64), ApiError> {
    let limit = q.limit.unwrap_or(DEFAULT_LIMIT);
    let offset = q.offset.unwrap_or(0);
    if !(1..=MAX_LIMIT).contains(&limit) || offset < 0 {
        return Err(ApiError::bad_request(
            "invalid_pagination",
            format!("limit must be 1..={MAX_LIMIT} and offset >= 0"),
        ));
    }
    Ok((limit, offset))
}

fn view_error(e: ViewError) -> ApiError {
    match e {
        ViewError::NotFound(_) => ApiError::not_found("sync_not_found", e.to_string()),
        ViewError::Database(_) => ApiError::internal(e.to_string()),
    }
}

/// `GET /v1/tenants/{tenant_id}/syncs` — every sync, by name.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/syncs",
    tag = "syncs",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    responses(
        (status = 200, description = "The tenant's syncs", body = ListResponse<SyncSummary>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_syncs(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    let dal = match tenant_dal(&state, &tenant_id).await {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    match views::list(&dal).await {
        Ok(items) => Json(ListResponse::new(items)).into_response(),
        Err(e) => view_error(e).into_response(),
    }
}

/// `GET /v1/tenants/{tenant_id}/syncs/{name}` — one sync, with its params.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/syncs/{name}",
    tag = "syncs",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Sync definition name"),
    ),
    responses(
        (status = 200, description = "The sync", body = SyncDetail),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "No such sync", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_sync(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let dal = match tenant_dal(&state, &tenant_id).await {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    match views::get(&dal, &name).await {
        Ok(detail) => Json(detail).into_response(),
        Err(e) => view_error(e).into_response(),
    }
}

/// `GET /v1/tenants/{tenant_id}/syncs/{name}/passes` — a page of the sync's
/// passes, newest first, each with its segments' executions.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/syncs/{name}/passes",
    tag = "syncs",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Sync definition name"),
        SyncPageQuery,
    ),
    responses(
        (status = 200, description = "The sync's passes", body = ListResponse<SyncPass>),
        (status = 400, description = "Invalid pagination", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "No such sync", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_passes(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
    Query(q): Query<SyncPageQuery>,
) -> impl IntoResponse {
    let (limit, offset) = match page(&q) {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };
    let dal = match tenant_dal(&state, &tenant_id).await {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    match views::passes(&dal, &name, limit, offset).await {
        Ok(items) => Json(ListResponse::new(items)).into_response(),
        Err(e) => view_error(e).into_response(),
    }
}

/// `GET /v1/tenants/{tenant_id}/syncs/{name}/dead-letters` — a page of the
/// records the sync set aside, newest first; `total` is how many it holds.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/syncs/{name}/dead-letters",
    tag = "syncs",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Sync definition name"),
        SyncPageQuery,
    ),
    responses(
        (status = 200, description = "The sync's dead-letters", body = TenantListResponse<SyncDeadLetter>),
        (status = 400, description = "Invalid pagination", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "No such sync", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_dead_letters(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
    Query(q): Query<SyncPageQuery>,
) -> impl IntoResponse {
    let (limit, offset) = match page(&q) {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };
    let dal = match tenant_dal(&state, &tenant_id).await {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    match views::dead_letters(&dal, &name, limit, offset).await {
        Ok((items, total)) => Json(TenantListResponse {
            tenant_id,
            items,
            total: total as usize,
        })
        .into_response(),
        Err(e) => view_error(e).into_response(),
    }
}

/// `GET /v1/tenants/{tenant_id}/syncs/{name}/streams` — the sync's streams,
/// each with its committed cursor and captured schema.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/syncs/{name}/streams",
    tag = "syncs",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Sync definition name"),
    ),
    responses(
        (status = 200, description = "The sync's streams", body = ListResponse<SyncStream>),
        (status = 404, description = "No such sync", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_streams(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let dal = match tenant_dal(&state, &tenant_id).await {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    match views::streams(&dal, &name).await {
        Ok(items) => Json(ListResponse::new(items)).into_response(),
        Err(e) => view_error(e).into_response(),
    }
}

/// `POST /v1/tenants/{tenant_id}/syncs/{name}/schema/accept` — accept the
/// stream's new shape: its stored schema empties and `broken` clears, and the
/// next write captures the shape it sees.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/syncs/{name}/schema/accept",
    tag = "syncs",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Sync definition name"),
    ),
    responses(
        (status = 200, description = "The new shape is accepted", body = SchemaAcceptedResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "No such sync", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn accept_schema(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let dal = match tenant_dal(&state, &tenant_id).await {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    match views::accept_schema(&dal, &name).await {
        Ok(stream) => Json(SchemaAcceptedResponse {
            name,
            stream,
            accepted: true,
        })
        .into_response(),
        Err(e) => view_error(e).into_response(),
    }
}

/// `POST /v1/tenants/{tenant_id}/syncs/{name}/run` — run a pass of the sync
/// now, as its cron fire does (CLOACINA-T-1005). Returns its first segment's
/// execution; the pass goes on.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/syncs/{name}/run",
    tag = "syncs",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Sync definition name"),
    ),
    responses(
        (status = 200, description = "The pass started", body = ExecuteResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "No such sync", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "The pass could not start", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn run_sync(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let dal = match tenant_dal(&state, &tenant_id).await {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    let ctx = match views::fire_context(&dal, &name).await {
        Ok(c) => c,
        Err(e) => return view_error(e).into_response(),
    };
    // As the execute route: "public" is the global runner.
    let runner = if tenant_id == "public" {
        state.runner.clone()
    } else {
        match state
            .tenant_runners
            .get_or_create(&tenant_id, dal.database().clone())
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return ApiError::internal(format!("tenant runner unavailable: {e}"))
                    .into_response();
            }
        }
    };
    match runner
        .fire_instance(cloacina_sync::WORKFLOW_NAME, ctx)
        .await
    {
        Ok(execution) => {
            let id =
                cloacina::database::universal_types::UniversalUuid::from(execution.execution_id);
            let _ = dal
                .workflow_execution()
                .set_trigger_origin(id, "manual")
                .await;
            Json(ExecuteResponse {
                execution_id: execution.execution_id.to_string(),
                workflow_name: cloacina_sync::WORKFLOW_NAME.to_string(),
                tenant_id,
                status: "scheduled".to_string(),
            })
            .into_response()
        }
        Err(e) => ApiError::internal(format!("the pass could not start: {e}")).into_response(),
    }
}
