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

//! The limit on a priority that a principal set for later runs
//! (CLOACINA-T-1045, CLOACINA-T-1123).
//!
//! A package declares priorities in its code (`#[workflow(priority = 2)]`),
//! and a schedule can be created with one. The runs start later, with no
//! principal in the request, so each run start asks here for the limit of
//! the principal that published the package or created the schedule, as it
//! is at that time:
//!
//! - `key:<id>`: a live key has its own `max_priority`, else the default of
//!   its role (a platform admin key: 0). A key that is gone has the default
//!   of the role it had.
//! - `oidc:<issuer>:<sub>`: the current OIDC map, run on the claims of the
//!   person's newest login session in the tenant. A person with no session,
//!   or one that no rule maps to the tenant any more, has the default of the
//!   role recorded.
//! - `local:<account_id>`: the default of the account's role; a disabled or
//!   deleted account has the default of the role recorded.
//!
//! A package with no record of its publisher (published before
//! CLOACINA-T-1123) is held to the default of `write`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use cloacina::dal::unified::PrioritySetter;
use cloacina::execution_planner::{PriorityAuthority, PriorityChange};
use cloacina_api_types::{LOWEST_PRIORITY, PriorityGrant};
use tracing::warn;

use crate::routes::authz::{LimitKind, Role};

/// The audit action of a change of the priority given to the runs of a
/// package or a schedule.
pub const ACTION_PRIORITY_HELD: &str = "priority.held";

/// What the limit of a person is read from: the admin database (keys, login
/// sessions, local accounts, the audit record) and the OIDC map.
#[derive(Clone)]
pub struct PrincipalLookup {
    pub admin: cloacina::Database,
    pub oidc_policy: Arc<crate::oidc::MappingPolicy>,
}

/// The limits of one tenant's packages and schedules.
pub struct PublisherLimit {
    tenant: String,
    lookup: Option<PrincipalLookup>,
    /// The priority last given to each workflow that was held while its
    /// package had no record of its publisher, so the log line is written
    /// when the given priority changes, not for each run.
    last_given: Mutex<HashMap<String, PriorityGrant>>,
}

impl PublisherLimit {
    /// With no lookup: each setter has the default of the role recorded.
    pub fn for_tenant(tenant: impl Into<String>) -> Self {
        Self {
            tenant: tenant.into(),
            lookup: None,
            last_given: Mutex::new(HashMap::new()),
        }
    }

    /// With the lookup of keys, login sessions and accounts.
    pub fn with_lookup(tenant: impl Into<String>, lookup: PrincipalLookup) -> Self {
        Self {
            lookup: Some(lookup),
            ..Self::for_tenant(tenant)
        }
    }

    /// True when `grant` is not what this process last saw for the workflow.
    fn changed(&self, workflow_name: &str, grant: PriorityGrant) -> bool {
        let mut last = self.last_given.lock().unwrap_or_else(|e| e.into_inner());
        last.insert(workflow_name.to_string(), grant) != Some(grant)
    }
}

/// The highest priority the code of a package with no record of its
/// publisher may set: the default of the role that may publish.
pub fn declared_priority_limit() -> Option<u8> {
    Role::Write.default_limits().get(LimitKind::MaxPriority)
}

/// The default limit of a role string. A role that may set no priority, or
/// no role, gets the lowest.
pub fn role_limit(role: &str) -> u8 {
    Role::parse(role)
        .and_then(|r| r.default_limits().get(LimitKind::MaxPriority))
        .unwrap_or(LOWEST_PRIORITY)
}

/// The setter of a request: the person behind a minted key, else the key;
/// and its role (`admin` for a platform admin key).
pub async fn setter_of(
    admin: &cloacina::Database,
    auth: &crate::routes::auth::AuthenticatedKey,
) -> PrioritySetter {
    let dal = cloacina::dal::DAL::new(admin.clone());
    let issued_via = match dal.api_keys().get_key(auth.key_id).await {
        Ok(info) => info.and_then(|k| k.issued_via),
        Err(e) => {
            warn!(key_id = %auth.key_id, error = %e, "priority: the key row was not read");
            None
        }
    };
    PrioritySetter {
        by: issued_via.unwrap_or_else(|| format!("key:{}", auth.key_id)),
        role: if auth.is_admin {
            Role::Admin.as_str().to_string()
        } else {
            auth.permissions.clone()
        },
    }
}

