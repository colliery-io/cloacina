/*
 *  Copyright 2025 Colliery Software
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

//! DAL for reactor owner addresses (CLOACI-T-0851 / A-0012 Amendment 3).
//!
//! **A routing hint, not ownership.** The advisory lock is the sole authority
//! on who owns a reactor; rows here only say where the last successful claimant
//! said it could be reached. Reads answer "where should I redirect this
//! inject", and a stale answer costs one wasted hop — the fallback (redirect
//! again, or the delivery outbox) covers it. No reader may treat presence or
//! absence of a row as evidence about ownership.

use super::models::{NewUnifiedReactorOwnerAddress, UnifiedReactorOwnerAddress};
use super::DAL;
use crate::database::schema::unified::reactor_owner_addresses;
use crate::database::universal_types::UniversalTimestamp;
use crate::error::ValidationError;
use diesel::prelude::*;

/// DAL accessor for the `reactor_owner_addresses` routing-hint table.
pub struct ReactorOwnerAddressesDAL<'a> {
    dal: &'a DAL,
}

impl<'a> ReactorOwnerAddressesDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Publish (or overwrite) the owner address for a reactor.
    ///
    /// Delete-then-insert rather than upsert: the unique index is an
    /// EXPRESSION index (`COALESCE(tenant_id,'')`), which `ON CONFLICT` cannot
    /// target portably across postgres and sqlite. The two statements are not
    /// atomic, but this table is a hint — the worst interleaving leaves either
    /// the old address (a wasted redirect) or none (outbox fallback), both of
    /// which the routing design already absorbs.
    pub async fn publish(
        &self,
        tenant_id: Option<&str>,
        reactor_name: &str,
        address: &str,
    ) -> Result<(), ValidationError> {
        let tenant_owned = tenant_id.map(|t| t.to_string());
        let row = NewUnifiedReactorOwnerAddress {
            tenant_id: tenant_owned.clone(),
            reactor_name: reactor_name.to_string(),
            address: address.to_string(),
            claimed_at: UniversalTimestamp::now(),
        };
        let name_owned = reactor_name.to_string();
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction(|conn| {
                match &tenant_owned {
                    Some(t) => diesel::delete(
                        reactor_owner_addresses::table
                            .filter(reactor_owner_addresses::tenant_id.eq(t))
                            .filter(reactor_owner_addresses::reactor_name.eq(&name_owned)),
                    )
                    .execute(conn)?,
                    None => diesel::delete(
                        reactor_owner_addresses::table
                            .filter(reactor_owner_addresses::tenant_id.is_null())
                            .filter(reactor_owner_addresses::reactor_name.eq(&name_owned)),
                    )
                    .execute(conn)?,
                };
                diesel::insert_into(reactor_owner_addresses::table)
                    .values(&row)
                    .execute(conn)
            })
        })?;
        Ok(())
    }

    /// Remove the address row for a reactor this replica is releasing.
    ///
    /// Callers pass the address they published so a NEW owner's row is not
    /// torn down by a stale releaser: release-after-takeover would otherwise
    /// delete the successor's perfectly valid address. Deleting nothing is
    /// success — the row may already have been overwritten, which is the
    /// normal takeover sequence.
    pub async fn remove_if_ours(
        &self,
        tenant_id: Option<&str>,
        reactor_name: &str,
        our_address: &str,
    ) -> Result<(), ValidationError> {
        let tenant_owned = tenant_id.map(|t| t.to_string());
        let name_owned = reactor_name.to_string();
        let addr_owned = our_address.to_string();
        crate::interact_on_backend!(self.dal, |conn| {
            match &tenant_owned {
                Some(t) => diesel::delete(
                    reactor_owner_addresses::table
                        .filter(reactor_owner_addresses::tenant_id.eq(t))
                        .filter(reactor_owner_addresses::reactor_name.eq(&name_owned))
                        .filter(reactor_owner_addresses::address.eq(&addr_owned)),
                )
                .execute(conn),
                None => diesel::delete(
                    reactor_owner_addresses::table
                        .filter(reactor_owner_addresses::tenant_id.is_null())
                        .filter(reactor_owner_addresses::reactor_name.eq(&name_owned))
                        .filter(reactor_owner_addresses::address.eq(&addr_owned)),
                )
                .execute(conn),
            }
        })?;
        Ok(())
    }

    /// Look up the advertised address for a reactor. `None` means "no hint" —
    /// callers fall back to the outbox; it does NOT mean the reactor is
    /// unowned.
    pub async fn lookup(
        &self,
        tenant_id: Option<&str>,
        reactor_name: &str,
    ) -> Result<Option<String>, ValidationError> {
        let tenant_owned = tenant_id.map(|t| t.to_string());
        let name_owned = reactor_name.to_string();
        let row: Option<UnifiedReactorOwnerAddress> =
            crate::interact_on_backend!(self.dal, |conn| {
                match &tenant_owned {
                    Some(t) => reactor_owner_addresses::table
                        .filter(reactor_owner_addresses::tenant_id.eq(t))
                        .filter(reactor_owner_addresses::reactor_name.eq(&name_owned))
                        .first(conn)
                        .optional(),
                    None => reactor_owner_addresses::table
                        .filter(reactor_owner_addresses::tenant_id.is_null())
                        .filter(reactor_owner_addresses::reactor_name.eq(&name_owned))
                        .first(conn)
                        .optional(),
                }
            })?;
        Ok(row.map(|r| r.address))
    }
}

// Gated on sqlite like the sibling DAL tests (login_throttle): these use an
// in-memory sqlite database, which cannot exist in a postgres-only build —
// found as 6 panics in CI's Feature Build (postgres-only) lane.
#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use crate::dal::unified::DAL;
    use crate::database::Database;

    async fn dal() -> DAL {
        let url = format!(
            "file:reactor_owner_addr_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations().await.expect("migrations");
        DAL::new(db)
    }

    #[tokio::test]
    async fn publish_then_lookup_round_trips() {
        let dal = dal().await;
        let d = dal.reactor_owner_addresses();
        d.publish(Some("t1"), "rx", "http://pod-a:8080")
            .await
            .unwrap();
        assert_eq!(
            d.lookup(Some("t1"), "rx").await.unwrap().as_deref(),
            Some("http://pod-a:8080")
        );
    }

    /// Re-publishing (takeover) must overwrite, not error and not duplicate.
    #[tokio::test]
    async fn republish_overwrites_the_previous_owner() {
        let dal = dal().await;
        let d = dal.reactor_owner_addresses();
        d.publish(Some("t1"), "rx", "http://pod-a:8080")
            .await
            .unwrap();
        d.publish(Some("t1"), "rx", "http://pod-b:8080")
            .await
            .unwrap();
        assert_eq!(
            d.lookup(Some("t1"), "rx").await.unwrap().as_deref(),
            Some("http://pod-b:8080")
        );
    }

    /// The takeover race this API is shaped around: a STALE releaser must not
    /// tear down the successor's row. Otherwise the sequence
    /// (A claims, A dies, B claims, A's shutdown finally runs) ends with no
    /// address for a reactor that has a healthy owner.
    #[tokio::test]
    async fn stale_release_does_not_remove_the_new_owners_row() {
        let dal = dal().await;
        let d = dal.reactor_owner_addresses();
        d.publish(Some("t1"), "rx", "http://pod-a:8080")
            .await
            .unwrap();
        // B takes over.
        d.publish(Some("t1"), "rx", "http://pod-b:8080")
            .await
            .unwrap();
        // A's late release names ITS address, which no longer matches.
        d.remove_if_ours(Some("t1"), "rx", "http://pod-a:8080")
            .await
            .unwrap();
        assert_eq!(
            d.lookup(Some("t1"), "rx").await.unwrap().as_deref(),
            Some("http://pod-b:8080"),
            "the successor's address must survive a stale release"
        );
    }

    #[tokio::test]
    async fn matching_release_removes_the_row() {
        let dal = dal().await;
        let d = dal.reactor_owner_addresses();
        d.publish(Some("t1"), "rx", "http://pod-a:8080")
            .await
            .unwrap();
        d.remove_if_ours(Some("t1"), "rx", "http://pod-a:8080")
            .await
            .unwrap();
        assert_eq!(d.lookup(Some("t1"), "rx").await.unwrap(), None);
    }

    /// Untenanted and tenanted entries are distinct; and two tenants may each
    /// own a same-named reactor.
    #[tokio::test]
    async fn tenant_scoping_is_respected() {
        let dal = dal().await;
        let d = dal.reactor_owner_addresses();
        d.publish(None, "rx", "http://pod-a:8080").await.unwrap();
        d.publish(Some("t1"), "rx", "http://pod-b:8080")
            .await
            .unwrap();
        d.publish(Some("t2"), "rx", "http://pod-c:8080")
            .await
            .unwrap();

        assert_eq!(
            d.lookup(None, "rx").await.unwrap().as_deref(),
            Some("http://pod-a:8080")
        );
        assert_eq!(
            d.lookup(Some("t1"), "rx").await.unwrap().as_deref(),
            Some("http://pod-b:8080")
        );
        assert_eq!(
            d.lookup(Some("t2"), "rx").await.unwrap().as_deref(),
            Some("http://pod-c:8080")
        );
    }

    #[tokio::test]
    async fn lookup_of_an_unpublished_reactor_is_none() {
        let dal = dal().await;
        assert_eq!(
            dal.reactor_owner_addresses()
                .lookup(Some("t1"), "ghost")
                .await
                .unwrap(),
            None
        );
    }
}
