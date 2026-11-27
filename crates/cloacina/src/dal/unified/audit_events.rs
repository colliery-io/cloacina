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

//! Audit-event DAL (CLOACINA-T-1024): a durable record of who did what, when,
//! in which tenant.
//!
//! The table lives in the admin schema and carries a tenant column, so use
//! this DAL on the admin database. Rows are append-only: there is an insert
//! and bounded reads, no update and no delete. Retention is not handled here.
//!
//! The list is newest first, ordered by `id` (a database sequence). The
//! cursor is the `id` of the last row of a page: the next page holds rows
//! with a smaller `id`.

use std::collections::HashMap;

use super::DAL;
use super::models::{NewUnifiedAuditEvent, UnifiedAuditEvent};
use crate::database::schema::unified::audit_events;
use crate::database::universal_types::UniversalTimestamp;
use crate::error::ValidationError;
use diesel::prelude::*;

/// The action of a package upload; [`AuditEventDAL::package_uploaders`] reads
/// rows with it.
pub const ACTION_PACKAGE_UPLOAD: &str = "package.upload";

/// Outcome of an action that ran.
pub const OUTCOME_OK: &str = "ok";
/// Outcome of an action that authorization refused.
pub const OUTCOME_DENIED: &str = "denied";

/// Default page size of [`AuditEventDAL::list`].
pub const DEFAULT_LIST_LIMIT: i64 = 50;
/// Largest page size of [`AuditEventDAL::list`].
pub const MAX_LIST_LIMIT: i64 = 200;

/// An audit row to write.
#[derive(Debug, Clone)]
pub struct NewAuditEvent {
    /// The tenant the action applied to; `None` for a platform action.
    pub tenant_id: Option<String>,
    pub actor_key_id: String,
    pub actor_key_name: String,
    /// The key's `issued_via` (`oidc:<issuer>:<sub>`, `local:<account_id>`).
    pub actor_identity: Option<String>,
    /// A short fixed string, for example `package.upload`.
    pub action: String,
    pub target_kind: String,
    pub target: Option<String>,
    pub outcome: String,
    /// A small JSON object. Never a secret, a request body or a context.
    pub detail: serde_json::Value,
}

/// An audit row as read.
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub id: i64,
    pub created_at: UniversalTimestamp,
    pub tenant_id: Option<String>,
    pub actor_key_id: String,
    pub actor_key_name: String,
    pub actor_identity: Option<String>,
    pub action: String,
    pub target_kind: String,
    pub target: Option<String>,
    pub outcome: String,
    pub detail: serde_json::Value,
}

impl AuditEvent {
    /// The best name of the actor: the identity when the key has one, else
    /// the key name.
    pub fn actor_label(&self) -> &str {
        self.actor_identity
            .as_deref()
            .unwrap_or(&self.actor_key_name)
    }
}

/// Filters of [`AuditEventDAL::list`]. Every field narrows the result.
#[derive(Debug, Clone, Default)]
pub struct AuditEventFilter {
    /// Only rows of this tenant.
    pub tenant_id: Option<String>,
    /// Only rows with this action.
    pub action: Option<String>,
    /// Only rows whose key id, key name or identity equals this.
    pub actor: Option<String>,
    /// Only rows at or after this time.
    pub since: Option<UniversalTimestamp>,
    /// Only rows before this time.
    pub until: Option<UniversalTimestamp>,
    /// Only rows with an `id` below this one (the cursor).
    pub before_id: Option<i64>,
    /// Page size; `None` is [`DEFAULT_LIST_LIMIT`]. Capped at
    /// [`MAX_LIST_LIMIT`].
    pub limit: Option<i64>,
    /// With `tenant_id`: only rows after the newest [`ACTION_TENANT_REMOVED`]
    /// row of that tenant, so a tenant made again with a removed name reads
    /// none of the old tenant's rows (CLOACINA-T-1095).
    pub current_tenant_only: bool,
}

/// The action of the row that tenant removal writes inside its transaction
/// (`DatabaseAdmin::remove_tenant`): the rows of the tenant before it belong
/// to a tenant that was removed.
pub const ACTION_TENANT_REMOVED: &str = "tenant.removed";

fn to_domain(r: UnifiedAuditEvent) -> AuditEvent {
    AuditEvent {
        id: r.id,
        created_at: r.created_at,
        tenant_id: r.tenant_id,
        actor_key_id: r.actor_key_id,
        actor_key_name: r.actor_key_name,
        actor_identity: r.actor_identity,
        action: r.action,
        target_kind: r.target_kind,
        target: r.target,
        outcome: r.outcome,
        detail: serde_json::from_str(&r.detail)
            .unwrap_or_else(|_| serde_json::Value::Object(Default::default())),
    }
}

