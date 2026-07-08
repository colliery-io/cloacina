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

//! In-memory roster of registered execution agents (CLOACI-T-0631).
//!
//! Tracks the live fleet for the `FleetExecutor` (T-0633) to consult during
//! capacity-aware selection. Per-replica: the roster is local to whichever
//! `cloacina-server` instance the agent registered against. Heartbeat
//! liveness sweeping lands in T-0634.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cloacina::fleet::EphemeralKeyEntry;
use cloacina::security::ServerKeyPool;

/// Snapshot of a registered agent.
#[derive(Debug, Clone)]
pub struct AgentRecord {
    pub agent_id: String,
    pub max_concurrency: u32,
    pub in_flight: u32,
    pub available_capacity: u32,
    pub target_triple: String,
    pub capabilities: Vec<String>,
    pub last_heartbeat: Instant,
    /// Tenant scope inherited from the authenticated API key that registered
    /// this agent. The `FleetExecutor` only assigns work whose tenant matches.
    pub tenant_id: Option<String>,
    /// CLOACI-T-0861 / D-5 — the agent's unused one-time ephemeral key pool. The
    /// `FleetExecutor` consumes exactly one key per secret-bearing dispatch (via
    /// [`AgentRegistry::consume_secret_key`], which mutates the LIVE record — a
    /// `snapshot()` clone is read-only and never the consumption source). The
    /// agent tops it up via `POST /v1/agent/keys` when the pool runs low.
    pub key_pool: ServerKeyPool,
}

