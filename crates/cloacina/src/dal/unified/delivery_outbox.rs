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

//! Unified Delivery Outbox DAL (substrate — CLOACI-S-0012 / A-0006, task T-0625).
//!
//! The delivery outbox is the **durable system of record** for the interservice
//! communication substrate: a row is enqueued (ideally in the same transaction
//! as the state change that produced it), pushed to its addressed recipient over
//! WebSocket, and acked. State machine: `pending → delivered → acked`, with
//! `delivered → pending` as the redelivery path (sweeper reclaim / reconnect
//! resync).
//!
//! Distinct from [`super::task_outbox`], the transient competing-consumer
//! scheduler→executor claim queue. Rows here are addressed and retained until
//! acked.
//!
//! State transitions are implemented as **atomic compare-and-set** updates
//! (filter on the expected current state; zero rows affected ⇒ the transition
//! was not permitted from the row's actual state ⇒ [`ValidationError::InvalidStateTransition`]).
//! The substrate is Postgres-only at runtime; the SQLite arms exist for unified
//! schema parity and test coverage.

use super::models::{NewUnifiedDeliveryOutbox, UnifiedDeliveryOutbox};
use super::DAL;
use crate::database::schema::unified::delivery_outbox;
use crate::database::universal_types::{UniversalBinary, UniversalTimestamp};
use crate::error::ValidationError;
use crate::models::delivery_outbox::{DeliveryOutbox, NewDeliveryOutbox};
use diesel::prelude::*;

const STATE_PENDING: &str = "pending";
const STATE_DELIVERED: &str = "delivered";
const STATE_ACKED: &str = "acked";

/// Data access layer for delivery-outbox operations with runtime backend selection.
#[derive(Clone)]
pub struct DeliveryOutboxDAL<'a> {
    dal: &'a DAL,
}

fn to_domain(r: UnifiedDeliveryOutbox) -> DeliveryOutbox {
    DeliveryOutbox {
        id: r.id,
        recipient: r.recipient,
        kind: r.kind,
        tenant_id: r.tenant_id,
        payload: r.payload.into_inner(),
        delivery_state: r.delivery_state,
        delivery_attempts: r.delivery_attempts,
        created_at: r.created_at,
        delivered_at: r.delivered_at,
        acked_at: r.acked_at,
    }
}

