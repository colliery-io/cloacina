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
}
