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

//! Agent pools (CLOACINA-T-1021).
//!
//! A tenant's managed agents are grouped in pools; each pool has its own size
//! of agent and its own Deployment. These routes read and set a pool's
//! settings: resource requests and limits, max concurrency, image, service
//! account and its annotations, extra egress rules, and the agent count of a
//! pool other than `default`.
//!
//! Every tenant has the pool `default`. It is the pool the autoscaler scales
//! (its max is the tenant's agent limit, `/v1/tenants/{tenant_id}/limits`) and
//! the pool of every task that names none. Another pool can be created and
//! sized; it runs a fixed number of agents, and gets the tasks that name it
//! (CLOACINA-T-1027).
//!
//! AuthZ (enforced by the authz table):
//! - `PUT` / `DELETE` → **god-only** (`platform + admin`), as the agent limit.
//! - `GET` (one pool, or the list) → **tenant-scoped read**.
//!
//! The settings are validated here, so that a value Kubernetes would refuse is
//! a `400` at the API and not a failed reconcile later.

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use tracing::warn;

use cloacina_api_types::{
    AgentPoolInfo, AgentPoolSettings, DEFAULT_AGENT_POOL, ListResponse, agent_pool_of,
};

use crate::AppState;
use crate::actuator::kubernetes::AgentResources;
use crate::actuator::settings;
use crate::routes::audit::{self as audit_record, AuditEvent};
use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;

/// A tenant's pools, `default` first and always present.
async fn load_pools(
    state: &AppState,
    tenant_id: &str,
) -> Result<Vec<(String, AgentPoolSettings)>, ApiError> {
    settings::load_pools(&state.database, tenant_id)
        .await
        .map_err(|e| {
            warn!(tenant = %tenant_id, "agent pools read failed: {}", e);
            ApiError::internal("failed to read agent pools")
        })
}

/// Live agents of the tenant, counted by pool.
async fn live_by_pool(
    state: &AppState,
    tenant_id: &str,
) -> Result<std::collections::HashMap<String, u32>, ApiError> {
    let live = state.agent_registry.live_agents().await.map_err(|e| {
        warn!("agent roster read failed: {}", e);
        ApiError::internal("failed to read agent roster")
    })?;
    let mut counts = std::collections::HashMap::new();
    for agent in live
        .iter()
        .filter(|a| a.tenant_id.as_deref() == Some(tenant_id))
    {
        *counts
            .entry(agent_pool_of(&agent.capabilities).to_string())
            .or_insert(0) += 1;
    }
    Ok(counts)
}

/// The desired agent count of the pool `default`: the tenant's desired count.
async fn default_desired(state: &AppState, tenant_id: &str) -> Result<u32, ApiError> {
    cloacina::dal::DAL::new(state.database.clone())
        .agent_desired()
        .get_desired(tenant_id)
        .await
        .map_err(|e| {
            warn!("desired count lookup failed: {}", e);
            ApiError::internal("failed to read desired count")
        })
}

/// Build the view of each pool in `pools`.
async fn pool_infos(
    state: &AppState,
    tenant_id: &str,
    pools: Vec<(String, AgentPoolSettings)>,
) -> Result<Vec<AgentPoolInfo>, ApiError> {
    let live = live_by_pool(state, tenant_id).await?;
    let default_desired = default_desired(state, tenant_id).await?;
    Ok(pools
        .into_iter()
        .map(|(pool, settings)| AgentPoolInfo {
            tenant_id: tenant_id.to_string(),
            desired_agents: if pool == DEFAULT_AGENT_POOL {
                default_desired
            } else {
                settings::fixed_agent_count(&settings)
            },
            live_agents: live.get(&pool).copied().unwrap_or(0),
            pool,
            settings,
        })
        .collect())
}

/// One pool's view, or a `404` when the tenant has no such pool.
async fn pool_info(
    state: &AppState,
    tenant_id: &str,
    pool: &str,
) -> Result<AgentPoolInfo, ApiError> {
    let pools = load_pools(state, tenant_id).await?;
    let one: Vec<_> = pools.into_iter().filter(|(name, _)| name == pool).collect();
    pool_infos(state, tenant_id, one)
        .await?
        .pop()
        .ok_or_else(|| pool_not_found(pool))
}

fn pool_not_found(pool: &str) -> ApiError {
    ApiError::not_found(
        "pool_not_found",
        format!("the tenant has no agent pool '{pool}'"),
    )
}

