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

//! The autoscale step of the fleet control loop (CLOACINA-T-1029): read each
//! tenant's pools, agents, placements and waiting work; decide each pool's
//! count with [`super::decide_pool`]; write it; set the pool gauges.
//!
//! Runs on the fleet leader only (CLOACINA-A-0008). The actuator reads the
//! counts it writes: `agent_desired_counts` for `default`, `agent_pool_scale`
//! for the other pools.

use std::collections::HashSet;

use cloacina::dal::DAL;
use cloacina::dal::unified::FleetAgent;
use cloacina::dal::unified::task_execution::{FleetPlacement, WaitingDemand};
use cloacina::database::Database;
use cloacina_api_types::{AgentPoolSettings, DEFAULT_AGENT_POOL, agent_pool_of};
use tracing::{info, warn};

use super::{
    AgentSize, DemandGroup, PoolDecision, PoolView, Room, ScaleConfig, SpareTimers, TaskSize,
    decide_pool, pack_waiting,
};
use crate::agent_registry::{AgentLoad, fleet_load, free_slots};

/// The slots of an agent whose pool sets no `max_concurrency`: the agent's
/// own default (`--max-concurrency`).
const AGENT_DEFAULT_SLOTS: u32 = 4;

/// The size of one new agent of a pool: what a live agent of the pool
/// reports, else what the pool's settings give it (its totals, its memory
/// less the agent's default own share, its slots).
pub fn pool_agent_size(settings: &AgentPoolSettings, live: &[&FleetAgent]) -> AgentSize {
    if let Some(agent) = live.first() {
        return AgentSize {
            slots: agent.max_concurrency,
            cpu_millis: agent.cpu_millis,
            memory_bytes: agent.memory_bytes,
        };
    }
    let (cpu, memory) = crate::actuator::settings::pool_agent_totals(settings);
    AgentSize {
        slots: settings.max_concurrency.unwrap_or(AGENT_DEFAULT_SLOTS),
        cpu_millis: cpu.and_then(|c| cloacina_api_types::parse_cpu_millis(&c).ok()),
        memory_bytes: memory
            .and_then(|m| cloacina_api_types::parse_memory_bytes(&m).ok())
            .map(|m| m.saturating_sub(cloacina_api_types::DEFAULT_AGENT_RESERVED_MEMORY_BYTES)),
    }
}

/// The free room of a live agent.
fn room_of(agent: &FleetAgent, load: &AgentLoad) -> Room {
    let size = AgentSize {
        slots: agent.max_concurrency,
        cpu_millis: agent.cpu_millis,
        memory_bytes: agent.memory_bytes,
    };
    Room {
        size,
        free_slots: free_slots(agent, load),
        free_cpu_millis: agent.cpu_millis.map(|t| t.saturating_sub(load.cpu_millis)),
        free_memory_bytes: agent
            .memory_bytes
            .map(|t| t.saturating_sub(load.memory_bytes)),
    }
}

/// What one pool looks like now: the inputs of [`decide_pool`] that come
/// from its agents and its work, and the figures of its gauges.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PoolNow {
    pub live: u32,
    pub needed: u32,
    pub busy: bool,
    pub waiting: u64,
    pub unplaceable: u64,
    pub cpu_requested_millis: u64,
    pub cpu_offered_millis: u64,
    pub memory_requested_bytes: u64,
    pub memory_offered_bytes: u64,
    /// Live agents with no task in flight and none placed (CLOACINA-T-1145).
    pub idle: u32,
}

impl PoolNow {
    /// A spare agent (CLOACINA-T-1145): work in flight, an idle agent, and
    /// nothing waiting that needs one (only work no agent of the pool can
    /// hold, or none).
    pub fn spare(&self) -> bool {
        self.busy && self.idle > 0 && self.needed == 0 && self.waiting <= self.unplaceable
    }
}

