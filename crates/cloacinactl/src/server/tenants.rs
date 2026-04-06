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

//! Tenant management endpoints.
//!
//! Wraps existing `DatabaseAdmin` (create_tenant, remove_tenant) as REST.
//! Tenants are isolated Postgres schemas.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;
use tracing::{info, warn};

use cloacina::database::{DatabaseAdmin, TenantConfig};

use crate::commands::serve::AppState;
use crate::server::auth::AuthenticatedKey;

/// Request body for creating a tenant.
#[derive(Deserialize)]
pub struct CreateTenantRequest {
    /// Schema name (alphanumeric + underscore, no SQL injection)
    pub schema_name: String,
    /// Database username for this tenant
    pub username: String,
    /// Optional password (auto-generated if empty)
    #[serde(default)]
    pub password: String,
}

/// POST /tenants — create a new tenant (Postgres schema + user + migrations).
/// Admin-only: only is_admin keys can create tenants.
pub async fn create_tenant(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Json(body): Json<CreateTenantRequest>,
) -> impl IntoResponse {
    if !auth.is_admin {
        return AuthenticatedKey::admin_required_response().into_response();
    }

    let admin = DatabaseAdmin::new(state.database.clone());
    let config = TenantConfig {
        schema_name: body.schema_name.clone(),
        username: body.username.clone(),
        password: body.password,
    };

    match admin.create_tenant(config).await {
        Ok(credentials) => {
            info!("Created tenant: {}", body.schema_name);
            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "schema_name": credentials.schema_name,
                    "username": credentials.username,
                    "password": credentials.password,
                    "connection_string": credentials.connection_string,
                })),
            )
                .into_response()
        }
        Err(e) => {
            warn!("Failed to create tenant '{}': {}", body.schema_name, e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response()
        }
    }
}

/// DELETE /tenants/:schema_name — remove a tenant (drop schema + user).
/// Admin-only: only is_admin keys can remove tenants.
pub async fn remove_tenant(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(schema_name): Path<String>,
) -> impl IntoResponse {
    if !auth.is_admin {
        return AuthenticatedKey::admin_required_response().into_response();
    }

    let admin = DatabaseAdmin::new(state.database.clone());

    // Use schema_name as both schema and username (convention)
    match admin.remove_tenant(&schema_name, &schema_name).await {
        Ok(()) => {
            info!("Removed tenant: {}", schema_name);
            Json(serde_json::json!({"status": "removed", "schema_name": schema_name}))
                .into_response()
        }
        Err(e) => {
            warn!("Failed to remove tenant '{}': {}", schema_name, e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response()
        }
    }
}

/// GET /tenants — list tenant schemas.
pub async fn list_tenants(State(state): State<AppState>) -> impl IntoResponse {
    let admin = DatabaseAdmin::new(state.database.clone());

    match admin.list_tenant_schemas().await {
        Ok(schemas) => {
            let tenants: Vec<_> = schemas
                .into_iter()
                .map(|s| serde_json::json!({"schema_name": s}))
                .collect();
            Json(serde_json::json!({"tenants": tenants})).into_response()
        }
        Err(e) => {
            warn!("Failed to list tenants: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response()
        }
    }
}