/// Two pools of one tenant that name the same ServiceAccount must give it the
/// same annotations: the actuator writes the ServiceAccount once per pool, and
/// two sets would overwrite each other on every reconcile. Returns the reason
/// when `settings` for `pool` disagrees with another pool.
fn service_account_conflict(
    pools: &[(String, AgentPoolSettings)],
    pool: &str,
    settings: &AgentPoolSettings,
) -> Option<String> {
    let name = settings.service_account_name.as_deref()?;
    pools
        .iter()
        .filter(|(other, _)| other != pool)
        .find(|(_, other)| {
            other.service_account_name.as_deref() == Some(name)
                && other.service_account_annotations != settings.service_account_annotations
        })
        .map(|(other, _)| {
            format!(
                "service_account_annotations: the pool '{other}' names the service account \
                 '{name}' with other annotations; give both pools the same annotations or \
                 another service account"
            )
        })
}

/// Why a pool cannot be removed now, if it cannot: it still has agents. The
/// rule is to refuse while the pool has agents, not to drain it: set its
/// `max_agents` to 0, wait until no agent of it is alive, then remove it.
fn removal_blocker(pool: &str, settings: &AgentPoolSettings, live_agents: u32) -> Option<String> {
    let wanted = settings::fixed_agent_count(settings);
    if wanted > 0 {
        return Some(format!(
            "the pool '{pool}' is set to {wanted} agents; set max_agents to 0 first"
        ));
    }
    if live_agents > 0 {
        return Some(format!(
            "the pool '{pool}' still has {live_agents} live agents; try again when they have \
             stopped"
        ));
    }
    None
}

