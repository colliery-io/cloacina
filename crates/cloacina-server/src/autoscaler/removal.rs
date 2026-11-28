/*
 *  Copyright 2026 Colliery Software
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

//! Which agents of a pool go first when its count is lowered
//! (CLOACINA-T-1145).
//!
//! On each tick the autoscale step writes, for each pool, its idle agents in
//! the order they go (the one registered last first) and its busy agents.
//! The actuator reads them in the same tick, before it removes agents: the
//! Docker actuator stops the containers of the idle agents first; the
//! Kubernetes actuator gives their pods a low `pod-deletion-cost` and the
//! busy ones a high one. An agent with no entry (not registered yet) goes
//! after the idle ones and before the busy ones.
//!
//! The list lives in the memory of the replica that leads the tick, which is
//! the one that reconciles right after. An entry older than its expiry is
//! not used: the actuator then removes as before.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use cloacina::dal::unified::FleetAgent;
use cloacina::dal::unified::task_execution::FleetPlacement;
use cloacina_api_types::agent_pool_of;

/// One agent, as the actuators find it: by id (a Kubernetes agent's id is
/// its pod name), or by the key it registered with (a Docker container's
/// own key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRef {
    pub agent_id: String,
    pub owner_key_id: Option<uuid::Uuid>,
}

/// The agents of one pool on one tick.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PoolAgents {
    /// No task in flight and none placed: in the order they go.
    pub idle: Vec<AgentRef>,
    /// With work: they go last.
    pub busy: Vec<AgentRef>,
}

impl PoolAgents {
    /// The agents of `pool` among `live` (one tenant's), split by work.
    /// Pure.
    pub fn of(pool: &str, live: &[FleetAgent], placements: &[FleetPlacement]) -> Self {
        let placed: HashSet<&str> = placements.iter().map(|p| p.agent_id.as_str()).collect();
        let mut agents: Vec<&FleetAgent> = live
            .iter()
            .filter(|a| agent_pool_of(&a.capabilities) == pool)
            .collect();
        // The one registered last first; the id breaks a tie.
        agents.sort_by(|a, b| {
            (b.registered_at.0, &b.agent_id).cmp(&(a.registered_at.0, &a.agent_id))
        });
        let mut out = Self::default();
        for agent in agents {
            let reference = AgentRef {
                agent_id: agent.agent_id.clone(),
                owner_key_id: agent.owner_key_id,
            };
            if agent.in_flight == 0 && !placed.contains(agent.agent_id.as_str()) {
                out.idle.push(reference);
            } else {
                out.busy.push(reference);
            }
        }
        out
    }

    /// The first idle agent: the one that goes.
    pub fn first_idle(&self) -> Option<&AgentRef> {
        self.idle.first()
    }
}

type Plan = HashMap<(String, String), (Instant, PoolAgents)>;

fn plan() -> std::sync::MutexGuard<'static, Plan> {
    static PLAN: OnceLock<Mutex<Plan>> = OnceLock::new();
    PLAN.get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Write the agents of a pool for this tick, valid for `ttl`.
pub fn set_pool_agents(tenant: &str, pool: &str, agents: PoolAgents, ttl: Duration) {
    plan().insert(
        (tenant.to_string(), pool.to_string()),
        (Instant::now() + ttl, agents),
    );
}

/// The agents of a pool as the last tick wrote them; `None` when none were
/// written or they are past their expiry.
pub fn pool_agents(tenant: &str, pool: &str) -> Option<PoolAgents> {
    let mut plan = plan();
    let key = (tenant.to_string(), pool.to_string());
    match plan.get(&key) {
        Some((until, agents)) if Instant::now() < *until => Some(agents.clone()),
        Some(_) => {
            plan.remove(&key);
            None
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(id: &str, pool: &str, registered_s: i64, in_flight: u32) -> FleetAgent {
        let mut a = crate::agent_registry::test_fleet_agent(id, 4, Some("acme"));
        if pool != cloacina_api_types::DEFAULT_AGENT_POOL {
            a.capabilities = vec![format!("pool:{pool}")];
        }
        a.registered_at = cloacina::database::universal_types::UniversalTimestamp(
            chrono::DateTime::from_timestamp(1_800_000_000 + registered_s, 0).unwrap(),
        );
        a.in_flight = in_flight;
        a
    }

    fn ids(agents: &[AgentRef]) -> Vec<&str> {
        agents.iter().map(|a| a.agent_id.as_str()).collect()
    }

    /// Idle agents go the one registered last first; an agent with work in
    /// flight or a placement is busy; another pool's agents are not counted.
    #[test]
    fn idle_agents_go_the_one_registered_last_first() {
        let live = vec![
            agent("old-idle", "big", 0, 0),
            agent("busy", "big", 50, 2),
            agent("new-idle", "big", 100, 0),
            agent("placed", "big", 200, 0),
            agent("other-pool", "small", 300, 0),
        ];
        let placements = vec![FleetPlacement {
            task_id: cloacina::database::universal_types::UniversalUuid::new_v4(),
            agent_id: "placed".to_string(),
            cpu_request_millis: None,
            memory_request_bytes: None,
        }];
        let agents = PoolAgents::of("big", &live, &placements);
        assert_eq!(ids(&agents.idle), vec!["new-idle", "old-idle"]);
        assert_eq!(ids(&agents.busy), vec!["placed", "busy"]);
        assert_eq!(agents.first_idle().unwrap().agent_id, "new-idle");
    }

    #[test]
    fn a_written_list_is_read_back_until_its_expiry() {
        let agents = PoolAgents {
            idle: vec![AgentRef {
                agent_id: "a1".into(),
                owner_key_id: None,
            }],
            busy: Vec::new(),
        };
        set_pool_agents("t1145-plan", "big", agents.clone(), Duration::from_secs(60));
        assert_eq!(pool_agents("t1145-plan", "big"), Some(agents.clone()));
        assert_eq!(pool_agents("t1145-plan", "small"), None);
        set_pool_agents("t1145-plan", "big", agents, Duration::ZERO);
        assert_eq!(pool_agents("t1145-plan", "big"), None, "past its expiry");
    }
}
