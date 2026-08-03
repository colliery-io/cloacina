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

//! Unified fleet-agent roster DAL (CLOACI-T-0916).
//!
//! Persists execution-agent register/heartbeat state so every
//! cross-replica-relevant read — same-tenant selection, capacity views,
//! dead-agent reclaim eligibility — comes from ONE shared table instead of a
//! per-replica in-memory map. Liveness is heartbeat-recency: readers filter on
//! `last_heartbeat_at >= now - timeout` via [`FleetAgentDAL::list_live`].
//!
//! Dead-agent eviction ([`FleetAgentDAL::sweep_dead`]) claims each stale row
//! with a compare-and-set DELETE (`WHERE agent_id = ? AND last_heartbeat_at <
//! cutoff`), so when several replicas sweep concurrently exactly one owns each
//! evicted agent's reclaim.
//!
//! Work-packet dispatch needs no connection locality: packets ride the
//! delivery-outbox substrate and are delivered by whichever replica holds the
//! agent's delivery WebSocket (connection-ownership routing, A-0006).

use super::models::{NewUnifiedFleetAgent, UnifiedFleetAgent};
use super::DAL;
use crate::database::schema::unified::fleet_agents;
use crate::database::universal_types::UniversalTimestamp;
use crate::error::ValidationError;
use diesel::prelude::*;

/// Domain view of a roster row.
#[derive(Debug, Clone)]
pub struct FleetAgent {
    pub agent_id: String,
    pub tenant_id: Option<String>,
    pub target_triple: String,
    pub capabilities: Vec<String>,
    pub max_concurrency: u32,
    pub in_flight: u32,
    pub available_capacity: u32,
    pub registered_at: UniversalTimestamp,
    pub last_heartbeat_at: UniversalTimestamp,
}

/// Registration input (the fields an agent supplies on `POST /v1/agent/register`).
#[derive(Debug, Clone)]
pub struct FleetAgentRegistration {
    pub agent_id: String,
    pub tenant_id: Option<String>,
    pub target_triple: String,
    pub capabilities: Vec<String>,
    pub max_concurrency: u32,
}

fn to_domain(r: UnifiedFleetAgent) -> FleetAgent {
    FleetAgent {
        agent_id: r.agent_id,
        tenant_id: r.tenant_id,
        target_triple: r.target_triple,
        capabilities: serde_json::from_str(&r.capabilities).unwrap_or_default(),
        max_concurrency: r.max_concurrency.max(0) as u32,
        in_flight: r.in_flight.max(0) as u32,
        available_capacity: r.available_capacity.max(0) as u32,
        registered_at: r.registered_at,
        last_heartbeat_at: r.last_heartbeat_at,
    }
}

fn cutoff_for(timeout: std::time::Duration) -> UniversalTimestamp {
    let now = UniversalTimestamp::now();
    UniversalTimestamp(
        now.0
            - chrono::Duration::from_std(timeout).unwrap_or_else(|_| chrono::Duration::seconds(0)),
    )
}

/// Data access layer for the execution-agent fleet roster.
#[derive(Clone)]
pub struct FleetAgentDAL<'a> {
    dal: &'a DAL,
}