#[derive(Default)]
pub struct AgentRegistry {
    by_id: Mutex<HashMap<String, AgentRecord>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or overwrite an entry (overwrite handles re-registration cleanly
    /// after an agent restart with the same `agent_id`).
    pub fn register(&self, record: AgentRecord) {
        let mut g = self.by_id.lock().unwrap_or_else(|e| e.into_inner());
        g.insert(record.agent_id.clone(), record);
    }

    /// Update an existing entry's heartbeat fields. Returns `true` if the
    /// agent is in the roster; `false` if the server should reject (the
    /// agent likely needs to re-register).
    pub fn record_heartbeat(
        &self,
        agent_id: &str,
        in_flight: u32,
        available_capacity: u32,
    ) -> bool {
        let mut g = self.by_id.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(r) = g.get_mut(agent_id) {
            r.in_flight = in_flight;
            r.available_capacity = available_capacity;
            r.last_heartbeat = Instant::now();
            true
        } else {
            false
        }
    }

    /// CLOACI-T-0785: the tenant an agent registered under, if it's in the
    /// roster. `Some(tenant)` when registered (the tenant may itself be `None`
    /// for a global agent); `None` when the agent is unknown. Backs the
    /// caller-tenant guard on `heartbeat` / `result`.
    pub fn agent_tenant(&self, agent_id: &str) -> Option<Option<String>> {
        let g = self.by_id.lock().unwrap_or_else(|e| e.into_inner());
        g.get(agent_id).map(|r| r.tenant_id.clone())
    }

    /// CLOACI-T-0861 / D-5 — consume exactly ONE unused one-time key from the
    /// agent's pool (FIFO), removing it so it can never be handed out again.
    /// Returns `None` if the agent is unknown OR its pool is exhausted — the
    /// caller MUST then fail the dispatch cleanly (never plaintext, never reuse).
    pub fn consume_secret_key(&self, agent_id: &str) -> Option<EphemeralKeyEntry> {
        let mut g = self.by_id.lock().unwrap_or_else(|e| e.into_inner());
        g.get_mut(agent_id)?.key_pool.consume()
    }

    /// CLOACI-T-0861 / D-5 — append fresh one-time keys to the agent's pool (a
    /// replenish top-up). Returns the number accepted (de-duped by key_id), or
    /// `None` if the agent is unknown (it should re-register).
    pub fn replenish_secret_keys(
        &self,
        agent_id: &str,
        entries: Vec<EphemeralKeyEntry>,
    ) -> Option<usize> {
        let mut g = self.by_id.lock().unwrap_or_else(|e| e.into_inner());
        g.get_mut(agent_id).map(|r| r.key_pool.replenish(entries))
    }

    /// CLOACI-T-0861 / D-5 — how many more keys the agent should top up to reach
    /// `target` (0 if healthy / unknown). Backs the heartbeat replenish signal.
    pub fn key_pool_deficit(&self, agent_id: &str, target: usize) -> usize {
        let g = self.by_id.lock().unwrap_or_else(|e| e.into_inner());
        g.get(agent_id)
            .map(|r| r.key_pool.replenish_deficit(target))
            .unwrap_or(0)
    }

    /// Remove an entry. Idempotent.
    pub fn deregister(&self, agent_id: &str) {
        let mut g = self.by_id.lock().unwrap_or_else(|e| e.into_inner());
        g.remove(agent_id);
    }

    /// Remove agents whose last heartbeat is older than `timeout` and return
    /// the evicted records (CLOACI-T-0634). Returning full `AgentRecord`s (not
    /// just ids) lets the reclaim path match a dead agent's tenant when
    /// re-targeting its in-flight work to a live agent (so reclaim respects
    /// tenant isolation, REQ-008). Eviction itself is roster hygiene so
    /// selection + `has_capacity()` stop counting a dead agent.
    ///
    /// `timeout` should be a small multiple of the heartbeat interval (e.g.
    /// 3×) so a single missed beat doesn't evict a healthy agent.
    pub fn sweep_dead(&self, timeout: Duration) -> Vec<AgentRecord> {
        let now = Instant::now();
        let mut g = self.by_id.lock().unwrap_or_else(|e| e.into_inner());
        let dead_ids: Vec<String> = g
            .iter()
            .filter(|(_, r)| now.duration_since(r.last_heartbeat) > timeout)
            .map(|(id, _)| id.clone())
            .collect();
        let mut dead = Vec::with_capacity(dead_ids.len());
        for id in &dead_ids {
            if let Some(rec) = g.remove(id) {
                dead.push(rec);
            }
        }
        dead
    }

    /// Snapshot the current roster. Used by `FleetExecutor` capacity
    /// selection (T-0633) and by debug/metrics views.
    pub fn snapshot(&self) -> Vec<AgentRecord> {
        let g = self.by_id.lock().unwrap_or_else(|e| e.into_inner());
        g.values().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.by_id.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str, cap: u32, tenant: Option<&str>) -> AgentRecord {
        AgentRecord {
            agent_id: id.to_string(),
            max_concurrency: cap,
            in_flight: 0,
            available_capacity: cap,
            target_triple: "aarch64-apple-darwin".to_string(),
            capabilities: vec![],
            last_heartbeat: Instant::now(),
            tenant_id: tenant.map(|s| s.to_string()),
            key_pool: ServerKeyPool::new(),
        }
    }

    fn rec_with_pool(id: &str, keys: &[&str]) -> AgentRecord {
        let mut r = rec(id, 4, Some("public"));
        r.key_pool = ServerKeyPool::from_entries(
            keys.iter()
                .map(|k| EphemeralKeyEntry {
                    key_id: k.to_string(),
                    public_key_b64: "AAAA".to_string(),
                })
                .collect(),
        );
        r
    }

    /// D-5: each dispatch consumes a distinct key; exhaustion yields None; a
    /// replenish restores capacity. Consumption mutates the LIVE record.
    #[test]
    fn consume_secret_key_is_one_time_then_exhausts_and_replenishes() {
        let r = AgentRegistry::new();
        r.register(rec_with_pool("a1", &["k1", "k2"]));

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

        // Top up; consumption works again.
        let added = r
            .replenish_secret_keys(
                "a1",
                vec![EphemeralKeyEntry {
                    key_id: "k3".into(),
                    public_key_b64: "AAAA".into(),
                }],
            )
            .expect("known agent");
        assert_eq!(added, 1);
        assert_eq!(r.consume_secret_key("a1").unwrap().key_id, "k3");
    }

    #[test]
    fn consume_and_replenish_on_unknown_agent() {
        let r = AgentRegistry::new();
        assert!(r.consume_secret_key("ghost").is_none());
        assert!(r.replenish_secret_keys("ghost", vec![]).is_none());
        assert_eq!(r.key_pool_deficit("ghost", 8), 0);
    }

    #[test]
    fn register_then_snapshot_roundtrips() {
        let r = AgentRegistry::new();
        r.register(rec("a1", 4, Some("t1")));
        // CLOACI-T-0817: public agents register as Some("public"), not None
        // (the None tenant now belongs solely to the admin/bootstrap key).
        r.register(rec("a2", 2, Some("public")));
        let snap = r.snapshot();
        assert_eq!(snap.len(), 2);
        assert!(snap
            .iter()
            .any(|x| x.agent_id == "a1" && x.max_concurrency == 4));
        assert!(snap
            .iter()
            .any(|x| x.agent_id == "a2" && x.tenant_id.as_deref() == Some("public")));
    }

    #[test]
    fn heartbeat_updates_capacity() {
        let r = AgentRegistry::new();
        r.register(rec("a1", 4, Some("public")));
        assert!(r.record_heartbeat("a1", 3, 1));
        let snap = r.snapshot();
        assert_eq!(snap[0].in_flight, 3);
        assert_eq!(snap[0].available_capacity, 1);
    }

    #[test]
    fn heartbeat_on_unknown_agent_returns_false() {
        let r = AgentRegistry::new();
        assert!(!r.record_heartbeat("never-registered", 0, 0));
    }

    #[test]
    fn sweep_dead_removes_only_stale_agents() {
        let r = AgentRegistry::new();
        // Fresh agent (heartbeat = now).
        r.register(rec("fresh", 1, Some("public")));
        // Stale agent: register, then backdate its last_heartbeat.
        r.register(rec("stale", 1, Some("public")));
        {
            let mut g = r.by_id.lock().unwrap();
            g.get_mut("stale").unwrap().last_heartbeat = Instant::now() - Duration::from_secs(120);
        }
        let removed = r.sweep_dead(Duration::from_secs(60));
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].agent_id, "stale");
        let snap = r.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].agent_id, "fresh");
    }

    #[test]
    fn sweep_dead_noop_when_all_fresh() {
        let r = AgentRegistry::new();
        r.register(rec("a1", 1, Some("public")));
        r.register(rec("a2", 1, Some("public")));
        let removed = r.sweep_dead(Duration::from_secs(60));
        assert!(removed.is_empty());
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn deregister_is_idempotent() {
        let r = AgentRegistry::new();
        r.register(rec("a1", 1, None));
        r.deregister("a1");
        r.deregister("a1"); // no panic, no error
        assert!(r.is_empty());
    }

    #[test]
    fn register_same_id_overwrites() {
        let r = AgentRegistry::new();
        r.register(rec("a1", 1, None));
        r.register(rec("a1", 8, Some("t1")));
        let snap = r.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].max_concurrency, 8);
        assert_eq!(snap[0].tenant_id.as_deref(), Some("t1"));
    }
}