impl<'a> DeliveryOutboxDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    // ── enqueue ────────────────────────────────────────────────────

    /// Enqueues a new outbox row in the `pending` state.
    ///
    /// For the atomicity guarantee (REQ-1.1.1), producing code should perform
    /// the equivalent insert inside its own transaction; this convenience
    /// method opens its own connection and is used standalone and in tests.
    pub async fn enqueue(&self, new: NewDeliveryOutbox) -> Result<DeliveryOutbox, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.enqueue_postgres(new).await,
            self.enqueue_sqlite(new).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn enqueue_postgres(
        &self,
        new: NewDeliveryOutbox,
    ) -> Result<DeliveryOutbox, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let row = build_insert(new);
        let result: UnifiedDeliveryOutbox = conn
            .interact(move |conn| {
                diesel::insert_into(delivery_outbox::table)
                    .values(&row)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(to_domain(result))
    }

    #[cfg(feature = "sqlite")]
    async fn enqueue_sqlite(
        &self,
        new: NewDeliveryOutbox,
    ) -> Result<DeliveryOutbox, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let row = build_insert(new);
        let result: UnifiedDeliveryOutbox = conn
            .interact(move |conn| {
                diesel::insert_into(delivery_outbox::table)
                    .values(&row)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(to_domain(result))
    }

    // ── state transitions (atomic compare-and-set) ─────────────────

    /// `pending → delivered`: record that the row was pushed to its recipient.
    /// Increments `delivery_attempts` and stamps `delivered_at`.
    pub async fn mark_delivered(&self, id: i64) -> Result<(), ValidationError> {
        let affected = crate::dispatch_backend!(
            self.dal.backend(),
            self.mark_delivered_postgres(id).await,
            self.mark_delivered_sqlite(id).await
        )?;
        transition_result(id, STATE_PENDING, STATE_DELIVERED, affected)
    }

    #[cfg(feature = "postgres")]
    async fn mark_delivered_postgres(&self, id: i64) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(
                delivery_outbox::table
                    .filter(delivery_outbox::id.eq(id))
                    .filter(delivery_outbox::delivery_state.eq(STATE_PENDING)),
            )
            .set((
                delivery_outbox::delivery_state.eq(STATE_DELIVERED),
                delivery_outbox::delivered_at.eq(Some(now)),
                delivery_outbox::delivery_attempts.eq(delivery_outbox::delivery_attempts + 1),
            ))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?
        .map_err(ValidationError::from)
    }

    #[cfg(feature = "sqlite")]
    async fn mark_delivered_sqlite(&self, id: i64) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(
                delivery_outbox::table
                    .filter(delivery_outbox::id.eq(id))
                    .filter(delivery_outbox::delivery_state.eq(STATE_PENDING)),
            )
            .set((
                delivery_outbox::delivery_state.eq(STATE_DELIVERED),
                delivery_outbox::delivered_at.eq(Some(now)),
                delivery_outbox::delivery_attempts.eq(delivery_outbox::delivery_attempts + 1),
            ))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?
        .map_err(ValidationError::from)
    }

    /// `pending|delivered → acked`: recipient confirmed receipt.
    ///
    /// Accepts BOTH `pending` and `delivered` as valid source states to handle
    /// the relay-vs-recipient race: on localhost the recipient can process a
    /// push frame and ack it before the relay's own `mark_delivered` CAS
    /// lands (the sink's `try_send` returns before the relay's UPDATE runs).
    /// Restricting to `delivered → acked` only would cause those races to
    /// silently drop the ack and leave the row stuck in `delivered` until the
    /// sweeper reclaimed it. Either source state is fine semantically: the
    /// recipient saw the row, whichever bookkeeping order the server reached.
    pub async fn mark_acked(&self, id: i64) -> Result<(), ValidationError> {
        let affected = crate::dispatch_backend!(
            self.dal.backend(),
            self.mark_acked_postgres(id).await,
            self.mark_acked_sqlite(id).await
        )?;
        transition_result(id, "pending|delivered", STATE_ACKED, affected)
    }

    #[cfg(feature = "postgres")]
    async fn mark_acked_postgres(&self, id: i64) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(
                delivery_outbox::table
                    .filter(delivery_outbox::id.eq(id))
                    .filter(delivery_outbox::delivery_state.ne(STATE_ACKED)),
            )
            .set((
                delivery_outbox::delivery_state.eq(STATE_ACKED),
                delivery_outbox::acked_at.eq(Some(now)),
            ))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?
        .map_err(ValidationError::from)
    }

    #[cfg(feature = "sqlite")]
    async fn mark_acked_sqlite(&self, id: i64) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(
                delivery_outbox::table
                    .filter(delivery_outbox::id.eq(id))
                    .filter(delivery_outbox::delivery_state.ne(STATE_ACKED)),
            )
            .set((
                delivery_outbox::delivery_state.eq(STATE_ACKED),
                delivery_outbox::acked_at.eq(Some(now)),
            ))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?
        .map_err(ValidationError::from)
    }

    /// `delivered → pending`: redelivery path (sweeper reclaim / reconnect resync).
    pub async fn reset_to_pending(&self, id: i64) -> Result<(), ValidationError> {
        let affected = crate::dispatch_backend!(
            self.dal.backend(),
            self.reset_to_pending_postgres(id).await,
            self.reset_to_pending_sqlite(id).await
        )?;
        transition_result(id, STATE_DELIVERED, STATE_PENDING, affected)
    }

    #[cfg(feature = "postgres")]
    async fn reset_to_pending_postgres(&self, id: i64) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            diesel::update(
                delivery_outbox::table
                    .filter(delivery_outbox::id.eq(id))
                    .filter(delivery_outbox::delivery_state.eq(STATE_DELIVERED)),
            )
            .set(delivery_outbox::delivery_state.eq(STATE_PENDING))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?
        .map_err(ValidationError::from)
    }

    #[cfg(feature = "sqlite")]
    async fn reset_to_pending_sqlite(&self, id: i64) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            diesel::update(
                delivery_outbox::table
                    .filter(delivery_outbox::id.eq(id))
                    .filter(delivery_outbox::delivery_state.eq(STATE_DELIVERED)),
            )
            .set(delivery_outbox::delivery_state.eq(STATE_PENDING))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?
        .map_err(ValidationError::from)
    }

    /// Reset every `delivered`-state row addressed to `(recipient, tenant_id)`
    /// back to `pending`. Called on WS reconnect (T-0627) so the relay
    /// re-pushes the stuck-delivered set through its normal sink path,
    /// avoiding a separate handler-side resync that would race the relay.
    /// Returns the number of rows reset.
    ///
    /// `tenant_id = None` matches rows where the column is NULL.
    pub async fn reset_delivered_to_pending_for_recipient(
        &self,
        recipient: &str,
        tenant_id: Option<&str>,
    ) -> Result<usize, ValidationError> {
        let recipient = recipient.to_string();
        let tenant_id = tenant_id.map(|s| s.to_string());
        crate::dispatch_backend!(
            self.dal.backend(),
            self.reset_delivered_for_recipient_postgres(recipient, tenant_id)
                .await,
            self.reset_delivered_for_recipient_sqlite(recipient, tenant_id)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn reset_delivered_for_recipient_postgres(
        &self,
        recipient: String,
        tenant_id: Option<String>,
    ) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            // diesel: branch on the tenant filter because Option doesn't auto-map to IS NULL.
            let base = delivery_outbox::table
                .filter(delivery_outbox::recipient.eq(recipient))
                .filter(delivery_outbox::delivery_state.eq(STATE_DELIVERED));
            match tenant_id {
                Some(t) => diesel::update(base.filter(delivery_outbox::tenant_id.eq(t)))
                    .set(delivery_outbox::delivery_state.eq(STATE_PENDING))
                    .execute(conn),
                None => diesel::update(base.filter(delivery_outbox::tenant_id.is_null()))
                    .set(delivery_outbox::delivery_state.eq(STATE_PENDING))
                    .execute(conn),
            }
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?
        .map_err(ValidationError::from)
    }

    #[cfg(feature = "sqlite")]
    async fn reset_delivered_for_recipient_sqlite(
        &self,
        recipient: String,
        tenant_id: Option<String>,
    ) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            let base = delivery_outbox::table
                .filter(delivery_outbox::recipient.eq(recipient))
                .filter(delivery_outbox::delivery_state.eq(STATE_DELIVERED));
            match tenant_id {
                Some(t) => diesel::update(base.filter(delivery_outbox::tenant_id.eq(t)))
                    .set(delivery_outbox::delivery_state.eq(STATE_PENDING))
                    .execute(conn),
                None => diesel::update(base.filter(delivery_outbox::tenant_id.is_null()))
                    .set(delivery_outbox::delivery_state.eq(STATE_PENDING))
                    .execute(conn),
            }
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?
        .map_err(ValidationError::from)
    }

    /// Reassign every non-`acked` row from `from_recipient` to `to_recipient`,
    /// resetting them to `pending` (and clearing `delivered_at`) so the relay
    /// re-pushes them to the new recipient (CLOACI-T-0634 dead-agent reclaim).
    ///
    /// Called by the fleet heartbeat sweeper when an agent is evicted: its
    /// in-flight work packets (addressed to `agent:<deadId>`) are re-targeted
    /// to a live agent rather than left pinned to a recipient whose connection
    /// is gone (which would otherwise spin on `NoRoute` forever). The work
    /// keeps its `task_execution_id`, so the `FleetExecutor`'s awaiting
    /// rendezvous receives the new agent's result without any change.
    ///
    /// Returns the number of rows reassigned. OQ-2: if the "dead" agent is
    /// actually partitioned-but-alive, the re-addressed work may double-execute
    /// — this matches the at-least-once posture the thread executor has on
    /// heartbeat-loss (deliberate; see initiative OQ-2).
    pub async fn reassign_open_rows(
        &self,
        from_recipient: &str,
        to_recipient: &str,
    ) -> Result<usize, ValidationError> {
        let from_recipient = from_recipient.to_string();
        let to_recipient = to_recipient.to_string();
        crate::dispatch_backend!(
            self.dal.backend(),
            self.reassign_open_rows_postgres(from_recipient, to_recipient)
                .await,
            self.reassign_open_rows_sqlite(from_recipient, to_recipient)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn reassign_open_rows_postgres(
        &self,
        from_recipient: String,
        to_recipient: String,
    ) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            diesel::update(
                delivery_outbox::table
                    .filter(delivery_outbox::recipient.eq(from_recipient))
                    .filter(delivery_outbox::delivery_state.ne(STATE_ACKED)),
            )
            .set((
                delivery_outbox::recipient.eq(to_recipient),
                delivery_outbox::delivery_state.eq(STATE_PENDING),
                delivery_outbox::delivered_at
                    .eq(None::<crate::database::universal_types::UniversalTimestamp>),
            ))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?
        .map_err(ValidationError::from)
    }

    #[cfg(feature = "sqlite")]
    async fn reassign_open_rows_sqlite(
        &self,
        from_recipient: String,
        to_recipient: String,
    ) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            diesel::update(
                delivery_outbox::table
                    .filter(delivery_outbox::recipient.eq(from_recipient))
                    .filter(delivery_outbox::delivery_state.ne(STATE_ACKED)),
            )
            .set((
                delivery_outbox::recipient.eq(to_recipient),
                delivery_outbox::delivery_state.eq(STATE_PENDING),
                delivery_outbox::delivered_at
                    .eq(None::<crate::database::universal_types::UniversalTimestamp>),
            ))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?
        .map_err(ValidationError::from)
    }

    // ── queries ────────────────────────────────────────────────────

    /// Open (un-acked) rows addressed to `recipient`, ordered by id for
    /// deterministic replay (relay drain / reconnect resync).
    pub async fn list_open_for_recipient(
        &self,
        recipient: &str,
        limit: i64,
    ) -> Result<Vec<DeliveryOutbox>, ValidationError> {
        let recipient = recipient.to_string();
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_open_for_recipient_postgres(recipient, limit)
                .await,
            self.list_open_for_recipient_sqlite(recipient, limit).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_open_for_recipient_postgres(
        &self,
        recipient: String,
        limit: i64,
    ) -> Result<Vec<DeliveryOutbox>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let rows: Vec<UnifiedDeliveryOutbox> = conn
            .interact(move |conn| {
                delivery_outbox::table
                    .filter(delivery_outbox::recipient.eq(recipient))
                    .filter(delivery_outbox::delivery_state.ne(STATE_ACKED))
                    .order(delivery_outbox::id.asc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows.into_iter().map(to_domain).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn list_open_for_recipient_sqlite(
        &self,
        recipient: String,
        limit: i64,
    ) -> Result<Vec<DeliveryOutbox>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let rows: Vec<UnifiedDeliveryOutbox> = conn
            .interact(move |conn| {
                delivery_outbox::table
                    .filter(delivery_outbox::recipient.eq(recipient))
                    .filter(delivery_outbox::delivery_state.ne(STATE_ACKED))
                    .order(delivery_outbox::id.asc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows.into_iter().map(to_domain).collect())
    }

    /// Rows in the `pending` state across all recipients, oldest first — the
    /// relay's drain query (T-0626). Delivered-but-unacked rows are excluded
    /// (they await ack or sweeper reclaim, not (re)delivery on a fresh wake).
    pub async fn list_pending(&self, limit: i64) -> Result<Vec<DeliveryOutbox>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_pending_postgres(limit).await,
            self.list_pending_sqlite(limit).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_pending_postgres(
        &self,
        limit: i64,
    ) -> Result<Vec<DeliveryOutbox>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let rows: Vec<UnifiedDeliveryOutbox> = conn
            .interact(move |conn| {
                delivery_outbox::table
                    .filter(delivery_outbox::delivery_state.eq(STATE_PENDING))
                    .order(delivery_outbox::id.asc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows.into_iter().map(to_domain).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn list_pending_sqlite(
        &self,
        limit: i64,
    ) -> Result<Vec<DeliveryOutbox>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let rows: Vec<UnifiedDeliveryOutbox> = conn
            .interact(move |conn| {
                delivery_outbox::table
                    .filter(delivery_outbox::delivery_state.eq(STATE_PENDING))
                    .order(delivery_outbox::id.asc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows.into_iter().map(to_domain).collect())
    }

    /// Open rows enqueued before `cutoff`, oldest first — the safety-net
    /// sweeper's stuck-row scan (T-0628).
    pub async fn list_stuck(
        &self,
        cutoff: UniversalTimestamp,
        limit: i64,
    ) -> Result<Vec<DeliveryOutbox>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_stuck_postgres(cutoff, limit).await,
            self.list_stuck_sqlite(cutoff, limit).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_stuck_postgres(
        &self,
        cutoff: UniversalTimestamp,
        limit: i64,
    ) -> Result<Vec<DeliveryOutbox>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let rows: Vec<UnifiedDeliveryOutbox> = conn
            .interact(move |conn| {
                delivery_outbox::table
                    .filter(delivery_outbox::delivery_state.ne(STATE_ACKED))
                    .filter(delivery_outbox::created_at.lt(cutoff))
                    .order(delivery_outbox::created_at.asc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows.into_iter().map(to_domain).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn list_stuck_sqlite(
        &self,
        cutoff: UniversalTimestamp,
        limit: i64,
    ) -> Result<Vec<DeliveryOutbox>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let rows: Vec<UnifiedDeliveryOutbox> = conn
            .interact(move |conn| {
                delivery_outbox::table
                    .filter(delivery_outbox::delivery_state.ne(STATE_ACKED))
                    .filter(delivery_outbox::created_at.lt(cutoff))
                    .order(delivery_outbox::created_at.asc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows.into_iter().map(to_domain).collect())
    }

    /// Count of open (un-acked) rows — outbox-depth signal for monitoring (T-0628).
    pub async fn count_open(&self) -> Result<i64, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.count_open_postgres().await,
            self.count_open_sqlite().await
        )
    }

    #[cfg(feature = "postgres")]
    async fn count_open_postgres(&self) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let count: i64 = conn
            .interact(move |conn| {
                delivery_outbox::table
                    .filter(delivery_outbox::delivery_state.ne(STATE_ACKED))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(count)
    }

    #[cfg(feature = "sqlite")]
    async fn count_open_sqlite(&self) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let count: i64 = conn
            .interact(move |conn| {
                delivery_outbox::table
                    .filter(delivery_outbox::delivery_state.ne(STATE_ACKED))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(count)
    }
}

/// Builds the insertable row for a new `pending` outbox entry.
fn build_insert(new: NewDeliveryOutbox) -> NewUnifiedDeliveryOutbox {
    NewUnifiedDeliveryOutbox {
        recipient: new.recipient,
        kind: new.kind,
        tenant_id: new.tenant_id,
        payload: UniversalBinary::from(new.payload),
        delivery_state: STATE_PENDING.to_string(),
        delivery_attempts: 0,
        created_at: UniversalTimestamp::now(),
    }
}

/// Maps a compare-and-set affected-row count to a transition result: exactly
/// one row affected means the transition applied; zero means the row was not
/// in the expected `from` state (or does not exist), which we reject.
fn transition_result(
    id: i64,
    from: &str,
    to: &str,
    affected: usize,
) -> Result<(), ValidationError> {
    if affected == 1 {
        Ok(())
    } else {
        Err(ValidationError::InvalidStateTransition {
            id,
            from: from.to_string(),
            to: to.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    #[cfg(feature = "sqlite")]
    async fn unique_dal() -> DAL {
        let url = format!(
            "file:delivery_outbox_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    #[cfg(feature = "sqlite")]
    fn new_row(recipient: &str) -> NewDeliveryOutbox {
        NewDeliveryOutbox {
            recipient: recipient.to_string(),
            kind: "work".to_string(),
            tenant_id: None,
            payload: b"hello".to_vec(),
        }
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_enqueue_starts_pending() {
        let dal = unique_dal().await;
        let row = dal
            .delivery_outbox()
            .enqueue(new_row("agent:1"))
            .await
            .unwrap();
        assert_eq!(row.delivery_state, STATE_PENDING);
        assert_eq!(row.delivery_attempts, 0);
        assert_eq!(row.payload, b"hello".to_vec());
        assert!(row.delivered_at.is_none());
        assert!(row.acked_at.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_reassign_open_rows_retargets_and_resets() {
        // CLOACI-T-0634 dead-agent reclaim: a delivered-but-unacked row for a
        // dead agent is re-targeted to a live agent and reset to pending.
        let dal = unique_dal().await;
        let row = dal
            .delivery_outbox()
            .enqueue(new_row("agent:dead"))
            .await
            .unwrap();
        dal.delivery_outbox().mark_delivered(row.id).await.unwrap();

        // An acked row for the same dead agent must NOT be reassigned.
        let acked = dal
            .delivery_outbox()
            .enqueue(new_row("agent:dead"))
            .await
            .unwrap();
        dal.delivery_outbox()
            .mark_delivered(acked.id)
            .await
            .unwrap();
        dal.delivery_outbox().mark_acked(acked.id).await.unwrap();

        let moved = dal
            .delivery_outbox()
            .reassign_open_rows("agent:dead", "agent:live")
            .await
            .unwrap();
        assert_eq!(moved, 1, "only the non-acked row is reassigned");

        // The reassigned row is now pending + addressed to the live agent.
        let live_open = dal
            .delivery_outbox()
            .list_open_for_recipient("agent:live", 10)
            .await
            .unwrap();
        assert_eq!(live_open.len(), 1);
        assert_eq!(live_open[0].id, row.id);
        assert_eq!(live_open[0].delivery_state, STATE_PENDING);
        assert!(live_open[0].delivered_at.is_none());

        // The dead recipient has no remaining open rows.
        let dead_open = dal
            .delivery_outbox()
            .list_open_for_recipient("agent:dead", 10)
            .await
            .unwrap();
        assert!(dead_open.is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_full_lifecycle_pending_delivered_acked() {
        let dal = unique_dal().await;
        let row = dal
            .delivery_outbox()
            .enqueue(new_row("agent:1"))
            .await
            .unwrap();

        dal.delivery_outbox().mark_delivered(row.id).await.unwrap();
        dal.delivery_outbox().mark_acked(row.id).await.unwrap();

        // Acked rows leave the open set.
        let open = dal
            .delivery_outbox()
            .list_open_for_recipient("agent:1", 10)
            .await
            .unwrap();
        assert!(open.is_empty());
        assert_eq!(dal.delivery_outbox().count_open().await.unwrap(), 0);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_mark_delivered_increments_attempts_and_stamps() {
        let dal = unique_dal().await;
        let row = dal
            .delivery_outbox()
            .enqueue(new_row("agent:1"))
            .await
            .unwrap();
        dal.delivery_outbox().mark_delivered(row.id).await.unwrap();

        let open = dal
            .delivery_outbox()
            .list_open_for_recipient("agent:1", 10)
            .await
            .unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].delivery_state, STATE_DELIVERED);
        assert_eq!(open[0].delivery_attempts, 1);
        assert!(open[0].delivered_at.is_some());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_redelivery_resets_then_redelivers_incrementing_attempts() {
        let dal = unique_dal().await;
        let row = dal
            .delivery_outbox()
            .enqueue(new_row("agent:1"))
            .await
            .unwrap();
        dal.delivery_outbox().mark_delivered(row.id).await.unwrap();
        dal.delivery_outbox()
            .reset_to_pending(row.id)
            .await
            .unwrap();
        dal.delivery_outbox().mark_delivered(row.id).await.unwrap();

        let open = dal
            .delivery_outbox()
            .list_open_for_recipient("agent:1", 10)
            .await
            .unwrap();
        assert_eq!(open[0].delivery_attempts, 2);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_ack_on_pending_succeeds_for_relay_recipient_race() {
        // Race: on localhost the recipient can ack a pushed row before the
        // relay's own `mark_delivered` CAS lands. `mark_acked` therefore
        // accepts BOTH `pending` and `delivered` as valid source states —
        // either way the recipient saw the row, which is the only thing the
        // ack is asserting.
        let dal = unique_dal().await;
        let row = dal
            .delivery_outbox()
            .enqueue(new_row("agent:1"))
            .await
            .unwrap();
        dal.delivery_outbox().mark_acked(row.id).await.unwrap();

        // Row is `acked` and stops appearing in the open set.
        assert!(dal
            .delivery_outbox()
            .list_open_for_recipient("agent:1", 10)
            .await
            .unwrap()
            .is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_invalid_transition_rejected() {
        let dal = unique_dal().await;
        let row = dal
            .delivery_outbox()
            .enqueue(new_row("agent:1"))
            .await
            .unwrap();

        // Resetting a pending row is not permitted (reset is `delivered → pending` only).
        let err = dal
            .delivery_outbox()
            .reset_to_pending(row.id)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ValidationError::InvalidStateTransition { .. }
        ));

        // Once acked, the row is terminal — further state changes are rejected.
        dal.delivery_outbox().mark_acked(row.id).await.unwrap();
        let err = dal.delivery_outbox().mark_acked(row.id).await.unwrap_err();
        assert!(matches!(
            err,
            ValidationError::InvalidStateTransition { .. }
        ));
        let err = dal
            .delivery_outbox()
            .mark_delivered(row.id)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ValidationError::InvalidStateTransition { .. }
        ));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_open_for_recipient_isolates_and_orders() {
        let dal = unique_dal().await;
        let r1 = dal
            .delivery_outbox()
            .enqueue(new_row("agent:1"))
            .await
            .unwrap();
        let _r2 = dal
            .delivery_outbox()
            .enqueue(new_row("agent:2"))
            .await
            .unwrap();
        let r3 = dal
            .delivery_outbox()
            .enqueue(new_row("agent:1"))
            .await
            .unwrap();

        let open = dal
            .delivery_outbox()
            .list_open_for_recipient("agent:1", 10)
            .await
            .unwrap();
        assert_eq!(open.len(), 2);
        // ordered by id ascending
        assert_eq!(open[0].id, r1.id);
        assert_eq!(open[1].id, r3.id);
    }

    #[tokio::test]
    async fn test_reset_delivered_to_pending_isolates_by_recipient_and_tenant() {
        let dal = unique_dal().await;
        let make = |recipient: &str, tenant: Option<&str>| NewDeliveryOutbox {
            recipient: recipient.to_string(),
            kind: "work".to_string(),
            tenant_id: tenant.map(|s| s.to_string()),
            payload: b"x".to_vec(),
        };

        let target = dal
            .delivery_outbox()
            .enqueue(make("agent:1", Some("t1")))
            .await
            .unwrap();
        let other_recipient = dal
            .delivery_outbox()
            .enqueue(make("agent:2", Some("t1")))
            .await
            .unwrap();
        let other_tenant = dal
            .delivery_outbox()
            .enqueue(make("agent:1", Some("t2")))
            .await
            .unwrap();
        let global = dal
            .delivery_outbox()
            .enqueue(make("agent:1", None))
            .await
            .unwrap();

        // Mark all four delivered (simulating a prior push that was never acked).
        for id in [target.id, other_recipient.id, other_tenant.id, global.id] {
            dal.delivery_outbox().mark_delivered(id).await.unwrap();
        }

        // Reset only ("agent:1", "t1") — the auth context for a hypothetical reconnect.
        let n = dal
            .delivery_outbox()
            .reset_delivered_to_pending_for_recipient("agent:1", Some("t1"))
            .await
            .unwrap();
        assert_eq!(
            n, 1,
            "only the matching (recipient, tenant) row should reset"
        );

        // Inspect all "agent:1" rows: target -> pending; other_tenant and global -> still delivered.
        let agent1_rows = dal
            .delivery_outbox()
            .list_open_for_recipient("agent:1", 10)
            .await
            .unwrap();
        assert_eq!(agent1_rows.len(), 3);
        let by_id = |id: i64| agent1_rows.iter().find(|r| r.id == id).unwrap();
        assert_eq!(by_id(target.id).delivery_state, STATE_PENDING);
        assert_eq!(by_id(other_tenant.id).delivery_state, STATE_DELIVERED);
        assert_eq!(by_id(global.id).delivery_state, STATE_DELIVERED);

        // Other recipient is untouched.
        let agent2_rows = dal
            .delivery_outbox()
            .list_open_for_recipient("agent:2", 10)
            .await
            .unwrap();
        assert_eq!(agent2_rows.len(), 1);
        assert_eq!(agent2_rows[0].delivery_state, STATE_DELIVERED);
    }

    #[tokio::test]
    async fn test_reset_delivered_to_pending_matches_null_tenant() {
        let dal = unique_dal().await;
        let row = dal
            .delivery_outbox()
            .enqueue(NewDeliveryOutbox {
                recipient: "global:1".to_string(),
                kind: "work".to_string(),
                tenant_id: None,
                payload: b"x".to_vec(),
            })
            .await
            .unwrap();
        dal.delivery_outbox().mark_delivered(row.id).await.unwrap();

        let n = dal
            .delivery_outbox()
            .reset_delivered_to_pending_for_recipient("global:1", None)
            .await
            .unwrap();
        assert_eq!(n, 1, "reset must match NULL tenant via IS NULL, not = NULL");

        // And Some("anything") must NOT match a NULL-tenant row.
        let row2 = dal
            .delivery_outbox()
            .enqueue(NewDeliveryOutbox {
                recipient: "global:2".to_string(),
                kind: "work".to_string(),
                tenant_id: None,
                payload: b"x".to_vec(),
            })
            .await
            .unwrap();
        dal.delivery_outbox().mark_delivered(row2.id).await.unwrap();
        let n = dal
            .delivery_outbox()
            .reset_delivered_to_pending_for_recipient("global:2", Some("t1"))
            .await
            .unwrap();
        assert_eq!(n, 0, "Some(tenant) must not match a NULL-tenant row");
    }
}
