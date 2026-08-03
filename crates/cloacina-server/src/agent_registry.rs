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

//! Execution-agent fleet roster (CLOACI-T-0631, DB-backed in CLOACI-T-0916).
//!
//! The roster itself lives in the `fleet_agents` table (admin schema), so
//! register/heartbeat state, same-tenant selection, capacity views and
//! dead-agent reclaim are correct across server replicas behind a non-affine
//! load balancer — the per-replica in-memory map it replaces made heartbeats
//! flap and reclaim fire against agents alive on another replica. Reads are
//! heartbeat-recency-filtered ([`AgentRegistry::live_agents`]).
//!
//! Two things deliberately stay replica-local:
//!
//! - **A cached roster snapshot** ([`AgentRegistry::cached_agents`]) for the
//!   sync `TaskExecutor::has_capacity()` / `metrics()` calls, refreshed on
//!   every async roster touch and on each fleet-sweeper tick — never used for
//!   selection or reclaim decisions.
//! - **The one-time ephemeral secret-key pools** (CLOACI-T-0861 / D-5). A
//!   pooled key must be consumed exactly once, and the pool is seeded/topped
//!   up by the replica that handles the agent's `register`/`keys` call. The
//!   heartbeat replenish signal reports the LOCAL pool's deficit, so an agent
//!   heartbeating through several replicas establishes a pool on each of them
//!   over time; until the dispatching replica has one, a secret-bearing
//!   dispatch fails CLOSED (clean failure + retry — never plaintext, never a
//!   reused key). This is the documented residual of T-0916.

use std::collections::HashMap;
use std::sync::{Mutex, RwLock};
use std::time::Duration;

use cloacina::dal::unified::{FleetAgent, FleetAgentRegistration};
use cloacina::error::ValidationError;
use cloacina::fleet::EphemeralKeyEntry;
use cloacina::security::ServerKeyPool;

pub struct AgentRegistry {
    /// Admin (public-schema) database — the roster is server-global, like the
    /// delivery outbox.
    dal: cloacina::dal::DAL,
    /// Heartbeat-recency window for "live": heartbeat interval × allowed
    /// misses (CLOACI-T-0639).
    liveness_timeout: Duration,
    /// Per-replica one-time key pools (see module docs).
    key_pools: Mutex<HashMap<String, ServerKeyPool>>,
    /// Last-known live roster, for the sync `has_capacity()`/`metrics()` path.
    cache: RwLock<Vec<FleetAgent>>,
}

impl AgentRegistry {
    pub fn new(database: cloacina::database::Database, liveness_timeout: Duration) -> Self {
        Self {
            dal: cloacina::dal::DAL::new(database),
            liveness_timeout: liveness_timeout.max(Duration::from_secs(1)),
            key_pools: Mutex::new(HashMap::new()),
            cache: RwLock::new(Vec::new()),
        }
    }

    /// The heartbeat-recency window that defines "live" for every roster read.
    pub fn liveness_timeout(&self) -> Duration {
        self.liveness_timeout
    }

    // ── roster (DB-backed, multi-replica) ──────────────────────────────────

    /// Insert or overwrite an agent's registration (overwrite handles clean
    /// re-registration after an agent restart with the same `agent_id`), and
    /// seed this replica's one-time key pool for it wholesale.
    pub async fn register(
        &self,
        reg: FleetAgentRegistration,
        key_pool: ServerKeyPool,
    ) -> Result<(), ValidationError> {
        let agent_id = reg.agent_id.clone();
        self.dal.fleet_agents().upsert_registration(reg).await?;
        {
            let mut pools = self.key_pools.lock().unwrap_or_else(|e| e.into_inner());
            pools.insert(agent_id, key_pool);
        }
        self.refresh_cache().await;
        Ok(())
    }