/// `GET /v1/tenants/{tenant_id}/pools` — the tenant's agent pools. Always
/// includes `default`. Tenant-scoped read.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/pools",
    tag = "fleet",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    responses(
        (status = 200, description = "The tenant's agent pools, `default` first", body = ListResponse<AgentPoolInfo>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_pools(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    let result = async {
        let pools = load_pools(&state, &tenant_id).await?;
        pool_infos(&state, &tenant_id, pools).await
    }
    .await;
    match result {
        Ok(infos) => Json(ListResponse::new(infos)).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `GET /v1/tenants/{tenant_id}/pools/{pool}` — one agent pool. Tenant-scoped
/// read.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/pools/{pool}",
    tag = "fleet",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("pool" = String, Path, description = "Pool name; every tenant has `default`"),
    ),
    responses(
        (status = 200, description = "The pool", body = AgentPoolInfo),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "The tenant has no such pool", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_pool(
    State(state): State<AppState>,
    Path((tenant_id, pool)): Path<(String, String)>,
) -> impl IntoResponse {
    match pool_info(&state, &tenant_id, &pool).await {
        Ok(info) => Json(info).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `PUT /v1/tenants/{tenant_id}/pools/{pool}` — create a pool, or replace its
/// settings as a whole. God-only. The actuator applies the settings on its
/// next reconcile; a changed setting rolls the pool's agents.
#[utoipa::path(
    put,
    path = "/v1/tenants/{tenant_id}/pools/{pool}",
    tag = "fleet",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("pool" = String, Path, description = "Pool name: lowercase letters, digits and `-`, 40 characters at most"),
    ),
    request_body = AgentPoolSettings,
    responses(
        (status = 200, description = "Settings stored; returns the pool", body = AgentPoolInfo),
        (status = 400, description = "A setting or the pool name is not valid", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Platform-admin required", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn put_pool(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, pool)): Path<(String, String)>,
    Json(body): Json<AgentPoolSettings>,
) -> impl IntoResponse {
    let result = async {
        settings::validate(&pool, &body, &AgentResources::from_env())
            .map_err(|e| ApiError::bad_request("invalid_pool_settings", e))?;
        let pools = load_pools(&state, &tenant_id).await?;
        if let Some(conflict) = service_account_conflict(&pools, &pool, &body) {
            return Err(ApiError::bad_request("invalid_pool_settings", conflict));
        }

        let dal = cloacina::dal::DAL::new(state.database.clone());
        let write = if pool == DEFAULT_AGENT_POOL && body == AgentPoolSettings::default() {
            // `default` with no settings needs no row.
            dal.agent_pools()
                .delete(&tenant_id, &pool)
                .await
                .map(|_| ())
        } else {
            let text = serde_json::to_string(&body).map_err(|e| {
                warn!("agent pool settings do not serialize: {}", e);
                ApiError::internal("failed to store agent pool")
            })?;
            dal.agent_pools().set(&tenant_id, &pool, &text).await
        };
        write.map_err(|e| {
            warn!("set agent pool failed: {}", e);
            ApiError::internal("failed to store agent pool")
        })?;
        // The names of the settings that were given, not their values.
        let given: Vec<String> = serde_json::to_value(&body)
            .ok()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default();
        audit_record::record(
            &state,
            &auth,
            Some(&tenant_id),
            AuditEvent::ok(audit_record::actions::POOL_SET, pool.as_str())
                .detail(serde_json::json!({ "settings": given })),
        )
        .await;
        pool_info(&state, &tenant_id, &pool).await
    }
    .await;
    match result {
        Ok(info) => (StatusCode::OK, Json(info)).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `DELETE /v1/tenants/{tenant_id}/pools/{pool}` — remove a pool. God-only.
///
/// A pool is removed only when it has no agents: its `max_agents` is 0 and no
/// agent of it is alive (`409` otherwise). Then its Deployment and its
/// NetworkPolicy are deleted and its row is removed. For `default` the call
/// clears the pool's settings (back to the server-wide defaults); the pool
/// itself stays.
#[utoipa::path(
    delete,
    path = "/v1/tenants/{tenant_id}/pools/{pool}",
    tag = "fleet",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("pool" = String, Path, description = "Pool name"),
    ),
    responses(
        (status = 204, description = "Pool removed (for `default`: settings cleared)"),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Platform-admin required", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "The tenant has no such pool", body = cloacina_api_types::ErrorBody),
        (status = 409, description = "The pool still has agents", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn delete_pool(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, pool)): Path<(String, String)>,
) -> impl IntoResponse {
    let result = async {
        let dal = cloacina::dal::DAL::new(state.database.clone());
        let delete_row = || async {
            dal.agent_pools()
                .delete(&tenant_id, &pool)
                .await
                .map_err(|e| {
                    warn!("delete agent pool failed: {}", e);
                    ApiError::internal("failed to remove agent pool")
                })
        };
        let audit = || async {
            audit_record::record(
                &state,
                &auth,
                Some(&tenant_id),
                AuditEvent::ok(audit_record::actions::POOL_DELETE, pool.as_str()),
            )
            .await;
        };
        if pool == DEFAULT_AGENT_POOL {
            delete_row().await?;
            audit().await;
            return Ok(());
        }

        let info = pool_info(&state, &tenant_id, &pool).await?;
        if let Some(why) = removal_blocker(&pool, &info.settings, info.live_agents) {
            return Err(ApiError::conflict("pool_has_agents", why));
        }
        // Objects first, row last: if the actuator fails the pool stays listed
        // and the call can be repeated.
        state
            .fleet_actuator
            .remove_pool(&tenant_id, &pool)
            .await
            .map_err(|e| {
                warn!(tenant = %tenant_id, pool = %pool, "remove pool failed: {}", e);
                ApiError::internal("failed to remove the pool's agents")
            })?;
        delete_row().await?;
        audit().await;
        Ok(())
    }
    .await;
    match result {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => e.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn with_sa(name: &str, annotation: Option<&str>) -> AgentPoolSettings {
        let mut annotations = BTreeMap::new();
        if let Some(v) = annotation {
            annotations.insert("example.com/role".to_string(), v.to_string());
        }
        AgentPoolSettings {
            service_account_name: Some(name.to_string()),
            service_account_annotations: annotations,
            ..Default::default()
        }
    }

    #[test]
    fn pools_can_share_a_service_account_with_the_same_annotations() {
        let pools = vec![
            ("default".to_string(), AgentPoolSettings::default()),
            ("big".to_string(), with_sa("team-data", Some("a"))),
        ];
        // Same name, same annotations: fine.
        assert!(
            service_account_conflict(&pools, "small", &with_sa("team-data", Some("a"))).is_none()
        );
        // Another name: fine.
        assert!(
            service_account_conflict(&pools, "small", &with_sa("team-other", Some("b"))).is_none()
        );
        // No service account: fine.
        assert!(service_account_conflict(&pools, "small", &AgentPoolSettings::default()).is_none());
        // A pool does not conflict with its own stored settings.
        assert!(
            service_account_conflict(&pools, "big", &with_sa("team-data", Some("b"))).is_none()
        );
    }

    #[test]
    fn pools_cannot_give_one_service_account_two_sets_of_annotations() {
        let pools = vec![("big".to_string(), with_sa("team-data", Some("a")))];
        let why = service_account_conflict(&pools, "small", &with_sa("team-data", Some("b")))
            .expect("conflict");
        assert!(why.contains("'big'"), "{why}");
        assert!(service_account_conflict(&pools, "small", &with_sa("team-data", None)).is_some());
    }

    #[test]
    fn a_pool_is_removed_only_when_it_has_no_agents() {
        let sized = |n: u32| AgentPoolSettings {
            max_agents: Some(n),
            ..Default::default()
        };
        // Still set to run agents.
        assert!(
            removal_blocker("big", &sized(2), 0)
                .unwrap()
                .contains("set max_agents to 0")
        );
        // Set to 0 but agents are still alive (the scale-down has not finished).
        assert!(
            removal_blocker("big", &sized(0), 1)
                .unwrap()
                .contains("still has 1 live agents")
        );
        // Set to 0 and none alive.
        assert!(removal_blocker("big", &sized(0), 0).is_none());
    }
}
