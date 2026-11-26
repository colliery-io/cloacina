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

//! Audit record (CLOACINA-T-1024): who did what, when, in which tenant.
//!
//! Two parts:
//!
//! - [`record`]: the one helper a route calls after its action succeeds. It
//!   writes a row to `audit_events` in the admin database. A failed write is
//!   logged and counted (`cloacina_audit_write_errors_total`); it never fails
//!   the request.
//! - The read API: `GET /v1/tenants/{tenant_id}/audit` (tenant admin) and
//!   `GET /v1/audit` (platform admin).
//!
//! The table is in the admin schema with a tenant column, so a tenant's rows
//! outlive its schema and platform actions have a place.
//!
//! An event carries names and ids only. Never put a secret value, a request
//! body or an execution context in `detail`.

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::Method,
    response::IntoResponse,
};
use tracing::warn;

use cloacina::dal::unified::audit_events::{
    AuditEvent as AuditRow, AuditEventFilter, MAX_LIST_LIMIT, NewAuditEvent, OUTCOME_DENIED,
    OUTCOME_OK,
};
use cloacina::database::universal_types::UniversalTimestamp;
use cloacina_api_types::{AuditEventInfo, AuditListResponse, ListAuditQuery};

use crate::AppState;
use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;

/// An audited action: its fixed name (`package.upload`) and the kind of
/// thing it applies to (`package`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    pub name: &'static str,
    pub target_kind: &'static str,
}

const fn action(name: &'static str, target_kind: &'static str) -> Action {
    Action { name, target_kind }
}

/// The fixed actions. One per audited route.
pub mod actions {
    use super::{Action, action};

    pub const PACKAGE_UPLOAD: Action = action("package.upload", "package");
    pub const PACKAGE_DELETE: Action = action("package.delete", "package");
    pub const PACKAGE_ACTIVATE: Action = action("package.activate", "package");
    pub const WORKFLOW_PAUSE: Action = action("workflow.pause", "workflow");
    pub const WORKFLOW_RESUME: Action = action("workflow.resume", "workflow");
    pub const WORKFLOW_RUN: Action = action("workflow.run", "workflow");
    pub const TRIGGER_PAUSE: Action = action("trigger.pause", "trigger");
    pub const TRIGGER_RESUME: Action = action("trigger.resume", "trigger");
    pub const TRIGGER_FIRE: Action = action("trigger.fire", "trigger");
    pub const INSTANCE_CREATE: Action = action("instance.create", "instance");
    pub const INSTANCE_DELETE: Action = action("instance.delete", "instance");
    pub const KEY_CREATE: Action = action("key.create", "key");
    pub const KEY_REVOKE: Action = action("key.revoke", "key");
    pub const TENANT_CREATE: Action = action("tenant.create", "tenant");
    pub const TENANT_DELETE: Action = action("tenant.delete", "tenant");
    pub const SECRET_CREATE: Action = action("secret.create", "secret");
    pub const SECRET_ROTATE: Action = action("secret.rotate", "secret");
    pub const SECRET_DELETE: Action = action("secret.delete", "secret");
    pub const FLEET_PROVISION: Action = action("fleet.provision", "fleet");
    pub const FLEET_DEPROVISION: Action = action("fleet.deprovision", "fleet");
    pub const LIMIT_SET: Action = action("limit.set", "limit");
    pub const LIMIT_CLEAR: Action = action("limit.clear", "limit");
    pub const ACCOUNT_CREATE: Action = action("account.create", "account");
    pub const ACCOUNT_DISABLE: Action = action("account.disable", "account");
    pub const ACCOUNT_PASSWORD_RESET: Action = action("account.password_reset", "account");
}

/// One action to record. Build it with [`AuditEvent::ok`], add facts with
/// [`AuditEvent::detail`], and pass it to [`record`].
#[derive(Debug, Clone)]
pub struct AuditEvent {
    action: Action,
    target: Option<String>,
    outcome: &'static str,
    detail: serde_json::Value,
}

impl AuditEvent {
    /// An action that succeeded on `target` (a name or an id).
    pub fn ok(action: Action, target: impl Into<String>) -> Self {
        Self {
            action,
            target: Some(target.into()),
            outcome: OUTCOME_OK,
            detail: serde_json::json!({}),
        }
    }

    /// Facts of the action as a small JSON object (a version, an execution
    /// id). Names and ids only.
    pub fn detail(mut self, detail: serde_json::Value) -> Self {
        self.detail = detail;
        self
    }
}

