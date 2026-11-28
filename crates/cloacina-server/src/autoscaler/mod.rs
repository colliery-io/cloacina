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

//! Pool autoscaler (CLOACINA-T-1029; first version CLOACI-T-0811).
//!
//! A leader-gated step of the fleet control loop sets the agent count of each
//! pool of each tenant from the **waiting work that does not fit**:
//!
//! - **Leftover.** The pool's waiting tasks (Ready, on no agent) are packed,
//!   largest first, onto the free room of the pool's live agents. What is left
//!   does not fit.
//! - **Up.** The leftover is packed, largest first, into empty agents of the
//!   pool's size. The target is `max(desired, live + needed)`, clamped to the
//!   pool's `[min, max]`: agents that are starting are counted, so a tick
//!   while pods start adds no more. One step, not one agent at a time.
//! - **Down.** A pool goes back to its min when it has been idle (no task in
//!   flight, none waiting) for the idle cooldown, and its last scale-down is
//!   a cooldown ago. An agent that is stopped for any reason drains first
//!   (the agent's SIGTERM handling).
//! - **Step down** (CLOACINA-T-1145). A pool with work in flight that has
//!   had a spare agent (an idle agent, and nothing waiting that needs one)
//!   for the idle cooldown loses one agent per cooldown, down to its min.
//!   The actuator removes an idle agent, the one registered last
//!   ([`removal`]); a busy agent is never removed while an idle one exists.
//!
//! A task counts for what it requests, or one slot's share of an agent with
//! totals ([`crate::agent_registry::charge`]), as in selection. A task larger
//! than one agent of its pool cannot be placed by more agents: it is counted
//! as unplaceable and does not scale the pool.
//!
//! This module holds the pure pieces; [`step`] reads the database and writes
//! the counts; the loop is in `lib.rs`, behind [`leader`] (CLOACINA-A-0008).

pub mod leader;
pub mod removal;
pub mod step;

use std::time::Duration;

/// Tunable knobs, from the environment at boot via [`ScaleConfig::from_env`].
#[derive(Debug, Clone)]
pub struct ScaleConfig {
    /// Least time between two scale-downs of a pool (default 60 s).
    pub cooldown: Duration,
    /// How long a pool must be idle before it goes back to its min
    /// (default 300 s).
    pub idle: Duration,
    /// The min of the pool `default` (default `0`). Another pool has its own
    /// `min_agents`.
    pub floor: u32,
    /// How often the control loop ticks (default 30 s).
    pub interval: Duration,
}

impl Default for ScaleConfig {
    fn default() -> Self {
        Self {
            cooldown: Duration::from_secs(60),
            idle: Duration::from_secs(300),
            floor: 0,
            interval: Duration::from_secs(30),
        }
    }
}

impl ScaleConfig {
    /// Build from `CLOACINA_AUTOSCALE_*` env vars, falling back to the defaults
    /// for any var that is unset or unparsable:
    /// - `CLOACINA_AUTOSCALE_COOLDOWN_S` = 60
    /// - `CLOACINA_AUTOSCALE_IDLE_S`     = 300
    /// - `CLOACINA_AUTOSCALE_FLOOR`      = 0
    /// - `CLOACINA_AUTOSCALE_INTERVAL_S` = 30
    pub fn from_env() -> Self {
        let d = Self::default();
        let secs = |key: &str| env_parse::<u64>(key).map(Duration::from_secs);
        Self {
            cooldown: secs("CLOACINA_AUTOSCALE_COOLDOWN_S").unwrap_or(d.cooldown),
            idle: secs("CLOACINA_AUTOSCALE_IDLE_S").unwrap_or(d.idle),
            floor: env_parse("CLOACINA_AUTOSCALE_FLOOR").unwrap_or(d.floor),
            interval: env_parse::<u64>("CLOACINA_AUTOSCALE_INTERVAL_S")
                .filter(|n| *n > 0)
                .map(Duration::from_secs)
                .unwrap_or(d.interval),
        }
    }
}