/// Work out [`PoolNow`] for one pool of a tenant. Pure.
///
/// `max_new`: how many agents the pool may still get (its max less its live
/// agents).
pub fn pool_now(
    pool: &str,
    settings: &AgentPoolSettings,
    tenant_live: &[FleetAgent],
    placements: &[FleetPlacement],
    demand: &[WaitingDemand],
    max_new: u32,
) -> PoolNow {
    let live: Vec<&FleetAgent> = tenant_live
        .iter()
        .filter(|a| agent_pool_of(&a.capabilities) == pool)
        .collect();
    let live_owned: Vec<FleetAgent> = live.iter().map(|a| (*a).clone()).collect();
    let load = fleet_load(&live_owned, placements);
    let rooms: Vec<Room> = live
        .iter()
        .map(|a| room_of(a, &load.get(&a.agent_id).copied().unwrap_or_default()))
        .collect();
    let size = pool_agent_size(settings, &live);
    let groups: Vec<DemandGroup> = demand
        .iter()
        .filter(|d| d.pool == pool)
        .map(|d| DemandGroup {
            size: TaskSize {
                cpu_millis: d.cpu_request_millis,
                memory_bytes: d.memory_request_bytes,
            },
            count: d.count,
        })
        .collect();
    let packing = pack_waiting(&rooms, size, &groups, max_new);
    let waiting: u64 = groups.iter().map(|g| g.count).sum();
    let in_flight = live.iter().any(|a| a.in_flight > 0) || load.values().any(|l| l.tasks > 0);
    let idle = live
        .iter()
        .filter(|a| a.in_flight == 0 && load.get(&a.agent_id).is_none_or(|l| l.tasks == 0))
        .count() as u32;

    // Requested: what is in flight and what waits, each as it counts on an
    // agent of the pool's size. Offered: the totals of the live agents.
    let empty = Room::empty(size);
    let charge = |task: &TaskSize| {
        (
            crate::agent_registry::charge(task.cpu_millis, empty.size.cpu_millis, size.slots),
            crate::agent_registry::charge(task.memory_bytes, empty.size.memory_bytes, size.slots),
        )
    };
    let (mut cpu_req, mut mem_req) = load.values().fold((0u64, 0u64), |(c, m), l| {
        (c + l.cpu_millis, m + l.memory_bytes)
    });
    for g in &groups {
        let (c, m) = charge(&g.size);
        cpu_req = cpu_req.saturating_add(c.saturating_mul(g.count));
        mem_req = mem_req.saturating_add(m.saturating_mul(g.count));
    }
    PoolNow {
        live: live.len() as u32,
        needed: packing.agents_needed,
        busy: in_flight || waiting > packing.unplaceable,
        waiting,
        unplaceable: packing.unplaceable,
        cpu_requested_millis: cpu_req,
        cpu_offered_millis: live.iter().filter_map(|a| a.cpu_millis).sum(),
        memory_requested_bytes: mem_req,
        memory_offered_bytes: live.iter().filter_map(|a| a.memory_bytes).sum(),
        idle,
    }
}

/// The `[min, max]` and the stored count of a pool other than `default`.
/// With no `min_agents` the pool keeps its count at `max_agents`, as before
/// autoscaling.
pub fn other_pool_bounds(settings: &AgentPoolSettings, stored: Option<u32>) -> (u32, u32, u32) {
    let max = settings.max_agents.unwrap_or(0);
    let min = settings.min_agents.unwrap_or(max).min(max);
    (min, max, stored.unwrap_or(min).clamp(min, max))
}

/// What the step needs from the rest of the server.
pub struct StepInputs<'a> {
    pub admin: &'a Database,
    pub tenants: &'a crate::TenantDatabaseCache,
    pub live: &'a [FleetAgent],
    pub poll_tenants: &'a HashSet<String>,
    pub default_max_agents: u32,
    pub cfg: &'a ScaleConfig,
    /// The spare timers of the pools (CLOACINA-T-1145), in this replica's
    /// memory.
    pub spare: &'a SpareTimers,
}

