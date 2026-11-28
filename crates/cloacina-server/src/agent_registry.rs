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
use cloacina::security::ownership::{AgentOwner, Principal, Refusal};

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
    /// CLOACINA-T-1027: one placement at a time in this process. See
    /// [`AgentRegistry::placement_guard`].
    placement_lock: tokio::sync::Mutex<()>,
}

impl AgentRegistry {
    pub fn new(database: cloacina::database::Database, liveness_timeout: Duration) -> Self {
        Self {
            dal: cloacina::dal::DAL::new(database),
            liveness_timeout: liveness_timeout.max(Duration::from_secs(1)),
            key_pools: Mutex::new(HashMap::new()),
            cache: RwLock::new(Vec::new()),
            placement_lock: tokio::sync::Mutex::new(()),
        }
    }

    /// Hold this while a task is placed on an agent: from the read of the
    /// tasks in flight, through the selection, to the check after the
    /// placement was written (CLOACINA-T-1027).
    ///
    /// The scheduler dispatches many tasks at once. Without the guard they
    /// would all read the same load, pick the same agent, see each other's
    /// placements in the check and all withdraw. With it the placements of
    /// one server process go one after another, and each sees the ones
    /// before it. The check after the write is then only for another server
    /// replica, which this lock does not reach.
    pub async fn placement_guard(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.placement_lock.lock().await
    }

    /// The heartbeat-recency window that defines "live" for every roster read.
    pub fn liveness_timeout(&self) -> Duration {
        self.liveness_timeout
    }

    // ── roster (DB-backed, multi-replica) ──────────────────────────────────