/// Parse an env var into `T`, returning `None` if unset or unparsable.
fn env_parse<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|s| s.parse::<T>().ok())
}

/// Cooldown gate: is `cooldown` over since `last`?
///
/// Pure (clock injected as `now`). `last` is a wall-clock time read back from
/// the database, so the gate holds across replicas: leadership rotates per
/// tick (CLOACI-A-0008 refinement). Both times are UTC.
///
/// - `None` (never) → `true`.
/// - Within `cooldown` of `last` → `false`.
/// - At or after → `true`.
/// - A negative elapsed time (clock skew between replicas) → `false`.
pub fn should_act_at(
    last: Option<chrono::NaiveDateTime>,
    now: chrono::NaiveDateTime,
    cooldown: Duration,
) -> bool {
    match last {
        None => true,
        Some(t) => match (now - t).to_std() {
            Ok(elapsed) => elapsed >= cooldown,
            Err(_) => false,
        },
    }
}

/// The size of one agent of a pool: its slots, and its CPU and memory totals
/// (`None`: it reports none, and is packed by slots in that resource).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentSize {
    pub slots: u32,
    pub cpu_millis: Option<u64>,
    pub memory_bytes: Option<u64>,
}

/// The free room of one agent: what is left of an [`AgentSize`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Room {
    /// The agent's size (to work out a task's charge).
    pub size: AgentSize,
    pub free_slots: u32,
    /// Free CPU, `None` when the agent has no CPU total.
    pub free_cpu_millis: Option<u64>,
    /// Free memory, `None` when the agent has no memory total.
    pub free_memory_bytes: Option<u64>,
}

impl Room {
    /// An empty agent of `size`.
    pub fn empty(size: AgentSize) -> Self {
        Self {
            size,
            free_slots: size.slots,
            free_cpu_millis: size.cpu_millis,
            free_memory_bytes: size.memory_bytes,
        }
    }

    fn charges(&self, task: &TaskSize) -> (u64, u64) {
        use crate::agent_registry::charge;
        (
            charge(task.cpu_millis, self.size.cpu_millis, self.size.slots),
            charge(task.memory_bytes, self.size.memory_bytes, self.size.slots),
        )
    }

    /// Does the task fit in what is left?
    fn fits(&self, task: &TaskSize) -> bool {
        if self.free_slots == 0 {
            return false;
        }
        let (cpu, memory) = self.charges(task);
        self.free_cpu_millis.is_none_or(|free| cpu <= free)
            && self.free_memory_bytes.is_none_or(|free| memory <= free)
    }

    fn take(&mut self, task: &TaskSize) {
        let (cpu, memory) = self.charges(task);
        self.free_slots -= 1;
        self.free_cpu_millis = self.free_cpu_millis.map(|f| f.saturating_sub(cpu));
        self.free_memory_bytes = self.free_memory_bytes.map(|f| f.saturating_sub(memory));
    }
}

/// The requests of one task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskSize {
    pub cpu_millis: Option<u64>,
    pub memory_bytes: Option<u64>,
}

/// Tasks of one size that wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemandGroup {
    pub size: TaskSize,
    pub count: u64,
}

/// What packing the waiting work gave.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Packing {
    /// New agents of the pool's size the leftover needs (at most the room
    /// below the pool's max).
    pub agents_needed: u32,
    /// Tasks that fit on no agent of the pool's size: they cannot be placed
    /// by more agents.
    pub unplaceable: u64,
    /// Tasks that fit nowhere, also with `agents_needed` more agents: the
    /// pool's max holds them back.
    pub over_max: u64,
    /// Tasks that fit on the free room of the live agents (they wait for a
    /// dispatch try, not for an agent).
    pub fit_now: u64,
}