/// Run the autoscale step for every tenant. Errors are logged per tenant and
/// per pool; one tenant that cannot be read does not stop the others.
#[cfg(feature = "postgres")]
pub async fn run(inputs: StepInputs<'_>) {
    let admin_dal = DAL::new(inputs.admin.clone());
    let mut tenants: Vec<String> = match cloacina::database::DatabaseAdmin::new(
        inputs.admin.clone(),
    )
    .list_tenant_schemas()
    .await
    {
        Ok(t) => t,
        Err(e) => {
            warn!(error = %e, "fleet autoscale: tenant schemas could not be read; skipping the step");
            return;
        }
    };
    tenants.push("public".to_string());
    let stored = match admin_dal.agent_pool_scale().list_all().await {
        Ok(rows) => rows,
        Err(e) => {
            warn!(error = %e, "fleet autoscale: pool state could not be read; skipping the step");
            return;
        }
    };
    let desired_rows = match admin_dal.agent_desired().list_all().await {
        Ok(rows) => rows,
        Err(e) => {
            warn!(error = %e, "fleet autoscale: desired counts could not be read; skipping the step");
            return;
        }
    };
    let now = chrono::Utc::now().naive_utc();
    for tenant in tenants {
        let db = match inputs.tenants.resolve(&tenant, inputs.admin).await {
            Ok(db) => db,
            Err(e) => {
                warn!(tenant = %tenant, error = %e, "fleet autoscale: no database for the tenant; skipped");
                continue;
            }
        };
        let tenant_dal = DAL::new(db);
        let pools = match crate::actuator::settings::load_pools(inputs.admin, &tenant).await {
            Ok(p) => p,
            Err(e) => {
                warn!(tenant = %tenant, error = %e, "fleet autoscale: pools could not be read; skipped");
                continue;
            }
        };
        let demand = match tenant_dal.task_execution().waiting_demand_by_pool().await {
            Ok(d) => d,
            Err(e) => {
                warn!(tenant = %tenant, error = %e, "fleet autoscale: waiting work could not be read; skipped");
                continue;
            }
        };
        let placements = match tenant_dal.task_execution().fleet_placements().await {
            Ok(p) => p,
            Err(e) => {
                warn!(tenant = %tenant, error = %e, "fleet autoscale: placements could not be read; skipped");
                continue;
            }
        };
        let tenant_live: Vec<FleetAgent> = inputs
            .live
            .iter()
            .filter(|a| a.tenant_id.as_deref() == Some(tenant.as_str()))
            .cloned()
            .collect();

        for (pool, settings) in &pools {
            let state = stored
                .iter()
                .find(|s| s.tenant_id == tenant && s.pool == *pool);
            let (min, max, desired) = if pool == DEFAULT_AGENT_POOL {
                let max = match admin_dal
                    .agent_limits()
                    .effective_limit(&tenant, inputs.default_max_agents)
                    .await
                {
                    Ok(l) => l,
                    Err(e) => {
                        warn!(tenant = %tenant, error = %e, "fleet autoscale: the tenant limit could not be read; skipped");
                        continue;
                    }
                };
                // The floor (and the poll floor of CLOACINA-T-1077) holds
                // also over a tenant limit below it.
                let min = crate::fleet_poll_dispatcher::tenant_floor(
                    inputs.cfg.floor,
                    inputs.poll_tenants.contains(&tenant),
                );
                let desired = desired_rows
                    .iter()
                    .find(|(t, _)| *t == tenant)
                    .map(|(_, d)| *d)
                    .unwrap_or(0);
                (min, max.max(min), desired)
            } else {
                other_pool_bounds(settings, state.and_then(|s| s.desired_count))
            };
            let live_count = tenant_live
                .iter()
                .filter(|a| agent_pool_of(&a.capabilities) == pool.as_str())
                .count() as u32;
            let now_view = pool_now(
                pool,
                settings,
                &tenant_live,
                &placements,
                &demand,
                max.saturating_sub(live_count),
            );
            // CLOACINA-T-1145: which agents go first if the count is lowered,
            // for the actuator that reconciles right after this step.
            let agents = super::removal::PoolAgents::of(pool, &tenant_live, &placements);
            let ttl = inputs.cfg.interval.saturating_mul(2);
            super::removal::set_pool_agents(&tenant, pool, agents.clone(), ttl);
            let view = PoolView {
                desired,
                live: now_view.live,
                min,
                max,
                needed: now_view.needed,
                busy: now_view.busy,
                idle_since: state.and_then(|s| s.idle_since),
                last_scaled_down_at: state.and_then(|s| s.last_scaled_down_at),
                spare_since: inputs
                    .spare
                    .observe(&tenant, pool, now_view.spare(), now, ttl),
            };
            let decision = decide_pool(&view, now, inputs.cfg);
            set_gauges(&tenant, pool, &now_view, &view, decision);

            // The idle time. Written only when it changes, and only for a
            // pool above its min (an idle pool at its min has nothing to do).
            let stored_idle = view.idle_since.is_some();
            let idle = !now_view.busy;
            if idle != stored_idle
                && (desired > min || stored_idle)
                && let Err(e) = admin_dal
                    .agent_pool_scale()
                    .set_idle(&tenant, pool, idle)
                    .await
            {
                warn!(tenant = %tenant, pool = %pool, error = %e, "fleet autoscale: idle state not written");
            }

            match decision {
                PoolDecision::Hold => {}
                PoolDecision::Up(to) => {
                    let written = if pool == DEFAULT_AGENT_POOL {
                        admin_dal
                            .agent_desired()
                            .set_desired_autoscaled(&tenant, to)
                            .await
                    } else {
                        admin_dal
                            .agent_pool_scale()
                            .set_desired(&tenant, pool, to)
                            .await
                    };
                    match written {
                        Ok(()) => {
                            metrics::counter!("cloacina_fleet_pool_scaled_total", "direction" => "up")
                                .increment(1);
                            info!(
                                tenant = %tenant,
                                pool = %pool,
                                from = desired,
                                to,
                                waiting = now_view.waiting,
                                needed = now_view.needed,
                                "fleet autoscale: pool scaled up for waiting work"
                            );
                        }
                        Err(e) => {
                            warn!(tenant = %tenant, pool = %pool, error = %e, "fleet autoscale: scale-up not written")
                        }
                    }
                }
                PoolDecision::StepDown(to) => {
                    // Read the work again just before the write: the agent
                    // that goes must still be idle, and nothing wait.
                    let Some(chosen) = agents.first_idle().cloned() else {
                        continue;
                    };
                    let Some(fresh) =
                        still_spare(&tenant_dal, pool, settings, &tenant_live, &chosen, max).await
                    else {
                        continue;
                    };
                    super::removal::set_pool_agents(&tenant, pool, fresh, ttl);
                    if let Err(e) = write_scale_down(&admin_dal, &tenant, pool, to).await {
                        warn!(tenant = %tenant, pool = %pool, error = %e, "fleet autoscale: step-down not written");
                        continue;
                    }
                    metrics::counter!("cloacina_fleet_pool_scaled_total", "direction" => "down")
                        .increment(1);
                    info!(
                        tenant = %tenant,
                        pool = %pool,
                        from = desired,
                        to,
                        agent = %chosen.agent_id,
                        spare_s = inputs.cfg.idle.as_secs(),
                        "fleet autoscale: one idle agent of a pool with work goes"
                    );
                }
                PoolDecision::Down(to) => {
                    // Read the work again just before the write: a task placed
                    // since the first read keeps the pool.
                    if still_busy(&tenant_dal, pool, &tenant_live).await {
                        continue;
                    }
                    match write_scale_down(&admin_dal, &tenant, pool, to).await {
                        Ok(()) => {
                            metrics::counter!("cloacina_fleet_pool_scaled_total", "direction" => "down")
                                .increment(1);
                            info!(
                                tenant = %tenant,
                                pool = %pool,
                                from = desired,
                                to,
                                idle_s = inputs.cfg.idle.as_secs(),
                                "fleet autoscale: idle pool scaled down to its min"
                            );
                        }
                        Err(e) => {
                            warn!(tenant = %tenant, pool = %pool, error = %e, "fleet autoscale: scale-down not written")
                        }
                    }
                }
            }
        }
    }
}

