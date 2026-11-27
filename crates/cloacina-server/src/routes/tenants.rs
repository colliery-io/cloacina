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
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use tracing::{info, warn};

use cloacina::database::{AdminError, DatabaseAdmin, TenantConfig};
use cloacina::security::audit;
use cloacina_api_types::{
    CreateTenantRequest, ListResponse, TenantCreatedResponse, TenantRemovedResponse, TenantSummary,
};
use std::time::Instant;

use crate::AppState;
use crate::routes::audit::{self as audit_record, AuditEvent};
use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;

/// CLOACI-T-0812: compute the initial `desired_count` to auto-provision for a
/// freshly created tenant. The operator-configured `initial_agents`
/// (`CLOACINA_INITIAL_AGENTS`, default 1) is clamped to the platform default
/// ceiling (`default_max_agents`) so a fresh tenant never comes up above its
/// limit. `0` (either knob) disables auto-provision — the caller skips the
/// desired-count write.
pub(crate) fn initial_desired_count(initial_agents: u32, default_max_agents: u32) -> u32 {
    initial_agents.min(default_max_agents)
}

/// The answer for a failed tenant create or remove (CLOACINA-T-1035).
///
/// - the tenant exists: 409 `tenant_exists`, a message that names it;
/// - the name fails validation: 400 with `invalid_code` and the validation text
///   (it describes the name, it is not database text);
/// - anything else (SQL, pool, migration): 500 with a fixed text. The cause
///   is logged, never sent.
pub(crate) fn admin_error_response(
    tenant: &str,
    invalid_code: &'static str,
    what: &str,
    e: &AdminError,
) -> ApiError {
    match e {
        AdminError::TenantExists { .. } => ApiError::conflict(
            "tenant_exists",
            format!("tenant '{}' already exists", tenant),
        ),
        AdminError::InvalidSchema(_)
        | AdminError::InvalidUsername(_)
        | AdminError::InvalidConfig { .. } => ApiError::bad_request(invalid_code, e.to_string()),
        AdminError::Database(_) | AdminError::Pool(_) | AdminError::SqlExecution { .. } => {
            ApiError::internal_logged(what, e)
        }
    }
}

