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

//! `GET /v1/tenants/{tenant_id}/connectors` — the Sync connector catalog
//! (COLLIERY-I-0634 / CLOACINA-T-0993): the compiled connectors in the
//! provider search path and the declarative connector definitions.

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use cloacina_api_types::{
    ConnectorCheckResponse, ConnectorDiscoverResponse, ConnectorProbeRequest,
    ConnectorSpecResponse, ConnectorSummary, ListResponse,
};

use super::error::ApiError;
use crate::AppState;
use crate::routes::auth::AuthenticatedKey;

/// `GET /v1/tenants/{tenant_id}/connectors` — the connectors a sync can name.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/connectors",
    tag = "connectors",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    responses(
        (status = 200, description = "Compiled and declarative Sync connectors", body = ListResponse<ConnectorSummary>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "The connector definitions could not be loaded", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_connectors(Path(_tenant_id): Path<String>) -> impl IntoResponse {
    match cloacina_sync::catalog::catalog() {
        Ok(entries) => {
            let items: Vec<ConnectorSummary> = entries
                .into_iter()
                .map(|e| ConnectorSummary {
                    name: e.name,
                    kind: e.kind.to_string(),
                    provider: e.provider,
                    version: e.version,
                    roles: e.roles,
                    sync_modes: e.sync_modes,
                    streams: e.streams,
                    config_schema: e.config_schema,
                    secret_fields: e.secret_fields,
                })
                .collect();
            Json(ListResponse::new(items)).into_response()
        }
        Err(e) => ApiError::internal(format!("connector definitions: {e}")).into_response(),
    }
}

/// How long a check or discover may take before the route gives up.
const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// Run the probe workflow for `action` on the tenant's runner, where Sync's
/// tasks run, and wait for its answer (CLOACINA-T-1002).
async fn probe<T: serde::de::DeserializeOwned>(
    state: &AppState,
    tenant_id: &str,
    action: &str,
    request: &ConnectorProbeRequest,
) -> Result<T, ApiError> {
    use cloacina::executor::{WorkflowExecutor, WorkflowStatus};

    if let Err(e) = cloacina_sync::probe::validate(action, request) {
        return Err(ApiError::bad_request("probe_invalid", e.to_string()));
    }
    let tenant_db = state
        .tenant_databases
        .resolve(tenant_id, &state.database)
        .await
        .map_err(|e| ApiError::internal(format!("tenant database error: {e}")))?;
    // As the execute route: "public" is the global runner.
    let runner = if tenant_id == "public" {
        state.runner.clone()
    } else {
        state
            .tenant_runners
            .get_or_create(tenant_id, tenant_db.clone())
            .await
            .map_err(|e| ApiError::internal(format!("tenant runner unavailable: {e}")))?
    };
    let ctx = cloacina_sync::probe::context(action, request);
    let result = match tokio::time::timeout(
        PROBE_TIMEOUT,
        runner.execute(cloacina_sync::probe::PROBE_WORKFLOW, ctx),
    )
    .await
    {
        Err(_) => {
            return Err(ApiError::new(
                axum::http::StatusCode::GATEWAY_TIMEOUT,
                "probe_timeout",
                format!(
                    "the {action} did not answer in {}s",
                    PROBE_TIMEOUT.as_secs()
                ),
            ));
        }
        Ok(Err(e)) => {
            return Err(ApiError::internal(format!("{action}: {e}")));
        }
        Ok(Ok(r)) => r,
    };
    if result.status != WorkflowStatus::Completed {
        let dal = cloacina::dal::DAL::new(tenant_db);
        let why = cloacina_sync::dispatcher::failure(&dal, result.execution_id).await;
        return Err(ApiError::new(
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            "probe_failed",
            format!("the {action} failed: {why}"),
        ));
    }
    let answer = result
        .final_context
        .get(cloacina_sync::probe::keys::RESULT)
        .cloned()
        .unwrap_or_default();
    serde_json::from_value(answer).map_err(|e| ApiError::internal(format!("{action} answer: {e}")))
}

/// `POST /v1/tenants/{tenant_id}/connectors/check` — check one side of a sync
/// where connectors run: its config, its secret, and that it can reach its
/// service.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/connectors/check",
    tag = "connectors",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    request_body = ConnectorProbeRequest,
    responses(
        (status = 200, description = "What the connector's check said", body = ConnectorCheckResponse),
        (status = 400, description = "The side is not one the probe can run", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 422, description = "The check could not run, with the reason: a missing secret, a connector that does not load", body = cloacina_api_types::ErrorBody),
        (status = 504, description = "The check did not answer in time", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn check_connector(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
    Json(body): Json<ConnectorProbeRequest>,
) -> impl IntoResponse {
    match probe::<ConnectorCheckResponse>(&state, &tenant_id, "check", &body).await {
        Ok(answer) => Json(answer).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `POST /v1/tenants/{tenant_id}/connectors/discover` — the streams a source
/// offers, with their sync modes, keys and fields.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/connectors/discover",
    tag = "connectors",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    request_body = ConnectorProbeRequest,
    responses(
        (status = 200, description = "The source's streams", body = ConnectorDiscoverResponse),
        (status = 400, description = "The side is not a source the probe can run", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 422, description = "The discover could not run, with the reason", body = cloacina_api_types::ErrorBody),
        (status = 504, description = "The discover did not answer in time", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn discover_connector(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
    Json(body): Json<ConnectorProbeRequest>,
) -> impl IntoResponse {
    match probe::<ConnectorDiscoverResponse>(&state, &tenant_id, "discover", &body).await {
        Ok(answer) => Json(answer).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `POST /v1/tenants/{tenant_id}/connectors/spec` — what a compiled connector
/// says about itself: its config schema, roles and sync modes (CLOACINA-T-1006).
/// It runs where connectors run, as check and discover do.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/connectors/spec",
    tag = "connectors",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    request_body = ConnectorProbeRequest,
    responses(
        (status = 200, description = "The connector's spec", body = ConnectorSpecResponse),
        (status = 400, description = "The side is not a compiled connector", body = cloacina_api_types::ErrorBody),
        (status = 422, description = "The connector does not load, with the reason", body = cloacina_api_types::ErrorBody),
        (status = 504, description = "No answer in time", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn spec_connector(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
    Json(body): Json<ConnectorProbeRequest>,
) -> impl IntoResponse {
    match probe::<ConnectorSpecResponse>(&state, &tenant_id, "spec", &body).await {
        Ok(answer) => Json(answer).into_response(),
        Err(e) => e.into_response(),
    }
}