/// Pack the waiting work (CLOACINA-T-1029): first onto the free room of the
/// pool's live agents, then into empty agents of `size`, at most `max_new`
/// of them. Largest first by memory, then CPU (first-fit decreasing).
pub fn pack_waiting(
    live: &[Room],
    size: AgentSize,
    demand: &[DemandGroup],
    max_new: u32,
) -> Packing {
    let mut groups: Vec<DemandGroup> = demand.iter().copied().filter(|g| g.count > 0).collect();
    // Largest first: the charge on an agent of the pool's size.
    let empty = Room::empty(size);
    groups.sort_by(|a, b| {
        let (ac, am) = empty.charges(&a.size);
        let (bc, bm) = empty.charges(&b.size);
        (bm, bc).cmp(&(am, ac))
    });

    let mut live: Vec<Room> = live.to_vec();
    let mut new_agents: Vec<Room> = Vec::new();
    let mut out = Packing::default();
    for group in groups {
        if size.slots == 0 || !empty.fits(&group.size) {
            out.unplaceable += group.count;
            continue;
        }
        let mut left = group.count;
        while left > 0 {
            if let Some(room) = live.iter_mut().find(|r| r.fits(&group.size)) {
                room.take(&group.size);
                out.fit_now += 1;
            } else if let Some(room) = new_agents.iter_mut().find(|r| r.fits(&group.size)) {
                room.take(&group.size);
            } else if (new_agents.len() as u32) < max_new {
                let mut room = Room::empty(size);
                room.take(&group.size);
                new_agents.push(room);
            } else {
                // Nothing of this size fits any more; smaller groups may.
                out.over_max += left;
                break;
            }
            left -= 1;
        }
    }
    out.agents_needed = new_agents.len() as u32;
    out
}

/// What the autoscaler knows about one pool on one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolView {
    /// The stored count.
    pub desired: u32,
    /// Live (registered, heartbeating) agents of the pool.
    pub live: u32,
    pub min: u32,
    pub max: u32,
    /// New agents the leftover needs ([`Packing::agents_needed`]).
    pub needed: u32,
    /// A task in flight on the pool's agents, or a task waiting that more
    /// agents or a dispatch try can place.
    pub busy: bool,
    /// Since when the pool is idle, as stored.
    pub idle_since: Option<chrono::NaiveDateTime>,
    /// The last scale-down, as stored.
    pub last_scaled_down_at: Option<chrono::NaiveDateTime>,
    /// Since when the pool has had a spare agent: work in flight, at least
    /// one idle agent, and nothing waiting that needs one (CLOACINA-T-1145).
    /// From the fleet leader's memory ([`SpareTimers`]).
    pub spare_since: Option<chrono::NaiveDateTime>,
}

/// The change the autoscaler makes to one pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolDecision {
    /// Raise the count to this (waiting work, or a min above the count).
    Up(u32),
    /// Lower the count to this (idle for the idle cooldown, or a max below
    /// the count while idle).
    Down(u32),
    /// Lower the count by one: a partly busy pool with a spare agent for the
    /// idle cooldown (CLOACINA-T-1145). An idle agent goes.
    StepDown(u32),
    Hold,
}