/// POST /tenants — create a new tenant (Postgres schema + user + migrations).
/// Admin-only: only is_admin keys can create tenants.
#[utoipa::path(
    post,
    path = "/v1/tenants",
    tag = "tenants",
    request_body = CreateTenantRequest,
    responses(
        (status = 201, description = "Tenant created (credentials intentionally excluded)", body = TenantCreatedResponse),
        (status = 400, description = "Invalid tenant name", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Admin (god-mode) key required", body = cloacina_api_types::ErrorBody),
        (status = 409, description = "A tenant of this name exists (`tenant_exists`)", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn create_tenant(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Json(body): Json<CreateTenantRequest>,
) -> impl IntoResponse {
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

            // CLOACI-T-0812: auto-provision initial agent(s) so a fresh tenant
            // comes up with working compute automatically. We only set the
            // desired count here; the T-0811 control loop + T-0810 actuator
            // bring the agents up. The write is BEST-EFFORT: if it fails we log
            // a warning and STILL return tenant-created success — auto-provision
            // bookkeeping must not fail tenant creation, and the control loop
            // also reconciles the row on a later tick anyway.
            let initial = initial_desired_count(state.initial_agents, state.default_max_agents);
            if initial > 0 {
                let dal = cloacina::dal::DAL::new(state.database.clone());
                match dal
                    .agent_desired()
                    .set_desired(&credentials.schema_name, initial)
                    .await
                {
                    Err(e) => {
                        warn!(
                            tenant = %credentials.schema_name,
                            initial = initial,
                            error = %e,
                            "auto-provision: failed to set initial desired_count (best-effort; \
                             tenant creation still succeeds — control loop will reconcile later)"
                        );
                    }
                    _ => {
                        info!(
                            tenant = %credentials.schema_name,
                            initial = initial,
                            "auto-provisioned initial agent desired_count"
                        );
                    }
                }
            }

            // CLOACINA-T-1031: start the tenant's runner now, so its reconciler
            // and its schedulers run before any call for the tenant. Best
            // effort: the runner pass makes it on a later tick.
            if let Err(e) =
                crate::tenant_runner_cache::ensure_runner(&state, &credentials.schema_name).await
            {
                warn!(
                    tenant = %credentials.schema_name,
                    error = %e,
                    "tenant runner not started at create; the runner pass retries"
                );
            }

            // Note: password and connection_string intentionally excluded from
            // response to prevent credential leakage (SEC-08). The caller should
            // supply their own password via the request body, or retrieve it
            // through a secure channel.
            audit_record::note(
                Some(&credentials.schema_name),
                AuditEvent::ok(
                    audit_record::actions::TENANT_CREATE,
                    credentials.schema_name.as_str(),
                ),
            );
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
            admin_error_response(
                &body.name,
                "tenant_creation_failed",
                "failed to create tenant",
                &e,
            )
            .into_response()
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
/// Before step 4 the fleet actuator removes the tenant's compiler, when it
/// runs one (CLOACINA-T-1034).
///
/// Each step emits a structured audit event with duration. Per-step
/// failures bail out — but earlier steps stay committed, so a retry
/// picks up where the failure occurred. Each step is idempotent.
///
/// Closes SEC-14 (stale state after delete) and SEC-17 (unbounded
/// caches surviving delete).
#[utoipa::path(
    delete,
    path = "/v1/tenants/{schema_name}",
    tag = "tenants",
    params(("schema_name" = String, Path, description = "Tenant schema name")),
    responses(
        (status = 200, description = "Tenant removed — teardown report", body = TenantRemovedResponse),
        (status = 400, description = "Invalid tenant name", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Admin (god-mode) key required", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Teardown step failed (safe to retry)", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn remove_tenant(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(schema_name): Path<String>,
) -> impl IntoResponse {
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
            return ApiError::internal_logged("revoke keys failed", &e).into_response();
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

    // CLOACINA-T-1034: the tenant's compiler goes with the tenant, before its
    // schema: a compiler left behind would hold a database credential and poll
    // a schema that is gone. Nothing to do when the actuator runs none. A
    // failure stops the teardown here; a retry is safe.
    if let Err(e) = state.fleet_actuator.remove_compiler(&schema_name).await {
        warn!(
            tenant_id = %schema_name,
            error = %e,
            "tenant teardown: removing the tenant's compiler failed"
        );
        audit::log_tenant_teardown_outcome(
            &schema_name,
            false,
            teardown_started.elapsed().as_millis() as u64,
        );
        return ApiError::internal_logged("remove tenant compiler failed", &e).into_response();
    }

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
            audit_record::note(
                Some(&schema_name),
                AuditEvent::ok(audit_record::actions::TENANT_DELETE, schema_name.as_str())
                    .detail(serde_json::json!({ "revoked_keys": revoked_count })),
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
            admin_error_response(
                &schema_name,
                "tenant_removal_failed",
                "failed to remove tenant",
                &e,
            )
            .into_response()
        }
    }
}

/// GET /tenants — list tenant schemas.
/// Requires admin role — only admins can enumerate tenants.
#[utoipa::path(
    get,
    path = "/v1/tenants",
    tag = "tenants",
    responses(
        (status = 200, description = "Tenant schemas", body = ListResponse<TenantSummary>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Admin (god-mode) key required", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_tenants(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
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
            ApiError::internal_logged("failed to list tenants", &e).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_error(e: &AdminError) -> ApiError {
        admin_error_response(
            "team_a",
            "tenant_creation_failed",
            "failed to create tenant",
            e,
        )
    }

    /// CLOACINA-T-1035: a tenant that exists is a 409 that names it.
    #[test]
    fn an_existing_tenant_is_a_conflict_that_names_the_tenant() {
        let e = create_error(&AdminError::TenantExists {
            name: "team_a".to_string(),
        });
        assert_eq!(e.status, StatusCode::CONFLICT);
        assert_eq!(e.code, "tenant_exists");
        assert_eq!(e.message, "tenant 'team_a' already exists");
    }

    /// CLOACINA-T-1035: the text of a SQL failure stays out of the body.
    #[test]
    fn a_sql_failure_is_a_500_with_no_sql_text() {
        let e = create_error(&AdminError::SqlExecution {
            message: "Failed to grant permissions: relation \"pg_authid\" is locked".to_string(),
        });
        assert_eq!(e.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(e.code, "internal_error");
        assert!(e.message.contains("failed to create tenant"));
        assert!(!e.message.contains("pg_authid"));
        assert!(!e.message.contains("grant"));

        let e = create_error(&AdminError::Pool(
            "connection refused (db:5432)".to_string(),
        ));
        assert_eq!(e.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!e.message.contains("5432"));
    }

    /// A name that fails validation is still a 400 that says why.
    #[test]
    fn an_invalid_name_is_a_400_with_the_reason() {
        let schema_err = cloacina::database::connection::validate_schema_name("bad-name")
            .expect_err("a hyphen is not allowed in a schema name");
        let e = create_error(&AdminError::InvalidSchema(schema_err));
        assert_eq!(e.status, StatusCode::BAD_REQUEST);
        assert_eq!(e.code, "tenant_creation_failed");
        assert!(e.message.starts_with("Invalid schema name"));
    }
}