/// True when a key's tenant is `tenant` (a key with no tenant is in
/// `public`).
fn in_tenant(key_tenant: Option<&str>, tenant: &str) -> bool {
    key_tenant.unwrap_or("public") == tenant
}

/// The highest priority `setter` may set in `tenant` now.
pub async fn limit_now(lookup: &PrincipalLookup, tenant: &str, setter: &PrioritySetter) -> u8 {
    let recorded = role_limit(&setter.role);
    let dal = cloacina::dal::DAL::new(lookup.admin.clone());
    let now = chrono::Utc::now();

    if let Some(id) = setter.by.strip_prefix("key:") {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return recorded;
        };
        return match dal.api_keys().get_key(id).await {
            Ok(Some(key)) if key.is_live(now) => {
                let role = if key.is_admin {
                    role_limit(Role::Admin.as_str())
                } else {
                    role_limit(&key.permissions)
                };
                // As `Principal::from_key`: an own limit only on a role that
                // may set a priority.
                let may_set = key.is_admin
                    || Role::parse(&key.permissions)
                        .and_then(|r| r.default_limits().get(LimitKind::MaxPriority))
                        .is_some();
                match key.max_priority {
                    Some(own) if may_set => own,
                    _ => role,
                }
            }
            Ok(_) => recorded,
            Err(e) => {
                warn!(tenant, setter = %setter.by, error = %e, "priority: the key was not read; the recorded role applies");
                recorded
            }
        };
    }

    if setter.by.starts_with("oidc:") {
        let keys = match dal.api_keys().keys_issued_via(&setter.by).await {
            Ok(keys) => keys,
            Err(e) => {
                warn!(tenant, setter = %setter.by, error = %e, "priority: the login keys were not read; the recorded role applies");
                return recorded;
            }
        };
        let ids: Vec<uuid::Uuid> = keys
            .iter()
            .filter(|k| in_tenant(k.tenant_id.as_deref(), tenant))
            .map(|k| k.id)
            .collect();
        let claims = match dal.oidc_sessions().latest_claims_for_keys(ids).await {
            Ok(Some((issuer, raw))) => serde_json::from_str::<crate::oidc::IdentityClaims>(&raw)
                .ok()
                .map(|c| (issuer, c)),
            Ok(None) => None,
            Err(e) => {
                warn!(tenant, setter = %setter.by, error = %e, "priority: the login session was not read; the recorded role applies");
                None
            }
        };
        let Some((issuer, claims)) = claims else {
            return recorded;
        };
        return lookup
            .oidc_policy
            .resolve_all(&claims, &issuer)
            .into_iter()
            .find(|p| in_tenant(p.tenant.as_deref(), tenant))
            .map(|p| {
                let role = role_limit(&p.role);
                match p.max_priority {
                    Some(own) if role < LOWEST_PRIORITY => own,
                    _ => role,
                }
            })
            .unwrap_or(recorded);
    }

    if let Some(id) = setter.by.strip_prefix("local:") {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return recorded;
        };
        return match dal.local_accounts().get_by_id(id).await {
            Ok(Some(account)) if account.is_active() => role_limit(&account.role),
            Ok(_) => recorded,
            Err(e) => {
                warn!(tenant, setter = %setter.by, error = %e, "priority: the account was not read; the recorded role applies");
                recorded
            }
        };
    }

    recorded
}

