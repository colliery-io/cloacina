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

//! Integration tests for the per-tenant desired-agent-count DAL
//! (`AgentDesiredDAL`, CLOACI-T-0809). **Postgres only** — the DAL is gated on
//! the `postgres` feature, so these tests are too.
//!
//! Each test takes the shared Postgres fixture, resets + initializes it, then
//! drives `DAL::agent_desired()` end-to-end against a real Postgres schema.

#[cfg(feature = "postgres")]
mod postgres_tests {
    use crate::fixtures::get_or_init_postgres_fixture;
    use cloacina::dal::DAL;
    use serial_test::serial;

    /// get on a tenant with no row returns the default 0.
    #[tokio::test]
    #[serial]
    async fn test_get_unset_desired_is_zero() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.reset_database().await;
        fixture.initialize().await;
        let dal = DAL::new(fixture.get_database());

        let got = dal
            .agent_desired()
            .get_desired("nonexistent")
            .await
            .expect("get_desired should succeed");
        assert_eq!(got, 0, "absent tenant should default to 0");
    }

    /// set → get round-trips the desired value.
    #[tokio::test]
    #[serial]
    async fn test_set_then_get_desired() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.reset_database().await;
        fixture.initialize().await;
        let dal = DAL::new(fixture.get_database());

        dal.agent_desired()
            .set_desired("acme", 3)
            .await
            .expect("set_desired should succeed");

        let got = dal
            .agent_desired()
            .get_desired("acme")
            .await
            .expect("get_desired should succeed");
        assert_eq!(got, 3);
    }

    /// set is an upsert: a second set replaces the value with no error / dup row.
    #[tokio::test]
    #[serial]
    async fn test_set_desired_upserts() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.reset_database().await;
        fixture.initialize().await;
        let dal = DAL::new(fixture.get_database());

        dal.agent_desired()
            .set_desired("acme", 3)
            .await
            .expect("first set should succeed");
        dal.agent_desired()
            .set_desired("acme", 7)
            .await
            .expect("second set (upsert) should succeed");

        let got = dal
            .agent_desired()
            .get_desired("acme")
            .await
            .expect("get_desired should succeed");
        assert_eq!(got, 7, "upsert should replace the prior value");
    }

    /// Read a tenant's `last_autoscaled_at` back via `list_all_with_last`.
    async fn last_autoscaled_at(dal: &DAL, tenant: &str) -> Option<chrono::NaiveDateTime> {
        dal.agent_desired()
            .list_all_with_last()
            .await
            .expect("list_all_with_last should succeed")
            .into_iter()
            .find(|(t, _, _)| t == tenant)
            .and_then(|(_, _, last)| last)
    }

    /// The autoscaler write (`set_desired_autoscaled`) stamps `last_autoscaled_at`
    /// (SQL `now()`), while the manual write (`set_desired`) leaves it NULL — so
    /// manual scaling does not arm the autoscaler cooldown. This is the crux of
    /// the CLOACI-A-0008 cross-replica-cooldown fix.
    #[tokio::test]
    #[serial]
    async fn test_set_desired_autoscaled_stamps_last_but_manual_does_not() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.reset_database().await;
        fixture.initialize().await;
        let dal = DAL::new(fixture.get_database());

        // Manual write: desired set, last_autoscaled_at stays NULL.
        dal.agent_desired()
            .set_desired("acme", 2)
            .await
            .expect("set_desired should succeed");
        assert_eq!(dal.agent_desired().get_desired("acme").await.unwrap(), 2);
        assert!(
            last_autoscaled_at(&dal, "acme").await.is_none(),
            "manual set_desired must NOT stamp last_autoscaled_at"
        );

        // Autoscaler write: desired updated AND last_autoscaled_at stamped.
        dal.agent_desired()
            .set_desired_autoscaled("acme", 3)
            .await
            .expect("set_desired_autoscaled should succeed");
        assert_eq!(dal.agent_desired().get_desired("acme").await.unwrap(), 3);
        let stamped = last_autoscaled_at(&dal, "acme")
            .await
            .expect("autoscaled write must stamp last_autoscaled_at");

        // A subsequent manual write changes desired but must leave the existing
        // last_autoscaled_at untouched (it neither clears nor refreshes it).
        dal.agent_desired()
            .set_desired("acme", 5)
            .await
            .expect("set_desired should succeed");
        assert_eq!(dal.agent_desired().get_desired("acme").await.unwrap(), 5);
        assert_eq!(
            last_autoscaled_at(&dal, "acme").await,
            Some(stamped),
            "manual set_desired must leave last_autoscaled_at untouched"
        );
    }

    /// A fresh autoscaler write on a tenant with no prior row stamps a non-NULL
    /// `last_autoscaled_at` on insert (not only on conflict/update).
    #[tokio::test]
    #[serial]
    async fn test_set_desired_autoscaled_stamps_on_insert() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.reset_database().await;
        fixture.initialize().await;
        let dal = DAL::new(fixture.get_database());

        dal.agent_desired()
            .set_desired_autoscaled("brandnew", 1)
            .await
            .expect("set_desired_autoscaled should succeed on insert");
        assert!(
            last_autoscaled_at(&dal, "brandnew").await.is_some(),
            "autoscaled insert must stamp last_autoscaled_at"
        );
    }
}
