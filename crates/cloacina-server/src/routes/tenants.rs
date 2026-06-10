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
use tracing::{info, warn};

use cloacina::database::{DatabaseAdmin, TenantConfig};
use cloacina::security::audit;
use cloacina_api_types::{
    CreateTenantRequest, ListResponse, TenantCreatedResponse, TenantRemovedResponse, TenantSummary,
};
use std::time::Instant;

use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::AppState;

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
        // Per CLOACI-T-0594: schema_name == username == public name.
        // Keeps the public API ergonomic. If we ever need divergence,
        // the canonical name stays the request `name` field.
        schema_name: body.name.clone(),
        username: body.name.clone(),
        password: body.password.clone().unwrap_or_default(),
    };

    match admin.create_tenant(config).await {
        Ok(credentials) => {
            info!(
                tenant = %body.name,
                description = ?body.description,
                "Created tenant"
            );
            // Note: password and connection_string intentionally excluded from
            // response to prevent credential leakage (SEC-08). The caller should
            // supply their own password via the request body, or retrieve it
            // through a secure channel.
            (
                StatusCode::CREATED,
                Json(TenantCreatedResponse {
                    name: credentials.schema_name,
                    username: credentials.username,
                    description: body.description,
                }),
            )
                .into_response()
        }
        Err(e) => {
            warn!("Failed to create tenant '{}': {}", body.name, e);
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

    // Step 2: evict the tenant runner from cache (bounded drain).
    // CLOACI-T-0581: pathological tasks that ignore cooperative cancellation
    // can't block teardown. Past `tenant_deletion_drain_timeout`, the runner
    // is removed from the cache and we proceed without awaiting its shutdown
    // future. The shutdown continues in the background; any task that
    // ignored cancellation will error on its next DB write once step 4 drops
    // the schema.
    let step_started = Instant::now();
    let evict_outcome = state
        .tenant_runners
        .evict_with_timeout(&schema_name, state.tenant_deletion_drain_timeout)
        .await;
    audit::log_tenant_teardown_step(
        audit::events::TENANT_TEARDOWN_RUNNER_EVICTED,
        &schema_name,
        if evict_outcome.was_present() { 1 } else { 0 },
        step_started.elapsed().as_millis() as u64,
    );
    let runner_evicted = evict_outcome.was_present();

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
            Json(TenantRemovedResponse {
                status: "removed".to_string(),
                schema_name,
                revoked_keys: revoked_count,
                runner_evicted,
                db_cache_evicted: db_evicted,
            })
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
            let items: Vec<TenantSummary> = schemas
                .into_iter()
                .map(|s| TenantSummary { name: s })
                .collect();
            // CLOACI-T-0594 / API-03: unified `{items, total}` list envelope.
            // CLI's render::list reads body.items consistently across every
            // list endpoint. Per-tenant schema name is rendered under `name`
            // to match the CLOACI-T-0594 / API-01 unification.
            Json(ListResponse::new(items)).into_response()
        }
        Err(e) => {
            warn!("Failed to list tenants: {}", e);
            ApiError::internal(format!("failed to list tenants: {}", e)).into_response()
        }
    }
}
