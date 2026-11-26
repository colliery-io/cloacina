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

//! Agent pools (CLOACINA-T-1021).
//!
//! A tenant's managed agents are grouped in pools; each pool has its own size
//! of agent. One row per (tenant, pool) holds the pool's settings as one JSON
//! object: resource requests and limits, max concurrency, image, service
//! account and its annotations, extra egress rules, min and max agent count.
//! This DAL stores and returns the object as text: the server owns the shape
//! and validates it before a write.
//!
//! The pool `default` needs no row: with none it runs on the server-wide
//! defaults. The fleet actuator reads the rows on each reconcile, so a change
//! reaches the tenant's agents on the next tick of the fleet loop.

use super::DAL;
use super::models::{NewUnifiedAgentPool, UnifiedAgentPool};
use crate::database::schema::unified::agent_pools;
use crate::database::universal_types::UniversalTimestamp;
use crate::error::ValidationError;
use diesel::prelude::*;

/// DAL for the agent pools of a tenant.
pub struct AgentPoolsDAL<'a> {
    dal: &'a DAL,
}

impl<'a> AgentPoolsDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Store (or replace) the settings of one pool. `settings` is a JSON
    /// object as text. Keeps `created_at` on a replace.
    pub async fn set(
        &self,
        tenant_id: &str,
        pool: &str,
        settings: &str,
    ) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        let settings = settings.to_string();
        let row = NewUnifiedAgentPool {
            tenant_id: tenant_id.to_string(),
            pool: pool.to_string(),
            settings: settings.clone(),
            created_at: now,
            updated_at: now,
        };
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(agent_pools::table)
                .values(&row)
                .on_conflict((agent_pools::tenant_id, agent_pools::pool))
                .do_update()
                .set((
                    agent_pools::settings.eq(&settings),
                    agent_pools::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(())
    }

    /// The stored settings of one pool as JSON text, or `None` when the pool
    /// has no row.
    pub async fn get(
        &self,
        tenant_id: &str,
        pool: &str,
    ) -> Result<Option<String>, ValidationError> {
        let tenant_id = tenant_id.to_string();
        let pool = pool.to_string();
        let row: Option<UnifiedAgentPool> = crate::interact_on_backend!(self.dal, |conn| {
            agent_pools::table
                .filter(agent_pools::tenant_id.eq(&tenant_id))
                .filter(agent_pools::pool.eq(&pool))
                .select(UnifiedAgentPool::as_select())
                .first(conn)
                .optional()
        })
        .map_err(ValidationError::from)?;
        Ok(row.map(|r| r.settings))
    }

    /// Every stored pool of a tenant as `(pool, settings JSON text)`, by pool
    /// name.
    pub async fn list(&self, tenant_id: &str) -> Result<Vec<(String, String)>, ValidationError> {
        let tenant_id = tenant_id.to_string();
        let rows: Vec<UnifiedAgentPool> = crate::interact_on_backend!(self.dal, |conn| {
            agent_pools::table
                .filter(agent_pools::tenant_id.eq(&tenant_id))
                .order(agent_pools::pool.asc())
                .select(UnifiedAgentPool::as_select())
                .load(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(rows.into_iter().map(|r| (r.pool, r.settings)).collect())
    }

    /// The tenants that have at least one stored pool, by name.
    pub async fn list_tenants(&self) -> Result<Vec<String>, ValidationError> {
        let tenants: Vec<String> = crate::interact_on_backend!(self.dal, |conn| {
            agent_pools::table
                .select(agent_pools::tenant_id)
                .distinct()
                .order(agent_pools::tenant_id.asc())
                .load(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(tenants)
    }

    /// Remove one pool's row. Returns true when a row was removed.
    pub async fn delete(&self, tenant_id: &str, pool: &str) -> Result<bool, ValidationError> {
        let tenant_id = tenant_id.to_string();
        let pool = pool.to_string();
        let n = crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(
                agent_pools::table
                    .filter(agent_pools::tenant_id.eq(&tenant_id))
                    .filter(agent_pools::pool.eq(&pool)),
            )
            .execute(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(n > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    #[cfg(feature = "sqlite")]
    async fn dal() -> DAL {
        let url = format!(
            "file:agent_pools_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn agent_pools_absent_row_reads_none() {
        let dal = dal().await;
        let p = dal.agent_pools();
        assert_eq!(p.get("acme", "default").await.unwrap(), None);
        assert!(p.list("acme").await.unwrap().is_empty());
        assert!(p.list_tenants().await.unwrap().is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn agent_pools_set_get_replace() {
        let dal = dal().await;
        let p = dal.agent_pools();
        p.set("acme", "big", r#"{"memory_limit":"32Gi"}"#)
            .await
            .unwrap();
        assert_eq!(
            p.get("acme", "big").await.unwrap().as_deref(),
            Some(r#"{"memory_limit":"32Gi"}"#)
        );
        // A second set replaces the whole object.
        p.set("acme", "big", r#"{"max_concurrency":1}"#)
            .await
            .unwrap();
        assert_eq!(
            p.get("acme", "big").await.unwrap().as_deref(),
            Some(r#"{"max_concurrency":1}"#)
        );
        assert_eq!(p.list("acme").await.unwrap().len(), 1);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn agent_pools_are_keyed_by_tenant_and_pool() {
        let dal = dal().await;
        let p = dal.agent_pools();
        p.set("acme", "default", r#"{"max_concurrency":2}"#)
            .await
            .unwrap();
        p.set("acme", "big", r#"{"max_concurrency":1}"#)
            .await
            .unwrap();
        p.set("globex", "big", r#"{"max_concurrency":8}"#)
            .await
            .unwrap();
        assert_eq!(
            p.list("acme").await.unwrap(),
            vec![
                ("big".to_string(), r#"{"max_concurrency":1}"#.to_string()),
                (
                    "default".to_string(),
                    r#"{"max_concurrency":2}"#.to_string()
                ),
            ]
        );
        assert_eq!(
            p.get("globex", "big").await.unwrap().as_deref(),
            Some(r#"{"max_concurrency":8}"#)
        );
        assert_eq!(p.get("globex", "default").await.unwrap(), None);
        assert_eq!(
            p.list_tenants().await.unwrap(),
            vec!["acme".to_string(), "globex".to_string()]
        );
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn agent_pools_delete_removes_one_pool() {
        let dal = dal().await;
        let p = dal.agent_pools();
        p.set("acme", "big", "{}").await.unwrap();
        p.set("acme", "small", "{}").await.unwrap();
        assert!(p.delete("acme", "big").await.unwrap());
        assert_eq!(p.get("acme", "big").await.unwrap(), None);
        assert!(p.get("acme", "small").await.unwrap().is_some());
        assert!(!p.delete("acme", "big").await.unwrap());
    }
}