/// Write a lower count of a pool, and the time of the scale-down. The count
/// of `default` is its desired count; another pool's is in its scale row.
#[cfg(feature = "postgres")]
async fn write_scale_down(
    admin_dal: &DAL,
    tenant: &str,
    pool: &str,
    to: u32,
) -> Result<(), cloacina::error::ValidationError> {
    if pool == DEFAULT_AGENT_POOL {
        admin_dal
            .agent_desired()
            .set_desired_autoscaled(tenant, to)
            .await?;
        admin_dal
            .agent_pool_scale()
            .record_scale_down(tenant, pool, None)
            .await
    } else {
        admin_dal
            .agent_pool_scale()
            .record_scale_down(tenant, pool, Some(to))
            .await
    }
}

/// Is the pool still spare (CLOACINA-T-1145), read again just before a
/// step-down: `chosen` still idle, and nothing waiting that needs an agent.
/// Gives the pool's agents as read now. A read that fails counts as not
/// spare.
#[cfg(feature = "postgres")]
async fn still_spare(
    tenant_dal: &DAL,
    pool: &str,
    settings: &AgentPoolSettings,
    tenant_live: &[FleetAgent],
    chosen: &super::removal::AgentRef,
    max: u32,
) -> Option<super::removal::PoolAgents> {
    let placements = tenant_dal.task_execution().fleet_placements().await.ok()?;
    let demand = tenant_dal
        .task_execution()
        .waiting_demand_by_pool()
        .await
        .ok()?;
    let live = tenant_live
        .iter()
        .filter(|a| agent_pool_of(&a.capabilities) == pool)
        .count() as u32;
    let now = pool_now(
        pool,
        settings,
        tenant_live,
        &placements,
        &demand,
        max.saturating_sub(live),
    );
    let agents = super::removal::PoolAgents::of(pool, tenant_live, &placements);
    (now.spare() && agents.idle.first() == Some(chosen)).then_some(agents)
}

