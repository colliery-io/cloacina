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

//! Unified WS-ticket DAL (CLOACI-T-0916).
//!
//! Single-use, short-TTL WebSocket auth tickets, persisted so a ticket minted
//! on one server replica redeems on ANY replica (no session affinity —
//! mirrors the OIDC login-flow precedent, T-0801). Redemption is once per WS
//! connect, so a straight DB round-trip is fine (no hot-path cache needed).
//!
//! Single-use enforcement is an **atomic compare-and-set**:
//! `UPDATE ... SET redeemed_at = now WHERE ticket = ? AND redeemed_at IS NULL
//! AND expires_at > now` — exactly one concurrent redeemer sees
//! `rows_affected == 1`; every other attempt (replay, expiry, unknown ticket)
//! yields `None`. Works identically on both backends (no RETURNING needed:
//! the winner then reads the row it now exclusively owns).

use super::models::{NewUnifiedWsTicket, UnifiedWsTicket};
use super::DAL;
use crate::database::schema::unified::ws_tickets;
use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use diesel::prelude::*;

/// The authenticated-key identity a redeemed ticket carries.
#[derive(Debug, Clone)]
pub struct WsTicketAuth {
    pub key_id: uuid::Uuid,
    pub name: String,
    pub permissions: String,
    pub tenant_id: Option<String>,
    pub is_admin: bool,
}

/// Data access layer for single-use WebSocket auth tickets.
#[derive(Clone)]
pub struct WsTicketDAL<'a> {
    dal: &'a DAL,
}

impl<'a> WsTicketDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Issue a ticket carrying `auth`, valid for `ttl`. Returns the ticket
    /// string (a fresh UUID). Expired rows are pruned opportunistically first
    /// so the table stays bounded without a dedicated sweeper.
    pub async fn issue(
        &self,
        auth: WsTicketAuth,
        ttl: std::time::Duration,
    ) -> Result<String, ValidationError> {
        let ticket = uuid::Uuid::new_v4().to_string();
        let now = UniversalTimestamp::now();
        let expires_at = UniversalTimestamp(
            now.0
                + chrono::Duration::from_std(ttl).unwrap_or_else(|_| chrono::Duration::seconds(60)),
        );
        let row = NewUnifiedWsTicket {
            ticket: ticket.clone(),
            key_id: UniversalUuid(auth.key_id),
            key_name: auth.name,
            permissions: auth.permissions,
            tenant_id: auth.tenant_id,
            is_admin: UniversalBool::new(auth.is_admin),
            created_at: now,
            expires_at,
            redeemed_at: None,
        };
        crate::interact_on_backend!(self.dal, |conn| {
            // Opportunistic prune: expired tickets (redeemed or not) are dead
            // weight — delete before inserting the fresh one.
            diesel::delete(ws_tickets::table.filter(ws_tickets::expires_at.lt(now)))
                .execute(conn)?;
            diesel::insert_into(ws_tickets::table)
                .values(&row)
                .execute(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(ticket)
    }

    /// Redeem a ticket (single-use). Returns the carried auth identity iff
    /// the ticket exists, has not expired, and has not been redeemed before —
    /// atomically, so exactly one of any number of concurrent redeemers wins.
    pub async fn redeem(&self, ticket: &str) -> Result<Option<WsTicketAuth>, ValidationError> {
        let ticket = ticket.to_string();
        let now = UniversalTimestamp::now();
        let row: Option<UnifiedWsTicket> = crate::interact_on_backend!(self.dal, |conn| {
            // Atomic CAS: only an unredeemed, unexpired row transitions.
            let affected = diesel::update(
                ws_tickets::table
                    .filter(ws_tickets::ticket.eq(&ticket))
                    .filter(ws_tickets::redeemed_at.is_null())
                    .filter(ws_tickets::expires_at.gt(now)),
            )
            .set(ws_tickets::redeemed_at.eq(Some(now)))
            .execute(conn)?;
            if affected == 1 {
                // We won the CAS — the row is exclusively ours to read.
                ws_tickets::table
                    .filter(ws_tickets::ticket.eq(&ticket))
                    .first::<UnifiedWsTicket>(conn)
                    .optional()
            } else {
                Ok(None)
            }
        })
        .map_err(ValidationError::from)?;
        Ok(row.map(|r| WsTicketAuth {
            key_id: r.key_id.0,
            name: r.key_name,
            permissions: r.permissions,
            tenant_id: r.tenant_id,
            is_admin: r.is_admin.into(),
        }))
    }

    /// Delete all tickets whose `expires_at` has passed. Returns the count.
    /// (Also runs opportunistically inside [`issue`].)
    pub async fn prune_expired(&self) -> Result<usize, ValidationError> {
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(ws_tickets::table.filter(ws_tickets::expires_at.lt(now))).execute(conn)
        })
        .map_err(ValidationError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    #[cfg(feature = "sqlite")]
    fn shared_url() -> String {
        format!(
            "file:ws_tickets_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        )
    }

    #[cfg(feature = "sqlite")]
    async fn dal_for(url: &str) -> DAL {
        let db = Database::new(url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    #[cfg(feature = "sqlite")]
    fn auth(name: &str) -> WsTicketAuth {
        WsTicketAuth {
            key_id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            permissions: "read".to_string(),
            tenant_id: Some("t1".to_string()),
            is_admin: false,
        }
    }

    /// The multi-replica contract: a ticket issued through one DAL handle
    /// (replica A) redeems through a DIFFERENT DAL handle over the same
    /// database (replica B) — and only once.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn ticket_issued_on_one_handle_redeems_on_another_once() {
        let url = shared_url();
        let dal_a = dal_for(&url).await;
        // Second Database instance over the same shared-memory DB = replica B.
        let dal_b = DAL::new(Database::new(&url, "", 5));

        let ticket = dal_a
            .ws_tickets()
            .issue(auth("cross-replica"), std::time::Duration::from_secs(60))
            .await
            .unwrap();

        let redeemed = dal_b.ws_tickets().redeem(&ticket).await.unwrap();
        let redeemed = redeemed.expect("ticket minted on A must redeem on B");
        assert_eq!(redeemed.name, "cross-replica");
        assert_eq!(redeemed.tenant_id.as_deref(), Some("t1"));
        assert!(!redeemed.is_admin);

        // Single-use: a replay on EITHER handle fails.
        assert!(dal_a.ws_tickets().redeem(&ticket).await.unwrap().is_none());
        assert!(dal_b.ws_tickets().redeem(&ticket).await.unwrap().is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn unknown_ticket_is_rejected() {
        let dal = dal_for(&shared_url()).await;
        assert!(dal
            .ws_tickets()
            .redeem("not-a-real-ticket")
            .await
            .unwrap()
            .is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn expired_ticket_is_rejected_and_pruned() {
        let dal = dal_for(&shared_url()).await;
        let ticket = dal
            .ws_tickets()
            .issue(auth("expiring"), std::time::Duration::ZERO)
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert!(dal.ws_tickets().redeem(&ticket).await.unwrap().is_none());
        // And the sweeper path removes it.
        assert_eq!(dal.ws_tickets().prune_expired().await.unwrap(), 1);
    }
}
