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

use axum::Json;
use axum::extract::Path;
use axum::response::IntoResponse;
use cloacina_api_types::{ConnectorSummary, ListResponse};

use super::error::ApiError;

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
                })
                .collect();
            Json(ListResponse::new(items)).into_response()
        }
        Err(e) => ApiError::internal(format!("connector definitions: {e}")).into_response(),
    }
}