/// Write one audit row for `event`, done by `auth`, in `tenant` (`None` for a
/// platform action). Never fails the request: an error is logged and counted
/// in `cloacina_audit_write_errors_total`.
pub async fn record(
    state: &AppState,
    auth: &AuthenticatedKey,
    tenant: Option<&str>,
    event: AuditEvent,
) {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    // The person behind a key minted at login is on the key row
    // (`issued_via`), not on `AuthenticatedKey`.
    let actor_identity = match dal.api_keys().get_key(auth.key_id).await {
        Ok(info) => info.and_then(|k| k.issued_via),
        Err(e) => {
            warn!(key_id = %auth.key_id, error = %e, "audit: could not read the key's identity");
            None
        }
    };
    let row = NewAuditEvent {
        tenant_id: tenant.map(str::to_string),
        actor_key_id: auth.key_id.to_string(),
        actor_key_name: auth.name.clone(),
        actor_identity,
        action: event.action.name.to_string(),
        target_kind: event.action.target_kind.to_string(),
        target: event.target,
        outcome: event.outcome.to_string(),
        detail: event.detail,
    };
    if let Err(e) = dal.audit_events().insert(row).await {
        metrics::counter!("cloacina_audit_write_errors_total").increment(1);
        warn!(
            action = event.action.name,
            tenant = tenant.unwrap_or("<none>"),
            error = %e,
            "audit: write failed; the action itself is not affected"
        );
    }
}

/// The audited action behind a route, `path` is
/// the route as registered (no `/v1` prefix). `None` for a route that writes
/// no audit row.
pub fn action_for_route(method: &Method, path: &str) -> Option<Action> {
    use actions::*;
    let hit = match (method.as_str(), path) {
        ("POST", "/tenants/{tenant_id}/workflows") => PACKAGE_UPLOAD,
        ("DELETE", "/tenants/{tenant_id}/workflows/{name}/{version}") => PACKAGE_DELETE,
        ("POST", "/tenants/{tenant_id}/workflows/{name}/versions/{version}/activate") => {
            PACKAGE_ACTIVATE
        }
        ("POST", "/tenants/{tenant_id}/workflows/{name}/pause") => WORKFLOW_PAUSE,
        ("POST", "/tenants/{tenant_id}/workflows/{name}/resume") => WORKFLOW_RESUME,
        ("POST", "/tenants/{tenant_id}/workflows/{name}/execute") => WORKFLOW_RUN,
        ("POST", "/tenants/{tenant_id}/triggers/{name}/pause") => TRIGGER_PAUSE,
        ("POST", "/tenants/{tenant_id}/triggers/{name}/resume") => TRIGGER_RESUME,
        ("POST", "/tenants/{tenant_id}/triggers/{name}/fire") => TRIGGER_FIRE,
        ("POST", "/tenants/{tenant_id}/workflows/{name}/instances") => INSTANCE_CREATE,
        ("DELETE", "/tenants/{tenant_id}/workflows/{name}/instances/{instance}") => INSTANCE_DELETE,
        ("POST", "/auth/keys") | ("POST", "/tenants/{tenant_id}/keys") => KEY_CREATE,
        ("DELETE", "/auth/keys/{key_id}") | ("DELETE", "/tenants/{tenant_id}/keys/{key_id}") => {
            KEY_REVOKE
        }
        ("POST", "/tenants") => TENANT_CREATE,
        ("DELETE", "/tenants/{schema_name}") => TENANT_DELETE,
        ("POST", "/tenants/{tenant_id}/secrets") => SECRET_CREATE,
        ("PUT", "/tenants/{tenant_id}/secrets/{name}") => SECRET_ROTATE,
        ("DELETE", "/tenants/{tenant_id}/secrets/{name}") => SECRET_DELETE,
        ("POST", "/tenants/{tenant_id}/fleet/provision") => FLEET_PROVISION,
        ("POST", "/tenants/{tenant_id}/fleet/deprovision") => FLEET_DEPROVISION,
        ("POST", "/tenants/{tenant_id}/limits") => LIMIT_SET,
        ("DELETE", "/tenants/{tenant_id}/limits") => LIMIT_CLEAR,
        ("POST", "/tenants/{tenant_id}/accounts") => ACCOUNT_CREATE,
        ("DELETE", "/tenants/{tenant_id}/accounts/{account_id}") => ACCOUNT_DISABLE,
        ("POST", "/tenants/{tenant_id}/accounts/{account_id}/password") => ACCOUNT_PASSWORD_RESET,
        _ => return None,
    };
    Some(hit)
}

