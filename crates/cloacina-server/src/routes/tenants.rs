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
use cloacina::security::audit;
use std::time::Instant;

use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::AppState;

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
            // Note: password and connection_string intentionally excluded from
            // response to prevent credential leakage (SEC-08). The caller should
            // supply their own password via the request body, or retrieve it
            // through a secure channel.
            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "schema_name": credentials.schema_name,
                    "username": credentials.username,
                })),
            )
                .into_response()
        }
        Err(e) => {
            warn!("Failed to create tenant '{}': {}", body.schema_name, e);
            ApiError::bad_request("tenant_creation_failed", format!("{}", e)).into_response()
        }
    }
}

/// DELETE /tenants/:schema_name — remove a tenant via orchestrated teardown.
///
/// CLOACI-T-0581: replaces the old single-call `admin.remove_tenant` with
/// the four-step top-down order:
///   1. Revoke every still-active API key for the tenant (close the auth
///      surface so new requests fail).
///   2. Evict the tenant's `DefaultRunner` from `TenantRunnerCache`,
///      awaiting its graceful shutdown (drains in-flight executions,
///      stops scheduler loop, closes per-tenant DB pool).
///   3. Evict the tenant's `Database` from `TenantDatabaseCache`.
///   4. Drop schema + user via `DatabaseAdmin::remove_tenant`.
///
/// Each step emits a structured audit event with duration. Per-step
/// failures bail out — but earlier steps stay committed, so a retry
/// picks up where the failure occurred. Each step is idempotent.
///
/// Closes SEC-14 (stale state after delete) and SEC-17 (unbounded
/// caches surviving delete).
pub async fn remove_tenant(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(schema_name): Path<String>,
) -> impl IntoResponse {
    if !auth.is_admin {
        return AuthenticatedKey::admin_required_response().into_response();
    }

    let teardown_started = Instant::now();

    // Step 1: revoke keys.
    let step_started = Instant::now();
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let revoked_count = match dal.api_keys().revoke_keys_for_tenant(&schema_name).await {
        Ok(n) => n,
        Err(e) => {
            warn!(
                tenant_id = %schema_name,
                error = %e,
                "tenant teardown step 1 (revoke keys) failed"
            );
            audit::log_tenant_teardown_outcome(
                &schema_name,
                false,
                teardown_started.elapsed().as_millis() as u64,
            );
            return ApiError::internal(format!("revoke keys failed: {}", e)).into_response();
        }
    };
    audit::log_tenant_teardown_step(
        audit::events::TENANT_TEARDOWN_KEYS_REVOKED,
        &schema_name,
        revoked_count,
        step_started.elapsed().as_millis() as u64,
    );

    // Step 2: evict the tenant runner from cache.
    let step_started = Instant::now();
    let runner_evicted = match state.tenant_runners.evict(&schema_name).await {
        Ok(b) => b,
        Err(e) => {
            warn!(
                tenant_id = %schema_name,
                error = %e,
                "tenant teardown step 2 (runner eviction) failed"
            );
            audit::log_tenant_teardown_outcome(
                &schema_name,
                false,
                teardown_started.elapsed().as_millis() as u64,
            );
            return ApiError::internal(format!("runner eviction failed: {}", e)).into_response();
        }
    };
    audit::log_tenant_teardown_step(
        audit::events::TENANT_TEARDOWN_RUNNER_EVICTED,
        &schema_name,
        if runner_evicted { 1 } else { 0 },
        step_started.elapsed().as_millis() as u64,
    );

    // Step 3: evict the tenant database from cache.
    let step_started = Instant::now();
    let db_evicted = state.tenant_databases.evict(&schema_name).await;
    audit::log_tenant_teardown_step(
        audit::events::TENANT_TEARDOWN_DB_CACHE_EVICTED,
        &schema_name,
        if db_evicted { 1 } else { 0 },
        step_started.elapsed().as_millis() as u64,
    );

    // Step 4: drop schema + user.
    let step_started = Instant::now();
    let admin = DatabaseAdmin::new(state.database.clone());
    match admin.remove_tenant(&schema_name, &schema_name).await {
        Ok(()) => {
            audit::log_tenant_teardown_step(
                audit::events::TENANT_TEARDOWN_SCHEMA_DROPPED,
                &schema_name,
                1,
                step_started.elapsed().as_millis() as u64,
            );
            audit::log_tenant_teardown_outcome(
                &schema_name,
                true,
                teardown_started.elapsed().as_millis() as u64,
            );
            info!(
                tenant_id = %schema_name,
                revoked_keys = revoked_count,
                runner_evicted = runner_evicted,
                db_cache_evicted = db_evicted,
                "tenant teardown complete"
            );
            Json(serde_json::json!({
                "status": "removed",
                "schema_name": schema_name,
                "revoked_keys": revoked_count,
                "runner_evicted": runner_evicted,
                "db_cache_evicted": db_evicted,
            }))
            .into_response()
        }
        Err(e) => {
            warn!(
                tenant_id = %schema_name,
                error = %e,
                "tenant teardown step 4 (drop schema) failed"
            );
            audit::log_tenant_teardown_outcome(
                &schema_name,
                false,
                teardown_started.elapsed().as_millis() as u64,
            );
            ApiError::bad_request("tenant_removal_failed", format!("{}", e)).into_response()
        }
    }
}

/// GET /tenants — list tenant schemas.
/// Requires admin role — only admins can enumerate tenants.
pub async fn list_tenants(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    if !auth.is_admin {
        return AuthenticatedKey::admin_required_response().into_response();
    }
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
            ApiError::internal(format!("failed to list tenants: {}", e)).into_response()
        }
    }
}
