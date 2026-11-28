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

//! Authorization model (CLOACINA-T-1055; first built in CLOACI-I-0118).
//!
//! The unit is a [`Permission`]: one kind of action on one kind of resource.
//! A [`Role`] is a named set of permissions, fixed in this file. A route has
//! one [`Row`] in the [`AuthzTable`]: the [`Scope`] it applies in, the
//! permission it needs, the credential it accepts and the audit row it
//! writes.
//!
//! - [`evaluate`] is the matcher: pure, total, default-deny.
//! - [`guard`] is the one middleware of every `/v1` route, the WebSocket
//!   upgrades included: it finds the row, authenticates, runs the matcher,
//!   calls the handler and writes the audit row.
//! - [`check_coverage`] compares the routes the router registered with the
//!   table. The server does not start when a route has no row.
//! - [`render_markdown`] gives the tables of the security model page; a test
//!   keeps the page equal to the code.
//!
//! Fail closed: an unknown role holds no permission; a route with no row is
//! refused; the platform admin flag grants every permission but
//! [`Permission::AgentAct`].

use std::collections::HashMap;

use axum::Router;
use axum::extract::{FromRequestParts, MatchedPath, Query, RawPathParams, Request, State};
use axum::handler::Handler;
use axum::http::{Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::MethodRouter;
use tracing::warn;

use crate::AppState;
use crate::routes::audit::{self, Action, AuditEvent, actions};
use crate::routes::auth::{AuthenticatedKey, record_auth_span_fields, validate_token};
use crate::routes::error::ApiError;

// ---------------------------------------------------------------------------
// Permissions, roles, limits
// ---------------------------------------------------------------------------

/// One kind of action on one kind of resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Permission {
    SessionSelf,
    WsTicket,
    WorkflowsRead,
    PackagesPublish,
    WorkflowsOperate,
    WorkflowsRun,
    TriggersRead,
    TriggersOperate,
    ExecutionsRead,
    GraphsRead,
    ReactorsOperate,
    AccumulatorsInject,
    OpsRead,
    LimitsRead,
    PoolsRead,
    FleetRead,
    FleetScale,
    AgentsRead,
    KeysManage,
    AccountsManage,
    SecretsManage,
    TrustedKeysManage,
    AuditRead,
    AgentAct,
    TenantsManage,
    CompilerRead,
    LimitsManage,
    PoolsManage,
}

impl Permission {
    /// Every permission, in the order of the docs table.
    pub const ALL: [Permission; 28] = [
        Permission::SessionSelf,
        Permission::WsTicket,
        Permission::WorkflowsRead,
        Permission::PackagesPublish,
        Permission::WorkflowsOperate,
        Permission::WorkflowsRun,
        Permission::TriggersRead,
        Permission::TriggersOperate,
        Permission::ExecutionsRead,
        Permission::GraphsRead,
        Permission::ReactorsOperate,
        Permission::AccumulatorsInject,
        Permission::OpsRead,
        Permission::LimitsRead,
        Permission::PoolsRead,
        Permission::FleetRead,
        Permission::FleetScale,
        Permission::AgentsRead,
        Permission::KeysManage,
        Permission::AccountsManage,
        Permission::SecretsManage,
        Permission::TrustedKeysManage,
        Permission::AuditRead,
        Permission::AgentAct,
        Permission::TenantsManage,
        Permission::CompilerRead,
        Permission::LimitsManage,
        Permission::PoolsManage,
    ];

    /// The name in the docs and in deny logs: `resource.verb`.
    pub const fn name(self) -> &'static str {
        match self {
            Permission::SessionSelf => "session.self",
            Permission::WsTicket => "ws.ticket",
            Permission::WorkflowsRead => "workflows.read",
            Permission::PackagesPublish => "packages.publish",
            Permission::WorkflowsOperate => "workflows.operate",
            Permission::WorkflowsRun => "workflows.run",
            Permission::TriggersRead => "triggers.read",
            Permission::TriggersOperate => "triggers.operate",
            Permission::ExecutionsRead => "executions.read",
            Permission::GraphsRead => "graphs.read",
            Permission::ReactorsOperate => "reactors.operate",
            Permission::AccumulatorsInject => "accumulators.inject",
            Permission::OpsRead => "ops.read",
            Permission::LimitsRead => "limits.read",
            Permission::PoolsRead => "pools.read",
            Permission::FleetRead => "fleet.read",
            Permission::FleetScale => "fleet.scale",
            Permission::AgentsRead => "agents.read",
            Permission::KeysManage => "keys.manage",
            Permission::AccountsManage => "accounts.manage",
            Permission::SecretsManage => "secrets.manage",
            Permission::TrustedKeysManage => "trusted_keys.manage",
            Permission::AuditRead => "audit.read",
            Permission::AgentAct => "agent.act",
            Permission::TenantsManage => "tenants.manage",
            Permission::CompilerRead => "compiler.read",
            Permission::LimitsManage => "limits.manage",
            Permission::PoolsManage => "pools.manage",
        }
    }

    /// One line for the docs table.
    pub const fn allows(self) -> &'static str {
        match self {
            Permission::SessionSelf => "Refresh, log out and read the role of the caller's own key",
            Permission::WsTicket => "Get a WebSocket ticket",
            Permission::WorkflowsRead => {
                "List and read packages, their source, versions, instances and backfills"
            }
            Permission::PackagesPublish => "Upload a package, activate a version, delete a version",
            Permission::WorkflowsOperate => {
                "Pause and resume a workflow; create and delete an instance"
            }
            Permission::WorkflowsRun => {
                "Run a workflow by hand; create, cancel and retry a backfill"
            }
            Permission::TriggersRead => "List and read schedules and triggers",
            Permission::TriggersOperate => "Pause, resume and fire a trigger",
            Permission::ExecutionsRead => {
                "List and read executions, their events and tasks; the `exec_events:<id>` stream"
            }
            Permission::GraphsRead => {
                "Read graphs, reactors and accumulators; connect to a reactor's WebSocket and read its state"
            }
            Permission::ReactorsOperate => "Fire, pause and resume a reactor",
            Permission::AccumulatorsInject => "Inject an event into an accumulator",
            Permission::OpsRead => "The `ops_metrics:global` stream",
            Permission::LimitsRead => "Read the tenant's agent limit",
            Permission::PoolsRead => "Read the tenant's agent pools",
            Permission::FleetRead => "Read the tenant's fleet view",
            Permission::FleetScale => "Provision and deprovision agents",
            Permission::AgentsRead => "Read the agent roster",
            Permission::KeysManage => "Create, list and revoke API keys",
            Permission::AccountsManage => {
                "Create, list and disable local accounts; reset a password"
            }
            Permission::SecretsManage => "Read secret metadata; create, rotate and delete secrets",
            Permission::TrustedKeysManage => {
                "Add, list and revoke the tenant's trusted package-signing keys"
            }
            Permission::AuditRead => "Read the audit record",
            Permission::AgentAct => {
                "Act as an agent: register, heartbeat, top up one-time keys, report a result, fetch packages; the `agent:<id>` stream"
            }
            Permission::TenantsManage => "Create, list and delete tenants",
            Permission::CompilerRead => "Read the compiler's status",
            Permission::LimitsManage => "Set and clear a tenant's agent limit",
            Permission::PoolsManage => "Set and delete an agent pool",
        }
    }
}

/// The permissions the platform admin flag does NOT grant.
///
/// A person's key is not an agent: the bootstrap key cannot register as one
/// and cannot take an agent's stream. Only a key with the role `agent` acts
/// as an agent.
pub const NOT_GRANTED_BY_PLATFORM_FLAG: &[Permission] = &[Permission::AgentAct];

const READ: &[Permission] = &[
    Permission::SessionSelf,
    Permission::WsTicket,
    Permission::WorkflowsRead,
    Permission::TriggersRead,
    Permission::ExecutionsRead,
    Permission::GraphsRead,
    Permission::OpsRead,
    Permission::LimitsRead,
    Permission::PoolsRead,
    Permission::FleetRead,
];

const WRITE: &[Permission] = &[
    Permission::SessionSelf,
    Permission::WsTicket,
    Permission::WorkflowsRead,
    Permission::TriggersRead,
    Permission::ExecutionsRead,
    Permission::GraphsRead,
    Permission::OpsRead,
    Permission::LimitsRead,
    Permission::PoolsRead,
    Permission::FleetRead,
    Permission::PackagesPublish,
    Permission::WorkflowsOperate,
    Permission::WorkflowsRun,
    Permission::TriggersOperate,
    Permission::ReactorsOperate,
    Permission::AccumulatorsInject,
];

const ADMIN: &[Permission] = &[
    Permission::SessionSelf,
    Permission::WsTicket,
    Permission::WorkflowsRead,
    Permission::TriggersRead,
    Permission::ExecutionsRead,
    Permission::GraphsRead,
    Permission::OpsRead,
    Permission::LimitsRead,
    Permission::PoolsRead,
    Permission::FleetRead,
    Permission::PackagesPublish,
    Permission::WorkflowsOperate,
    Permission::WorkflowsRun,
    Permission::TriggersOperate,
    Permission::ReactorsOperate,
    Permission::AccumulatorsInject,
    Permission::FleetScale,
    Permission::AgentsRead,
    Permission::KeysManage,
    Permission::AccountsManage,
    Permission::SecretsManage,
    Permission::TrustedKeysManage,
    Permission::AuditRead,
];

const AGENT: &[Permission] = &[Permission::AgentAct, Permission::WsTicket];

/// A named set of permissions. The four roles are fixed in the code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    Read,
    Write,
    Admin,
    Agent,
}

impl Role {
    pub const ALL: [Role; 4] = [Role::Read, Role::Write, Role::Admin, Role::Agent];

    /// The role of a stored role string. An unknown string is no role: it
    /// holds no permission.
    pub fn parse(role: &str) -> Option<Role> {
        match role {
            "read" => Some(Role::Read),
            "write" => Some(Role::Write),
            "admin" => Some(Role::Admin),
            "agent" => Some(Role::Agent),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Role::Read => "read",
            Role::Write => "write",
            Role::Admin => "admin",
            Role::Agent => "agent",
        }
    }

    /// The permissions of the role.
    pub const fn permissions(self) -> &'static [Permission] {
        match self {
            Role::Read => READ,
            Role::Write => WRITE,
            Role::Admin => ADMIN,
            Role::Agent => AGENT,
        }
    }

    pub fn grants(self, permission: Permission) -> bool {
        self.permissions().contains(&permission)
    }

    /// The limits a principal of this role has when its key carries none of
    /// its own.
    pub const fn default_limits(self) -> Limits {
        match self {
            Role::Admin => Limits {
                max_priority: Some(0),
            },
            Role::Write => Limits {
                max_priority: Some(5),
            },
            Role::Read | Role::Agent => Limits { max_priority: None },
        }
    }

    /// A role a person may hold: a local account or an OIDC mapping rule.
    /// `agent` is for API keys only.
    pub fn is_for_people(self) -> bool {
        !matches!(self, Role::Agent)
    }
}