/// Data access layer for the audit record.
#[derive(Clone)]
pub struct AuditEventDAL<'a> {
    dal: &'a DAL,
}

impl<'a> AuditEventDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Append one audit row.
    pub async fn insert(&self, event: NewAuditEvent) -> Result<(), ValidationError> {
        let row = NewUnifiedAuditEvent {
            created_at: UniversalTimestamp::now(),
            tenant_id: event.tenant_id,
            actor_key_id: event.actor_key_id,
            actor_key_name: event.actor_key_name,
            actor_identity: event.actor_identity,
            action: event.action,
            target_kind: event.target_kind,
            target: event.target,
            outcome: event.outcome,
            detail: event.detail.to_string(),
        };
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(audit_events::table)
                .values(&row)
                .execute(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(())
    }

    /// Rows that match `filter`, newest first, at most the capped limit.
    pub async fn list(&self, filter: AuditEventFilter) -> Result<Vec<AuditEvent>, ValidationError> {
        let limit = filter
            .limit
            .unwrap_or(DEFAULT_LIST_LIMIT)
            .clamp(1, MAX_LIST_LIMIT);
        let rows: Vec<UnifiedAuditEvent> = crate::interact_on_backend!(self.dal, |conn| {
            let floor: Option<i64> = match (&filter.tenant_id, filter.current_tenant_only) {
                (Some(t), true) => audit_events::table
                    .filter(audit_events::tenant_id.eq(t.clone()))
                    .filter(audit_events::action.eq(ACTION_TENANT_REMOVED))
                    .select(diesel::dsl::max(audit_events::id))
                    .first(conn)?,
                _ => None,
            };
            let mut q = audit_events::table.into_boxed();
            if let Some(t) = &filter.tenant_id {
                q = q.filter(audit_events::tenant_id.eq(t.clone()));
            }
            if let Some(floor) = floor {
                q = q.filter(audit_events::id.gt(floor));
            }
            if let Some(a) = &filter.action {
                q = q.filter(audit_events::action.eq(a.clone()));
            }
            if let Some(actor) = &filter.actor {
                q = q.filter(
                    audit_events::actor_key_id
                        .eq(actor.clone())
                        .or(audit_events::actor_key_name.eq(actor.clone()))
                        .or(audit_events::actor_identity
                            .eq(actor.clone())
                            .assume_not_null()),
                );
            }
            if let Some(since) = filter.since {
                q = q.filter(audit_events::created_at.ge(since));
            }
            if let Some(until) = filter.until {
                q = q.filter(audit_events::created_at.lt(until));
            }
            if let Some(before) = filter.before_id {
                q = q.filter(audit_events::id.lt(before));
            }
            q.order(audit_events::id.desc()).limit(limit).load(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(rows.into_iter().map(to_domain).collect())
    }

    /// Who uploaded each of `package_ids` in `tenant_id`: the actor of the
    /// first `package.upload` row with outcome `ok` whose target is the
    /// package id. An upload of the same content again returns the same
    /// package id and writes a new row; the first uploader stays the answer.
    /// A package with no such row is absent from the map. Rows of a removed
    /// tenant of the same name (before its [`ACTION_TENANT_REMOVED`] row) are
    /// not read.
    pub async fn package_uploaders(
        &self,
        tenant_id: &str,
        package_ids: Vec<String>,
    ) -> Result<HashMap<String, String>, ValidationError> {
        if package_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let tenant = tenant_id.to_string();
        let rows: Vec<UnifiedAuditEvent> = crate::interact_on_backend!(self.dal, |conn| {
            let floor: Option<i64> = audit_events::table
                .filter(audit_events::tenant_id.eq(&tenant))
                .filter(audit_events::action.eq(ACTION_TENANT_REMOVED))
                .select(diesel::dsl::max(audit_events::id))
                .first(conn)?;
            audit_events::table
                .filter(audit_events::id.gt(floor.unwrap_or(0)))
                .filter(audit_events::tenant_id.eq(&tenant))
                .filter(audit_events::action.eq(ACTION_PACKAGE_UPLOAD))
                .filter(audit_events::outcome.eq(OUTCOME_OK))
                .filter(audit_events::target.eq_any(&package_ids))
                .order(audit_events::id.asc())
                .load(conn)
        })
        .map_err(ValidationError::from)?;
        let mut out = HashMap::new();
        for row in rows.into_iter().map(to_domain) {
            if let Some(target) = row.target.clone() {
                // Ascending order: the first row of a package wins.
                let label = row.actor_label().to_string();
                out.entry(target).or_insert(label);
            }
        }
        Ok(out)
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::database::Database;

    fn shared_url() -> String {
        format!(
            "file:audit_events_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        )
    }

    async fn dal_for(url: &str) -> DAL {
        let db = Database::new(url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    fn event(tenant: Option<&str>, key_name: &str, action: &str, target: &str) -> NewAuditEvent {
        NewAuditEvent {
            tenant_id: tenant.map(str::to_string),
            actor_key_id: format!("id-{key_name}"),
            actor_key_name: key_name.to_string(),
            actor_identity: None,
            action: action.to_string(),
            target_kind: "workflow".to_string(),
            target: Some(target.to_string()),
            outcome: OUTCOME_OK.to_string(),
            detail: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn audit_insert_then_list_returns_every_field() {
        let dal = dal_for(&shared_url()).await;
        dal.audit_events()
            .insert(NewAuditEvent {
                tenant_id: Some("acme".to_string()),
                actor_key_id: "key-1".to_string(),
                actor_key_name: "robin-laptop".to_string(),
                actor_identity: Some("oidc:https://idp.example:sub-1".to_string()),
                action: ACTION_PACKAGE_UPLOAD.to_string(),
                target_kind: "package".to_string(),
                target: Some("pkg-1".to_string()),
                outcome: OUTCOME_OK.to_string(),
                detail: serde_json::json!({"version": "1.2.0"}),
            })
            .await
            .unwrap();

        let rows = dal
            .audit_events()
            .list(AuditEventFilter::default())
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert!(r.id > 0);
        assert_eq!(r.tenant_id.as_deref(), Some("acme"));
        assert_eq!(r.actor_key_id, "key-1");
        assert_eq!(r.actor_key_name, "robin-laptop");
        assert_eq!(
            r.actor_identity.as_deref(),
            Some("oidc:https://idp.example:sub-1")
        );
        assert_eq!(r.action, "package.upload");
        assert_eq!(r.target_kind, "package");
        assert_eq!(r.target.as_deref(), Some("pkg-1"));
        assert_eq!(r.outcome, "ok");
        assert_eq!(r.detail["version"], "1.2.0");
        assert_eq!(r.actor_label(), "oidc:https://idp.example:sub-1");
        assert!((chrono::Utc::now() - r.created_at.0).num_seconds().abs() < 60);
    }

    #[tokio::test]
    async fn audit_list_filters_by_tenant_action_and_actor() {
        let dal = dal_for(&shared_url()).await;
        let a = dal.audit_events();
        a.insert(event(Some("acme"), "robin", "workflow.pause", "etl"))
            .await
            .unwrap();
        a.insert(event(Some("acme"), "sam", "workflow.run", "etl"))
            .await
            .unwrap();
        a.insert(event(Some("other"), "robin", "workflow.run", "etl"))
            .await
            .unwrap();
        a.insert(event(None, "root", "tenant.create", "acme"))
            .await
            .unwrap();
        let mut with_identity = event(Some("acme"), "minted", "workflow.resume", "etl");
        with_identity.actor_identity = Some("local:42".to_string());
        a.insert(with_identity).await.unwrap();

        let acme = a
            .list(AuditEventFilter {
                tenant_id: Some("acme".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(acme.len(), 3);
        assert!(acme.iter().all(|r| r.tenant_id.as_deref() == Some("acme")));

        let runs = a
            .list(AuditEventFilter {
                action: Some("workflow.run".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(runs.len(), 2);

        // The actor filter matches the key name, the key id or the identity.
        for (actor, expected) in [("robin", 2), ("id-sam", 1), ("local:42", 1), ("nobody", 0)] {
            let rows = a
                .list(AuditEventFilter {
                    actor: Some(actor.to_string()),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(rows.len(), expected, "actor {actor}");
        }

        let both = a
            .list(AuditEventFilter {
                tenant_id: Some("acme".to_string()),
                actor: Some("robin".to_string()),
                action: Some("workflow.pause".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(both.len(), 1);

        // All rows, with no filter: the platform row is there too.
        let all = a.list(AuditEventFilter::default()).await.unwrap();
        assert_eq!(all.len(), 5);
        assert!(all.iter().any(|r| r.tenant_id.is_none()));
    }

    #[tokio::test]
    async fn audit_list_is_newest_first_with_limit_and_cursor() {
        let dal = dal_for(&shared_url()).await;
        let a = dal.audit_events();
        for i in 0..5 {
            a.insert(event(
                Some("acme"),
                "robin",
                "workflow.run",
                &format!("wf-{i}"),
            ))
            .await
            .unwrap();
        }

        let page1 = a
            .list(AuditEventFilter {
                limit: Some(2),
                ..Default::default()
            })
            .await
            .unwrap();
        let targets: Vec<_> = page1.iter().map(|r| r.target.clone().unwrap()).collect();
        assert_eq!(targets, vec!["wf-4", "wf-3"]);

        let page2 = a
            .list(AuditEventFilter {
                limit: Some(2),
                before_id: Some(page1[1].id),
                ..Default::default()
            })
            .await
            .unwrap();
        let targets: Vec<_> = page2.iter().map(|r| r.target.clone().unwrap()).collect();
        assert_eq!(targets, vec!["wf-2", "wf-1"]);

        let page3 = a
            .list(AuditEventFilter {
                limit: Some(2),
                before_id: Some(page2[1].id),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page3.len(), 1);
        assert_eq!(page3[0].target.as_deref(), Some("wf-0"));

        // A limit of 0 or below reads one row; a huge limit is capped.
        let one = a
            .list(AuditEventFilter {
                limit: Some(0),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(one.len(), 1);
        let capped = a
            .list(AuditEventFilter {
                limit: Some(1_000_000),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(capped.len(), 5);
    }

    #[tokio::test]
    async fn audit_list_filters_by_time_range() {
        let dal = dal_for(&shared_url()).await;
        let a = dal.audit_events();
        a.insert(event(Some("acme"), "robin", "workflow.run", "etl"))
            .await
            .unwrap();
        let now = chrono::Utc::now();
        let hour = chrono::Duration::hours(1);

        let count = |since: Option<chrono::DateTime<chrono::Utc>>,
                     until: Option<chrono::DateTime<chrono::Utc>>| {
            let a = dal.audit_events();
            async move {
                a.list(AuditEventFilter {
                    since: since.map(UniversalTimestamp),
                    until: until.map(UniversalTimestamp),
                    ..Default::default()
                })
                .await
                .unwrap()
                .len()
            }
        };
        assert_eq!(count(Some(now - hour), None).await, 1);
        assert_eq!(count(Some(now + hour), None).await, 0);
        assert_eq!(count(None, Some(now + hour)).await, 1);
        assert_eq!(count(None, Some(now - hour)).await, 0);
        assert_eq!(count(Some(now - hour), Some(now + hour)).await, 1);
    }

    #[tokio::test]
    async fn audit_package_uploaders_gives_the_first_ok_upload_per_package() {
        let dal = dal_for(&shared_url()).await;
        let a = dal.audit_events();
        let upload = |tenant: &str, who: &str, pkg: &str| {
            let mut e = event(Some(tenant), who, ACTION_PACKAGE_UPLOAD, pkg);
            e.target_kind = "package".to_string();
            e
        };
        a.insert(upload("acme", "robin", "pkg-1")).await.unwrap();
        a.insert(upload("acme", "sam", "pkg-2")).await.unwrap();
        // The same content uploaded again by someone else: the first stays.
        a.insert(upload("acme", "jamie", "pkg-1")).await.unwrap();
        // Another tenant, the same id: not read.
        a.insert(upload("other", "jamie", "pkg-1")).await.unwrap();
        // A denied upload and another action on the same target: not read.
        let mut denied = upload("acme", "jamie", "pkg-2");
        denied.outcome = OUTCOME_DENIED.to_string();
        a.insert(denied).await.unwrap();
        a.insert(event(Some("acme"), "jamie", "package.delete", "pkg-2"))
            .await
            .unwrap();
        // An identity wins over the key name.
        let mut minted = upload("acme", "minted", "pkg-3");
        minted.actor_identity = Some("local:7".to_string());
        a.insert(minted).await.unwrap();

        let map = a
            .package_uploaders(
                "acme",
                vec![
                    "pkg-1".to_string(),
                    "pkg-2".to_string(),
                    "pkg-3".to_string(),
                    "pkg-9".to_string(),
                ],
            )
            .await
            .unwrap();
        assert_eq!(map.get("pkg-1").map(String::as_str), Some("robin"));
        assert_eq!(map.get("pkg-2").map(String::as_str), Some("sam"));
        assert_eq!(map.get("pkg-3").map(String::as_str), Some("local:7"));
        assert!(!map.contains_key("pkg-9"));
        assert!(
            a.package_uploaders("acme", vec![])
                .await
                .unwrap()
                .is_empty()
        );
    }
}