    /// Record a heartbeat (capacity + recency). Returns `true` iff the agent
    /// is in the roster — on ANY replica; `false` means it must re-register.
    pub async fn record_heartbeat(
        &self,
        agent_id: &str,
        in_flight: u32,
        available_capacity: u32,
    ) -> Result<bool, ValidationError> {
        let known = self
            .dal
            .fleet_agents()
            .record_heartbeat(agent_id, in_flight, available_capacity)
            .await?;
        if known {
            self.refresh_cache().await;
        }
        Ok(known)
    }

    /// The tenant an agent registered under, if it's in the roster (on any
    /// replica). `Some(tenant)` when registered (the tenant may itself be
    /// `None` for a global agent); `None` when the agent is unknown. Backs the
    /// caller-tenant guard on `heartbeat` / `result` (CLOACI-T-0785).
    pub async fn agent_tenant(
        &self,
        agent_id: &str,
    ) -> Result<Option<Option<String>>, ValidationError> {
        self.dal.fleet_agents().agent_tenant(agent_id).await
    }

    /// The live fleet (heartbeat within `liveness_timeout`), read from the
    /// shared table — the source of truth for selection, capacity views,
    /// operator listings and autoscaler utilization. Also refreshes the
    /// replica-local cache.
    pub async fn live_agents(&self) -> Result<Vec<FleetAgent>, ValidationError> {
        let live = self
            .dal
            .fleet_agents()
            .list_live(self.liveness_timeout)
            .await?;
        if let Ok(mut c) = self.cache.write() {
            *c = live.clone();
        }
        Ok(live)
    }

    /// Replica-local snapshot of the last roster read, for the sync
    /// `TaskExecutor::has_capacity()` / `metrics()` calls only. Refreshed by
    /// every async roster touch and each fleet-sweeper tick; may lag the DB by
    /// up to one heartbeat interval. Never used for selection or reclaim.
    pub fn cached_agents(&self) -> Vec<FleetAgent> {
        self.cache
            .read()
            .map(|c| c.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
    }

    /// Re-read the live roster into the local cache (best-effort).
    pub async fn refresh_cache(&self) {
        if let Ok(live) = self
            .dal
            .fleet_agents()
            .list_live(self.liveness_timeout)
            .await
        {
            if let Ok(mut c) = self.cache.write() {
                *c = live;
            }
        }
    }

    /// Evict agents whose heartbeat is older than `liveness_timeout`,
    /// returning the records THIS replica evicted (per-row compare-and-set
    /// delete in the DAL, so concurrent sweepers on other replicas never
    /// double-own a reclaim). Also drops any local key pool for the evicted
    /// agents and refreshes the cache.
    pub async fn sweep_dead(&self) -> Result<Vec<FleetAgent>, ValidationError> {
        let dead = self
            .dal
            .fleet_agents()
            .sweep_dead(self.liveness_timeout)
            .await?;
        if !dead.is_empty() {
            let mut pools = self.key_pools.lock().unwrap_or_else(|e| e.into_inner());
            for rec in &dead {
                pools.remove(&rec.agent_id);
            }
        }
        self.refresh_cache().await;
        Ok(dead)
    }

    /// Remove an agent from the roster (and this replica's key pool for it).
    /// Idempotent.
    pub async fn deregister(&self, agent_id: &str) -> Result<(), ValidationError> {
        self.dal.fleet_agents().delete(agent_id).await?;
        let mut pools = self.key_pools.lock().unwrap_or_else(|e| e.into_inner());
        pools.remove(agent_id);
        Ok(())
    }

    // ── one-time key pools (replica-local — see module docs) ───────────────

    /// CLOACI-T-0861 / D-5 — consume exactly ONE unused one-time key from the
    /// agent's LOCAL pool (FIFO), removing it so it can never be handed out
    /// again. Returns `None` if this replica holds no pool for the agent OR
    /// the pool is exhausted — the caller MUST then fail the dispatch cleanly
    /// (never plaintext, never reuse).
    pub fn consume_secret_key(&self, agent_id: &str) -> Option<EphemeralKeyEntry> {
        let mut pools = self.key_pools.lock().unwrap_or_else(|e| e.into_inner());
        pools.get_mut(agent_id)?.consume()
    }

    /// CLOACI-T-0861 / D-5 — append fresh one-time keys to this replica's pool
    /// for the agent (creating the pool if this replica has none yet — that is
    /// how pools self-establish on every replica the agent talks to). Returns
    /// the number accepted (de-duped by key_id). The caller is responsible for
    /// verifying the agent is registered (in the DB roster) first.
    pub fn replenish_secret_keys(&self, agent_id: &str, entries: Vec<EphemeralKeyEntry>) -> usize {
        let mut pools = self.key_pools.lock().unwrap_or_else(|e| e.into_inner());
        pools
            .entry(agent_id.to_string())
            .or_insert_with(ServerKeyPool::new)
            .replenish(entries)
    }

    /// CLOACI-T-0861 / D-5 — how many more keys the agent should top up to
    /// reach `target` on THIS replica (a replica with no pool yet reports the
    /// full target, which is what seeds it). Backs the heartbeat replenish
    /// signal.
    pub fn key_pool_deficit(&self, agent_id: &str, target: usize) -> usize {
        let pools = self.key_pools.lock().unwrap_or_else(|e| e.into_inner());
        pools
            .get(agent_id)
            .map(|p| p.replenish_deficit(target))
            .unwrap_or(target)
    }
}

/// Select a live fleet agent for a task: same tenant as the work, with spare
/// capacity, AND a target triple this package has a cdylib for. Greedy on
/// most-free-capacity so load spreads.
///
/// Tenant isolation (REQ-008) lives here: `a.tenant_id == *task_tenant` is the
/// only cross-tenant gate, so an agent only ever receives work in its own
/// tenant scope. CLOACI-T-0817: "public" is a real tenant — public work
/// carries `Some("public")` and matches only `Some("public")` agents.
/// `runnable_triples == None` means "any arch is fine" (interpreted package,
/// e.g. Python).
pub fn select_fleet_agent<'a>(
    live: &'a [FleetAgent],
    task_tenant: &Option<String>,
    runnable_triples: &Option<Vec<String>>,
) -> Option<&'a FleetAgent> {
    live.iter()
        .filter(|a| {
            a.available_capacity > 0
                && &a.tenant_id == task_tenant
                && runnable_triples
                    .as_ref()
                    .map_or(true, |ts| ts.iter().any(|t| t == &a.target_triple))
        })
        .max_by_key(|a| a.available_capacity)
}

