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

//! Agent desired count — per-tenant self-service provisioning state
//! (CLOACI-T-0809, CLOACI-I-0127). **Postgres only.**
//!
//! `desired_count` is the number of agents a tenant has requested. It is tenant
//! self-service provisioning state, bounded by the god-set `effective_limit`
//! from T-0808 (`AgentLimitsDAL`): the provision API increments it (+1) only
//! while under the limit, deprovision decrements it (−1, floor 0). This is the
//! operational target the actuator (T-0810) and the back-pressure autoscaler
//! (T-0811) reconcile/clamp to. Absent → 0 (no agents requested yet).

use crate::dal::unified::DAL;
use crate::error::ValidationError;

#[cfg(feature = "postgres")]
use diesel::prelude::*;

#[cfg(feature = "postgres")]
#[derive(Queryable)]
#[diesel(table_name = crate::database::schema::postgres::agent_desired_counts)]
struct AgentDesiredRow {
    #[allow(dead_code)]
    pub tenant_id: String,
    pub desired_count: i32,
    #[allow(dead_code)]
    pub updated_at: chrono::NaiveDateTime,
}

/// DAL for per-tenant desired agent count. Postgres only.
pub struct AgentDesiredDAL<'a> {
    dal: &'a DAL,
}

impl<'a> AgentDesiredDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// The tenant's desired agent count (`0` if no row is set yet). This is the
    /// provisioning target the actuator/autoscaler reconcile to.
    #[cfg(feature = "postgres")]
    pub async fn get_desired(&self, tenant_id: &str) -> Result<u32, ValidationError> {
        use crate::database::schema::postgres::agent_desired_counts as t;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let tenant = tenant_id.to_string();
        let row: Option<AgentDesiredRow> = conn
            .interact(move |conn| t::table.find(tenant).first(conn).optional())
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(row.map(|r| r.desired_count.max(0) as u32).unwrap_or(0))
    }

    /// Set (or replace) the tenant's desired agent count. Upserts on `tenant_id`;
    /// `updated_at` is managed by the column default (not written from Rust).
    #[cfg(feature = "postgres")]
    pub async fn set_desired(
        &self,
        tenant_id: &str,
        desired_count: u32,
    ) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::agent_desired_counts as t;
        let tenant = tenant_id.to_string();
        let desired = desired_count as i32;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            diesel::insert_into(t::table)
                .values((t::tenant_id.eq(&tenant), t::desired_count.eq(desired)))
                .on_conflict(t::tenant_id)
                .do_update()
                .set(t::desired_count.eq(desired))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }
}