/// Record an authorization denial on an audited route. Called by the authz
/// middleware. A denial on a route that writes no audit row (every read)
/// records nothing. `tenant` is the tenant named in the path, when the route
/// has one. `request_path` is the URL path: it holds names and ids only.
pub async fn record_denied(
    state: &AppState,
    auth: &AuthenticatedKey,
    tenant: Option<&str>,
    method: &Method,
    route: &str,
    request_path: &str,
    reason: &str,
) {
    let Some(action) = action_for_route(method, route) else {
        return;
    };
    let event = AuditEvent {
        action,
        target: None,
        outcome: OUTCOME_DENIED,
        detail: serde_json::json!({ "reason": reason, "path": request_path }),
    };
    record(state, auth, tenant, event).await;
}

/// The `detail` of a `package.upload` row: the package name and version as
/// registered, the content hash and the size. Name and version are absent
/// when the row cannot be read back.
pub async fn upload_detail(
    registry: &cloacina::registry::workflow_registry::WorkflowRegistryImpl<
        cloacina::dal::UnifiedRegistryStorage,
    >,
    package_id: uuid::Uuid,
    content_hash: String,
    size_bytes: usize,
) -> serde_json::Value {
    let mut detail = serde_json::json!({ "content_hash": content_hash, "size_bytes": size_bytes });
    if let Ok(Some(ins)) = registry.inspect_package_by_id(package_id).await {
        detail["package_name"] = ins.metadata.package_name.into();
        detail["version"] = ins.metadata.version.into();
    }
    detail
}

/// Who uploaded each package of `tenant`, by package id, from the audit
/// record. Empty on a read error (logged): the caller shows no uploader.
pub async fn package_uploaders(
    state: &AppState,
    tenant: &str,
    package_ids: Vec<String>,
) -> std::collections::HashMap<String, String> {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal
        .audit_events()
        .package_uploaders(tenant, package_ids)
        .await
    {
        Ok(map) => map,
        Err(e) => {
            warn!(tenant, error = %e, "audit: could not read package uploaders");
            Default::default()
        }
    }
}

/// Who uploaded one package of `tenant`, from the audit record.
pub async fn package_uploader(
    state: &AppState,
    tenant: &str,
    package_id: String,
) -> Option<String> {
    package_uploaders(state, tenant, vec![package_id.clone()])
        .await
        .remove(&package_id)
}

fn to_info(r: AuditRow) -> AuditEventInfo {
    AuditEventInfo {
        id: r.id,
        created_at: r.created_at.0.to_rfc3339(),
        tenant_id: r.tenant_id,
        actor_key_id: r.actor_key_id,
        actor_key_name: r.actor_key_name,
        actor_identity: r.actor_identity,
        action: r.action,
        target_kind: r.target_kind,
        target: r.target,
        outcome: r.outcome,
        detail: r.detail,
    }
}

fn parse_time(field: &str, value: Option<&str>) -> Result<Option<UniversalTimestamp>, ApiError> {
    match value {
        None => Ok(None),
        Some(v) => chrono::DateTime::parse_from_rfc3339(v)
            .map(|t| Some(UniversalTimestamp(t.with_timezone(&chrono::Utc))))
            .map_err(|_| {
                ApiError::bad_request(
                    "invalid_request",
                    format!("{field} must be an RFC 3339 time"),
                )
            }),
    }
}

/// Turn the query into a DAL filter for `tenant` (`None` reads every tenant).
fn filter_from(q: &ListAuditQuery, tenant: Option<String>) -> Result<AuditEventFilter, ApiError> {
    if let Some(limit) = q.limit
        && !(1..=MAX_LIST_LIMIT).contains(&limit)
    {
        return Err(ApiError::bad_request(
            "invalid_pagination",
            format!("limit must be 1..={MAX_LIST_LIMIT}"),
        ));
    }
    Ok(AuditEventFilter {
        tenant_id: tenant,
        action: q.action.clone(),
        actor: q.actor.clone(),
        since: parse_time("since", q.since.as_deref())?,
        until: parse_time("until", q.until.as_deref())?,
        before_id: q.cursor,
        limit: q.limit,
    })
}

async fn list_with(state: &AppState, filter: AuditEventFilter) -> axum::response::Response {
    let limit = filter
        .limit
        .unwrap_or(cloacina::dal::unified::audit_events::DEFAULT_LIST_LIMIT);
    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal.audit_events().list(filter).await {
        Ok(rows) => {
            // A full page may have older rows behind it.
            let next_cursor = if rows.len() as i64 == limit {
                rows.last().map(|r| r.id)
            } else {
                None
            };
            let items: Vec<AuditEventInfo> = rows.into_iter().map(to_info).collect();
            Json(AuditListResponse {
                total: items.len(),
                items,
                next_cursor,
            })
            .into_response()
        }
        Err(e) => {
            warn!("Failed to list audit events: {}", e);
            ApiError::internal(format!("{}", e)).into_response()
        }
    }
}