/// A numeric attribute of a principal that a route may ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitKind {
    /// The highest priority the principal may set. 0 is the highest priority
    /// (CLOACINA-T-1045).
    MaxPriority,
}

/// The numeric attributes of a principal. `None` means the principal may not
/// set the value at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Limits {
    pub max_priority: Option<u8>,
}

impl Limits {
    pub fn get(&self, kind: LimitKind) -> Option<u8> {
        match kind {
            LimitKind::MaxPriority => self.max_priority,
        }
    }
}

/// What the middleware grants a request it lets through: the limits of the
/// principal. A handler of a row with [`Row::limit`] reads it from the
/// request extensions and holds the request to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grant {
    pub limits: Limits,
}

// ---------------------------------------------------------------------------
// Principal, scope, matcher
// ---------------------------------------------------------------------------

/// The authenticated subject, as attributes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Principal {
    /// The key's tenant. `None` is a key with no tenant.
    pub tenant: Option<String>,
    /// The key's role. `None` when the stored role string is not one of the
    /// four: such a key holds no permission.
    pub role: Option<Role>,
    /// The platform admin flag (`AuthenticatedKey::is_admin`).
    pub platform_admin: bool,
    /// The numeric attributes: the role's defaults today; a key or an OIDC
    /// mapping rule may carry its own (CLOACINA-T-1045).
    pub limits: Limits,
}

impl Principal {
    pub fn from_key(key: &AuthenticatedKey) -> Principal {
        let role = Role::parse(&key.permissions);
        let mut limits = if key.is_admin {
            Role::Admin.default_limits()
        } else {
            role.map(Role::default_limits).unwrap_or_default()
        };
        // CLOACINA-T-1123: a key may carry its own limit. Only a role that
        // may set a priority at all gets one (the key routes refuse it on a
        // `read` or `agent` key).
        if let Some(own) = key.max_priority
            && limits.max_priority.is_some()
        {
            limits.max_priority = Some(own);
        }
        Principal {
            tenant: key.tenant_id.clone(),
            role,
            platform_admin: key.is_admin,
            limits,
        }
    }

    /// True when the principal holds `permission`, whatever the scope.
    pub fn holds(&self, permission: Permission) -> bool {
        let by_role = self.role.is_some_and(|r| r.grants(permission));
        if NOT_GRANTED_BY_PLATFORM_FLAG.contains(&permission) {
            // The named exception: the flag does not make a key an agent.
            return by_role;
        }
        self.platform_admin || by_role
    }
}

/// Where a row applies, before the request is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// No key is needed: liveness, the API document, the login routes.
    Public,
    /// The platform admin flag is needed.
    Platform,
    /// The tenant named by `{tenant_id}` in the path.
    TenantPath,
    /// The caller's own key: the handler reads the tenant from the key.
    Caller,
}

/// A [`Scope`] with the tenant of the request filled in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedScope {
    Platform,
    Tenant(String),
    Caller,
}

/// What a row needs from the principal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Need {
    /// Nothing: a public row.
    Nothing,
    /// One permission.
    Permission(Permission),
    /// The permission that the `{recipient}` of a delivery stream asks for
    /// ([`permission_for_recipient`]).
    ByRecipient,
}

/// How a row's request proves who it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Credential {
    /// None: a public row.
    Nothing,
    /// `Authorization: Bearer <key>`.
    Bearer,
    /// A bearer key, or a single-use ticket in `?token=`: a WebSocket
    /// upgrade from a browser cannot set a header.
    BearerOrTicket,
}

/// The audit row of a route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Audit {
    /// A read: no audit row. Only a GET row may say this.
    Read,
    /// The middleware writes a row under `action`. `target` names the path
    /// parameter that is the target when the handler notes none.
    Action {
        action: Action,
        target: Option<&'static str>,
    },
    /// A change with no audit row, and why.
    NoRow(&'static str),
}

/// The row of one route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub scope: Scope,
    pub need: Need,
    pub credential: Credential,
    pub audit: Audit,
    /// A limit of the principal that the handler holds the request to.
    pub limit: Option<LimitKind>,
}

impl Row {
    const fn new(scope: Scope, need: Need, credential: Credential) -> Row {
        Row {
            scope,
            need,
            credential,
            audit: Audit::Read,
            limit: None,
        }
    }

    /// No key needed.
    pub const fn public() -> Row {
        Row::new(Scope::Public, Need::Nothing, Credential::Nothing)
    }

    /// The platform admin flag and `permission`.
    pub const fn platform(permission: Permission) -> Row {
        Row::new(
            Scope::Platform,
            Need::Permission(permission),
            Credential::Bearer,
        )
    }

    /// `permission` in the tenant of the path.
    pub const fn tenant(permission: Permission) -> Row {
        Row::new(
            Scope::TenantPath,
            Need::Permission(permission),
            Credential::Bearer,
        )
    }

    /// `permission` on the caller's own key.
    pub const fn caller(permission: Permission) -> Row {
        Row::new(
            Scope::Caller,
            Need::Permission(permission),
            Credential::Bearer,
        )
    }

    /// The delivery stream: the recipient decides the permission.
    pub const fn by_recipient() -> Row {
        Row::new(Scope::Caller, Need::ByRecipient, Credential::Bearer)
    }

    /// Also accept a single-use ticket (a WebSocket upgrade).
    pub const fn ticket(mut self) -> Row {
        self.credential = Credential::BearerOrTicket;
        self
    }

    /// The middleware writes an audit row under `action`.
    pub const fn audit(mut self, action: Action, target: Option<&'static str>) -> Row {
        self.audit = Audit::Action { action, target };
        self
    }

    /// A change that writes no audit row, and why.
    pub const fn no_audit(mut self, why: &'static str) -> Row {
        self.audit = Audit::NoRow(why);
        self
    }

    /// The handler holds the request to this limit of the principal.
    pub const fn limit(mut self, kind: LimitKind) -> Row {
        self.limit = Some(kind);
        self
    }
}

/// Deny reason: the platform admin flag is needed.
pub const DENY_PLATFORM: &str = "platform_admin_required";
/// Deny reason: the key is not of the tenant in the path.
pub const DENY_TENANT: &str = "tenant_access_denied";
/// Deny reason: the key's role does not hold the permission.
pub const DENY_ROLE: &str = "insufficient_role";

/// The answer of the matcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Permit,
    Deny(&'static str),
}

impl Decision {
    pub fn is_permit(&self) -> bool {
        matches!(self, Decision::Permit)
    }
}

/// The matcher. Pure, total, default-deny.
///
/// 1. The scope: `Platform` needs the platform admin flag. `Tenant(t)` needs
///    a key of tenant `t`, or the flag. `Caller` has no test of its own.
/// 2. The permission: the principal's role holds it, or the principal has the
///    flag and the permission is not in [`NOT_GRANTED_BY_PLATFORM_FLAG`].
pub fn evaluate(principal: &Principal, scope: &ResolvedScope, need: Permission) -> Decision {
    match scope {
        ResolvedScope::Platform => {
            if !principal.platform_admin {
                return Decision::Deny(DENY_PLATFORM);
            }
        }
        ResolvedScope::Tenant(tenant) => {
            // A key with no tenant and no flag reaches no tenant.
            let in_tenant = principal.tenant.as_deref() == Some(tenant.as_str());
            if !principal.platform_admin && !in_tenant {
                return Decision::Deny(DENY_TENANT);
            }
        }
        ResolvedScope::Caller => {}
    }
    if principal.holds(need) {
        Decision::Permit
    } else {
        Decision::Deny(DENY_ROLE)
    }
}

/// The matcher for a key, on the caller's own scope. A WebSocket handler
/// calls it for a command that arrives after the upgrade, so a command is
/// checked like a REST route.
pub fn authorize_caller(key: &AuthenticatedKey, need: Permission) -> Decision {
    evaluate(&Principal::from_key(key), &ResolvedScope::Caller, need)
}

