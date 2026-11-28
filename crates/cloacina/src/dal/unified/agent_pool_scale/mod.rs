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

//! The autoscaler's state for each agent pool of a tenant (CLOACINA-T-1029).
//! **Postgres only**, like the desired count of `default`.
//!
//! One row per (tenant, pool):
//! - `desired_count`: the count of a pool other than `default`. `None` for
//!   `default`, whose count is `agent_desired_counts` (the provision routes
//!   write it).
//! - `idle_since`: since when the pool has had no task in flight and none
//!   waiting. `None`: it has work.
//! - `last_scaled_down_at`: the last scale-down, so two scale-downs are a
//!   cooldown apart.
//!
//! The times are written with the database clock (`now()`), so the rules
//! hold whichever server replica leads the fleet loop (CLOACINA-A-0008).

use crate::dal::unified::DAL;
use crate::error::ValidationError;

#[cfg(feature = "postgres")]
use diesel::prelude::*;

/// The stored state of one pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolScaleState {
    pub tenant_id: String,
    pub pool: String,
    /// The count of a pool other than `default`. `None` for `default`, or a
    /// pool the autoscaler has not set yet.
    pub desired_count: Option<u32>,
    /// Since when the pool has been idle. `None`: it has work.
    pub idle_since: Option<chrono::NaiveDateTime>,
    /// The last scale-down.
    pub last_scaled_down_at: Option<chrono::NaiveDateTime>,
}

#[cfg(feature = "postgres")]
#[derive(Queryable)]
#[diesel(table_name = crate::database::schema::postgres::agent_pool_scale)]
struct Row {
    tenant_id: String,
    pool: String,
    desired_count: Option<i32>,
    idle_since: Option<chrono::NaiveDateTime>,
    last_scaled_down_at: Option<chrono::NaiveDateTime>,
    #[allow(dead_code)]
    updated_at: chrono::NaiveDateTime,
}

/// DAL for the autoscaler's per-pool state. Postgres only.
pub struct AgentPoolScaleDAL<'a> {
    dal: &'a DAL,
}

impl<'a> AgentPoolScaleDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    #[cfg(feature = "postgres")]
    async fn conn(&self) -> Result<deadpool_diesel::postgres::Object, ValidationError> {
        self.dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))
    }

    /// Every stored pool state.
    #[cfg(feature = "postgres")]
    pub async fn list_all(&self) -> Result<Vec<PoolScaleState>, ValidationError> {
        use crate::database::schema::postgres::agent_pool_scale as t;
        let rows: Vec<Row> = self
            .conn()
            .await?
            .interact(move |conn| t::table.load(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows
            .into_iter()
            .map(|r| PoolScaleState {
                tenant_id: r.tenant_id,
                pool: r.pool,
                desired_count: r.desired_count.map(|d| d.max(0) as u32),
                idle_since: r.idle_since,
                last_scaled_down_at: r.last_scaled_down_at,
            })
            .collect())
    }

    /// Set the count of a pool other than `default` (a scale-up, or the
    /// first count of a pool).
    #[cfg(feature = "postgres")]
    pub async fn set_desired(
        &self,
        tenant_id: &str,
        pool: &str,
        desired: u32,
    ) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::agent_pool_scale as t;
        let (tenant, pool) = (tenant_id.to_string(), pool.to_string());
        let desired = desired as i32;
        self.conn()
            .await?
            .interact(move |conn| {
                diesel::insert_into(t::table)
                    .values((
                        t::tenant_id.eq(&tenant),
                        t::pool.eq(&pool),
                        t::desired_count.eq(Some(desired)),
                    ))
                    .on_conflict((t::tenant_id, t::pool))
                    .do_update()
                    .set((
                        t::desired_count.eq(Some(desired)),
                        t::updated_at.eq(diesel::dsl::now),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }

    /// Record whether the pool is idle now. Idle keeps an `idle_since` that
    /// is already set (the idle time goes on); busy clears it.
    #[cfg(feature = "postgres")]
    pub async fn set_idle(
        &self,
        tenant_id: &str,
        pool: &str,
        idle: bool,
    ) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::agent_pool_scale as t;
        let (tenant, pool) = (tenant_id.to_string(), pool.to_string());
        self.conn()
            .await?
            .interact(move |conn| {
                if idle {
                    diesel::insert_into(t::table)
                        .values((
                            t::tenant_id.eq(&tenant),
                            t::pool.eq(&pool),
                            t::idle_since.eq(diesel::dsl::now),
                        ))
                        .on_conflict((t::tenant_id, t::pool))
                        .do_update()
                        .set(t::idle_since.eq(diesel::dsl::sql::<
                            diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>,
                        >(
                            "COALESCE(agent_pool_scale.idle_since, now())"
                        )))
                        .execute(conn)
                } else {
                    diesel::insert_into(t::table)
                        .values((
                            t::tenant_id.eq(&tenant),
                            t::pool.eq(&pool),
                            t::idle_since.eq(None::<chrono::NaiveDateTime>),
                        ))
                        .on_conflict((t::tenant_id, t::pool))
                        .do_update()
                        .set(t::idle_since.eq(None::<chrono::NaiveDateTime>))
                        .execute(conn)
                }
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }

    /// Record a scale-down: `last_scaled_down_at = now()`, and for a pool
    /// other than `default` its new count.
    #[cfg(feature = "postgres")]
    pub async fn record_scale_down(
        &self,
        tenant_id: &str,
        pool: &str,
        desired: Option<u32>,
    ) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::agent_pool_scale as t;
        let (tenant, pool) = (tenant_id.to_string(), pool.to_string());
        let desired = desired.map(|d| d as i32);
        self.conn()
            .await?
            .interact(move |conn| {
                diesel::insert_into(t::table)
                    .values((
                        t::tenant_id.eq(&tenant),
                        t::pool.eq(&pool),
                        t::desired_count.eq(desired),
                        t::last_scaled_down_at.eq(diesel::dsl::now),
                    ))
                    .on_conflict((t::tenant_id, t::pool))
                    .do_update()
                    .set((
                        t::desired_count.eq(desired),
                        t::last_scaled_down_at.eq(diesel::dsl::now),
                        t::updated_at.eq(diesel::dsl::now),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }

    /// Remove the state of one pool (the pool was removed). Idempotent.
    #[cfg(feature = "postgres")]
    pub async fn delete_pool(&self, tenant_id: &str, pool: &str) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::agent_pool_scale as t;
        let (tenant, pool) = (tenant_id.to_string(), pool.to_string());
        self.conn()
            .await?
            .interact(move |conn| {
                diesel::delete(
                    t::table
                        .filter(t::tenant_id.eq(tenant))
                        .filter(t::pool.eq(pool)),
                )
                .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }

    /// Remove the state of every pool of a tenant (the tenant was removed).
    /// Idempotent.
    #[cfg(feature = "postgres")]
    pub async fn delete_tenant(&self, tenant_id: &str) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::agent_pool_scale as t;
        let tenant = tenant_id.to_string();
        self.conn()
            .await?
            .interact(move |conn| {
                diesel::delete(t::table.filter(t::tenant_id.eq(tenant))).execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }
}