/// Is there work on the pool now: a task placed on one of its agents, or a
/// task of the pool waiting. A read that fails counts as busy.
#[cfg(feature = "postgres")]
async fn still_busy(tenant_dal: &DAL, pool: &str, tenant_live: &[FleetAgent]) -> bool {
    let placements = match tenant_dal.task_execution().fleet_placements().await {
        Ok(p) => p,
        Err(_) => return true,
    };
    let pool_agents: HashSet<&str> = tenant_live
        .iter()
        .filter(|a| agent_pool_of(&a.capabilities) == pool)
        .map(|a| a.agent_id.as_str())
        .collect();
    if placements
        .iter()
        .any(|p| pool_agents.contains(p.agent_id.as_str()))
    {
        return true;
    }
    match tenant_dal.task_execution().waiting_demand_by_pool().await {
        Ok(d) => d.iter().any(|d| d.pool == pool && d.count > 0),
        Err(_) => true,
    }
}

fn set_gauges(tenant: &str, pool: &str, now: &PoolNow, view: &PoolView, decision: PoolDecision) {
    let desired = match decision {
        PoolDecision::Up(n) | PoolDecision::Down(n) | PoolDecision::StepDown(n) => n,
        PoolDecision::Hold => view.desired,
    };
    let labels = [("tenant", tenant.to_string()), ("pool", pool.to_string())];
    metrics::gauge!("cloacina_fleet_pool_agents_desired", &labels).set(desired as f64);
    metrics::gauge!("cloacina_fleet_pool_agents_ready", &labels).set(now.live as f64);
    metrics::gauge!("cloacina_fleet_pool_cpu_requested_millis", &labels)
        .set(now.cpu_requested_millis as f64);
    metrics::gauge!("cloacina_fleet_pool_cpu_offered_millis", &labels)
        .set(now.cpu_offered_millis as f64);
    metrics::gauge!("cloacina_fleet_pool_memory_requested_bytes", &labels)
        .set(now.memory_requested_bytes as f64);
    metrics::gauge!("cloacina_fleet_pool_memory_offered_bytes", &labels)
        .set(now.memory_offered_bytes as f64);
    metrics::gauge!("cloacina_fleet_pool_waiting_tasks", &labels).set(now.waiting as f64);
    metrics::gauge!("cloacina_fleet_pool_waiting_unplaceable_tasks", &labels)
        .set(now.unplaceable as f64);
}