/// GET /tenants/:tenant_id/audit — the audit record of one tenant, newest
/// first. Tenant admin.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/audit",
    tag = "audit",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ListAuditQuery,
    ),
    responses(
        (status = 200, description = "A page of audit rows, newest first", body = AuditListResponse),
        (status = 400, description = "Invalid limit or time", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or role denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_tenant_audit(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
    Query(q): Query<ListAuditQuery>,
) -> impl IntoResponse {
    // The path names the tenant; the `tenant` query field is not read here.
    match filter_from(&q, Some(tenant_id)) {
        Ok(filter) => list_with(&state, filter).await,
        Err(e) => e.into_response(),
    }
}

/// GET /audit — the audit record of every tenant and of platform actions,
/// newest first. Platform admin.
#[utoipa::path(
    get,
    path = "/v1/audit",
    tag = "audit",
    params(ListAuditQuery),
    responses(
        (status = 200, description = "A page of audit rows, newest first", body = AuditListResponse),
        (status = 400, description = "Invalid limit or time", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Platform admin required", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_audit(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Query(q): Query<ListAuditQuery>,
) -> impl IntoResponse {
    match filter_from(&q, q.tenant.clone()) {
        Ok(filter) => list_with(&state, filter).await,
        Err(e) => e.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every audited route is a real route: it has a row in the
    /// authorization table. A renamed route would otherwise stop recording
    /// denials with no test failing.
    #[test]
    fn audit_route_map_names_only_routes_of_the_authz_table() {
        let table = crate::routes::authz::build_authz_table();
        let mut audited = 0;
        for (method, path) in table.keys() {
            if action_for_route(method, path).is_some() {
                audited += 1;
                assert_ne!(method, Method::GET, "a read is not audited: {path}");
            }
        }
        // 25 actions; key create and key revoke each have two routes.
        assert_eq!(audited, 27);
    }

    #[test]
    fn audit_actions_are_named_kind_dot_verb() {
        assert_eq!(actions::PACKAGE_UPLOAD.name, "package.upload");
        assert_eq!(actions::PACKAGE_UPLOAD.target_kind, "package");
        assert_eq!(
            actions::ACCOUNT_PASSWORD_RESET.name,
            "account.password_reset"
        );
        assert_eq!(actions::ACCOUNT_PASSWORD_RESET.target_kind, "account");
        // The DAL reads uploads by this name.
        assert_eq!(
            actions::PACKAGE_UPLOAD.name,
            cloacina::dal::unified::audit_events::ACTION_PACKAGE_UPLOAD
        );
    }

    #[test]
    fn audit_route_map_gives_action_and_kind() {
        assert_eq!(
            action_for_route(&Method::POST, "/tenants/{tenant_id}/workflows"),
            Some(actions::PACKAGE_UPLOAD)
        );
        assert_eq!(
            action_for_route(&Method::PUT, "/tenants/{tenant_id}/secrets/{name}"),
            Some(actions::SECRET_ROTATE)
        );
        assert_eq!(
            action_for_route(&Method::GET, "/tenants/{tenant_id}/workflows"),
            None
        );
        assert_eq!(action_for_route(&Method::POST, "/auth/refresh"), None);
    }

    #[test]
    fn audit_filter_checks_limit_and_times() {
        let ok = filter_from(
            &ListAuditQuery {
                action: Some("workflow.run".into()),
                since: Some("2026-01-02T03:04:05Z".into()),
                limit: Some(10),
                cursor: Some(99),
                ..Default::default()
            },
            Some("acme".into()),
        )
        .unwrap();
        assert_eq!(ok.tenant_id.as_deref(), Some("acme"));
        assert_eq!(ok.action.as_deref(), Some("workflow.run"));
        assert_eq!(ok.before_id, Some(99));
        assert_eq!(ok.limit, Some(10));
        assert!(ok.since.is_some());
        assert!(ok.until.is_none());

        for bad in [0, -1, MAX_LIST_LIMIT + 1] {
            let q = ListAuditQuery {
                limit: Some(bad),
                ..Default::default()
            };
            assert!(filter_from(&q, None).is_err(), "limit {bad}");
        }
        let q = ListAuditQuery {
            until: Some("yesterday".into()),
            ..Default::default()
        };
        assert!(filter_from(&q, None).is_err());
    }

    #[test]
    fn audit_event_builder_sets_outcome_and_detail() {
        let e = AuditEvent::ok(actions::WORKFLOW_RUN, "etl")
            .detail(serde_json::json!({"execution_id": "e-1"}));
        assert_eq!(e.action.name, "workflow.run");
        assert_eq!(e.action.target_kind, "workflow");
        assert_eq!(e.target.as_deref(), Some("etl"));
        assert_eq!(e.outcome, "ok");
        assert_eq!(e.detail["execution_id"], "e-1");
    }
}