impl<'a> FleetAgentDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Insert or overwrite an agent's registration (overwrite handles clean
    /// re-registration after an agent restart with the same `agent_id`).
    /// Registration counts as a heartbeat.
    pub async fn upsert_registration(
        &self,
        reg: FleetAgentRegistration,
    ) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        let row = NewUnifiedFleetAgent {
            agent_id: reg.agent_id,
            tenant_id: reg.tenant_id,
            target_triple: reg.target_triple,
            capabilities: serde_json::to_string(&reg.capabilities)
                .unwrap_or_else(|_| "[]".to_string()),
            max_concurrency: reg.max_concurrency.min(i32::MAX as u32) as i32,
            in_flight: 0,
            available_capacity: reg.max_concurrency.min(i32::MAX as u32) as i32,
            registered_at: now,
            last_heartbeat_at: now,
        };
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(fleet_agents::table)
                .values(&row)
                .on_conflict(fleet_agents::agent_id)
                .do_update()
                .set(&row)
                .execute(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(())
    }

    /// Record a heartbeat: refresh `last_heartbeat_at` and the capacity
    /// fields. Returns `true` iff the agent is in the roster (`false` ⇒ the
    /// caller should tell the agent to re-register).
    pub async fn record_heartbeat(
        &self,
        agent_id: &str,
        in_flight: u32,
        available_capacity: u32,
    ) -> Result<bool, ValidationError> {
        let agent_id = agent_id.to_string();
        let now = UniversalTimestamp::now();
        let affected = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(fleet_agents::table.filter(fleet_agents::agent_id.eq(&agent_id)))
                .set((
                    fleet_agents::in_flight.eq(in_flight.min(i32::MAX as u32) as i32),
                    fleet_agents::available_capacity
                        .eq(available_capacity.min(i32::MAX as u32) as i32),
                    fleet_agents::last_heartbeat_at.eq(now),
                ))
                .execute(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(affected == 1)
    }

    /// The tenant an agent registered under, if it's in the roster.
    /// `Some(tenant)` when registered (the tenant may itself be `None` for a
    /// global agent); `None` when the agent is unknown.
    pub async fn agent_tenant(
        &self,
        agent_id: &str,
    ) -> Result<Option<Option<String>>, ValidationError> {
        let agent_id = agent_id.to_string();
        let row: Option<Option<String>> = crate::interact_on_backend!(self.dal, |conn| {
            fleet_agents::table
                .filter(fleet_agents::agent_id.eq(&agent_id))
                .select(fleet_agents::tenant_id)
                .first::<Option<String>>(conn)
                .optional()
        })
        .map_err(ValidationError::from)?;
        Ok(row)
    }

    /// All agents whose heartbeat is fresher than `timeout` — the live fleet.
    /// Every cross-replica-relevant read (selection, capacity, operator
    /// views, autoscaler utilization) goes through this recency filter.
    pub async fn list_live(
        &self,
        timeout: std::time::Duration,
    ) -> Result<Vec<FleetAgent>, ValidationError> {
        let cutoff = cutoff_for(timeout);
        let rows: Vec<UnifiedFleetAgent> = crate::interact_on_backend!(self.dal, |conn| {
            fleet_agents::table
                .filter(fleet_agents::last_heartbeat_at.ge(cutoff))
                .order(fleet_agents::agent_id.asc())
                .load(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(rows.into_iter().map(to_domain).collect())
    }

    /// Evict agents whose heartbeat is older than `timeout`, returning the
    /// rows THIS caller evicted. Per-row compare-and-set delete: when several
    /// replicas sweep concurrently, each stale agent is owned by exactly one
    /// sweeper — so dead-agent reclaim never double-fires.
    pub async fn sweep_dead(
        &self,
        timeout: std::time::Duration,
    ) -> Result<Vec<FleetAgent>, ValidationError> {
        let cutoff = cutoff_for(timeout);
        let rows: Vec<UnifiedFleetAgent> = crate::interact_on_backend!(self.dal, |conn| {
            let stale: Vec<UnifiedFleetAgent> = fleet_agents::table
                .filter(fleet_agents::last_heartbeat_at.lt(cutoff))
                .load(conn)?;
            let mut owned = Vec::with_capacity(stale.len());
            for row in stale {
                // CAS delete: 1 row affected ⇒ we own this eviction (a
                // concurrent sweeper — or a last-instant heartbeat — makes
                // this 0 and we skip).
                let affected = diesel::delete(
                    fleet_agents::table
                        .filter(fleet_agents::agent_id.eq(&row.agent_id))
                        .filter(fleet_agents::last_heartbeat_at.lt(cutoff)),
                )
                .execute(conn)?;
                if affected == 1 {
                    owned.push(row);
                }
            }
            Ok::<_, diesel::result::Error>(owned)
        })
        .map_err(ValidationError::from)?;
        Ok(rows.into_iter().map(to_domain).collect())
    }

    /// Remove an agent from the roster. Idempotent.
    pub async fn delete(&self, agent_id: &str) -> Result<(), ValidationError> {
        let agent_id = agent_id.to_string();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(fleet_agents::table.filter(fleet_agents::agent_id.eq(&agent_id)))
                .execute(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use std::time::Duration;

    #[cfg(feature = "sqlite")]
    fn shared_url() -> String {
        format!(
            "file:fleet_agents_test_{}?mode=memory&cache=shared",
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
    fn reg(id: &str, tenant: Option<&str>, cap: u32) -> FleetAgentRegistration {
        FleetAgentRegistration {
            agent_id: id.to_string(),
            tenant_id: tenant.map(str::to_string),
            target_triple: "aarch64-apple-darwin".to_string(),
            capabilities: vec!["python".to_string()],
            max_concurrency: cap,
        }
    }

    /// The multi-replica contract: an agent registering + heartbeating through
    /// one DAL handle (replica A) is visible — with fresh capacity — through a
    /// DIFFERENT DAL handle over the same database (replica B).
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn roster_visible_across_dal_handles() {
        let url = shared_url();
        let dal_a = dal_for(&url).await;
        let dal_b = DAL::new(Database::new(&url, "", 5));

        dal_a
            .fleet_agents()
            .upsert_registration(reg("a1", Some("acme"), 4))
            .await
            .unwrap();

        // Replica B sees the live agent + its tenant.
        let live = dal_b
            .fleet_agents()
            .list_live(Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].agent_id, "a1");
        assert_eq!(live[0].tenant_id.as_deref(), Some("acme"));
        assert_eq!(live[0].available_capacity, 4);
        assert_eq!(
            dal_b.fleet_agents().agent_tenant("a1").await.unwrap(),
            Some(Some("acme".to_string()))
        );

        // A heartbeat landing on replica B does NOT flap ("not registered").
        assert!(dal_b
            .fleet_agents()
            .record_heartbeat("a1", 3, 1)
            .await
            .unwrap());
        let live = dal_a
            .fleet_agents()
            .list_live(Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(live[0].in_flight, 3);
        assert_eq!(live[0].available_capacity, 1);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn heartbeat_on_unknown_agent_reports_not_registered() {
        let dal = dal_for(&shared_url()).await;
        assert!(!dal
            .fleet_agents()
            .record_heartbeat("ghost", 0, 0)
            .await
            .unwrap());
        assert_eq!(
            dal.fleet_agents().agent_tenant("ghost").await.unwrap(),
            None
        );
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn reregistration_overwrites() {
        let dal = dal_for(&shared_url()).await;
        dal.fleet_agents()
            .upsert_registration(reg("a1", None, 1))
            .await
            .unwrap();
        dal.fleet_agents()
            .upsert_registration(reg("a1", Some("t1"), 8))
            .await
            .unwrap();
        let live = dal
            .fleet_agents()
            .list_live(Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].max_concurrency, 8);
        assert_eq!(live[0].tenant_id.as_deref(), Some("t1"));
    }

    /// Recency filtering + sweep ownership: a zero-timeout cutoff makes every
    /// row stale; the sweeper evicts and OWNS it exactly once.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn sweep_dead_evicts_stale_rows_once() {
        let url = shared_url();
        let dal_a = dal_for(&url).await;
        let dal_b = DAL::new(Database::new(&url, "", 5));
        dal_a
            .fleet_agents()
            .upsert_registration(reg("stale", Some("t1"), 2))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Fresh-enough timeout: nothing evicted, agent still live.
        assert!(dal_b
            .fleet_agents()
            .sweep_dead(Duration::from_secs(60))
            .await
            .unwrap()
            .is_empty());

        // Zero timeout: the sweep on replica B evicts the agent registered on A.
        let dead = dal_b
            .fleet_agents()
            .sweep_dead(Duration::ZERO)
            .await
            .unwrap();
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].agent_id, "stale");
        assert_eq!(dead[0].tenant_id.as_deref(), Some("t1"));

        // Second sweep (any replica) owns nothing — reclaim can't double-fire.
        assert!(dal_a
            .fleet_agents()
            .sweep_dead(Duration::ZERO)
            .await
            .unwrap()
            .is_empty());
        assert!(dal_a
            .fleet_agents()
            .list_live(Duration::from_secs(60))
            .await
            .unwrap()
            .is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn delete_is_idempotent() {
        let dal = dal_for(&shared_url()).await;
        dal.fleet_agents()
            .upsert_registration(reg("a1", None, 1))
            .await
            .unwrap();
        dal.fleet_agents().delete("a1").await.unwrap();
        dal.fleet_agents().delete("a1").await.unwrap();
        assert!(dal
            .fleet_agents()
            .list_live(Duration::from_secs(60))
            .await
            .unwrap()
            .is_empty());
    }
}