/// The permission a delivery stream asks for, by the kind of its recipient.
/// An unknown kind has none: the stream is refused.
pub fn permission_for_recipient(recipient: &str) -> Option<Permission> {
    match recipient.split_once(':').map(|(kind, _)| kind) {
        Some("agent") => Some(Permission::AgentAct),
        Some("exec_events") => Some(Permission::ExecutionsRead),
        Some("ops_metrics") => Some(Permission::OpsRead),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The table
// ---------------------------------------------------------------------------

/// One route and its row.
#[derive(Clone, Debug)]
pub struct RouteRow {
    pub method: Method,
    /// The path as the client calls it, with the `/v1` prefix.
    pub path: &'static str,
    pub row: Row,
}

/// The authorization table: one [`Row`] for each route of the server.
#[derive(Clone, Debug, Default)]
pub struct AuthzTable {
    rows: Vec<RouteRow>,
    index: HashMap<(Method, String), usize>,
    duplicates: Vec<String>,
}

impl AuthzTable {
    /// Add a row. A second row for one route is a fault that
    /// [`AuthzTable::faults`] names.
    pub fn add(&mut self, method: Method, path: &'static str, row: Row) {
        let key = (method.clone(), path.to_string());
        if self.index.contains_key(&key) {
            self.duplicates.push(format!("{method} {path}"));
            return;
        }
        self.index.insert(key, self.rows.len());
        self.rows.push(RouteRow { method, path, row });
    }

    pub fn get(&self, method: &Method, path: &str) -> Option<&Row> {
        self.index
            .get(&(method.clone(), path.to_string()))
            .map(|i| &self.rows[*i].row)
    }

    /// The rows in the order they were added.
    pub fn rows(&self) -> &[RouteRow] {
        &self.rows
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Rows that contradict themselves. Empty for a sound table.
    pub fn faults(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .duplicates
            .iter()
            .map(|d| format!("{d}: two rows"))
            .collect();
        for r in &self.rows {
            let name = format!("{} {}", r.method, r.path);
            let has = |param: &str| r.path.contains(&format!("{{{param}}}"));
            let public = r.row.scope == Scope::Public;
            if public != (r.row.need == Need::Nothing)
                || public != (r.row.credential == Credential::Nothing)
            {
                out.push(format!(
                    "{name}: a public row needs nothing; any other row needs a permission and a credential"
                ));
            }
            if r.row.scope == Scope::TenantPath && !has("tenant_id") {
                out.push(format!("{name}: a tenant row with no {{tenant_id}}"));
            }
            if r.row.need == Need::ByRecipient && !has("recipient") {
                out.push(format!("{name}: a recipient row with no {{recipient}}"));
            }
            if r.method != Method::GET && r.row.audit == Audit::Read {
                out.push(format!(
                    "{name}: a route that changes state names its audit action, or says why it has none"
                ));
            }
            if let Audit::Action {
                target: Some(param),
                ..
            } = r.row.audit
                && !has(param)
            {
                out.push(format!(
                    "{name}: audit target {{{param}}} is not in the path"
                ));
            }
        }
        out
    }
}

/// The table of the server.
pub fn build_authz_table() -> AuthzTable {
    use Permission as P;
    use actions as a;

    let mut t = AuthzTable::default();
    let get = Method::GET;
    let post = Method::POST;
    let put = Method::PUT;
    let delete = Method::DELETE;

    // ----- No key -----
    t.add(get.clone(), "/health", Row::public());
    t.add(get.clone(), "/ready", Row::public());
    t.add(get.clone(), "/metrics", Row::public());
    t.add(get.clone(), "/openapi.json", Row::public());
    // The login routes mint the key that every other route needs. The
    // handler notes the audit row: it knows the key it minted.
    t.add(
        post.clone(),
        "/v1/auth/local/login",
        Row::public().audit(a::AUTH_LOGIN, None),
    );
    t.add(get.clone(), "/v1/auth/oidc/login", Row::public());
    t.add(
        get.clone(),
        "/v1/auth/callback",
        Row::public().audit(a::AUTH_LOGIN, None),
    );

    // ----- The caller's own session -----
    t.add(
        post.clone(),
        "/v1/auth/ws-ticket",
        Row::caller(P::WsTicket)
            .no_audit("a ticket lives 60 seconds and changes nothing that lasts"),
    );
    t.add(
        post.clone(),
        "/v1/auth/refresh",
        Row::caller(P::SessionSelf)
            .no_audit("a refresh replaces the caller's key with one of the same scope"),
    );
    t.add(
        post.clone(),
        "/v1/auth/logout",
        Row::caller(P::SessionSelf).audit(a::AUTH_LOGOUT, None),
    );
    t.add(get.clone(), "/v1/auth/whoami", Row::caller(P::SessionSelf));

    // ----- Platform -----
    t.add(
        post.clone(),
        "/v1/tenants",
        Row::platform(P::TenantsManage).audit(a::TENANT_CREATE, None),
    );
    t.add(get.clone(), "/v1/tenants", Row::platform(P::TenantsManage));
    t.add(
        delete.clone(),
        "/v1/tenants/{schema_name}",
        Row::platform(P::TenantsManage).audit(a::TENANT_DELETE, Some("schema_name")),
    );
    t.add(
        get.clone(),
        "/v1/compiler/status",
        Row::platform(P::CompilerRead),
    );
    t.add(get.clone(), "/v1/audit", Row::platform(P::AuditRead));
    // The global key routes: a tenant admin uses the tenant routes below.
    t.add(
        post.clone(),
        "/v1/auth/keys",
        Row::platform(P::KeysManage).audit(a::KEY_CREATE, None),
    );
    t.add(get.clone(), "/v1/auth/keys", Row::platform(P::KeysManage));
    t.add(
        delete.clone(),
        "/v1/auth/keys/{key_id}",
        Row::platform(P::KeysManage).audit(a::KEY_REVOKE, Some("key_id")),
    );
    // A limit and a pool set the size, the cloud identity and the network
    // reach of a tenant's agents: the platform sets them, the tenant reads.
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/limits",
        Row::platform(P::LimitsManage).audit(a::LIMIT_SET, Some("tenant_id")),
    );
    t.add(
        delete.clone(),
        "/v1/tenants/{tenant_id}/limits",
        Row::platform(P::LimitsManage).audit(a::LIMIT_CLEAR, Some("tenant_id")),
    );
    t.add(
        put.clone(),
        "/v1/tenants/{tenant_id}/pools/{pool}",
        Row::platform(P::PoolsManage).audit(a::POOL_SET, Some("pool")),
    );
    t.add(
        delete.clone(),
        "/v1/tenants/{tenant_id}/pools/{pool}",
        Row::platform(P::PoolsManage).audit(a::POOL_DELETE, Some("pool")),
    );

    // ----- Tenant: administration -----
    t.add(
        get.clone(),
        "/v1/tenants/{tenant_id}/audit",
        Row::tenant(P::AuditRead),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/keys",
        Row::tenant(P::KeysManage).audit(a::KEY_CREATE, None),
    );
    t.add(
        get.clone(),
        "/v1/tenants/{tenant_id}/keys",
        Row::tenant(P::KeysManage),
    );
    t.add(
        delete.clone(),
        "/v1/tenants/{tenant_id}/keys/{key_id}",
        Row::tenant(P::KeysManage).audit(a::KEY_REVOKE, Some("key_id")),
    );
    // CLOACINA-T-1123: the highest priority a key may set.
    t.add(
        Method::PATCH,
        "/v1/tenants/{tenant_id}/keys/{key_id}",
        Row::tenant(P::KeysManage).audit(a::KEY_UPDATE, Some("key_id")),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/accounts",
        Row::tenant(P::AccountsManage).audit(a::ACCOUNT_CREATE, None),
    );
    t.add(
        get.clone(),
        "/v1/tenants/{tenant_id}/accounts",
        Row::tenant(P::AccountsManage),
    );
    t.add(
        delete.clone(),
        "/v1/tenants/{tenant_id}/accounts/{account_id}",
        Row::tenant(P::AccountsManage).audit(a::ACCOUNT_DISABLE, Some("account_id")),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/accounts/{account_id}/password",
        Row::tenant(P::AccountsManage).audit(a::ACCOUNT_PASSWORD_RESET, Some("account_id")),
    );
    // Even the names of a secret's fields are sensitive: the reads need the
    // same permission as the writes.
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/secrets",
        Row::tenant(P::SecretsManage).audit(a::SECRET_CREATE, None),
    );
    t.add(
        get.clone(),
        "/v1/tenants/{tenant_id}/secrets",
        Row::tenant(P::SecretsManage),
    );
    t.add(
        get.clone(),
        "/v1/tenants/{tenant_id}/secrets/{name}",
        Row::tenant(P::SecretsManage),
    );
    t.add(
        put.clone(),
        "/v1/tenants/{tenant_id}/secrets/{name}",
        Row::tenant(P::SecretsManage).audit(a::SECRET_ROTATE, Some("name")),
    );
    t.add(
        delete.clone(),
        "/v1/tenants/{tenant_id}/secrets/{name}",
        Row::tenant(P::SecretsManage).audit(a::SECRET_DELETE, Some("name")),
    );
    // The keys a tenant trusts for package signing (CLOACINA-A-0018): the
    // tenant admin manages them, and a platform admin for any tenant. The
    // list needs the same permission as the writes, like API keys.
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/trusted-keys",
        Row::tenant(P::TrustedKeysManage).audit(a::TRUSTED_KEY_ADD, None),
    );
    t.add(
        get.clone(),
        "/v1/tenants/{tenant_id}/trusted-keys",
        Row::tenant(P::TrustedKeysManage),
    );
    t.add(
        delete.clone(),
        "/v1/tenants/{tenant_id}/trusted-keys/{key_id}",
        Row::tenant(P::TrustedKeysManage).audit(a::TRUSTED_KEY_REVOKE, Some("key_id")),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/fleet/provision",
        Row::tenant(P::FleetScale).audit(a::FLEET_PROVISION, Some("tenant_id")),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/fleet/deprovision",
        Row::tenant(P::FleetScale).audit(a::FLEET_DEPROVISION, Some("tenant_id")),
    );
    // The roster: the handler gives a tenant admin its own tenant's agents.
    t.add(get.clone(), "/v1/agents", Row::caller(P::AgentsRead));

    // ----- Tenant: reads -----
    t.add(
        get.clone(),
        "/v1/tenants/{tenant_id}/limits",
        Row::tenant(P::LimitsRead),
    );
    t.add(
        get.clone(),
        "/v1/tenants/{tenant_id}/pools",
        Row::tenant(P::PoolsRead),
    );
    t.add(
        get.clone(),
        "/v1/tenants/{tenant_id}/pools/{pool}",
        Row::tenant(P::PoolsRead),
    );
    t.add(
        get.clone(),
        "/v1/tenants/{tenant_id}/fleet",
        Row::tenant(P::FleetRead),
    );
    for path in [
        "/v1/tenants/{tenant_id}/workflows",
        "/v1/tenants/{tenant_id}/workflows/{name}",
        "/v1/tenants/{tenant_id}/workflows/{name}/source",
        "/v1/tenants/{tenant_id}/workflows/{name}/versions",
        "/v1/tenants/{tenant_id}/workflows/{name}/instances",
        "/v1/tenants/{tenant_id}/workflows/{name}/instances/{instance}",
        // Backfills (CLOACINA-T-1020): list and read one.
        "/v1/tenants/{tenant_id}/workflows/{name}/backfills",
        "/v1/tenants/{tenant_id}/workflows/{name}/backfills/{id}",
    ] {
        t.add(get.clone(), path, Row::tenant(P::WorkflowsRead));
    }
    for path in [
        "/v1/tenants/{tenant_id}/triggers",
        "/v1/tenants/{tenant_id}/triggers/{name}",
        "/v1/tenants/{tenant_id}/triggers/{name}/interface",
    ] {
        t.add(get.clone(), path, Row::tenant(P::TriggersRead));
    }
    for path in [
        "/v1/tenants/{tenant_id}/executions",
        "/v1/tenants/{tenant_id}/executions/{exec_id}",
        "/v1/tenants/{tenant_id}/executions/{exec_id}/events",
        "/v1/tenants/{tenant_id}/executions/{exec_id}/tasks",
    ] {
        t.add(get.clone(), path, Row::tenant(P::ExecutionsRead));
    }

    // ----- Tenant: writes -----
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/workflows",
        Row::tenant(P::PackagesPublish).audit(a::PACKAGE_UPLOAD, None),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/workflows/{name}/versions/{version}/activate",
        Row::tenant(P::PackagesPublish).audit(a::PACKAGE_ACTIVATE, Some("name")),
    );
    t.add(
        delete.clone(),
        "/v1/tenants/{tenant_id}/workflows/{name}/{version}",
        Row::tenant(P::PackagesPublish).audit(a::PACKAGE_DELETE, Some("name")),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/workflows/{name}/pause",
        Row::tenant(P::WorkflowsOperate).audit(a::WORKFLOW_PAUSE, Some("name")),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/workflows/{name}/resume",
        Row::tenant(P::WorkflowsOperate).audit(a::WORKFLOW_RESUME, Some("name")),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/workflows/{name}/instances",
        Row::tenant(P::WorkflowsOperate).audit(a::INSTANCE_CREATE, None),
    );
    t.add(
        delete.clone(),
        "/v1/tenants/{tenant_id}/workflows/{name}/instances/{instance}",
        Row::tenant(P::WorkflowsOperate).audit(a::INSTANCE_DELETE, Some("instance")),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/workflows/{name}/execute",
        // CLOACINA-T-1045: the run may ask for a priority; the handler holds
        // it to the highest priority the principal may set.
        Row::tenant(P::WorkflowsRun)
            .audit(a::WORKFLOW_RUN, Some("name"))
            .limit(LimitKind::MaxPriority),
    );
    // Backfills (CLOACINA-T-1020) start and stop runs of a workflow by hand.
    // The id of a new backfill is made by the handler, which notes it.
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/workflows/{name}/backfills",
        Row::tenant(P::WorkflowsRun).audit(a::BACKFILL_CREATE, None),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/workflows/{name}/backfills/{id}/cancel",
        Row::tenant(P::WorkflowsRun).audit(a::BACKFILL_CANCEL, Some("id")),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/workflows/{name}/backfills/{id}/retry",
        Row::tenant(P::WorkflowsRun).audit(a::BACKFILL_RETRY, Some("id")),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/triggers/{name}/pause",
        Row::tenant(P::TriggersOperate).audit(a::TRIGGER_PAUSE, Some("name")),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/triggers/{name}/resume",
        Row::tenant(P::TriggersOperate).audit(a::TRIGGER_RESUME, Some("name")),
    );
    t.add(
        post.clone(),
        "/v1/tenants/{tenant_id}/triggers/{name}/fire",
        Row::tenant(P::TriggersOperate).audit(a::TRIGGER_FIRE, Some("name")),
    );

    // ----- Graphs: the handler reads the tenant from the key -----
    for path in [
        "/v1/health/accumulators",
        "/v1/health/reactors",
        "/v1/health/reactors/{name}/fires",
        "/v1/health/reactors/{name}/fires/timeseries",
        "/v1/health/reactors/{name}/interface",
        "/v1/health/accumulators/{name}/interface",
        "/v1/health/graphs",
        "/v1/health/graphs/{name}",
    ] {
        t.add(get.clone(), path, Row::caller(P::GraphsRead));
    }
    t.add(
        post.clone(),
        "/v1/health/reactors/{name}/fire",
        Row::caller(P::ReactorsOperate).audit(a::REACTOR_FIRE, Some("name")),
    );
    t.add(
        post.clone(),
        "/v1/health/accumulators/{name}/inject",
        Row::caller(P::AccumulatorsInject).audit(a::ACCUMULATOR_INJECT, Some("name")),
    );

    // ----- WebSockets: the upgrade request goes through the same check -----
    // Every frame of the accumulator socket is an inject: the connect needs
    // the permission and is the audited event.
    t.add(
        get.clone(),
        "/v1/ws/accumulator/{name}",
        Row::caller(P::AccumulatorsInject)
            .ticket()
            .audit(a::ACCUMULATOR_CONNECT, Some("name")),
    );
    // A reader may connect and read the state. Fire, pause and resume are
    // checked and audited for each command (`routes/ws.rs`).
    t.add(
        get.clone(),
        "/v1/ws/reactor/{name}",
        Row::caller(P::GraphsRead).ticket(),
    );
    t.add(
        get.clone(),
        "/v1/ws/delivery/{recipient}",
        Row::by_recipient().ticket(),
    );

    // ----- Agents: a key of role `agent` only -----
    t.add(
        post.clone(),
        "/v1/agent/register",
        Row::caller(P::AgentAct).audit(a::AGENT_REGISTER, None),
    );
    t.add(
        post.clone(),
        "/v1/agent/heartbeat",
        Row::caller(P::AgentAct).no_audit("one every few seconds for each agent"),
    );
    t.add(
        post.clone(),
        "/v1/agent/keys",
        Row::caller(P::AgentAct).audit(a::AGENT_KEYS, None),
    );
    t.add(
        post.clone(),
        "/v1/agent/result",
        Row::caller(P::AgentAct)
            .no_audit("one for each task; the execution history holds the outcome"),
    );
    // CLOACINA-T-1058: a packet the agent cannot decode, named by its push id.
    t.add(
        post.clone(),
        "/v1/agent/undecodable",
        Row::caller(P::AgentAct)
            .no_audit("one for each refused packet; the execution history holds the outcome"),
    );
    for path in [
        "/v1/agent/artifact/{digest}",
        "/v1/agent/source/{digest}",
        // CLOACINA-T-1073: the vendored Python dependencies of a package.
        "/v1/agent/python-vendor/{digest}",
        "/v1/agent/providers/{digest}",
    ] {
        t.add(get.clone(), path, Row::caller(P::AgentAct));
    }

    t
}