/// Decide the count of one pool (CLOACINA-T-1029). Pure.
pub fn decide_pool(view: &PoolView, now: chrono::NaiveDateTime, cfg: &ScaleConfig) -> PoolDecision {
    let min = view.min.min(view.max);
    if view.desired < min {
        return PoolDecision::Up(min);
    }
    if view.needed > 0 {
        let target = view
            .desired
            .max(view.live.saturating_add(view.needed))
            .min(view.max)
            .max(min);
        if target > view.desired {
            return PoolDecision::Up(target);
        }
        return PoolDecision::Hold;
    }
    if view.busy {
        // CLOACINA-T-1145: a pool with work and a spare agent for the idle
        // cooldown loses one agent, a cooldown after its last scale-down.
        // Not while agents start (fewer live than stored): one of those
        // would be the one to go.
        let spare_long_enough = view
            .spare_since
            .is_some_and(|since| should_act_at(Some(since), now, cfg.idle));
        if spare_long_enough
            && view.desired > min
            && view.live >= view.desired
            && should_act_at(view.last_scaled_down_at, now, cfg.cooldown)
        {
            return PoolDecision::StepDown(view.desired - 1);
        }
        return PoolDecision::Hold;
    }
    // Idle: back to the min after the idle cooldown, a cooldown after the
    // last scale-down. A max lowered under the count waits for this too: a
    // pool with work is never shrunk.
    let idle_long_enough = view
        .idle_since
        .is_some_and(|since| should_act_at(Some(since), now, cfg.idle));
    if view.desired > min
        && idle_long_enough
        && should_act_at(view.last_scaled_down_at, now, cfg.cooldown)
    {
        return PoolDecision::Down(min);
    }
    PoolDecision::Hold
}

/// Since when each pool has had a spare agent (CLOACINA-T-1145), in the
/// memory of this replica. Leadership can move between replicas: a replica
/// that did not see a pool for more than `max_gap` (it did not lead) starts
/// the pool's timer again. So a step-down can come later than the idle
/// cooldown, never sooner.
#[derive(Default)]
pub struct SpareTimers {
    timers: std::sync::Mutex<std::collections::HashMap<(String, String), SpareTimer>>,
}

#[derive(Clone, Copy)]
struct SpareTimer {
    since: chrono::NaiveDateTime,
    seen: chrono::NaiveDateTime,
}

