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

//! Integration tests for the autoscaler's per-pool state
//! (`AgentPoolScaleDAL`, CLOACINA-T-1029). **Postgres only**, like the DAL.

#[cfg(feature = "postgres")]
mod postgres_tests {
    use crate::fixtures::get_or_init_postgres_fixture;
    use cloacina::dal::DAL;
    use cloacina::dal::unified::PoolScaleState;
    use serial_test::serial;

    async fn state(dal: &DAL, tenant: &str, pool: &str) -> Option<PoolScaleState> {
        dal.agent_pool_scale()
            .list_all()
            .await
            .expect("list_all should succeed")
            .into_iter()
            .find(|s| s.tenant_id == tenant && s.pool == pool)
    }

    /// A count, the idle time and a scale-down are stored per (tenant, pool);
    /// idle keeps the first time it was set, busy clears it; removal is per
    /// pool and per tenant.
    #[tokio::test]
    #[serial]
    async fn test_pool_scale_state_round_trip() {
        let fixture = get_or_init_postgres_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.reset_database().await;
        fixture.initialize().await;
        let dal = DAL::new(fixture.get_database());
        let scale = dal.agent_pool_scale();

        assert!(state(&dal, "t1029a", "big").await.is_none());
        scale.set_desired("t1029a", "big", 3).await.unwrap();
        let s = state(&dal, "t1029a", "big").await.unwrap();
        assert_eq!(s.desired_count, Some(3));
        assert_eq!(s.idle_since, None);

        scale.set_idle("t1029a", "big", true).await.unwrap();
        let first = state(&dal, "t1029a", "big").await.unwrap().idle_since;
        assert!(first.is_some());
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        scale.set_idle("t1029a", "big", true).await.unwrap();
        assert_eq!(
            state(&dal, "t1029a", "big").await.unwrap().idle_since,
            first,
            "the idle time goes on; it is not reset"
        );
        scale.set_idle("t1029a", "big", false).await.unwrap();
        assert_eq!(state(&dal, "t1029a", "big").await.unwrap().idle_since, None);

        scale
            .record_scale_down("t1029a", "big", Some(0))
            .await
            .unwrap();
        let s = state(&dal, "t1029a", "big").await.unwrap();
        assert_eq!(s.desired_count, Some(0));
        assert!(s.last_scaled_down_at.is_some());

        // `default` keeps its count elsewhere: none here.
        scale
            .record_scale_down("t1029a", "default", None)
            .await
            .unwrap();
        let d = state(&dal, "t1029a", "default").await.unwrap();
        assert_eq!(d.desired_count, None);
        assert!(d.last_scaled_down_at.is_some());

        scale.set_desired("t1029b", "big", 1).await.unwrap();
        scale.delete_pool("t1029a", "big").await.unwrap();
        assert!(state(&dal, "t1029a", "big").await.is_none());
        assert!(state(&dal, "t1029a", "default").await.is_some());
        scale.delete_tenant("t1029a").await.unwrap();
        assert!(state(&dal, "t1029a", "default").await.is_none());
        assert!(state(&dal, "t1029b", "big").await.is_some());
        scale.delete_tenant("t1029b").await.unwrap();
    }
}