// ---------------------------------------------------------------------------
// The routes the router registered, and the start-up check
// ---------------------------------------------------------------------------

/// A router that records each route it registers, so the start-up check can
/// compare the routes with the table (axum does not list its routes).
///
/// Every route of the server is registered through this type.
pub struct RouteSet<S = AppState> {
    prefix: &'static str,
    router: Router<S>,
    registered: Vec<(Method, String)>,
}

impl<S: Clone + Send + Sync + 'static> RouteSet<S> {
    /// Routes under `prefix` (`""` or `"/v1"`). Paths are given without it.
    pub fn new(prefix: &'static str) -> Self {
        RouteSet {
            prefix,
            router: Router::new(),
            registered: Vec::new(),
        }
    }

    /// Register `method_router` for `method` on `path`.
    pub fn add(mut self, method: Method, path: &str, method_router: MethodRouter<S>) -> Self {
        self.registered
            .push((method, format!("{}{}", self.prefix, path)));
        self.router = self.router.route(path, method_router);
        self
    }

    pub fn get<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.add(Method::GET, path, axum::routing::get(handler))
    }

    pub fn post<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.add(Method::POST, path, axum::routing::post(handler))
    }

    pub fn put<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.add(Method::PUT, path, axum::routing::put(handler))
    }

    pub fn delete<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.add(Method::DELETE, path, axum::routing::delete(handler))
    }

    pub fn patch<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.add(Method::PATCH, path, axum::routing::patch(handler))
    }

    /// The routes registered so far, with the prefix.
    pub fn registered(&self) -> &[(Method, String)] {
        &self.registered
    }

    /// The axum router and the routes it holds.
    pub fn into_parts(self) -> (Router<S>, Vec<(Method, String)>) {
        (self.router, self.registered)
    }
}

/// Why the routes and the table do not agree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoverageError {
    /// Routes the router serves that have no row.
    pub routes_without_row: Vec<String>,
    /// Rows for a route the router does not serve.
    pub rows_without_route: Vec<String>,
    /// Rows that contradict themselves ([`AuthzTable::faults`]).
    pub faults: Vec<String>,
}

