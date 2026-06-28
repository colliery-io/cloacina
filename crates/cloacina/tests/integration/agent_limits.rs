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

//! Integration tests for the per-tenant agent-capacity limits DAL
//! (`AgentLimitsDAL`, CLOACI-T-0808). **Postgres only** — the DAL is gated on
//! the `postgres` feature, so these tests are too.
//!
//! Each test takes the shared Postgres fixture, resets + initializes it, then
//! drives `DAL::agent_limits()` end-to-end against a real Postgres schema.

#[cfg(feature = "postgres")]
mod postgres_tests {
    use crate::fixtures::get_or_init_postgres_fixture;
    use cloacina::dal::DAL;
    use serial_test::serial;

    /// set → get round-trips the override value.
    #[tokio::test]
    #[serial]
    async fn test_set_then_get_tenant_limit() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.reset_database().await;
        fixture.initialize().await;
        let dal = DAL::new(fixture.get_database());

        dal.agent_limits()
            .set_tenant_limit("acme", 6)
            .await
            .expect("set_tenant_limit should succeed");

        let got = dal
            .agent_limits()
            .get_tenant_limit("acme")
            .await
            .expect("get_tenant_limit should succeed");
        assert_eq!(got, Some(6));
    }

    /// get on a tenant with no exception returns None (the default applies).
    #[tokio::test]
    #[serial]
    async fn test_get_unset_tenant_limit_is_none() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.reset_database().await;
        fixture.initialize().await;
        let dal = DAL::new(fixture.get_database());

        let got = dal
            .agent_limits()
            .get_tenant_limit("nonexistent")
            .await
            .expect("get_tenant_limit should succeed");
        assert_eq!(got, None);
    }

    /// effective_limit returns the override when set, else the supplied default.
    #[tokio::test]
    #[serial]
    async fn test_effective_limit_override_vs_default() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.reset_database().await;
        fixture.initialize().await;
        let dal = DAL::new(fixture.get_database());

        dal.agent_limits()
            .set_tenant_limit("acme", 6)
            .await
            .expect("set_tenant_limit should succeed");

        let with_override = dal
            .agent_limits()
            .effective_limit("acme", 4)
            .await
            .expect("effective_limit should succeed");
        assert_eq!(with_override, 6, "override should win over default");

        let fallback = dal
            .agent_limits()
            .effective_limit("nobody", 4)
            .await
            .expect("effective_limit should succeed");
        assert_eq!(fallback, 4, "no override should fall back to default");
    }

    /// set is an upsert: a second set replaces the value with no error / dup row.
    #[tokio::test]
    #[serial]
    async fn test_set_tenant_limit_upserts() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.reset_database().await;
        fixture.initialize().await;
        let dal = DAL::new(fixture.get_database());

        dal.agent_limits()
            .set_tenant_limit("acme", 6)
            .await
            .expect("first set should succeed");
        dal.agent_limits()
            .set_tenant_limit("acme", 8)
            .await
            .expect("second set (upsert) should succeed");

        let got = dal
            .agent_limits()
            .get_tenant_limit("acme")
            .await
            .expect("get_tenant_limit should succeed");
        assert_eq!(got, Some(8), "upsert should replace the prior value");
    }

    /// clear removes the exception (returns true), after which get is None and
    /// effective_limit falls back to the default; clearing a missing tenant
    /// returns false.
    #[tokio::test]
    #[serial]
    async fn test_clear_tenant_limit() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.reset_database().await;
        fixture.initialize().await;
        let dal = DAL::new(fixture.get_database());

        dal.agent_limits()
            .set_tenant_limit("acme", 6)
            .await
            .expect("set_tenant_limit should succeed");

        let cleared = dal
            .agent_limits()
            .clear_tenant_limit("acme")
            .await
            .expect("clear_tenant_limit should succeed");
        assert!(cleared, "clearing an existing exception should return true");

        let got = dal
            .agent_limits()
            .get_tenant_limit("acme")
            .await
            .expect("get_tenant_limit should succeed");
        assert_eq!(got, None, "exception should be gone after clear");

        let effective = dal
            .agent_limits()
            .effective_limit("acme", 4)
            .await
            .expect("effective_limit should succeed");
        assert_eq!(effective, 4, "effective limit should revert to default");

        let cleared_again = dal
            .agent_limits()
            .clear_tenant_limit("never")
            .await
            .expect("clear_tenant_limit should succeed");
        assert!(
            !cleared_again,
            "clearing a tenant with no exception should return false"
        );
    }
}