/// The priority of a schedule created with one (CLOACINA-T-1123): what it
/// asks for and what its fires get now, held to its creator's limit, and the
/// sentence when the two differ. `(None, None)` for a schedule created with no
/// priority.
pub async fn schedule_priority(
    state: &crate::AppState,
    tenant: &str,
    dal: &cloacina::dal::DAL,
    schedule_id: uuid::Uuid,
) -> (Option<PriorityGrant>, Option<String>) {
    let row = match dal
        .priority_grants()
        .get(
            cloacina::dal::unified::priority_grants::KIND_SCHEDULE,
            &schedule_id.to_string(),
        )
        .await
    {
        Ok(row) => row,
        Err(e) => {
            warn!(tenant, schedule = %schedule_id, error = %e, "priority: the schedule's record was not read");
            None
        }
    };
    let Some((row, asked)) = row.and_then(|r| r.asked().map(|a| (r, a))) else {
        return (None, None);
    };
    let lookup = PrincipalLookup {
        admin: state.database.clone(),
        oidc_policy: state.oidc_policy.clone(),
    };
    let limit = limit_now(&lookup, tenant, &row.setter()).await;
    let grant = PriorityGrant::held_to(asked, Some(limit));
    let notice = grant.is_held().then(|| {
        format!(
            "priority {} was asked for and its runs get priority {}: {} is the highest \
             priority its creator may set now",
            grant.asked, grant.given, grant.given
        )
    });
    (Some(grant), notice)
}

/// Record the priority a schedule is created with, and who created it
/// (CLOACINA-T-1123). A write that fails is logged; the schedule then fires
/// at the priority its workflow declares.
pub async fn record_schedule_priority(
    state: &crate::AppState,
    tenant: &str,
    dal: &cloacina::dal::DAL,
    schedule_id: uuid::Uuid,
    asked: u8,
    auth: &crate::routes::auth::AuthenticatedKey,
) {
    let setter = setter_of(&state.database, auth).await;
    if let Err(e) = dal
        .priority_grants()
        .record(
            cloacina::dal::unified::priority_grants::KIND_SCHEDULE,
            &schedule_id.to_string(),
            &setter,
            Some(asked),
        )
        .await
    {
        warn!(tenant, schedule = %schedule_id, error = %e, "priority: the schedule's priority was not recorded; it fires at the priority its workflow declares");
    }
}

/// The sentence of the upload answer: the limit the runs of a package are
/// held to, when it is below the highest priority.
pub fn upload_notice(limit: u8) -> Option<String> {
    (limit > 0).then(|| {
        format!(
            "the runs of this package are held to priority {limit} or lower: {limit} is the \
             highest priority its publisher may set now. A workflow or task that declares a \
             higher priority (a smaller number) runs at {limit}"
        )
    })
}

#[async_trait]
impl PriorityAuthority for PublisherLimit {
    async fn declared_limit(&self, _workflow_name: &str) -> Option<u8> {
        declared_priority_limit()
    }

    async fn run_held(&self, workflow_name: &str, grant: PriorityGrant) {
        if self.changed(workflow_name, grant) {
            warn!(
                tenant = %self.tenant,
                workflow = %workflow_name,
                priority_asked = grant.asked,
                priority_given = grant.given,
                "priority lowered: the workflow asks for priority {} and its runs get \
                 priority {}",
                grant.asked,
                grant.given
            );
        }
    }

    async fn limit_of(&self, setter: &PrioritySetter) -> Option<u8> {
        Some(match &self.lookup {
            Some(lookup) => limit_now(lookup, &self.tenant, setter).await,
            None => role_limit(&setter.role),
        })
    }