impl std::fmt::Display for CoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "the routes and the authorization table do not agree (routes/authz.rs):"
        )?;
        for r in &self.routes_without_row {
            writeln!(f, "  route with no row: {r}")?;
        }
        for r in &self.rows_without_route {
            writeln!(f, "  row with no route: {r}")?;
        }
        for r in &self.faults {
            writeln!(f, "  bad row: {r}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CoverageError {}

/// Compare the routes the router registered with the table, both ways.
/// The server calls it before it listens and exits on an error.
pub fn check_coverage(
    registered: &[(Method, String)],
    table: &AuthzTable,
) -> Result<(), CoverageError> {
    let mut err = CoverageError {
        faults: table.faults(),
        ..Default::default()
    };
    for (method, path) in registered {
        if table.get(method, path).is_none() {
            err.routes_without_row.push(format!("{method} {path}"));
        }
    }
    for r in table.rows() {
        if !registered
            .iter()
            .any(|(m, p)| *m == r.method && p == r.path)
        {
            err.rows_without_route
                .push(format!("{} {}", r.method, r.path));
        }
    }
    err.routes_without_row.sort();
    err.rows_without_route.sort();
    if err == CoverageError::default() {
        Ok(())
    } else {
        Err(err)
    }
}

// ---------------------------------------------------------------------------
// The middleware
// ---------------------------------------------------------------------------

fn deny_response(reason: &str) -> Response {
    let err = match reason {
        DENY_PLATFORM => ApiError::forbidden("admin_required", "admin access required"),
        DENY_TENANT => ApiError::forbidden("tenant_access_denied", "access denied for this tenant"),
        _ => ApiError::forbidden("insufficient_permissions", "insufficient permissions"),
    };
    err.into_response()
}

fn unclassified() -> Response {
    ApiError::forbidden("authz_unclassified", "route not authorized").into_response()
}

/// What a request offers as proof: read before any await, since a request
/// body is not `Sync`.
struct Offered {
    /// The value of the `Authorization` header.
    authorization: Option<String>,
    /// The value of `?token=`.
    ticket: Option<String>,
}

impl Offered {
    fn of(request: &Request) -> Offered {
        Offered {
            authorization: request
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string),
            ticket: Query::<crate::routes::ws::WsAuthQuery>::try_from_uri(request.uri())
                .ok()
                .and_then(|q| q.0.token),
        }
    }
}

/// The key of a request, by the credential its row accepts.
async fn authenticate(
    state: &AppState,
    credential: Credential,
    offered: Offered,
) -> Result<Option<AuthenticatedKey>, Box<Response>> {
    use crate::routes::ws::record_ws_auth_failure;
    match credential {
        Credential::Nothing => Ok(None),
        Credential::Bearer => {
            // A REST route takes the exact `Bearer ` prefix, as before.
            let Some(token) = offered
                .authorization
                .as_deref()
                .and_then(|v| v.strip_prefix("Bearer "))
            else {
                return Err(Box::new(
                    ApiError::unauthorized("missing or malformed Authorization header")
                        .into_response(),
                ));
            };
            validate_token(state, token)
                .await
                .map(Some)
                .map_err(|e| Box::new(e.into_response()))
        }
        Credential::BearerOrTicket => {
            let bearer = offered.authorization.as_deref().and_then(|v| {
                v.strip_prefix("Bearer ")
                    .or_else(|| v.strip_prefix("bearer "))
            });
            if let Some(token) = bearer {
                return validate_token(state, token).await.map(Some).map_err(|e| {
                    record_ws_auth_failure("invalid_signature");
                    // CLOACINA-T-1083: an expired key keeps its own code.
                    if e.code == crate::routes::auth::KEY_EXPIRED_CODE {
                        return Box::new(e.into_response());
                    }
                    Box::new(ApiError::unauthorized("invalid bearer token").into_response())
                });
            }
            // A value in `?token=` is a single-use ticket, never an API key.
            let Some(ticket) = offered.ticket else {
                record_ws_auth_failure("not_authorized");
                return Err(Box::new(
                    ApiError::unauthorized("missing auth token").into_response(),
                ));
            };
            match state.ws_tickets.consume(&ticket).await {
                Some(key) => Ok(Some(key)),
                None => {
                    record_ws_auth_failure("ticket_expired");
                    Err(Box::new(
                        ApiError::unauthorized("invalid or expired WebSocket ticket")
                            .into_response(),
                    ))
                }
            }
        }
    }
}

fn ran_ok(status: StatusCode) -> bool {
    status.is_success() || status == StatusCode::SWITCHING_PROTOCOLS
}

/// Write the audit rows of a request the handler answered.
async fn write_audit(
    state: &AppState,
    key: Option<&AuthenticatedKey>,
    row: &Row,
    default_tenant: Option<&str>,
    params: &[(String, String)],
    status: StatusCode,
    notes: audit::Notes,
) {
    let Audit::Action { action, target } = row.audit else {
        return;
    };
    if notes.noted.is_empty() {
        // The handler noted nothing: the row of the route gives the target.
        // A request with no key and no note (a failed login) has no actor.
        let Some(key) = key else { return };
        if !ran_ok(status) {
            return;
        }
        let target = target.and_then(|name| {
            params
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        });
        let event = match target {
            Some(t) => AuditEvent::ok(action, t),
            None => AuditEvent::ok_untargeted(action),
        };
        audit::record(state, key, default_tenant, event).await;
        return;
    }
    for noted in notes.noted {
        if noted.event.action() != action {
            warn!(
                noted = noted.event.action().name,
                row = action.name,
                "audit: the handler noted another action than its route's row; the row wins"
            );
        }
        let event = noted.event.with_action(action);
        match (noted.actor, key) {
            (Some(actor), _) => {
                audit::record_as(state, actor, noted.tenant.as_deref(), event).await
            }
            (None, Some(key)) => audit::record(state, key, noted.tenant.as_deref(), event).await,
            (None, None) => warn!(
                action = action.name,
                "audit: a note with no actor on a route with no key; no row written"
            ),
        }
    }
}

/// The middleware of every `/v1` route.
///
/// 1. Find the row of the matched route. No row: refused.
/// 2. Authenticate as the row says. The key goes into the request extensions
///    for the handler.
/// 3. Run the matcher. A refusal on an audited route writes a `denied` row.
/// 4. Call the handler, then write the audit row of the route.
pub async fn guard(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let matched_path = match request.extensions().get::<MatchedPath>() {
        // Inside the `/v1` nest the matched path may come with or without
        // the prefix; the table holds the full path.
        Some(mp) if mp.as_str().starts_with("/v1/") => mp.as_str().to_string(),
        Some(mp) => format!("/v1{}", mp.as_str()),
        None => {
            warn!(%method, "authz: no matched path; refused");
            return unclassified();
        }
    };
    let Some(row) = state.authz_table.get(&method, &matched_path).copied() else {
        warn!(%method, path = %matched_path, "authz: route with no row; refused");
        return unclassified();
    };

    let offered = Offered::of(&request);
    let key = match authenticate(&state, row.credential, offered).await {
        Ok(key) => key,
        Err(response) => return *response,
    };

    let (mut parts, body) = request.into_parts();
    let params: Vec<(String, String)> = RawPathParams::from_request_parts(&mut parts, &state)
        .await
        .map(|p| {
            p.iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let param = |name: &str| {
        params
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    };
    let mut request = Request::from_parts(parts, body);

    let Some(key) = key else {
        // A public row: nothing to match. The handler notes its own audit
        // row (a login names the key it minted).
        let (response, notes) = audit::collect_notes(next.run(request)).await;
        let status = response.status();
        write_audit(&state, None, &row, None, &params, status, notes).await;
        return response;
    };
    record_auth_span_fields(&tracing::Span::current(), &key);
    let principal = Principal::from_key(&key);

    // The scope, and the tenant an audit row of this request belongs to.
    let (scope, tenant) = match row.scope {
        Scope::Platform => (ResolvedScope::Platform, None),
        Scope::Caller => (ResolvedScope::Caller, key.tenant_id.clone()),
        Scope::TenantPath => match param("tenant_id") {
            Some(t) => (ResolvedScope::Tenant(t.clone()), Some(t)),
            None => {
                warn!(path = %matched_path, "authz: a tenant row with no tenant in the path; refused");
                return deny_response(DENY_TENANT);
            }
        },
        Scope::Public => {
            // `AuthzTable::faults` refuses a public row with a credential.
            return unclassified();
        }
    };
    let need = match row.need {
        Need::Permission(p) => Some(p),
        Need::ByRecipient => param("recipient")
            .as_deref()
            .and_then(permission_for_recipient),
        Need::Nothing => None,
    };
    let decision = match need {
        Some(need) => evaluate(&principal, &scope, need),
        // A recipient of no known kind, or a row that needs nothing but has
        // a key: no permission can be held.
        None => Decision::Deny(DENY_ROLE),
    };

    if let Decision::Deny(reason) = decision {
        if row.credential == Credential::BearerOrTicket {
            crate::routes::ws::record_ws_auth_failure("not_authorized");
        }
        if let Audit::Action { action, .. } = row.audit {
            // Inside the `/v1` nest `request.uri()` has the prefix stripped;
            // the path the client called is in `OriginalUri`. It holds names
            // and ids only.
            let request_path = request
                .extensions()
                .get::<axum::extract::OriginalUri>()
                .map(|u| u.0.path().to_string())
                .unwrap_or_else(|| request.uri().path().to_string());
            let event = AuditEvent::denied(
                action,
                None,
                serde_json::json!({ "reason": reason, "path": request_path }),
            );
            audit::record(&state, &key, tenant.as_deref(), event).await;
        }
        return deny_response(reason);
    }

    request.extensions_mut().insert(key.clone());
    request.extensions_mut().insert(Grant {
        limits: principal.limits,
    });
    let (response, notes) = audit::collect_notes(next.run(request)).await;
    let status = response.status();
    write_audit(
        &state,
        Some(&key),
        &row,
        tenant.as_deref(),
        &params,
        status,
        notes,
    )
    .await;
    response
}

// ---------------------------------------------------------------------------
// The docs tables
// ---------------------------------------------------------------------------

fn scope_text(scope: Scope) -> &'static str {
    match scope {
        Scope::Public => "public",
        Scope::Platform => "platform",
        Scope::TenantPath => "tenant in the path",
        Scope::Caller => "caller's key",
    }
}

/// The tables of the security model page: the roles, and every route.
pub fn render_markdown(table: &AuthzTable) -> String {
    let mut out = String::new();
    out.push_str("#### Permissions and roles\n\n");
    out.push_str(
        "| Permission | Allows | `read` | `write` | `admin` | `agent` | Platform admin flag |\n",
    );
    out.push_str("|---|---|---|---|---|---|---|\n");
    for p in Permission::ALL {
        let mark = |yes: bool| if yes { "yes" } else { "" };
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} | {} |\n",
            p.name(),
            p.allows(),
            mark(Role::Read.grants(p)),
            mark(Role::Write.grants(p)),
            mark(Role::Admin.grants(p)),
            mark(Role::Agent.grants(p)),
            if NOT_GRANTED_BY_PLATFORM_FLAG.contains(&p) {
                "no"
            } else {
                "yes"
            },
        ));
    }
    out.push_str("\n#### Every route\n\n");
    out.push_str("| Method | Path | Scope | Permission | Audit |\n");
    out.push_str("|---|---|---|---|---|\n");
    for r in table.rows() {
        let need = match r.row.need {
            Need::Nothing => "none".to_string(),
            Need::Permission(p) => format!("`{}`", p.name()),
            Need::ByRecipient => "by recipient".to_string(),
        };
        let audit = match r.row.audit {
            Audit::Read => "none (read)".to_string(),
            Audit::Action { action, .. } => format!("`{}`", action.name),
            Audit::NoRow(why) => format!("none: {why}"),
        };
        out.push_str(&format!(
            "| {} | `{}` | {} | {} | {} |\n",
            r.method,
            r.path,
            scope_text(r.row.scope),
            need,
            audit
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(role: &str, tenant: Option<&str>, platform_admin: bool) -> Principal {
        Principal::from_key(&AuthenticatedKey {
            key_id: uuid::Uuid::nil(),
            name: "k".into(),
            permissions: role.into(),
            tenant_id: tenant.map(str::to_string),
            is_admin: platform_admin,
            max_priority: None,
        })
    }

    /// CLOACINA-T-1123: a key's own limit replaces its role's, on a role
    /// that may set a priority; a `read` or `agent` key sets none.
    #[test]
    fn a_key_may_carry_its_own_priority_limit() {
        let key = |role: &str, own: Option<u8>| AuthenticatedKey {
            key_id: uuid::Uuid::nil(),
            name: "k".into(),
            permissions: role.into(),
            tenant_id: Some("acme".into()),
            is_admin: false,
            max_priority: own,
        };
        let limit = |k: &AuthenticatedKey| Principal::from_key(k).limits.max_priority;
        assert_eq!(limit(&key("write", None)), Some(5));
        assert_eq!(limit(&key("write", Some(2))), Some(2));
        assert_eq!(limit(&key("write", Some(8))), Some(8));
        assert_eq!(limit(&key("admin", Some(3))), Some(3));
        assert_eq!(limit(&key("read", Some(2))), None);
        assert_eq!(limit(&key("agent", Some(2))), None);
    }

    /// The scope of a row for a request whose path names the tenant `acme`.
    fn resolved(scope: Scope) -> ResolvedScope {
        match scope {
            Scope::Platform => ResolvedScope::Platform,
            Scope::TenantPath => ResolvedScope::Tenant("acme".into()),
            Scope::Caller => ResolvedScope::Caller,
            Scope::Public => panic!("a public row is not matched"),
        }
    }

    // ----- roles -----

    #[test]
    fn roles_build_on_each_other_and_agent_stands_apart() {
        for p in Role::Read.permissions() {
            assert!(Role::Write.grants(*p), "write lacks {}", p.name());
        }
        for p in Role::Write.permissions() {
            assert!(Role::Admin.grants(*p), "admin lacks {}", p.name());
        }
        assert_eq!(
            Role::Agent.permissions(),
            &[Permission::AgentAct, Permission::WsTicket]
        );
        // A person's key is not an agent.
        for role in [Role::Read, Role::Write, Role::Admin] {
            assert!(!role.grants(Permission::AgentAct));
        }
        // What the platform sets, no role holds.
        for p in [
            Permission::TenantsManage,
            Permission::CompilerRead,
            Permission::LimitsManage,
            Permission::PoolsManage,
        ] {
            for role in Role::ALL {
                assert!(!role.grants(p), "{} holds {}", role.as_str(), p.name());
            }
        }
    }

    #[test]
    fn read_cannot_change_a_reactor_or_an_accumulator_and_write_can() {
        for p in [Permission::ReactorsOperate, Permission::AccumulatorsInject] {
            assert!(!Role::Read.grants(p));
            assert!(Role::Write.grants(p));
            assert!(Role::Admin.grants(p));
            assert!(!Role::Agent.grants(p));
        }
    }

    #[test]
    fn role_strings_parse_and_an_unknown_one_is_no_role() {
        for role in Role::ALL {
            assert_eq!(Role::parse(role.as_str()), Some(role));
        }
        for unknown in ["", "Read", "ADMIN", "owner", "read ", "superuser"] {
            assert_eq!(Role::parse(unknown), None, "{unknown:?}");
        }
        assert!(Role::Read.is_for_people());
        assert!(!Role::Agent.is_for_people());
    }

    #[test]
    fn an_unknown_role_grants_nothing() {
        let table = build_authz_table();
        for tenant in [None, Some("acme")] {
            let p = principal("owner", tenant, false);
            assert_eq!(p.role, None);
            for permission in Permission::ALL {
                assert!(!p.holds(permission), "{}", permission.name());
            }
            for r in table.rows() {
                let Need::Permission(need) = r.row.need else {
                    continue;
                };
                assert!(
                    !evaluate(&p, &resolved(r.row.scope), need).is_permit(),
                    "an unknown role passed {} {}",
                    r.method,
                    r.path
                );
            }
        }
    }

    // ----- the platform admin flag -----

    #[test]
    fn the_platform_flag_grants_every_permission_but_agent_act() {
        assert_eq!(NOT_GRANTED_BY_PLATFORM_FLAG, &[Permission::AgentAct]);
        // The bootstrap key: no tenant, role `admin`, the flag.
        let bootstrap = principal("admin", None, true);
        for permission in Permission::ALL {
            let expected = permission != Permission::AgentAct;
            assert_eq!(
                bootstrap.holds(permission),
                expected,
                "{}",
                permission.name()
            );
            for scope in [
                ResolvedScope::Platform,
                ResolvedScope::Tenant("acme".into()),
                ResolvedScope::Caller,
            ] {
                assert_eq!(
                    evaluate(&bootstrap, &scope, permission).is_permit(),
                    expected,
                    "{} in {scope:?}",
                    permission.name()
                );
            }
        }
        // The flag does not make a key an agent, whatever its role string...
        for role in ["read", "write", "admin", "owner"] {
            assert_eq!(
                evaluate(
                    &principal(role, None, true),
                    &ResolvedScope::Caller,
                    Permission::AgentAct
                ),
                Decision::Deny(DENY_ROLE)
            );
        }
        // ...and only the role `agent` does.
        assert!(
            evaluate(
                &principal("agent", Some("acme"), false),
                &ResolvedScope::Caller,
                Permission::AgentAct
            )
            .is_permit()
        );
    }

    #[test]
    fn deny_reasons_are_stable() {
        let reader = principal("read", Some("acme"), false);
        assert_eq!(
            evaluate(&reader, &ResolvedScope::Platform, Permission::TenantsManage),
            Decision::Deny(DENY_PLATFORM)
        );
        assert_eq!(
            evaluate(
                &reader,
                &ResolvedScope::Tenant("beta".into()),
                Permission::WorkflowsRead
            ),
            Decision::Deny(DENY_TENANT)
        );
        assert_eq!(
            evaluate(
                &reader,
                &ResolvedScope::Tenant("acme".into()),
                Permission::WorkflowsRun
            ),
            Decision::Deny(DENY_ROLE)
        );
        // The tenant is tested before the role.
        assert_eq!(
            evaluate(
                &principal("admin", Some("acme"), false),
                &ResolvedScope::Tenant("beta".into()),
                Permission::KeysManage
            ),
            Decision::Deny(DENY_TENANT)
        );
        // A key with no tenant and no flag reaches no tenant.
        assert_eq!(
            evaluate(
                &principal("admin", None, false),
                &ResolvedScope::Tenant("public".into()),
                Permission::WorkflowsRead
            ),
            Decision::Deny(DENY_TENANT)
        );
    }

    // ----- parity with the ordered levels this model replaced -----

    /// The route table as it was before CLOACINA-T-1055: the path without
    /// `/v1`, the scope (P platform, T tenant in the path, A any key) and the
    /// lowest level (R read, W write, A admin).
    const OLD_TABLE: &[(&str, &str, char, char)] = &[
        ("POST", "/tenants", 'P', 'A'),
        ("GET", "/tenants", 'P', 'A'),
        ("DELETE", "/tenants/{schema_name}", 'P', 'A'),
        ("GET", "/compiler/status", 'P', 'A'),
        ("GET", "/agents", 'A', 'A'),
        ("POST", "/tenants/{tenant_id}/keys", 'T', 'A'),
        ("GET", "/tenants/{tenant_id}/keys", 'T', 'A'),
        ("DELETE", "/tenants/{tenant_id}/keys/{key_id}", 'T', 'A'),
        ("POST", "/tenants/{tenant_id}/accounts", 'T', 'A'),
        ("GET", "/tenants/{tenant_id}/accounts", 'T', 'A'),
        (
            "DELETE",
            "/tenants/{tenant_id}/accounts/{account_id}",
            'T',
            'A',
        ),
        (
            "POST",
            "/tenants/{tenant_id}/accounts/{account_id}/password",
            'T',
            'A',
        ),
        ("POST", "/tenants/{tenant_id}/secrets", 'T', 'A'),
        ("GET", "/tenants/{tenant_id}/secrets", 'T', 'A'),
        ("GET", "/tenants/{tenant_id}/secrets/{name}", 'T', 'A'),
        ("PUT", "/tenants/{tenant_id}/secrets/{name}", 'T', 'A'),
        ("DELETE", "/tenants/{tenant_id}/secrets/{name}", 'T', 'A'),
        ("POST", "/tenants/{tenant_id}/limits", 'P', 'A'),
        ("DELETE", "/tenants/{tenant_id}/limits", 'P', 'A'),
        ("GET", "/tenants/{tenant_id}/limits", 'T', 'R'),
        ("GET", "/tenants/{tenant_id}/pools", 'T', 'R'),
        ("GET", "/tenants/{tenant_id}/pools/{pool}", 'T', 'R'),
        ("PUT", "/tenants/{tenant_id}/pools/{pool}", 'P', 'A'),
        ("DELETE", "/tenants/{tenant_id}/pools/{pool}", 'P', 'A'),
        ("POST", "/tenants/{tenant_id}/fleet/provision", 'T', 'A'),
        ("POST", "/tenants/{tenant_id}/fleet/deprovision", 'T', 'A'),
        ("GET", "/tenants/{tenant_id}/fleet", 'T', 'R'),
        ("POST", "/auth/keys", 'P', 'A'),
        ("GET", "/auth/keys", 'P', 'A'),
        ("DELETE", "/auth/keys/{key_id}", 'P', 'A'),
        ("GET", "/tenants/{tenant_id}/audit", 'T', 'A'),
        ("GET", "/audit", 'P', 'A'),
        ("GET", "/tenants/{tenant_id}/workflows", 'T', 'R'),
        ("GET", "/tenants/{tenant_id}/workflows/{name}", 'T', 'R'),
        (
            "GET",
            "/tenants/{tenant_id}/workflows/{name}/source",
            'T',
            'R',
        ),
        (
            "GET",
            "/tenants/{tenant_id}/workflows/{name}/versions",
            'T',
            'R',
        ),
        (
            "GET",
            "/tenants/{tenant_id}/workflows/{name}/instances",
            'T',
            'R',
        ),
        (
            "GET",
            "/tenants/{tenant_id}/workflows/{name}/instances/{instance}",
            'T',
            'R',
        ),
        ("GET", "/tenants/{tenant_id}/triggers", 'T', 'R'),
        ("GET", "/tenants/{tenant_id}/triggers/{name}", 'T', 'R'),
        (
            "GET",
            "/tenants/{tenant_id}/triggers/{name}/interface",
            'T',
            'R',
        ),
        ("GET", "/tenants/{tenant_id}/executions", 'T', 'R'),
        ("GET", "/tenants/{tenant_id}/executions/{exec_id}", 'T', 'R'),
        (
            "GET",
            "/tenants/{tenant_id}/executions/{exec_id}/events",
            'T',
            'R',
        ),
        (
            "GET",
            "/tenants/{tenant_id}/executions/{exec_id}/tasks",
            'T',
            'R',
        ),
        ("POST", "/tenants/{tenant_id}/workflows", 'T', 'W'),
        (
            "POST",
            "/tenants/{tenant_id}/workflows/{name}/pause",
            'T',
            'W',
        ),
        (
            "POST",
            "/tenants/{tenant_id}/workflows/{name}/resume",
            'T',
            'W',
        ),
        (
            "DELETE",
            "/tenants/{tenant_id}/workflows/{name}/{version}",
            'T',
            'W',
        ),
        (
            "POST",
            "/tenants/{tenant_id}/workflows/{name}/versions/{version}/activate",
            'T',
            'W',
        ),
        (
            "POST",
            "/tenants/{tenant_id}/workflows/{name}/execute",
            'T',
            'W',
        ),
        (
            "POST",
            "/tenants/{tenant_id}/workflows/{name}/instances",
            'T',
            'W',
        ),
        (
            "DELETE",
            "/tenants/{tenant_id}/workflows/{name}/instances/{instance}",
            'T',
            'W',
        ),
        // Backfills (CLOACINA-T-1020), as main had them in the old table.
        (
            "GET",
            "/tenants/{tenant_id}/workflows/{name}/backfills",
            'T',
            'R',
        ),
        (
            "GET",
            "/tenants/{tenant_id}/workflows/{name}/backfills/{id}",
            'T',
            'R',
        ),
        (
            "POST",
            "/tenants/{tenant_id}/workflows/{name}/backfills",
            'T',
            'W',
        ),
        (
            "POST",
            "/tenants/{tenant_id}/workflows/{name}/backfills/{id}/cancel",
            'T',
            'W',
        ),
        (
            "POST",
            "/tenants/{tenant_id}/workflows/{name}/backfills/{id}/retry",
            'T',
            'W',
        ),
        (
            "POST",
            "/tenants/{tenant_id}/triggers/{name}/pause",
            'T',
            'W',
        ),
        (
            "POST",
            "/tenants/{tenant_id}/triggers/{name}/resume",
            'T',
            'W',
        ),
        (
            "POST",
            "/tenants/{tenant_id}/triggers/{name}/fire",
            'T',
            'W',
        ),
        ("POST", "/auth/ws-ticket", 'A', 'R'),
        ("POST", "/auth/refresh", 'A', 'R'),
        ("POST", "/auth/logout", 'A', 'R'),
        ("GET", "/auth/whoami", 'A', 'R'),
        ("POST", "/agent/register", 'A', 'R'),
        ("POST", "/agent/heartbeat", 'A', 'R'),
        ("POST", "/agent/result", 'A', 'R'),
        ("GET", "/agent/artifact/{digest}", 'A', 'R'),
        ("GET", "/agent/source/{digest}", 'A', 'R'),
        ("GET", "/agent/providers/{digest}", 'A', 'R'),
        ("GET", "/health/accumulators", 'A', 'R'),
        ("GET", "/health/reactors", 'A', 'R'),
        ("POST", "/health/reactors/{name}/fire", 'A', 'R'),
        ("GET", "/health/reactors/{name}/fires", 'A', 'R'),
        ("GET", "/health/reactors/{name}/fires/timeseries", 'A', 'R'),
        ("POST", "/health/accumulators/{name}/inject", 'A', 'R'),
        ("GET", "/health/reactors/{name}/interface", 'A', 'R'),
        ("GET", "/health/accumulators/{name}/interface", 'A', 'R'),
        ("GET", "/health/graphs", 'A', 'R'),
        ("GET", "/health/graphs/{name}", 'A', 'R'),
    ];

    fn old_decision(
        role: &str,
        tenant: Option<&str>,
        flag: bool,
        scope: char,
        level: char,
    ) -> bool {
        let level_ok = match level {
            'R' => true,
            'W' => flag || role == "admin" || role == "write",
            'A' => flag || role == "admin",
            _ => unreachable!(),
        };
        match scope {
            'P' => flag,
            'T' => (flag || tenant == Some("acme")) && level_ok,
            'A' => level_ok,
            _ => unreachable!(),
        }
    }

    /// `read`, `write` and `admin` grant what they granted before, on every
    /// route of the old table and for every shape of key, except:
    /// - the agent routes, which no person's key passes now;
    /// - reactor fire and accumulator inject, which `read` no longer passes.
    #[test]
    fn parity_with_the_ordered_levels_minus_the_listed_exceptions() {
        let table = build_authz_table();
        assert_eq!(OLD_TABLE.len(), 81);
        let mut exceptions = 0;
        for (method, path, scope, level) in OLD_TABLE {
            let method = Method::from_bytes(method.as_bytes()).unwrap();
            let full = format!("/v1{path}");
            let row = table
                .get(&method, &full)
                .unwrap_or_else(|| panic!("{method} {full} lost its row"));
            let Need::Permission(need) = row.need else {
                panic!("{method} {full} names no permission");
            };
            assert_eq!(row.credential, Credential::Bearer, "{method} {full}");
            let agent_route = path.starts_with("/agent/");
            let now_write = matches!(
                *path,
                "/health/reactors/{name}/fire" | "/health/accumulators/{name}/inject"
            );
            for flag in [false, true] {
                for role in ["read", "write", "admin"] {
                    for tenant in [None, Some("acme"), Some("public")] {
                        let old = old_decision(role, tenant, flag, *scope, *level);
                        let expected = if agent_route {
                            false
                        } else if now_write {
                            old && (flag || role != "read")
                        } else {
                            old
                        };
                        if expected != old {
                            exceptions += 1;
                        }
                        let new =
                            evaluate(&principal(role, tenant, flag), &resolved(row.scope), need)
                                .is_permit();
                        assert_eq!(
                            new, expected,
                            "{method} {full}: role {role}, tenant {tenant:?}, flag {flag}"
                        );
                    }
                }
            }
        }
        // 6 agent routes x 18 keys, and 2 routes x the 3 `read` keys with no flag.
        assert_eq!(exceptions, 6 * 18 + 2 * 3);
    }

    // ----- the agent role -----

    #[test]
    fn an_agent_key_passes_the_agent_set_and_nothing_else() {
        let table = build_authz_table();
        let agent = principal("agent", Some("acme"), false);
        let mut passed: Vec<String> = Vec::new();
        for r in table.rows() {
            let need = match r.row.need {
                Need::Permission(p) => p,
                // The delivery stream: checked by recipient below.
                Need::ByRecipient | Need::Nothing => continue,
            };
            if evaluate(&agent, &resolved(r.row.scope), need).is_permit() {
                passed.push(format!("{} {}", r.method, r.path));
            }
        }
        passed.sort();
        assert_eq!(
            passed,
            vec![
                "GET /v1/agent/artifact/{digest}",
                "GET /v1/agent/providers/{digest}",
                "GET /v1/agent/python-vendor/{digest}",
                "GET /v1/agent/source/{digest}",
                "POST /v1/agent/heartbeat",
                "POST /v1/agent/keys",
                "POST /v1/agent/register",
                "POST /v1/agent/result",
                "POST /v1/agent/undecodable",
                "POST /v1/auth/ws-ticket",
            ]
        );
    }

    #[test]
    fn a_delivery_stream_asks_for_the_permission_of_its_recipient() {
        assert_eq!(
            permission_for_recipient("agent:6f1c"),
            Some(Permission::AgentAct)
        );
        assert_eq!(
            permission_for_recipient("exec_events:42"),
            Some(Permission::ExecutionsRead)
        );
        assert_eq!(
            permission_for_recipient("ops_metrics:global"),
            Some(Permission::OpsRead)
        );
        for unknown in ["", "agent", "other:1", ":x", "Agent:1"] {
            assert_eq!(permission_for_recipient(unknown), None, "{unknown:?}");
        }
        // An agent takes an agent stream and no other; a reader the reverse.
        let agent = AuthenticatedKey {
            key_id: uuid::Uuid::nil(),
            name: "a".into(),
            permissions: "agent".into(),
            tenant_id: Some("acme".into()),
            is_admin: false,
            max_priority: None,
        };
        let reader = AuthenticatedKey {
            permissions: "read".into(),
            ..agent.clone()
        };
        let bootstrap = AuthenticatedKey {
            permissions: "admin".into(),
            tenant_id: None,
            is_admin: true,
            ..agent.clone()
        };
        assert!(authorize_caller(&agent, Permission::AgentAct).is_permit());
        assert!(!authorize_caller(&agent, Permission::ExecutionsRead).is_permit());
        assert!(!authorize_caller(&agent, Permission::OpsRead).is_permit());
        assert!(!authorize_caller(&reader, Permission::AgentAct).is_permit());
        assert!(authorize_caller(&reader, Permission::ExecutionsRead).is_permit());
        assert!(!authorize_caller(&bootstrap, Permission::AgentAct).is_permit());
        assert!(authorize_caller(&bootstrap, Permission::OpsRead).is_permit());
    }

    // ----- limits -----

    #[test]
    fn a_principal_carries_the_limits_of_its_role() {
        assert_eq!(
            principal("read", Some("acme"), false).limits.max_priority,
            None
        );
        assert_eq!(
            principal("agent", Some("acme"), false).limits.max_priority,
            None
        );
        assert_eq!(
            principal("write", Some("acme"), false).limits.max_priority,
            Some(5)
        );
        assert_eq!(
            principal("admin", Some("acme"), false).limits.max_priority,
            Some(0)
        );
        assert_eq!(
            principal("owner", Some("acme"), false).limits,
            Limits::default()
        );
        assert_eq!(principal("read", None, true).limits.max_priority, Some(0));
        assert_eq!(
            principal("write", None, false)
                .limits
                .get(LimitKind::MaxPriority),
            Some(5)
        );
        // A row can ask for a limit; the handler reads it from the `Grant`.
        let row = Row::tenant(Permission::WorkflowsRun).limit(LimitKind::MaxPriority);
        assert_eq!(row.limit, Some(LimitKind::MaxPriority));
        assert_eq!(Row::tenant(Permission::WorkflowsRun).limit, None);
    }

    // ----- the table -----

    #[test]
    fn the_table_is_sound_and_every_permission_has_a_route() {
        let table = build_authz_table();
        assert_eq!(table.faults(), Vec::<String>::new());
        // 81 rows before CLOACINA-T-1055 (76, and the five backfill routes of
        // CLOACINA-T-1020), `POST /v1/agent/keys`, three WebSockets and seven
        // public routes; the three trusted key routes of CLOACINA-T-1061;
        // `POST /v1/agent/undecodable` of CLOACINA-T-1058;
        // `GET /v1/agent/python-vendor/{digest}` of CLOACINA-T-1073;
        // `PATCH /v1/tenants/{tenant_id}/keys/{key_id}` of CLOACINA-T-1123.
        assert_eq!(table.len(), 98);
        for permission in Permission::ALL {
            let by_row = table
                .rows()
                .iter()
                .any(|r| r.row.need == Need::Permission(permission));
            let by_recipient = ["agent:1", "exec_events:1", "ops_metrics:global"]
                .iter()
                .any(|r| permission_for_recipient(r) == Some(permission));
            assert!(by_row || by_recipient, "{} has no route", permission.name());
        }
        let names: std::collections::HashSet<_> =
            Permission::ALL.iter().map(|p| p.name()).collect();
        assert_eq!(names.len(), Permission::ALL.len());
    }

    #[test]
    fn the_table_names_the_rows_this_task_changed() {
        let table = build_authz_table();
        let row = |m: Method, p: &str| *table.get(&m, p).unwrap_or_else(|| panic!("{m} {p}"));
        // CLOACINA-T-1040: the key top-up has a row.
        assert_eq!(
            row(Method::POST, "/v1/agent/keys").need,
            Need::Permission(Permission::AgentAct)
        );
        // The WebSockets have rows and take a ticket.
        for path in [
            "/v1/ws/accumulator/{name}",
            "/v1/ws/reactor/{name}",
            "/v1/ws/delivery/{recipient}",
        ] {
            assert_eq!(
                row(Method::GET, path).credential,
                Credential::BearerOrTicket
            );
        }
        assert_eq!(
            row(Method::GET, "/v1/ws/accumulator/{name}").need,
            Need::Permission(Permission::AccumulatorsInject)
        );
        assert_eq!(
            row(Method::GET, "/v1/ws/delivery/{recipient}").need,
            Need::ByRecipient
        );
        // No REST route takes a ticket.
        for r in table.rows() {
            if !r.path.starts_with("/v1/ws/") {
                assert_ne!(r.row.credential, Credential::BearerOrTicket, "{}", r.path);
            }
        }
        assert_eq!(
            row(Method::POST, "/v1/health/reactors/{name}/fire").need,
            Need::Permission(Permission::ReactorsOperate)
        );
        assert_eq!(
            row(Method::POST, "/v1/health/accumulators/{name}/inject").need,
            Need::Permission(Permission::AccumulatorsInject)
        );
        assert_eq!(table.get(&Method::GET, "/v1/totally/unknown"), None);
    }

    /// CLOACINA-A-0018: the tenant admin manages the trusted keys of its
    /// tenant, and a platform admin those of any tenant. No other role does.
    #[test]
    fn trusted_keys_are_for_the_tenant_admin_and_the_platform_admin() {
        let p = Permission::TrustedKeysManage;
        assert_eq!(p.name(), "trusted_keys.manage");
        assert!(Role::Admin.grants(p));
        for role in [Role::Read, Role::Write, Role::Agent] {
            assert!(!role.grants(p), "{} holds {}", role.as_str(), p.name());
        }

        let table = build_authz_table();
        for (method, path, audit) in [
            (
                Method::POST,
                "/v1/tenants/{tenant_id}/trusted-keys",
                Some(actions::TRUSTED_KEY_ADD),
            ),
            (Method::GET, "/v1/tenants/{tenant_id}/trusted-keys", None),
            (
                Method::DELETE,
                "/v1/tenants/{tenant_id}/trusted-keys/{key_id}",
                Some(actions::TRUSTED_KEY_REVOKE),
            ),
        ] {
            let row = table
                .get(&method, path)
                .unwrap_or_else(|| panic!("{method} {path} has no row"));
            assert_eq!(row.scope, Scope::TenantPath, "{method} {path}");
            assert_eq!(row.need, Need::Permission(p), "{method} {path}");
            assert_eq!(row.credential, Credential::Bearer, "{method} {path}");
            match (row.audit, audit) {
                (Audit::Action { action, .. }, Some(want)) => assert_eq!(action, want),
                (Audit::Read, None) => {}
                (got, _) => panic!("{method} {path}: audit {got:?}"),
            }
        }

        // The path names the tenant `acme`.
        let scope = ResolvedScope::Tenant("acme".into());
        let permit = |role: &str, tenant: Option<&str>, flag: bool| {
            evaluate(&principal(role, tenant, flag), &scope, p).is_permit()
        };
        assert!(permit("admin", Some("acme"), false));
        // A platform admin, for any tenant, whatever its role string.
        assert!(permit("admin", None, true));
        assert!(permit("read", Some("other"), true));
        // The admin of another tenant, and a key with no tenant.
        assert!(!permit("admin", Some("other"), false));
        assert!(!permit("admin", None, false));
        for role in ["read", "write", "agent", "owner"] {
            assert!(!permit(role, Some("acme"), false), "{role}");
        }
    }

    #[test]
    fn every_route_that_changes_state_names_its_audit_row() {
        let table = build_authz_table();
        let mut audited = 0;
        let mut unaudited = Vec::new();
        for r in table.rows() {
            match r.row.audit {
                Audit::Action { .. } => audited += 1,
                Audit::NoRow(why) => {
                    assert!(!why.is_empty());
                    unaudited.push(format!("{} {}", r.method, r.path));
                }
                Audit::Read => assert_eq!(r.method, Method::GET, "{}", r.path),
            }
        }
        // 32 before CLOACINA-T-1055 (29, and backfill create, cancel and
        // retry); plus reactor fire, accumulator inject,
        // the accumulator connect, agent register, agent key top-up, the two
        // logins and logout; the add and the revoke of a trusted key
        // (CLOACINA-T-1061); the key update (CLOACINA-T-1123).
        assert_eq!(audited, 43);
        unaudited.sort();
        assert_eq!(
            unaudited,
            vec![
                "POST /v1/agent/heartbeat",
                "POST /v1/agent/result",
                "POST /v1/agent/undecodable",
                "POST /v1/auth/refresh",
                "POST /v1/auth/ws-ticket",
            ]
        );
        // A row that forgets is a fault, so the server does not start.
        let mut bad = AuthzTable::default();
        bad.add(
            Method::POST,
            "/v1/tenants/{tenant_id}/things",
            Row::tenant(Permission::WorkflowsOperate),
        );
        assert_eq!(bad.faults().len(), 1);
        assert!(bad.faults()[0].contains("audit"));
    }

    #[test]
    fn a_row_that_contradicts_itself_is_a_fault() {
        let mut t = AuthzTable::default();
        t.add(Method::GET, "/v1/a", Row::tenant(Permission::WorkflowsRead));
        t.add(Method::GET, "/v1/b/{name}", Row::by_recipient());
        t.add(
            Method::DELETE,
            "/v1/c/{name}",
            Row::caller(Permission::KeysManage).audit(actions::KEY_REVOKE, Some("key_id")),
        );
        t.add(Method::GET, "/v1/d", Row::public().ticket());
        t.add(Method::GET, "/v1/d", Row::public());
        let faults = t.faults();
        assert_eq!(faults.len(), 5, "{faults:?}");
    }

    // ----- the start-up check -----

    async fn nothing() {}

    /// The table for a router of two routes.
    fn small_table() -> AuthzTable {
        let mut t = AuthzTable::default();
        t.add(Method::GET, "/health", Row::public());
        t.add(
            Method::GET,
            "/v1/tenants/{tenant_id}/workflows",
            Row::tenant(Permission::WorkflowsRead),
        );
        t
    }

    #[test]
    fn a_route_with_no_row_fails_the_start_up_check() {
        let table = small_table();
        let top = RouteSet::<()>::new("").get("/health", nothing);
        let v1 = RouteSet::<()>::new("/v1").get("/tenants/{tenant_id}/workflows", nothing);
        let mut registered = top.registered().to_vec();
        registered.extend_from_slice(v1.registered());
        assert_eq!(check_coverage(&registered, &table), Ok(()));

        // A new route, and nobody gave it a row.
        let v1 = v1.post("/tenants/{tenant_id}/things", nothing);
        let mut registered = top.registered().to_vec();
        registered.extend_from_slice(v1.registered());
        let err = check_coverage(&registered, &table).unwrap_err();
        assert_eq!(
            err.routes_without_row,
            vec!["POST /v1/tenants/{tenant_id}/things"]
        );
        assert!(err.rows_without_route.is_empty());
        assert!(
            err.to_string()
                .contains("route with no row: POST /v1/tenants/{tenant_id}/things")
        );

        // The same path with another method is another route.
        let v1 = RouteSet::<()>::new("/v1").delete("/tenants/{tenant_id}/workflows", nothing);
        let err = check_coverage(v1.registered(), &table).unwrap_err();
        assert_eq!(
            err.routes_without_row,
            vec!["DELETE /v1/tenants/{tenant_id}/workflows"]
        );
    }

    #[test]
    fn a_row_with_no_route_fails_the_start_up_check() {
        let table = small_table();
        let top = RouteSet::<()>::new("").get("/health", nothing);
        let err = check_coverage(top.registered(), &table).unwrap_err();
        assert!(err.routes_without_row.is_empty());
        assert_eq!(
            err.rows_without_route,
            vec!["GET /v1/tenants/{tenant_id}/workflows"]
        );
    }

    #[test]
    fn the_start_up_check_refuses_a_bad_row() {
        let mut table = small_table();
        table.add(
            Method::PUT,
            "/v1/tenants/{tenant_id}/things",
            Row::tenant(Permission::WorkflowsOperate),
        );
        let set = RouteSet::<()>::new("")
            .get("/health", nothing)
            .get("/v1/tenants/{tenant_id}/workflows", nothing)
            .put("/v1/tenants/{tenant_id}/things", nothing);
        let err = check_coverage(set.registered(), &table).unwrap_err();
        assert!(err.routes_without_row.is_empty() && err.rows_without_route.is_empty());
        assert_eq!(err.faults.len(), 1);
    }

    /// The OpenAPI document is made from the handlers' own annotations: a
    /// second net under the start-up check.
    #[test]
    fn every_documented_route_has_a_row() {
        use utoipa::OpenApi;

        let table = build_authz_table();
        let doc = crate::openapi::ApiDoc::openapi();
        let mut missing = Vec::new();
        for (path, item) in doc.paths.paths.iter() {
            for (method, op) in [
                (Method::GET, &item.get),
                (Method::POST, &item.post),
                (Method::PUT, &item.put),
                (Method::DELETE, &item.delete),
                (Method::PATCH, &item.patch),
            ] {
                if op.is_some() && table.get(&method, path).is_none() {
                    missing.push(format!("{method} {path}"));
                }
            }
        }
        missing.sort();
        assert!(
            missing.is_empty(),
            "documented routes with no row:\n  {}",
            missing.join("\n  ")
        );
    }

    // ----- the docs -----

    const DOC_START: &str = "<!-- authz-table:start (generated from crates/cloacina-server/src/routes/authz.rs; do not edit) -->";
    const DOC_END: &str = "<!-- authz-table:end -->";

    /// The tables of the security model page are the ones this file renders.
    /// To rewrite the page after a change to the table:
    /// `CLOACINA_UPDATE_AUTHZ_DOC=1 cargo test -p cloacina-server --lib the_security_model_page`
    #[test]
    fn the_security_model_page_holds_the_generated_tables() {
        let page_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/src/content/docs/explanation/service/security-model.md");
        let page = std::fs::read_to_string(&page_path).expect("the security model page");
        let start = page.find(DOC_START).expect("the start marker") + DOC_START.len();
        let end = page.find(DOC_END).expect("the end marker");
        let want = format!("\n\n{}\n", render_markdown(&build_authz_table()));
        if page[start..end] == want {
            return;
        }
        if std::env::var("CLOACINA_UPDATE_AUTHZ_DOC").is_ok() {
            let new = format!("{}{}{}", &page[..start], want, &page[end..]);
            std::fs::write(&page_path, new).expect("write the page");
            return;
        }
        panic!(
            "the tables of {} differ from the authorization table. Run:\n  \
             CLOACINA_UPDATE_AUTHZ_DOC=1 cargo test -p cloacina-server --lib the_security_model_page",
            page_path.display()
        );
    }

    #[test]
    fn the_rendered_tables_name_every_permission_and_route() {
        let table = build_authz_table();
        let text = render_markdown(&table);
        for p in Permission::ALL {
            assert!(text.contains(&format!("| `{}` |", p.name())));
        }
        for r in table.rows() {
            assert!(text.contains(&format!("| {} | `{}` |", r.method, r.path)));
        }
        assert!(text.contains("| `agent.act` |"));
        assert!(
            text.contains("| POST | `/v1/agent/heartbeat` | caller's key | `agent.act` | none: ")
        );
    }
}