    /// Register an agent for the key `by`, and seed this replica's one-time
    /// key pool for it wholesale. The key becomes the owner of the row
    /// (CLOACINA-T-1051). `Ok(Err(_))`: another tenant or another key holds
    /// the id; nothing is written and the key pool of the holder is kept.
    pub async fn register(
        &self,
        reg: FleetAgentRegistration,
        key_pool: ServerKeyPool,
        by: &Principal,
    ) -> Result<Result<(), Refusal>, ValidationError> {
        let agent_id = reg.agent_id.clone();
        if let Err(refusal) = self.dal.fleet_agents().register(reg, by).await? {
            return Ok(Err(refusal));
        }
        {
            let mut pools = self.key_pools.lock().unwrap_or_else(|e| e.into_inner());
            pools.insert(agent_id, key_pool);
        }
        self.refresh_cache().await;
        Ok(Ok(()))
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

    /// Who holds an agent id: the tenant and the owner key of its roster
    /// row, read from the shared table (so the answer is the same on every
    /// replica). `None` when the agent is unknown. Backs the ownership checks
    /// of the agent routes and of the delivery socket.
    pub async fn agent_owner(&self, agent_id: &str) -> Result<Option<AgentOwner>, ValidationError> {
        self.dal.fleet_agents().agent_owner(agent_id).await
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

/// What a task asks of an agent (CLOACINA-T-1027): its pool and its CPU and
/// memory requests. Built from the task's
/// [`TaskPlacement`](cloacina_api_types::TaskPlacement).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskDemand<'a> {
    /// The pool the task runs in.
    pub pool: &'a str,
    /// CPU request in millicores. `None` = the task declares none.
    pub cpu_millis: Option<u64>,
    /// Memory request in bytes. `None` = the task declares none.
    pub memory_bytes: Option<u64>,
}

impl<'a> TaskDemand<'a> {
    /// Work for `pool` with no request.
    pub fn pool(pool: &'a str) -> Self {
        Self {
            pool,
            cpu_millis: None,
            memory_bytes: None,
        }
    }

    /// The demand of a task with this placement.
    pub fn of(placement: &'a cloacina_api_types::TaskPlacement) -> Self {
        Self {
            pool: placement.pool_name(),
            cpu_millis: placement.cpu_millis(),
            memory_bytes: placement.memory_bytes(),
        }
    }
}

/// What is in flight on one agent by the server's own placements
/// (CLOACINA-T-1027): the number of tasks, and the sum of what they count for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AgentLoad {
    pub tasks: u32,
    pub cpu_millis: u64,
    pub memory_bytes: u64,
}

/// The load of each agent that has any, by agent id.
pub type FleetLoad = HashMap<String, AgentLoad>;

/// What a task counts for on an agent, in one resource (CLOACINA-T-1027).
///
/// - The agent reports no total for the resource: nothing. It is packed by
///   its slot count only.
/// - The task has a request: the request.
/// - The task has no request: one slot's share, the agent's total divided by
///   its max concurrency. So an agent full of tasks with no request is full
///   at exactly its slot count, as before requests existed.
pub fn charge(request: Option<u64>, total: Option<u64>, max_concurrency: u32) -> u64 {
    match (total, request) {
        (None, _) => 0,
        (Some(_), Some(request)) => request,
        (Some(total), None) => total / u64::from(max_concurrency.max(1)),
    }
}

/// Sum the tasks in flight (the placed task rows of
/// [`fleet_placements`](cloacina::dal::unified::task_execution::TaskExecutionDAL::fleet_placements))
/// into a load per agent. A placement on an agent that is not in `live` is
/// left out: the agent is gone.
pub fn fleet_load(
    live: &[FleetAgent],
    placements: &[cloacina::dal::unified::task_execution::FleetPlacement],
) -> FleetLoad {
    let mut load = FleetLoad::new();
    for placement in placements {
        let Some(agent) = live.iter().find(|a| a.agent_id == placement.agent_id) else {
            continue;
        };
        let entry = load.entry(agent.agent_id.clone()).or_default();
        entry.tasks += 1;
        entry.cpu_millis = entry.cpu_millis.saturating_add(charge(
            placement.cpu_request_millis,
            agent.cpu_millis,
            agent.max_concurrency,
        ));
        entry.memory_bytes = entry.memory_bytes.saturating_add(charge(
            placement.memory_request_bytes,
            agent.memory_bytes,
            agent.max_concurrency,
        ));
    }
    load
}

/// Free slots of an agent: what its last heartbeat reported, and never more
/// than its max concurrency less the tasks the server has placed on it. The
/// slot count is the upper cap whatever the requests say.
pub(crate) fn free_slots(agent: &FleetAgent, load: &AgentLoad) -> u32 {
    agent
        .available_capacity
        .min(agent.max_concurrency.saturating_sub(load.tasks))
}

/// Free amount of one resource on an agent. `None` = the agent reports no
/// total for it.
fn free_of(total: Option<u64>, used: u64) -> Option<u64> {
    total.map(|total| total.saturating_sub(used))
}

/// Does the task still fit on the agent: a free slot, and room for what the
/// task counts for in CPU and in memory.
pub fn fits(agent: &FleetAgent, load: &AgentLoad, demand: &TaskDemand<'_>) -> bool {
    if free_slots(agent, load) == 0 {
        return false;
    }
    let room = |request: Option<u64>, total: Option<u64>, used: u64| match free_of(total, used) {
        None => true,
        Some(free) => charge(request, total, agent.max_concurrency) <= free,
    };
    room(demand.cpu_millis, agent.cpu_millis, load.cpu_millis)
        && room(demand.memory_bytes, agent.memory_bytes, load.memory_bytes)
}

/// Is more placed on the agent than it can hold: more tasks than its max
/// concurrency, or more CPU or memory than its total.
///
/// This is the check after a placement was written (the race rule): a server
/// replica writes its placement, reads the load again, and takes the
/// placement back when this is true. Two replicas that picked the last free
/// memory of one agent at the same moment cannot both pass it.
pub fn over_committed(agent: &FleetAgent, load: &AgentLoad) -> bool {
    load.tasks > agent.max_concurrency
        || agent
            .cpu_millis
            .is_some_and(|total| load.cpu_millis > total)
        || agent
            .memory_bytes
            .is_some_and(|total| load.memory_bytes > total)
}

/// Can this agent run this package at all (not: does it have room now)?
///
/// `runnable_triples == None` means "any arch is fine" (an interpreted
/// package, e.g. Python). The one place that decides it, so that a further
/// rule about which agents can run a package has one home.
fn can_run_package(agent: &FleetAgent, runnable_triples: &Option<Vec<String>>) -> bool {
    runnable_triples
        .as_ref()
        .is_none_or(|ts| ts.iter().any(|t| t == &agent.target_triple))
}

/// Select a live fleet agent for a task: same tenant as the work, in the pool
/// the task names, able to run the package, with a free slot, and with room
/// for the task's CPU and memory requests.
///
/// Tenant isolation (REQ-008) lives here: `a.tenant_id == *task_tenant` is the
/// only cross-tenant gate, so an agent only ever receives work in its own
/// tenant scope. CLOACI-T-0817: "public" is a real tenant — public work
/// carries `Some("public")` and matches only `Some("public")` agents.
///
/// Pools (CLOACINA-T-1021): an agent is in the pool its `pool:<name>`
/// capability names, or in `default` when it has none
/// ([`cloacina_api_types::agent_pool_of`]). Work for a pool goes only to an
/// agent of that pool. A task with no pool is work for
/// [`cloacina_api_types::DEFAULT_AGENT_POOL`].
///
/// Packing (CLOACINA-T-1027): `load` is what the server has in flight on each
/// agent. The task must fit ([`fits`]). Among the agents where it fits, the
/// one with the least free memory wins, so that the agents with the most room
/// stay free for large tasks; then the one with the most free slots; then the
/// lowest agent id. An agent that reports no memory total ranks after every
/// agent that does. With no totals anywhere this is the rule from before
/// requests: the most free slots.
pub fn select_fleet_agent<'a>(
    live: &'a [FleetAgent],
    task_tenant: &Option<String>,
    runnable_triples: &Option<Vec<String>>,
    demand: &TaskDemand<'_>,
    load: &FleetLoad,
) -> Option<&'a FleetAgent> {
    let no_load = AgentLoad::default();
    live.iter()
        .filter(|a| {
            &a.tenant_id == task_tenant
                && cloacina_api_types::agent_pool_of(&a.capabilities) == demand.pool
                && can_run_package(a, runnable_triples)
        })
        .filter_map(|a| {
            let agent_load = load.get(&a.agent_id).unwrap_or(&no_load);
            fits(a, agent_load, demand).then(|| {
                let free_memory =
                    free_of(a.memory_bytes, agent_load.memory_bytes).unwrap_or(u64::MAX);
                let free = free_slots(a, agent_load);
                (
                    a,
                    (free_memory, std::cmp::Reverse(free), a.agent_id.as_str()),
                )
            })
        })
        .min_by(|(_, x), (_, y)| x.cmp(y))
        .map(|(a, _)| a)
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
        owner_key_id: None,
        cpu_millis: None,
        memory_bytes: None,
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
        let chosen = select_fleet_agent(
            &roster,
            &Some("public".to_string()),
            &None,
            &TaskDemand::pool("default"),
            &FleetLoad::new(),
        )
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
        let chosen = select_fleet_agent(
            &roster,
            &Some("acme".to_string()),
            &None,
            &TaskDemand::pool("default"),
            &FleetLoad::new(),
        )
        .expect("acme work must select the acme agent");
        assert_eq!(chosen.agent_id, "acme-1");

        assert!(
            select_fleet_agent(
                &roster,
                &Some("gamma".to_string()),
                &None,
                &TaskDemand::pool("default"),
                &FleetLoad::new()
            )
            .is_none(),
            "work for a tenant with no agents must not leak onto another tenant"
        );
    }

    #[test]
    fn public_work_does_not_select_a_none_tenant_agent() {
        let roster = vec![agent("admin-1", 32, None)];
        assert!(
            select_fleet_agent(
                &roster,
                &Some("public".to_string()),
                &None,
                &TaskDemand::pool("default"),
                &FleetLoad::new()
            )
            .is_none(),
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
        let chosen = select_fleet_agent(
            &roster,
            &Some("t1".to_string()),
            &triples,
            &TaskDemand::pool("default"),
            &FleetLoad::new(),
        )
        .unwrap();
        assert_eq!(chosen.agent_id, "ok");
    }

    // ── pools (CLOACINA-T-1021) ──────────────────────────────────────────────

    fn pool_agent(id: &str, cap: u32, pool: Option<&str>) -> FleetAgent {
        let mut a = agent(id, cap, Some("acme"));
        a.capabilities = vec!["python".to_string()];
        if let Some(pool) = pool {
            a.capabilities.push(format!("pool:{pool}"));
        }
        a
    }

    /// Default work never goes to an agent of another pool, even when that
    /// agent has the most free capacity or is the only one alive.
    #[test]
    fn default_work_never_selects_an_agent_of_another_pool() {
        let tenant = Some("acme".to_string());
        let roster = vec![
            pool_agent("big-1", 64, Some("big")),
            pool_agent("default-1", 2, Some("default")),
        ];
        let chosen = select_fleet_agent(
            &roster,
            &tenant,
            &None,
            &TaskDemand::pool("default"),
            &FleetLoad::new(),
        )
        .unwrap();
        assert_eq!(chosen.agent_id, "default-1");

        let only_big = vec![pool_agent("big-1", 64, Some("big"))];
        assert!(
            select_fleet_agent(
                &only_big,
                &tenant,
                &None,
                &TaskDemand::pool("default"),
                &FleetLoad::new()
            )
            .is_none()
        );
    }

    /// An agent that registered no pool capability (started by hand, or from
    /// before pools) is in `default`.
    #[test]
    fn an_agent_with_no_pool_capability_counts_as_default() {
        let tenant = Some("acme".to_string());
        let roster = vec![
            pool_agent("big-1", 64, Some("big")),
            pool_agent("plain-1", 4, None),
        ];
        let chosen = select_fleet_agent(
            &roster,
            &tenant,
            &None,
            &TaskDemand::pool("default"),
            &FleetLoad::new(),
        )
        .unwrap();
        assert_eq!(chosen.agent_id, "plain-1");
        // And it is not picked for another pool's work.
        let chosen = select_fleet_agent(
            &roster,
            &tenant,
            &None,
            &TaskDemand::pool("big"),
            &FleetLoad::new(),
        )
        .unwrap();
        assert_eq!(chosen.agent_id, "big-1");
    }

    /// The pool rule does not weaken the tenant rule.
    #[test]
    fn pool_selection_stays_inside_the_tenant() {
        let mut other = pool_agent("other-big", 64, Some("big"));
        other.tenant_id = Some("globex".to_string());
        let roster = vec![other];
        assert!(
            select_fleet_agent(
                &roster,
                &Some("acme".to_string()),
                &None,
                &TaskDemand::pool("big"),
                &FleetLoad::new()
            )
            .is_none()
        );
    }

    // ── requests and packing (CLOACINA-T-1027) ───────────────────────────────

    use cloacina::dal::unified::task_execution::FleetPlacement;
    use cloacina::database::universal_types::UniversalUuid;

    const GI: u64 = 1 << 30;

    /// An agent of `pool` with totals: `slots` slots, `cpus` CPUs, `gi` GiB.
    fn sized_agent(id: &str, pool: &str, slots: u32, cpus: u64, gi: u64) -> FleetAgent {
        let mut a = pool_agent(id, slots, Some(pool));
        a.cpu_millis = Some(cpus * 1000);
        a.memory_bytes = Some(gi * GI);
        a
    }

    fn demand(pool: &str, cpus: Option<u64>, gi: Option<u64>) -> TaskDemand<'_> {
        TaskDemand {
            pool,
            cpu_millis: cpus.map(|c| c * 1000),
            memory_bytes: gi.map(|g| g * GI),
        }
    }

    fn placed(agent: &str, cpus: Option<u64>, gi: Option<u64>) -> FleetPlacement {
        FleetPlacement {
            task_id: UniversalUuid::new_v4(),
            agent_id: agent.to_string(),
            cpu_request_millis: cpus.map(|c| c * 1000),
            memory_request_bytes: gi.map(|g| g * GI),
        }
    }

    fn acme() -> Option<String> {
        Some("acme".to_string())
    }

    /// Place tasks one after another as the executor does: select, then add
    /// the placement, then select the next against the new load.
    fn place_all(
        roster: &[FleetAgent],
        demands: &[TaskDemand<'_>],
    ) -> (Vec<Option<String>>, Vec<FleetPlacement>) {
        let mut placements = Vec::new();
        let mut chosen = Vec::new();
        for d in demands {
            let load = fleet_load(roster, &placements);
            let agent = select_fleet_agent(roster, &acme(), &None, d, &load);
            if let Some(a) = agent {
                placements.push(FleetPlacement {
                    task_id: UniversalUuid::new_v4(),
                    agent_id: a.agent_id.clone(),
                    cpu_request_millis: d.cpu_millis,
                    memory_request_bytes: d.memory_bytes,
                });
            }
            chosen.push(agent.map(|a| a.agent_id.clone()));
        }
        (chosen, placements)
    }

    /// The acceptance case: an agent with 32 Gi holds one 24 Gi task and four
    /// 2 Gi tasks together; a second 24 Gi task finds no agent (it waits).
    #[test]
    fn a_32gi_agent_holds_one_24gi_and_four_2gi_tasks_and_a_second_24gi_waits() {
        let roster = vec![sized_agent("big-1", "large", 8, 16, 32)];
        let large = demand("large", Some(1), Some(24));
        let small = demand("large", Some(1), Some(2));
        let (chosen, placements) = place_all(&roster, &[large, small, small, small, small, large]);
        let on_big = Some("big-1".to_string());
        assert_eq!(
            chosen,
            vec![
                on_big.clone(),
                on_big.clone(),
                on_big.clone(),
                on_big.clone(),
                on_big,
                None
            ]
        );
        let load = fleet_load(&roster, &placements);
        assert_eq!(
            load["big-1"],
            AgentLoad {
                tasks: 5,
                cpu_millis: 5000,
                memory_bytes: 32 * GI
            }
        );
        // Full to the byte: not even a 1 Gi task fits, though three slots are free.
        assert!(
            select_fleet_agent(
                &roster,
                &acme(),
                &None,
                &demand("large", None, Some(1)),
                &load
            )
            .is_none()
        );
        // When a 2 Gi task ends, a 2 Gi task fits again; the 24 Gi one still waits.
        let load = fleet_load(&roster, &placements[..4]);
        assert!(select_fleet_agent(&roster, &acme(), &None, &small, &load).is_some());
        assert!(select_fleet_agent(&roster, &acme(), &None, &large, &load).is_none());
    }

    /// The acceptance case: a `large` task never lands on `default`, and a
    /// task with no pool never lands on `large`, whatever room there is.
    #[test]
    fn a_large_task_never_lands_on_default_and_default_work_never_on_large() {
        let roster = vec![
            sized_agent("default-1", "default", 64, 64, 256),
            sized_agent("big-1", "large", 2, 8, 32),
        ];
        let none = FleetLoad::new();
        let large = demand("large", Some(4), Some(24));
        assert_eq!(
            select_fleet_agent(&roster, &acme(), &None, &large, &none)
                .unwrap()
                .agent_id,
            "big-1"
        );
        let plain = TaskDemand::pool("default");
        assert_eq!(
            select_fleet_agent(&roster, &acme(), &None, &plain, &none)
                .unwrap()
                .agent_id,
            "default-1"
        );
        // The large agent is full: the large task waits; it does not spill
        // onto the default agent, which has room for it.
        let load = fleet_load(&roster, &[placed("big-1", Some(4), Some(24))]);
        assert!(select_fleet_agent(&roster, &acme(), &None, &large, &load).is_none());
        // No agent of the pool at all: the same.
        let only_default = vec![sized_agent("default-1", "default", 64, 64, 256)];
        assert!(select_fleet_agent(&only_default, &acme(), &None, &large, &none).is_none());
    }

    /// The acceptance case: with no request and agents that report no totals,
    /// selection is what it was before requests: the most free slots, and an
    /// agent is full at its slot count.
    #[test]
    fn no_request_and_no_totals_behaves_as_before() {
        let mut busy = pool_agent("busy", 4, None);
        busy.available_capacity = 1;
        let roster = vec![
            busy,
            pool_agent("idle", 4, None),
            pool_agent("small", 2, None),
        ];
        let plain = TaskDemand::pool("default");
        assert_eq!(
            select_fleet_agent(&roster, &acme(), &None, &plain, &FleetLoad::new())
                .unwrap()
                .agent_id,
            "idle"
        );
        // A request is not compared with an agent that reports no totals.
        let asks = demand("default", Some(64), Some(512));
        assert_eq!(
            select_fleet_agent(&roster, &acme(), &None, &asks, &FleetLoad::new())
                .unwrap()
                .agent_id,
            "idle"
        );
        // Full at the slot count: four placed tasks fill a 4-slot agent.
        let one = vec![pool_agent("only", 4, None)];
        let four: Vec<FleetPlacement> = (0..4).map(|_| placed("only", None, None)).collect();
        let load = fleet_load(&one, &four);
        assert_eq!(
            load["only"],
            AgentLoad {
                tasks: 4,
                cpu_millis: 0,
                memory_bytes: 0
            }
        );
        assert!(select_fleet_agent(&one, &acme(), &None, &plain, &load).is_none());
    }

    /// A task with no request counts as one slot's share of an agent that
    /// reports totals. So such an agent holds exactly its slot count of them,
    /// and a task with a request sees the room they took.
    #[test]
    fn a_task_with_no_request_counts_as_one_slots_share() {
        assert_eq!(charge(None, Some(32 * GI), 4), 8 * GI);
        assert_eq!(charge(Some(2 * GI), Some(32 * GI), 4), 2 * GI);
        assert_eq!(charge(Some(2 * GI), None, 4), 0);
        assert_eq!(charge(None, None, 4), 0);
        assert_eq!(
            charge(None, Some(32 * GI), 0),
            32 * GI,
            "no division by zero"
        );

        let roster = vec![sized_agent("a", "default", 4, 4, 32)];
        let plain = TaskDemand::pool("default");
        let (chosen, placements) = place_all(&roster, &[plain; 5]);
        assert_eq!(chosen.iter().filter(|c| c.is_some()).count(), 4);
        assert_eq!(chosen[4], None);
        assert_eq!(
            fleet_load(&roster, &placements)["a"],
            AgentLoad {
                tasks: 4,
                cpu_millis: 4000,
                memory_bytes: 32 * GI
            }
        );

        // Two tasks with no request hold 16 Gi: a 24 Gi task does not fit,
        // a 16 Gi task does.
        let load = fleet_load(&roster, &[placed("a", None, None), placed("a", None, None)]);
        assert!(!fits(
            &roster[0],
            &load["a"],
            &demand("default", None, Some(24))
        ));
        assert!(fits(
            &roster[0],
            &load["a"],
            &demand("default", None, Some(16))
        ));
    }

    /// The slot count stays an upper cap: small requests do not put more
    /// tasks on an agent than its max concurrency, and the agent's own
    /// heartbeat figure counts too.
    #[test]
    fn the_slot_count_stays_an_upper_cap() {
        let roster = vec![sized_agent("a", "default", 2, 64, 256)];
        let tiny = demand("default", Some(1), Some(1));
        let (chosen, _) = place_all(&roster, &[tiny, tiny, tiny]);
        assert_eq!(chosen[2], None, "two slots, two tasks");

        // The agent says it is full (work the server does not count, such as
        // graph firings): nothing is placed, whatever the memory says.
        let mut full = sized_agent("a", "default", 2, 64, 256);
        full.available_capacity = 0;
        assert!(select_fleet_agent(&[full], &acme(), &None, &tiny, &FleetLoad::new()).is_none());
    }

    /// CPU is packed like memory.
    #[test]
    fn a_task_that_does_not_fit_in_cpu_waits() {
        let roster = vec![sized_agent("a", "default", 8, 4, 64)];
        let load = fleet_load(&roster, &[placed("a", Some(3), Some(1))]);
        assert!(!fits(
            &roster[0],
            &load["a"],
            &demand("default", Some(2), Some(1))
        ));
        assert!(fits(
            &roster[0],
            &load["a"],
            &demand("default", Some(1), Some(1))
        ));
    }

    /// The placement rule: among the agents where the task fits, the one with
    /// the least free memory; then the most free slots; then the lowest id.
    #[test]
    fn the_agent_with_the_least_free_memory_that_still_fits_wins() {
        let roster = vec![
            sized_agent("empty", "large", 8, 16, 32),
            sized_agent("half", "large", 8, 16, 32),
            sized_agent("nearly-full", "large", 8, 16, 32),
        ];
        let load = fleet_load(
            &roster,
            &[
                placed("half", Some(1), Some(16)),
                placed("nearly-full", Some(1), Some(28)),
            ],
        );
        let pick = |gi: u64| {
            select_fleet_agent(
                &roster,
                &acme(),
                &None,
                &demand("large", Some(1), Some(gi)),
                &load,
            )
            .map(|a| a.agent_id.as_str())
        };
        // 2 Gi fits everywhere: it goes where the least is free.
        assert_eq!(pick(2), Some("nearly-full"));
        // 8 Gi does not fit there: the next fullest.
        assert_eq!(pick(8), Some("half"));
        // 24 Gi only fits on the empty one, which the small tasks left alone.
        assert_eq!(pick(24), Some("empty"));
        assert_eq!(pick(40), None);

        // Equal free memory: the most free slots, then the lowest id.
        let mut fewer_slots = sized_agent("b-fewer-slots", "large", 8, 16, 32);
        fewer_slots.available_capacity = 3;
        let tie = vec![
            fewer_slots,
            sized_agent("c-twin", "large", 8, 16, 32),
            sized_agent("a-twin", "large", 8, 16, 32),
        ];
        let chosen = select_fleet_agent(
            &tie,
            &acme(),
            &None,
            &demand("large", Some(1), Some(2)),
            &FleetLoad::new(),
        )
        .unwrap();
        assert_eq!(chosen.agent_id, "a-twin");
    }

    /// An agent that reports no memory total ranks after the ones that do,
    /// and is packed by its slots only.
    #[test]
    fn an_agent_with_no_totals_is_packed_by_slots_and_ranks_last() {
        let roster = vec![
            pool_agent("by-hand", 64, Some("large")),
            sized_agent("sized", "large", 2, 8, 32),
        ];
        let d = demand("large", Some(1), Some(24));
        let none = FleetLoad::new();
        assert_eq!(
            select_fleet_agent(&roster, &acme(), &None, &d, &none)
                .unwrap()
                .agent_id,
            "sized"
        );
        // The sized agent is full: the task goes to the one with no totals.
        let load = fleet_load(&roster, &[placed("sized", Some(1), Some(24))]);
        assert_eq!(
            select_fleet_agent(&roster, &acme(), &None, &d, &load)
                .unwrap()
                .agent_id,
            "by-hand"
        );
    }

    /// The race rule: two replicas each place a 24 Gi task on the last free
    /// memory of one agent. Each writes its placement and reads the load
    /// again. The load is over the agent's total, so the replica that reads
    /// it takes its placement back. The agent never keeps both.
    #[test]
    fn two_replicas_on_the_last_free_memory_do_not_both_keep_the_agent() {
        let roster = vec![sized_agent("big-1", "large", 8, 16, 32)];
        let agent = &roster[0];
        let large = demand("large", Some(1), Some(24));

        // Both replicas read the same empty load and both pick the agent.
        let before = FleetLoad::new();
        assert!(select_fleet_agent(&roster, &acme(), &None, &large, &before).is_some());
        let a = placed("big-1", Some(1), Some(24));
        let b = placed("big-1", Some(1), Some(24));

        // Replica A wrote and read before B wrote: A keeps the agent.
        let seen_by_a = fleet_load(&roster, std::slice::from_ref(&a));
        assert!(!over_committed(agent, &seen_by_a["big-1"]));
        // Replica B wrote second: it sees both placements and withdraws.
        let seen_by_b = fleet_load(&roster, &[a.clone(), b.clone()]);
        assert!(over_committed(agent, &seen_by_b["big-1"]));
        // After B took its placement back, the agent holds one 24 Gi task.
        let after = fleet_load(&roster, std::slice::from_ref(&a));
        assert_eq!(after["big-1"].memory_bytes, 24 * GI);
        assert!(!over_committed(agent, &after["big-1"]));

        // Both wrote before either read: both see 48 Gi and both withdraw.
        // The agent holds neither, and both tasks wait for the next try.
        let both = fleet_load(&roster, &[a, b]);
        assert_eq!(both["big-1"].memory_bytes, 48 * GI);
        assert!(over_committed(agent, &both["big-1"]));
    }

    /// The check after a placement also covers the slot count and CPU, and
    /// an agent with no totals is over only by its slot count.
    #[test]
    fn over_committed_covers_slots_cpu_and_memory() {
        let sized = sized_agent("a", "default", 2, 4, 32);
        let load = |tasks, cpus: u64, gi: u64| AgentLoad {
            tasks,
            cpu_millis: cpus * 1000,
            memory_bytes: gi * GI,
        };
        assert!(
            !over_committed(&sized, &load(2, 4, 32)),
            "exactly full is not over"
        );
        assert!(over_committed(&sized, &load(3, 1, 1)));
        assert!(over_committed(&sized, &load(1, 5, 1)));
        assert!(over_committed(&sized, &load(1, 1, 33)));
        let by_hand = pool_agent("b", 2, None);
        assert!(!over_committed(&by_hand, &load(2, 999, 999)));
        assert!(over_committed(&by_hand, &load(3, 0, 0)));
    }

    /// A placement on an agent that is no longer live is not in the load.
    #[test]
    fn the_load_leaves_out_agents_that_are_gone() {
        let roster = vec![sized_agent("here", "default", 4, 4, 32)];
        let load = fleet_load(
            &roster,
            &[placed("here", None, Some(2)), placed("gone", None, Some(2))],
        );
        assert_eq!(load.len(), 1);
        assert_eq!(load["here"].tasks, 1);
    }

    /// The demand of a task comes from its declared placement.
    #[test]
    fn a_demand_is_read_from_the_tasks_placement() {
        let placement = cloacina_api_types::TaskPlacement {
            pool: Some("large".to_string()),
            cpu: Some("500m".to_string()),
            memory: Some("32Gi".to_string()),
            ..Default::default()
        };
        assert_eq!(
            TaskDemand::of(&placement),
            TaskDemand {
                pool: "large",
                cpu_millis: Some(500),
                memory_bytes: Some(32 * GI)
            }
        );
        let nothing = cloacina_api_types::TaskPlacement::default();
        assert_eq!(TaskDemand::of(&nothing), TaskDemand::pool("default"));
    }
}