    async fn given_changed(&self, change: &PriorityChange) {
        let held = change.grant.is_held();
        if held {
            warn!(
                tenant = %self.tenant,
                workflow = %change.workflow_name,
                kind = %change.kind,
                target = %change.target,
                set_by = %change.setter.by,
                priority_asked = change.grant.asked,
                priority_given = change.grant.given,
                "priority lowered: the {} asks for priority {} and its runs get priority {}, \
                 the highest its {} may set now",
                change.kind,
                change.grant.asked,
                change.grant.given,
                if change.kind == "schedule" { "creator" } else { "publisher" },
            );
        }
        let Some(lookup) = &self.lookup else {
            return;
        };
        let dal = cloacina::dal::DAL::new(lookup.admin.clone());
        let row = cloacina::dal::unified::audit_events::NewAuditEvent {
            tenant_id: Some(self.tenant.clone()),
            actor_key_id: "server".to_string(),
            actor_key_name: "server".to_string(),
            actor_identity: Some(change.setter.by.clone()),
            action: ACTION_PRIORITY_HELD.to_string(),
            target_kind: change.kind.clone(),
            target: Some(change.target.clone()),
            outcome: cloacina::dal::unified::audit_events::OUTCOME_OK.to_string(),
            detail: serde_json::json!({
                "workflow": change.workflow_name,
                "set_by": change.setter.by,
                "set_by_role": change.setter.role,
                "priority_asked": change.grant.asked,
                "priority_given": change.grant.given,
                "priority_lowered": held,
            }),
        };
        if let Err(e) = dal.audit_events().insert(row).await {
            metrics::counter!("cloacina_audit_write_errors_total").increment(1);
            warn!(tenant = %self.tenant, error = %e, "audit: the priority.held row was not written");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_package_with_no_publisher_may_declare_what_a_write_principal_may_set() {
        let limit = PublisherLimit::for_tenant("acme");
        assert_eq!(limit.declared_limit("orders").await, Some(5));
        assert_eq!(
            PriorityGrant::held_to(2, declared_priority_limit()).given,
            5
        );
        assert_eq!(
            PriorityGrant::held_to(8, declared_priority_limit()).given,
            8
        );
    }

    #[tokio::test]
    async fn with_no_lookup_a_setter_has_the_default_of_the_role_recorded() {
        let limit = PublisherLimit::for_tenant("acme");
        let setter = |role: &str| PrioritySetter {
            by: "key:00000000-0000-0000-0000-000000000000".to_string(),
            role: role.to_string(),
        };
        assert_eq!(limit.limit_of(&setter("admin")).await, Some(0));
        assert_eq!(limit.limit_of(&setter("write")).await, Some(5));
        assert_eq!(limit.limit_of(&setter("read")).await, Some(10));
        assert_eq!(limit.limit_of(&setter("owner")).await, Some(10));
    }

    #[test]
    fn the_upload_notice_names_the_limit() {
        assert_eq!(upload_notice(0), None);
        let notice = upload_notice(5).expect("a notice");
        assert!(notice.contains("priority 5"), "{notice}");
    }

    #[test]
    fn the_ageing_interval_is_two_minutes_unless_the_setting_says_otherwise() {
        use std::time::Duration;
        let parse = crate::parse_priority_aging_interval;
        assert_eq!(parse(None), Duration::from_secs(120));
        assert_eq!(parse(Some("")), Duration::from_secs(120));
        assert_eq!(parse(Some("300")), Duration::from_secs(300));
        assert_eq!(parse(Some(" 60 ")), Duration::from_secs(60));
        assert_eq!(parse(Some("0")), Duration::ZERO, "0 switches ageing off");
        assert_eq!(parse(Some("2m")), Duration::from_secs(120), "not a number");
    }

    /// CLOACINA-T-1123: the value of `--priority-aging-interval-s` is the
    /// interval of the runners. The default is set, so other tests see none
    /// changed.
    #[test]
    fn the_flag_sets_the_ageing_interval_of_the_runners() {
        use std::time::Duration;
        crate::set_priority_aging_interval_seconds(120).unwrap();
        assert_eq!(crate::priority_aging_interval(), Duration::from_secs(120));
        assert!(crate::set_priority_aging_interval_seconds(u64::MAX).is_err());
    }

    #[test]
    fn the_log_line_is_for_a_change_of_the_given_priority_not_for_each_run() {
        let limit = PublisherLimit::for_tenant("acme");
        let held = PriorityGrant::held_to(2, Some(5));
        assert!(limit.changed("orders", held), "the first held run");
        assert!(!limit.changed("orders", held), "the same again");
        assert!(!limit.changed("orders", held));
        let other = PriorityGrant::held_to(2, Some(7));
        assert!(limit.changed("orders", other), "the given priority moved");
        assert!(limit.changed("billing", held), "another workflow");
    }
}
