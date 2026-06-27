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

//! Agent capacity limits — per-tenant exceptions to the global default
//! (CLOACI-T-0808, CLOACI-I-0127). **Postgres only.**
//!
//! The global default (`CLOACINA_DEFAULT_MAX_AGENTS`) is server config; this DAL
//! stores ONLY the per-tenant overrides an admin grants. `effective_limit` is the
//! override if present, else the default — the hard ceiling that the provision
//! API (T-0809) and the back-pressure autoscaler (T-0811) clamp to. Setting an
//! exception is a god-only op; a tenant may read its own effective limit.

use crate::dal::unified::DAL;
use crate::error::ValidationError;

#[cfg(feature = "postgres")]
use diesel::prelude::*;

#[cfg(feature = "postgres")]
#[derive(Queryable)]
#[diesel(table_name = crate::database::schema::postgres::agent_capacity_limits)]
struct AgentLimitRow {
    #[allow(dead_code)]
    pub tenant_id: String,
    pub max_agents: i32,
    #[allow(dead_code)]
    pub created_at: chrono::NaiveDateTime,
    #[allow(dead_code)]
    pub updated_at: chrono::NaiveDateTime,
}

/// DAL for per-tenant agent-capacity limits. Postgres only.
pub struct AgentLimitsDAL<'a> {
    dal: &'a DAL,
}

impl<'a> AgentLimitsDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Set (or replace) a tenant's agent-capacity exception (god-only op,
    /// T-0808). Upserts on `tenant_id`; `created_at`/`updated_at` are managed by
    /// the column defaults.
    #[cfg(feature = "postgres")]
    pub async fn set_tenant_limit(
        &self,
        tenant_id: &str,
        max_agents: u32,
    ) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::agent_capacity_limits as t;
        let tenant = tenant_id.to_string();
        let max = max_agents as i32;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            diesel::insert_into(t::table)
                .values((t::tenant_id.eq(&tenant), t::max_agents.eq(max)))
                .on_conflict(t::tenant_id)
                .do_update()
                .set(t::max_agents.eq(max))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }

    /// The tenant's exception, if one is set (`None` → the default applies).
    #[cfg(feature = "postgres")]
    pub async fn get_tenant_limit(&self, tenant_id: &str) -> Result<Option<u32>, ValidationError> {
        use crate::database::schema::postgres::agent_capacity_limits as t;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let tenant = tenant_id.to_string();
        let row: Option<AgentLimitRow> = conn
            .interact(move |conn| t::table.find(tenant).first(conn).optional())
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(row.map(|r| r.max_agents.max(0) as u32))
    }

    /// Remove a tenant's exception (revert to the default). Returns true if a row
    /// was removed.
    #[cfg(feature = "postgres")]
    pub async fn clear_tenant_limit(&self, tenant_id: &str) -> Result<bool, ValidationError> {
        use crate::database::schema::postgres::agent_capacity_limits as t;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let tenant = tenant_id.to_string();
        let n: usize = conn
            .interact(move |conn| diesel::delete(t::table.find(tenant)).execute(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n > 0)
    }

    /// The effective limit for a tenant: the exception if set, else `default`.
    /// This is the value the provision API and autoscaler enforce.
    #[cfg(feature = "postgres")]
    pub async fn effective_limit(
        &self,
        tenant_id: &str,
        default: u32,
    ) -> Result<u32, ValidationError> {
        Ok(self.get_tenant_limit(tenant_id).await?.unwrap_or(default))
    }
}