/// Describe the pool metrics. Called once at server start.
pub fn describe_metrics() {
    metrics::describe_gauge!(
        "cloacina_fleet_pool_agents_desired",
        "Agents the autoscaler wants for a tenant's pool (CLOACINA-T-1029)."
    );
    metrics::describe_gauge!(
        "cloacina_fleet_pool_agents_ready",
        "Live agents of a tenant's pool."
    );
    metrics::describe_gauge!(
        "cloacina_fleet_pool_cpu_requested_millis",
        "CPU in flight and waiting on a tenant's pool, in millicores."
    );
    metrics::describe_gauge!(
        "cloacina_fleet_pool_cpu_offered_millis",
        "CPU the live agents of a tenant's pool offer, in millicores."
    );
    metrics::describe_gauge!(
        "cloacina_fleet_pool_memory_requested_bytes",
        "Memory in flight and waiting on a tenant's pool, in bytes."
    );
    metrics::describe_gauge!(
        "cloacina_fleet_pool_memory_offered_bytes",
        "Memory the live agents of a tenant's pool offer to tasks, in bytes."
    );
    metrics::describe_gauge!(
        "cloacina_fleet_pool_waiting_tasks",
        "Tasks of a tenant's pool that wait for an agent."
    );
    metrics::describe_gauge!(
        "cloacina_fleet_pool_waiting_unplaceable_tasks",
        "Waiting tasks larger than one agent of their pool: more agents cannot place them."
    );
    metrics::describe_counter!(
        "cloacina_fleet_pool_scaled_total",
        "Changes of a pool's agent count by the autoscaler, by direction (up, down)."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const GI: u64 = 1 << 30;

    fn agent(id: &str, pool: &str, slots: u32, memory: Option<u64>) -> FleetAgent {
        let mut a = crate::agent_registry::test_fleet_agent(id, slots, Some("acme"));
        if pool != DEFAULT_AGENT_POOL {
            a.capabilities = vec![format!("pool:{pool}")];
        }
        a.memory_bytes = memory;
        a
    }

    fn demand(pool: &str, memory: Option<u64>, count: u64) -> WaitingDemand {
        WaitingDemand {
            pool: pool.to_string(),
            cpu_request_millis: None,
            memory_request_bytes: memory,
            count,
        }
    }

    fn sized(memory: &str, slots: u32) -> AgentPoolSettings {
        AgentPoolSettings {
            memory_limit: Some(memory.to_string()),
            max_concurrency: Some(slots),
            min_agents: Some(0),
            max_agents: Some(3),
            ..Default::default()
        }
    }

    /// With no live agent, the size comes from the settings, less the
    /// agent's own memory share.
    #[test]
    fn the_size_of_a_new_agent_comes_from_the_settings() {
        let size = pool_agent_size(&sized("8Gi", 8), &[]);
        assert_eq!(size.slots, 8);
        assert_eq!(
            size.memory_bytes,
            Some(8 * GI - cloacina_api_types::DEFAULT_AGENT_RESERVED_MEMORY_BYTES)
        );
        let slots_only = pool_agent_size(&AgentPoolSettings::default(), &[]);
        assert_eq!((slots_only.slots, slots_only.memory_bytes), (4, None));
        let live = agent("a1", "big", 2, Some(30 * GI));
        let from_live = pool_agent_size(&sized("8Gi", 8), &[&live]);
        assert_eq!(
            (from_live.slots, from_live.memory_bytes),
            (2, Some(30 * GI))
        );
    }

    /// The acceptance case: ten 2 Gi tasks on 8 Gi agents with max 3, at
    /// zero agents. The agents offer 8 Gi less their share, so three tasks
    /// fit on each: three agents hold nine, the max holds the tenth back.
    #[test]
    fn ten_2gi_tasks_at_zero_ask_for_the_max() {
        let now = pool_now(
            "big",
            &sized("8Gi", 8),
            &[],
            &[],
            &[demand("big", Some(2 * GI), 10)],
            3,
        );
        assert_eq!(now.needed, 3);
        assert!(now.busy);
        assert_eq!(now.waiting, 10);
        assert_eq!(now.memory_requested_bytes, 20 * GI);
    }

    /// Work of another pool or another tenant's agents does not count.
    #[test]
    fn only_the_pools_own_agents_and_work_count() {
        let mut other_tenant = agent("x1", "big", 8, Some(8 * GI));
        other_tenant.tenant_id = Some("beta".to_string());
        let default_agent = agent("d1", DEFAULT_AGENT_POOL, 4, None);
        let tenant_live = vec![default_agent];
        let now = pool_now(
            "big",
            &sized("8Gi", 8),
            &tenant_live,
            &[],
            &[demand(DEFAULT_AGENT_POOL, None, 5)],
            3,
        );
        assert_eq!((now.live, now.needed, now.busy), (0, 0, false));
    }

    /// An agent with work in flight keeps the pool busy.
    #[test]
    fn an_agent_with_work_in_flight_keeps_the_pool_busy() {
        let mut a = agent("a1", "big", 8, Some(8 * GI));
        a.in_flight = 1;
        a.available_capacity = 7;
        let now = pool_now("big", &sized("8Gi", 8), &[a], &[], &[], 2);
        assert!(now.busy);
        assert_eq!(now.needed, 0);
        // A placement the server holds counts too.
        let idle = agent("a1", "big", 8, Some(8 * GI));
        let placed = FleetPlacement {
            task_id: cloacina::database::universal_types::UniversalUuid::new_v4(),
            agent_id: "a1".to_string(),
            cpu_request_millis: None,
            memory_request_bytes: Some(GI),
        };
        let now = pool_now("big", &sized("8Gi", 8), &[idle], &[placed], &[], 2);
        assert!(now.busy);
    }

    /// A task larger than one agent does not keep the pool busy or scale it.
    #[test]
    fn an_unplaceable_task_neither_scales_nor_holds_the_pool() {
        let now = pool_now(
            "big",
            &sized("8Gi", 8),
            &[],
            &[],
            &[demand("big", Some(16 * GI), 1)],
            3,
        );
        assert_eq!((now.needed, now.unplaceable, now.busy), (0, 1, false));
    }

    /// CLOACINA-T-1145: a pool with a busy agent and an idle one, and
    /// nothing waiting, has a spare agent; waiting work that fits, or no idle
    /// agent, is not spare; an unplaceable task does not count.
    #[test]
    fn a_busy_pool_with_an_idle_agent_and_nothing_waiting_is_spare() {
        let mut busy = agent("a1", "big", 8, Some(8 * GI));
        busy.in_flight = 1;
        let idle = agent("a2", "big", 8, Some(8 * GI));
        let pool = vec![busy.clone(), idle];
        let now = pool_now("big", &sized("8Gi", 8), &pool, &[], &[], 1);
        assert_eq!((now.busy, now.idle), (true, 1));
        assert!(now.spare());
        let waiting = pool_now(
            "big",
            &sized("8Gi", 8),
            &pool,
            &[],
            &[demand("big", Some(GI), 1)],
            1,
        );
        assert!(!waiting.spare(), "a waiting task needs the idle agent");
        let unplaceable = pool_now(
            "big",
            &sized("8Gi", 8),
            &pool,
            &[],
            &[demand("big", Some(16 * GI), 1)],
            1,
        );
        assert!(unplaceable.spare(), "no agent of the pool can hold it");
        let all_busy = pool_now("big", &sized("8Gi", 8), &[busy], &[], &[], 1);
        assert!(!all_busy.spare());
        // A whole idle pool is idle, not spare.
        let none_busy = pool_now(
            "big",
            &sized("8Gi", 8),
            &[agent("a3", "big", 8, Some(8 * GI))],
            &[],
            &[],
            1,
        );
        assert!(!none_busy.busy && !none_busy.spare());
    }

    #[test]
    fn a_pool_with_no_min_keeps_its_count_at_its_max() {
        let fixed = AgentPoolSettings {
            max_agents: Some(2),
            ..Default::default()
        };
        assert_eq!(other_pool_bounds(&fixed, None), (2, 2, 2));
        assert_eq!(other_pool_bounds(&fixed, Some(0)), (2, 2, 2));
        let scaled = sized("8Gi", 8);
        assert_eq!(other_pool_bounds(&scaled, None), (0, 3, 0));
        assert_eq!(other_pool_bounds(&scaled, Some(2)), (0, 3, 2));
        assert_eq!(other_pool_bounds(&scaled, Some(9)), (0, 3, 3));
    }
}
