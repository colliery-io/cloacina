/*
 *  Copyright 2026 Colliery Software
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

//! Reactor-triggered workflow subscriptions — DB-backed event log + fan-out.
//!
//! Implements the data layer for CLOACI-I-0100. Two tables:
//!
//! - `reactor_firings` — append-only log written by the reactor runtime
//!   on every fire. Each row carries the same boundary cache the
//!   in-process CG traversal consumed.
//! - `reactor_trigger_subscriptions` — one row per (reactor, workflow,
//!   tenant) tuple. The poller advances `last_seen_fired_at` as it
//!   dispatches workflows from new firings.
//!
//! Watermark advance is the at-least-once contract: if the dispatcher
//! crashes between dispatch and watermark advance, the next poll
//! re-dispatches. Workflow idempotency is the user's concern (same as
//! cron-triggered workflows).

use super::DAL;
use crate::database::schema::unified::{reactor_firings, reactor_trigger_subscriptions};
use crate::database::universal_types::{
    UniversalBinary, UniversalBool, UniversalTimestamp, UniversalUuid,
};
use crate::error::ValidationError;
use diesel::prelude::*;
use uuid::Uuid;

/// One reactor firing event. Carries the boundary cache payload the
/// in-process CG traversal consumed; subscribers receive the same data
/// as their workflow's input context.
#[derive(Debug, Clone, Queryable)]
pub struct ReactorFiring {
    pub id: UniversalUuid,
    pub reactor_name: String,
    pub tenant_id: String,
    pub payload: Option<UniversalBinary>,
    pub fired_at: UniversalTimestamp,
    pub created_at: UniversalTimestamp,
}

/// One subscription binding a workflow to a reactor's firings.
#[derive(Debug, Clone, Queryable)]
pub struct ReactorSubscription {
    pub id: UniversalUuid,
    pub reactor_name: String,
    pub workflow_name: String,
    pub tenant_id: String,
    pub enabled: UniversalBool,
    pub last_seen_fired_at: Option<UniversalTimestamp>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
    /// CLOACI-T-0602 — optional CEL filter expression. When `Some`, the
    /// scheduler evaluates it against the firing payload before dispatch;
    /// `Some(_) && false` means "skip + advance watermark". `None`
    /// preserves the original unfiltered behavior (fire on every firing).
    pub predicate_expression: Option<String>,
}

/// Data access layer for reactor subscriptions + firings.
#[derive(Clone)]
pub struct ReactorSubscriptionsDAL<'a> {
    dal: &'a DAL,
}

impl<'a> ReactorSubscriptionsDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    // ─────────────────────────────────────────────────────────────────
    // Firings
    // ─────────────────────────────────────────────────────────────────

    /// Insert a firing row. Called by the reactor runtime on every
    /// fire; best-effort from the caller's perspective (a DAL failure
    /// is logged but doesn't fail the in-process CG dispatch).
    pub async fn insert_firing(
        &self,
        reactor: &str,
        tenant: &str,
        payload: Option<Vec<u8>>,
        fired_at: UniversalTimestamp,
    ) -> Result<Uuid, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.insert_firing_postgres(reactor, tenant, payload, fired_at)
                .await,
            self.insert_firing_sqlite(reactor, tenant, payload, fired_at)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn insert_firing_postgres(
        &self,
        reactor: &str,
        tenant: &str,
        payload: Option<Vec<u8>>,
        fired_at: UniversalTimestamp,
    ) -> Result<Uuid, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();
        let reactor = reactor.to_string();
        let tenant = tenant.to_string();
        let id_for_move = id;
        conn.interact(move |conn| {
            diesel::insert_into(reactor_firings::table)
                .values((
                    reactor_firings::id.eq(id_for_move),
                    reactor_firings::reactor_name.eq(reactor),
                    reactor_firings::tenant_id.eq(tenant),
                    reactor_firings::payload.eq(payload.map(UniversalBinary::new)),
                    reactor_firings::fired_at.eq(fired_at),
                    reactor_firings::created_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(id.0)
    }

    #[cfg(feature = "sqlite")]
    async fn insert_firing_sqlite(
        &self,
        reactor: &str,
        tenant: &str,
        payload: Option<Vec<u8>>,
        fired_at: UniversalTimestamp,
    ) -> Result<Uuid, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();
        let reactor = reactor.to_string();
        let tenant = tenant.to_string();
        let id_for_move = id;
        conn.interact(move |conn| {
            diesel::insert_into(reactor_firings::table)
                .values((
                    reactor_firings::id.eq(id_for_move),
                    reactor_firings::reactor_name.eq(reactor),
                    reactor_firings::tenant_id.eq(tenant),
                    reactor_firings::payload.eq(payload.map(UniversalBinary::new)),
                    reactor_firings::fired_at.eq(fired_at),
                    reactor_firings::created_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(id.0)
    }

    /// Poll firings for a subscription. Returns rows strictly newer
    /// than `after`, in `fired_at` order, capped at `limit`. The
    /// caller advances the watermark as it dispatches each row.
    pub async fn poll_unconsumed(
        &self,
        tenant: &str,
        reactor: &str,
        after: Option<UniversalTimestamp>,
        limit: i64,
    ) -> Result<Vec<ReactorFiring>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.poll_unconsumed_postgres(tenant, reactor, after, limit)
                .await,
            self.poll_unconsumed_sqlite(tenant, reactor, after, limit)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn poll_unconsumed_postgres(
        &self,
        tenant: &str,
        reactor: &str,
        after: Option<UniversalTimestamp>,
        limit: i64,
    ) -> Result<Vec<ReactorFiring>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let tenant = tenant.to_string();
        let reactor = reactor.to_string();
        let rows: Vec<ReactorFiring> = conn
            .interact(move |conn| {
                let mut q = reactor_firings::table
                    .filter(reactor_firings::tenant_id.eq(tenant))
                    .filter(reactor_firings::reactor_name.eq(reactor))
                    .into_boxed();
                if let Some(after) = after {
                    q = q.filter(reactor_firings::fired_at.gt(after));
                }
                q.order(reactor_firings::fired_at.asc())
                    .limit(limit)
                    .load::<ReactorFiring>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows)
    }

    #[cfg(feature = "sqlite")]
    async fn poll_unconsumed_sqlite(
        &self,
        tenant: &str,
        reactor: &str,
        after: Option<UniversalTimestamp>,
        limit: i64,
    ) -> Result<Vec<ReactorFiring>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let tenant = tenant.to_string();
        let reactor = reactor.to_string();
        let rows: Vec<ReactorFiring> = conn
            .interact(move |conn| {
                let mut q = reactor_firings::table
                    .filter(reactor_firings::tenant_id.eq(tenant))
                    .filter(reactor_firings::reactor_name.eq(reactor))
                    .into_boxed();
                if let Some(after) = after {
                    q = q.filter(reactor_firings::fired_at.gt(after));
                }
                q.order(reactor_firings::fired_at.asc())
                    .limit(limit)
                    .load::<ReactorFiring>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows)
    }

    /// TTL prune. Deletes firings whose `fired_at` is older than the
    /// cutoff. Returns the row count deleted.
    pub async fn prune_firings_older_than(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<usize, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.prune_firings_older_than_postgres(cutoff).await,
            self.prune_firings_older_than_sqlite(cutoff).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn prune_firings_older_than_postgres(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let n = conn
            .interact(move |conn| {
                diesel::delete(reactor_firings::table.filter(reactor_firings::fired_at.lt(cutoff)))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n)
    }

    #[cfg(feature = "sqlite")]
    async fn prune_firings_older_than_sqlite(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let n = conn
            .interact(move |conn| {
                diesel::delete(reactor_firings::table.filter(reactor_firings::fired_at.lt(cutoff)))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n)
    }

    // ─────────────────────────────────────────────────────────────────
    // Subscriptions
    // ─────────────────────────────────────────────────────────────────

    /// Create a subscription. Idempotent: calling twice with the same
    /// `(reactor, workflow, tenant)` upserts; the second call's
    /// `predicate` (if any) replaces the first one's.
    ///
    /// `predicate` is an optional CEL expression (CLOACI-T-0602). When
    /// `Some(_)`, the expression is compiled at subscribe time and any
    /// syntax error is returned as a `ValidationError` before the row is
    /// written, so a bad expression never lands in the DB. The scheduler
    /// re-compiles + caches at dispatch time.
    pub async fn subscribe(
        &self,
        reactor: &str,
        workflow: &str,
        tenant: &str,
        predicate: Option<&str>,
    ) -> Result<Uuid, ValidationError> {
        if let Some(expr) = predicate {
            // Compile-time validation: reject malformed expressions before
            // they reach the DB. Cheap (single parse), centralizes the
            // error message at the API boundary.
            cel_interpreter::Program::compile(expr)
                .map_err(|e| ValidationError::InvalidPredicate(e.to_string()))?;
        }
        let predicate = predicate.map(str::to_string);
        crate::dispatch_backend!(
            self.dal.backend(),
            self.subscribe_postgres(reactor, workflow, tenant, predicate.clone())
                .await,
            self.subscribe_sqlite(reactor, workflow, tenant, predicate)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn subscribe_postgres(
        &self,
        reactor: &str,
        workflow: &str,
        tenant: &str,
        predicate: Option<String>,
    ) -> Result<Uuid, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();
        let reactor = reactor.to_string();
        let workflow = workflow.to_string();
        let tenant = tenant.to_string();
        let predicate_for_update = predicate.clone();
        let row: ReactorSubscription = conn
            .interact(move |conn| {
                diesel::insert_into(reactor_trigger_subscriptions::table)
                    .values((
                        reactor_trigger_subscriptions::id.eq(id),
                        reactor_trigger_subscriptions::reactor_name.eq(&reactor),
                        reactor_trigger_subscriptions::workflow_name.eq(&workflow),
                        reactor_trigger_subscriptions::tenant_id.eq(&tenant),
                        reactor_trigger_subscriptions::enabled.eq(UniversalBool::from(true)),
                        reactor_trigger_subscriptions::created_at.eq(now),
                        reactor_trigger_subscriptions::updated_at.eq(now),
                        reactor_trigger_subscriptions::predicate_expression.eq(&predicate),
                    ))
                    .on_conflict((
                        reactor_trigger_subscriptions::reactor_name,
                        reactor_trigger_subscriptions::workflow_name,
                        reactor_trigger_subscriptions::tenant_id,
                    ))
                    .do_update()
                    .set((
                        reactor_trigger_subscriptions::updated_at.eq(now),
                        reactor_trigger_subscriptions::predicate_expression
                            .eq(&predicate_for_update),
                    ))
                    .get_result::<ReactorSubscription>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(row.id.0)
    }

    #[cfg(feature = "sqlite")]
    async fn subscribe_sqlite(
        &self,
        reactor: &str,
        workflow: &str,
        tenant: &str,
        predicate: Option<String>,
    ) -> Result<Uuid, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let new_id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();
        let reactor = reactor.to_string();
        let workflow = workflow.to_string();
        let tenant = tenant.to_string();
        let row: ReactorSubscription = conn
            .interact(move |conn| {
                // SQLite: try insert; on conflict, update predicate +
                // updated_at and re-read.
                let insert_result = diesel::insert_into(reactor_trigger_subscriptions::table)
                    .values((
                        reactor_trigger_subscriptions::id.eq(new_id),
                        reactor_trigger_subscriptions::reactor_name.eq(&reactor),
                        reactor_trigger_subscriptions::workflow_name.eq(&workflow),
                        reactor_trigger_subscriptions::tenant_id.eq(&tenant),
                        reactor_trigger_subscriptions::enabled.eq(UniversalBool::from(true)),
                        reactor_trigger_subscriptions::created_at.eq(now),
                        reactor_trigger_subscriptions::updated_at.eq(now),
                        reactor_trigger_subscriptions::predicate_expression.eq(&predicate),
                    ))
                    .execute(conn);
                match insert_result {
                    Ok(_) => reactor_trigger_subscriptions::table
                        .filter(reactor_trigger_subscriptions::id.eq(new_id))
                        .first::<ReactorSubscription>(conn),
                    Err(diesel::result::Error::DatabaseError(
                        diesel::result::DatabaseErrorKind::UniqueViolation,
                        _,
                    )) => {
                        // Existing row — overwrite predicate so the
                        // upsert semantics match postgres.
                        diesel::update(
                            reactor_trigger_subscriptions::table
                                .filter(reactor_trigger_subscriptions::reactor_name.eq(&reactor))
                                .filter(reactor_trigger_subscriptions::workflow_name.eq(&workflow))
                                .filter(reactor_trigger_subscriptions::tenant_id.eq(&tenant)),
                        )
                        .set((
                            reactor_trigger_subscriptions::updated_at.eq(now),
                            reactor_trigger_subscriptions::predicate_expression.eq(&predicate),
                        ))
                        .execute(conn)?;
                        reactor_trigger_subscriptions::table
                            .filter(reactor_trigger_subscriptions::reactor_name.eq(&reactor))
                            .filter(reactor_trigger_subscriptions::workflow_name.eq(&workflow))
                            .filter(reactor_trigger_subscriptions::tenant_id.eq(&tenant))
                            .first::<ReactorSubscription>(conn)
                    }
                    Err(e) => Err(e),
                }
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(row.id.0)
    }

    /// Advance the watermark for a subscription. Caller is the
    /// dispatcher loop; the watermark advances after each row is
    /// dispatched (at-least-once on crash).
    pub async fn advance_watermark(
        &self,
        subscription_id: Uuid,
        new_last_seen: UniversalTimestamp,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.advance_watermark_postgres(subscription_id, new_last_seen)
                .await,
            self.advance_watermark_sqlite(subscription_id, new_last_seen)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn advance_watermark_postgres(
        &self,
        subscription_id: Uuid,
        new_last_seen: UniversalTimestamp,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let sid = UniversalUuid(subscription_id);
        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::id.eq(sid)),
            )
            .set((
                reactor_trigger_subscriptions::last_seen_fired_at.eq(Some(new_last_seen)),
                reactor_trigger_subscriptions::updated_at.eq(now),
            ))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn advance_watermark_sqlite(
        &self,
        subscription_id: Uuid,
        new_last_seen: UniversalTimestamp,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let sid = UniversalUuid(subscription_id);
        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::id.eq(sid)),
            )
            .set((
                reactor_trigger_subscriptions::last_seen_fired_at.eq(Some(new_last_seen)),
                reactor_trigger_subscriptions::updated_at.eq(now),
            ))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }

    /// Remove a subscription. Returns true if a row was deleted.
    pub async fn unsubscribe(
        &self,
        reactor: &str,
        workflow: &str,
        tenant: &str,
    ) -> Result<bool, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.unsubscribe_postgres(reactor, workflow, tenant).await,
            self.unsubscribe_sqlite(reactor, workflow, tenant).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn unsubscribe_postgres(
        &self,
        reactor: &str,
        workflow: &str,
        tenant: &str,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let reactor = reactor.to_string();
        let workflow = workflow.to_string();
        let tenant = tenant.to_string();
        let n = conn
            .interact(move |conn| {
                diesel::delete(
                    reactor_trigger_subscriptions::table
                        .filter(reactor_trigger_subscriptions::reactor_name.eq(reactor))
                        .filter(reactor_trigger_subscriptions::workflow_name.eq(workflow))
                        .filter(reactor_trigger_subscriptions::tenant_id.eq(tenant)),
                )
                .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n > 0)
    }

    #[cfg(feature = "sqlite")]
    async fn unsubscribe_sqlite(
        &self,
        reactor: &str,
        workflow: &str,
        tenant: &str,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let reactor = reactor.to_string();
        let workflow = workflow.to_string();
        let tenant = tenant.to_string();
        let n = conn
            .interact(move |conn| {
                diesel::delete(
                    reactor_trigger_subscriptions::table
                        .filter(reactor_trigger_subscriptions::reactor_name.eq(reactor))
                        .filter(reactor_trigger_subscriptions::workflow_name.eq(workflow))
                        .filter(reactor_trigger_subscriptions::tenant_id.eq(tenant)),
                )
                .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n > 0)
    }

    /// List all enabled subscriptions across every tenant. Used by the
    /// unified scheduler's reactor poll tick (CLOACI-I-0100 / T-0599).
    pub async fn list_all_enabled(&self) -> Result<Vec<ReactorSubscription>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_all_enabled_postgres().await,
            self.list_all_enabled_sqlite().await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_all_enabled_postgres(&self) -> Result<Vec<ReactorSubscription>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let rows = conn
            .interact(move |conn| {
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::enabled.eq(UniversalBool::from(true)))
                    .load::<ReactorSubscription>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows)
    }

    #[cfg(feature = "sqlite")]
    async fn list_all_enabled_sqlite(&self) -> Result<Vec<ReactorSubscription>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let rows = conn
            .interact(move |conn| {
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::enabled.eq(UniversalBool::from(true)))
                    .load::<ReactorSubscription>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows)
    }

    /// List enabled subscriptions for a tenant.
    pub async fn list_subscriptions(
        &self,
        tenant: &str,
    ) -> Result<Vec<ReactorSubscription>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_subscriptions_postgres(tenant).await,
            self.list_subscriptions_sqlite(tenant).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_subscriptions_postgres(
        &self,
        tenant: &str,
    ) -> Result<Vec<ReactorSubscription>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let tenant = tenant.to_string();
        let rows = conn
            .interact(move |conn| {
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::tenant_id.eq(tenant))
                    .filter(reactor_trigger_subscriptions::enabled.eq(UniversalBool::from(true)))
                    .load::<ReactorSubscription>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows)
    }

    #[cfg(feature = "sqlite")]
    async fn list_subscriptions_sqlite(
        &self,
        tenant: &str,
    ) -> Result<Vec<ReactorSubscription>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let tenant = tenant.to_string();
        let rows = conn
            .interact(move |conn| {
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::tenant_id.eq(tenant))
                    .filter(reactor_trigger_subscriptions::enabled.eq(UniversalBool::from(true)))
                    .load::<ReactorSubscription>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows)
    }
}