#[cfg(test)]
pub(crate) fn test_fleet_agent(id: &str, cap: u32, tenant: Option<&str>) -> FleetAgent {
    FleetAgent {
        agent_id: id.to_string(),
        tenant_id: tenant.map(str::to_string),
        target_triple: "aarch64-apple-darwin".to_string(),
        capabilities: vec![],
        max_concurrency: cap,
        in_flight: 0,
        available_capacity: cap,
        registered_at: cloacina::database::universal_types::UniversalTimestamp::now(),
        last_heartbeat_at: cloacina::database::universal_types::UniversalTimestamp::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── replica-local key pools (the deliberate residual) ────────────────────

    fn pool_registry() -> AgentRegistry {
        // The DB is never touched by the pool paths; any URL works unconnected.
        AgentRegistry::new(
            cloacina::database::Database::new(
                "postgres://cloacina:cloacina@localhost:15432/cloacina",
                "cloacina",
                1,
            ),
            Duration::from_secs(60),
        )
    }

    fn entries(keys: &[&str]) -> Vec<EphemeralKeyEntry> {
        keys.iter()
            .map(|k| EphemeralKeyEntry {
                key_id: k.to_string(),
                public_key_b64: "AAAA".to_string(),
            })
            .collect()
    }

    /// D-5: each dispatch consumes a distinct key; exhaustion yields None; a
    /// replenish restores capacity.
    #[test]
    fn consume_secret_key_is_one_time_then_exhausts_and_replenishes() {
        let r = pool_registry();
        assert_eq!(r.replenish_secret_keys("a1", entries(&["k1", "k2"])), 2);

        let first = r.consume_secret_key("a1").expect("k1");
        let second = r.consume_secret_key("a1").expect("k2");
        assert_ne!(
            first.key_id, second.key_id,
            "each dispatch spends a fresh key"
        );
        assert!(
            r.consume_secret_key("a1").is_none(),
            "exhausted pool yields no key"
        );
        assert_eq!(r.key_pool_deficit("a1", 4), 4);

        assert_eq!(r.replenish_secret_keys("a1", entries(&["k3"])), 1);
        assert_eq!(r.consume_secret_key("a1").unwrap().key_id, "k3");
    }

    /// A replica that has never seen the agent's register/keys call holds no
    /// pool: consumption fails CLOSED, and the heartbeat replenish signal
    /// reports the FULL deficit so the agent seeds a pool here.
    #[test]
    fn poolless_replica_fails_closed_and_requests_full_target() {
        let r = pool_registry();
        assert!(r.consume_secret_key("elsewhere").is_none());
        assert_eq!(
            r.key_pool_deficit("elsewhere", 8),
            8,
            "a poolless replica must ask the agent for a full top-up"
        );
        // The top-up creates the local pool (self-healing across replicas).
        assert_eq!(r.replenish_secret_keys("elsewhere", entries(&["k1"])), 1);
        assert!(r.consume_secret_key("elsewhere").is_some());
    }

    // ── selection over the live (DB-read) roster ─────────────────────────────

    use super::test_fleet_agent as agent;

    /// CLOACI-T-0817: public-namespace work selects a `Some("public")` agent —
    /// never a named tenant's agent nor the bootstrap/admin `None` agent.
    #[test]
    fn public_work_selects_a_public_agent_only() {
        let roster = vec![
            agent("acme-1", 16, Some("acme")),
            agent("public-1", 8, Some("public")),
            agent("admin-1", 32, None),
        ];
        let chosen = select_fleet_agent(&roster, &Some("public".to_string()), &None)
            .expect("a public agent must be selectable for public work");
        assert_eq!(chosen.agent_id, "public-1");
    }

    /// A named tenant's work matches only that tenant's agents — isolation is
    /// preserved in both directions.
    #[test]
    fn named_tenant_work_is_isolated_from_public_and_others() {
        let roster = vec![
            agent("public-1", 32, Some("public")),
            agent("acme-1", 4, Some("acme")),
            agent("beta-1", 64, Some("beta")),
        ];
        let chosen = select_fleet_agent(&roster, &Some("acme".to_string()), &None)
            .expect("acme work must select the acme agent");
        assert_eq!(chosen.agent_id, "acme-1");

        assert!(
            select_fleet_agent(&roster, &Some("gamma".to_string()), &None).is_none(),
            "work for a tenant with no agents must not leak onto another tenant"
        );
    }

    #[test]
    fn public_work_does_not_select_a_none_tenant_agent() {
        let roster = vec![agent("admin-1", 32, None)];
        assert!(
            select_fleet_agent(&roster, &Some("public".to_string()), &None).is_none(),
            "a None-tenant (bootstrap/admin) agent must not serve public work"
        );
    }

    #[test]
    fn selection_skips_agents_without_capacity_or_runnable_arch() {
        let mut full = agent("full", 4, Some("t1"));
        full.available_capacity = 0;
        let mut wrong_arch = agent("wrong-arch", 4, Some("t1"));
        wrong_arch.target_triple = "x86_64-unknown-linux".to_string();
        let ok = agent("ok", 2, Some("t1"));
        let roster = vec![full, wrong_arch, ok];

        let triples = Some(vec!["aarch64-apple-darwin".to_string()]);
        let chosen = select_fleet_agent(&roster, &Some("t1".to_string()), &triples).unwrap();
        assert_eq!(chosen.agent_id, "ok");
    }
}