impl SpareTimers {
    /// Record what this tick saw of a pool; give since when it is spare.
    pub fn observe(
        &self,
        tenant: &str,
        pool: &str,
        spare: bool,
        now: chrono::NaiveDateTime,
        max_gap: Duration,
    ) -> Option<chrono::NaiveDateTime> {
        let mut timers = self.timers.lock().unwrap_or_else(|e| e.into_inner());
        let key = (tenant.to_string(), pool.to_string());
        if !spare {
            timers.remove(&key);
            return None;
        }
        let timer = match timers.get(&key) {
            Some(t) if !should_act_at(Some(t.seen), now, max_gap) => SpareTimer {
                since: t.since,
                seen: now,
            },
            _ => SpareTimer {
                since: now,
                seen: now,
            },
        };
        timers.insert(key, timer);
        Some(timer.since)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GI: u64 = 1 << 30;

    fn cfg() -> ScaleConfig {
        ScaleConfig::default()
    }

    fn mem_size(slots: u32, memory: u64) -> AgentSize {
        AgentSize {
            slots,
            cpu_millis: None,
            memory_bytes: Some(memory),
        }
    }

    fn tasks(memory: Option<u64>, count: u64) -> DemandGroup {
        DemandGroup {
            size: TaskSize {
                cpu_millis: None,
                memory_bytes: memory,
            },
            count,
        }
    }

    fn view(desired: u32, live: u32, min: u32, max: u32, needed: u32, busy: bool) -> PoolView {
        PoolView {
            desired,
            live,
            min,
            max,
            needed,
            busy,
            idle_since: None,
            last_scaled_down_at: None,
            spare_since: None,
        }
    }

    // ---- step down (CLOACINA-T-1145) ----------------------------------------

    /// A pool with work and a spare agent for the idle cooldown loses one
    /// agent per cooldown, down to its min.
    #[test]
    fn a_busy_pool_with_a_spare_agent_steps_down_one() {
        let mut v = view(3, 3, 1, 5, 0, true);
        // Spare, not long enough.
        v.spare_since = Some(now() - chrono::Duration::seconds(299));
        assert_eq!(decide_pool(&v, now(), &cfg()), PoolDecision::Hold);
        v.spare_since = Some(now() - chrono::Duration::seconds(301));
        assert_eq!(decide_pool(&v, now(), &cfg()), PoolDecision::StepDown(2));
        // A cooldown after the last scale-down.
        v.last_scaled_down_at = Some(now() - chrono::Duration::seconds(30));
        assert_eq!(decide_pool(&v, now(), &cfg()), PoolDecision::Hold);
        v.last_scaled_down_at = Some(now() - chrono::Duration::seconds(61));
        assert_eq!(decide_pool(&v, now(), &cfg()), PoolDecision::StepDown(2));
        // At its min: no lower.
        let mut at_min = view(1, 1, 1, 5, 0, true);
        at_min.spare_since = Some(now() - chrono::Duration::hours(1));
        assert_eq!(decide_pool(&at_min, now(), &cfg()), PoolDecision::Hold);
        // Agents still starting: no step.
        let mut starting = view(3, 2, 0, 5, 0, true);
        starting.spare_since = Some(now() - chrono::Duration::hours(1));
        assert_eq!(decide_pool(&starting, now(), &cfg()), PoolDecision::Hold);
        // Not spare: no step, whatever the pool's stored idle time.
        let mut busy = view(3, 3, 0, 5, 0, true);
        busy.idle_since = Some(now() - chrono::Duration::hours(1));
        assert_eq!(decide_pool(&busy, now(), &cfg()), PoolDecision::Hold);
    }

    /// Work that needs a new agent wins over a spare agent.
    #[test]
    fn waiting_work_that_needs_an_agent_is_not_a_step_down() {
        let mut v = view(3, 3, 0, 5, 1, true);
        v.spare_since = Some(now() - chrono::Duration::hours(1));
        assert_eq!(decide_pool(&v, now(), &cfg()), PoolDecision::Up(4));
    }

    /// A whole idle pool keeps the T-1029 rule: straight to its min.
    #[test]
    fn an_idle_pool_still_goes_straight_to_its_min() {
        let mut v = view(4, 4, 1, 5, 0, false);
        v.idle_since = Some(now() - chrono::Duration::hours(1));
        assert_eq!(decide_pool(&v, now(), &cfg()), PoolDecision::Down(1));
    }

    /// The spare timer runs while the pool stays spare, ends when it is
    /// not, and starts again after a gap (another replica led).
    #[test]
    fn the_spare_timer_runs_ends_and_starts_again_after_a_gap() {
        let timers = SpareTimers::default();
        let gap = Duration::from_secs(60);
        let t0 = now();
        let at = |s: i64| t0 + chrono::Duration::seconds(s);
        assert_eq!(timers.observe("t", "p", true, at(0), gap), Some(at(0)));
        assert_eq!(timers.observe("t", "p", true, at(30), gap), Some(at(0)));
        assert_eq!(
            timers.observe("t", "q", true, at(30), gap),
            Some(at(30)),
            "per pool"
        );
        assert_eq!(timers.observe("t", "p", false, at(60), gap), None);
        assert_eq!(timers.observe("t", "p", true, at(90), gap), Some(at(90)));
        // Not seen for longer than the gap: it starts again.
        assert_eq!(timers.observe("t", "p", true, at(200), gap), Some(at(200)));
    }

    fn now() -> chrono::NaiveDateTime {
        chrono::Utc::now().naive_utc()
    }

    // ---- pack_waiting -------------------------------------------------------

    /// The acceptance case: ten 2 Gi tasks, agents of 8 Gi, max 3. Four fit
    /// on an agent, so three agents hold twelve; the pool needs all three.
    #[test]
    fn ten_2gi_tasks_on_8gi_agents_need_three() {
        let p = pack_waiting(&[], mem_size(8, 8 * GI), &[tasks(Some(2 * GI), 10)], 3);
        assert_eq!(p.agents_needed, 3);
        assert_eq!(p.over_max, 0);
        // With a max of 2, two agents and two tasks over the max.
        let p = pack_waiting(&[], mem_size(8, 8 * GI), &[tasks(Some(2 * GI), 10)], 2);
        assert_eq!((p.agents_needed, p.over_max), (2, 2));
    }

    /// One waiting task on a pool at zero needs one agent.
    #[test]
    fn a_first_task_on_an_empty_pool_needs_one_agent() {
        let p = pack_waiting(&[], mem_size(4, 8 * GI), &[tasks(None, 1)], 5);
        assert_eq!(p.agents_needed, 1);
    }

    /// Work that fits on the free room of the live agents needs no agent.
    #[test]
    fn work_that_fits_on_live_agents_needs_none() {
        let size = mem_size(4, 8 * GI);
        let mut half = Room::empty(size);
        half.free_slots = 2;
        half.free_memory_bytes = Some(4 * GI);
        let p = pack_waiting(&[half], size, &[tasks(Some(2 * GI), 2)], 5);
        assert_eq!((p.agents_needed, p.fit_now), (0, 2));
        let p = pack_waiting(&[half], size, &[tasks(Some(2 * GI), 3)], 5);
        assert_eq!((p.agents_needed, p.fit_now), (1, 2));
    }

    /// By requests, not by count: one 24 Gi task and four 2 Gi tasks fit on
    /// one 32 Gi agent; three 24 Gi tasks need three.
    #[test]
    fn the_count_is_by_requests_not_by_tasks() {
        let size = mem_size(8, 32 * GI);
        let p = pack_waiting(
            &[],
            size,
            &[tasks(Some(2 * GI), 4), tasks(Some(24 * GI), 1)],
            10,
        );
        assert_eq!(p.agents_needed, 1);
        let p = pack_waiting(&[], size, &[tasks(Some(24 * GI), 3)], 10);
        assert_eq!(p.agents_needed, 3);
    }

    /// The slot count caps an agent whatever the requests.
    #[test]
    fn slots_cap_an_agent() {
        let p = pack_waiting(&[], mem_size(2, 64 * GI), &[tasks(Some(GI), 5)], 10);
        assert_eq!(p.agents_needed, 3);
        // An agent with no totals is packed by slots only.
        let slots_only = AgentSize {
            slots: 4,
            cpu_millis: None,
            memory_bytes: None,
        };
        let p = pack_waiting(&[], slots_only, &[tasks(Some(100 * GI), 9)], 10);
        assert_eq!(p.agents_needed, 3);
    }

    /// A task with no request counts as one slot's share.
    #[test]
    fn a_task_with_no_request_counts_one_slots_share() {
        // 4 slots of 8 Gi: a share is 2 Gi. Two 3 Gi tasks and two with none
        // take 10 Gi: two agents.
        let p = pack_waiting(
            &[],
            mem_size(4, 8 * GI),
            &[tasks(Some(3 * GI), 2), tasks(None, 2)],
            10,
        );
        assert_eq!(p.agents_needed, 2);
    }

    /// A task larger than one agent of the pool is not placed by more agents.
    #[test]
    fn a_task_larger_than_an_agent_is_unplaceable() {
        let p = pack_waiting(
            &[],
            mem_size(4, 8 * GI),
            &[tasks(Some(16 * GI), 2), tasks(Some(GI), 1)],
            10,
        );
        assert_eq!((p.agents_needed, p.unplaceable), (1, 2));
    }

    // ---- decide_pool --------------------------------------------------------

    #[test]
    fn a_pool_at_zero_starts_what_the_work_needs() {
        assert_eq!(
            decide_pool(&view(0, 0, 0, 3, 1, true), now(), &cfg()),
            PoolDecision::Up(1)
        );
        assert_eq!(
            decide_pool(&view(0, 0, 0, 3, 3, true), now(), &cfg()),
            PoolDecision::Up(3)
        );
    }

    /// Agents that are starting are counted: no more while they start.
    #[test]
    fn starting_agents_are_counted() {
        // Desired 3, one live, two more needed: three is enough.
        assert_eq!(
            decide_pool(&view(3, 1, 0, 5, 2, true), now(), &cfg()),
            PoolDecision::Hold
        );
        // Three more needed: four.
        assert_eq!(
            decide_pool(&view(3, 1, 0, 5, 3, true), now(), &cfg()),
            PoolDecision::Up(4)
        );
    }

    #[test]
    fn the_max_holds() {
        assert_eq!(
            decide_pool(&view(1, 1, 0, 3, 9, true), now(), &cfg()),
            PoolDecision::Up(3)
        );
        assert_eq!(
            decide_pool(&view(3, 3, 0, 3, 2, true), now(), &cfg()),
            PoolDecision::Hold
        );
    }

    /// The min holds (a pool's `min_agents`, the floor of `default`, the
    /// poll floor of CLOACINA-T-1077).
    #[test]
    fn the_min_holds() {
        assert_eq!(
            decide_pool(&view(0, 0, 1, 3, 0, false), now(), &cfg()),
            PoolDecision::Up(1)
        );
        let mut v = view(3, 3, 1, 3, 0, false);
        v.idle_since = Some(now() - chrono::Duration::hours(1));
        assert_eq!(decide_pool(&v, now(), &cfg()), PoolDecision::Down(1));
    }

    #[test]
    fn an_idle_pool_goes_to_its_min_after_the_idle_cooldown() {
        let mut v = view(3, 3, 0, 3, 0, false);
        // Idle, not marked yet.
        assert_eq!(decide_pool(&v, now(), &cfg()), PoolDecision::Hold);
        v.idle_since = Some(now() - chrono::Duration::seconds(299));
        assert_eq!(decide_pool(&v, now(), &cfg()), PoolDecision::Hold);
        v.idle_since = Some(now() - chrono::Duration::seconds(301));
        assert_eq!(decide_pool(&v, now(), &cfg()), PoolDecision::Down(0));
        // A scale-down a moment ago: the cooldown holds the next.
        v.last_scaled_down_at = Some(now() - chrono::Duration::seconds(10));
        assert_eq!(decide_pool(&v, now(), &cfg()), PoolDecision::Hold);
    }

    /// A pool with work is never shrunk, whatever its stored idle time.
    #[test]
    fn a_busy_pool_is_not_shrunk() {
        let mut v = view(3, 3, 0, 3, 0, true);
        v.idle_since = Some(now() - chrono::Duration::hours(1));
        assert_eq!(decide_pool(&v, now(), &cfg()), PoolDecision::Hold);
        // A max lowered under the count waits for the pool to be idle too.
        let mut v = view(5, 5, 0, 3, 0, true);
        v.idle_since = Some(now() - chrono::Duration::hours(1));
        assert_eq!(decide_pool(&v, now(), &cfg()), PoolDecision::Hold);
        v.busy = false;
        assert_eq!(decide_pool(&v, now(), &cfg()), PoolDecision::Down(0));
    }

    // ---- should_act_at --------------------------------------------------------

    #[test]
    fn should_act_at_rules() {
        let now = now();
        let cd = Duration::from_secs(60);
        assert!(should_act_at(None, now, cd));
        assert!(!should_act_at(
            Some(now - chrono::Duration::seconds(10)),
            now,
            cd
        ));
        assert!(should_act_at(
            Some(now - chrono::Duration::seconds(60)),
            now,
            cd
        ));
        assert!(!should_act_at(
            Some(now + chrono::Duration::seconds(5)),
            now,
            cd
        ));
    }

    #[test]
    fn config_defaults() {
        let c = ScaleConfig::default();
        assert_eq!(c.cooldown, Duration::from_secs(60));
        assert_eq!(c.idle, Duration::from_secs(300));
        assert_eq!(c.floor, 0);
        assert_eq!(c.interval, Duration::from_secs(30));
    }
}
