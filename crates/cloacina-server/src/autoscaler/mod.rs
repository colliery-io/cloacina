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

//! Back-pressure autoscaler (CLOACI-T-0811, CLOACI-I-0127).
//!
//! A leader-gated control loop adjusts each tenant's `desired_count` (T-0809
//! `AgentDesiredDAL`) up or down based on the fleet's **utilization**, clamped to
//! `[floor, effective_limit]` (T-0808 `AgentLimitsDAL::effective_limit`). The
//! T-0810 actuator then reconciles *actual → desired*. The decision lives in our
//! control plane, not in K8s HPA: HPA cannot see per-tenant signal, and a tenant
//! is the isolation boundary (REQ-008).
//!
//! This module holds the **pure** decision pieces (no DB, no clock side-effects)
//! so they're unit-testable in isolation:
//! - [`decide`] — Up / Down / Hold from utilization vs thresholds + clamp.
//! - [`should_act_at`] — cooldown gate (testable without a DB).
//! - [`tenant_utilizations`] — per-tenant utilization from a roster snapshot.
//!
//! Leadership (the Postgres advisory lock that ensures only ONE replica runs the
//! loop) lives in [`leader`]; the loop wiring lives in `lib.rs`.
//!
//! ## Signal (v1: utilization)
//! Per-tenant utilization = Σ`in_flight` / Σ`max_concurrency` over that tenant's
//! live agents (heartbeat-updated). Σ`max_concurrency == 0` → utilization `0.0`
//! (don't scale up a tenant that has no live capacity to begin with).
//!
//! NOTE (future refinement): the per-tenant **Ready-task backlog** would be a
//! better *leading* indicator (utilization is reactive — it only rises once work
//! is already running and the fleet is saturated). Backlog lives in per-tenant
//! schemas, so reading it here means a per-tenant DAL hop per tick; deferred.
//! Utilization is the v1 signal.

pub mod leader;

use crate::agent_registry::AgentRecord;
use std::collections::HashMap;
use std::time::Duration;

/// The scaling action the autoscaler decides for one tenant on one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleAction {
    /// Utilization is over the up-threshold and we're under the effective limit
    /// → request one more agent.
    Up,
    /// Utilization is under the down-threshold and we're above the floor →
    /// release one agent.
    Down,
    /// In the hysteresis band, or already clamped at the limit/floor → no change.
    Hold,
}

/// Tunable knobs for the back-pressure autoscaler, sourced from the environment
/// at boot via [`ScaleConfig::from_env`].
#[derive(Debug, Clone)]
pub struct ScaleConfig {
    /// Scale UP when utilization is strictly greater than this (default `0.8`).
    pub up_threshold: f64,
    /// Scale DOWN when utilization is strictly less than this (default `0.2`).
    /// The gap between `down_threshold` and `up_threshold` is the hysteresis
    /// band that prevents thrash.
    pub down_threshold: f64,
    /// Minimum wall-clock between consecutive scale changes for a tenant
    /// (default 60s). Enforced by [`should_act_at`].
    pub cooldown: Duration,
    /// Lower bound on a tenant's `desired_count` (default `0`). Scale-down never
    /// goes below this.
    pub floor: u32,
    /// How often the control loop ticks (default 30s).
    pub interval: Duration,
}

impl Default for ScaleConfig {
    fn default() -> Self {
        Self {
            up_threshold: 0.8,
            down_threshold: 0.2,
            cooldown: Duration::from_secs(60),
            floor: 0,
            interval: Duration::from_secs(30),
        }
    }
}

impl ScaleConfig {
    /// Build from `CLOACINA_AUTOSCALE_*` env vars, falling back to the defaults
    /// for any var that is unset or unparsable:
    /// - `CLOACINA_AUTOSCALE_UP_THRESHOLD`   = 0.8
    /// - `CLOACINA_AUTOSCALE_DOWN_THRESHOLD` = 0.2
    /// - `CLOACINA_AUTOSCALE_COOLDOWN_S`     = 60
    /// - `CLOACINA_AUTOSCALE_FLOOR`          = 0
    /// - `CLOACINA_AUTOSCALE_INTERVAL_S`     = 30
    pub fn from_env() -> Self {
        let d = Self::default();
        let up_threshold = env_parse("CLOACINA_AUTOSCALE_UP_THRESHOLD").unwrap_or(d.up_threshold);
        let down_threshold =
            env_parse("CLOACINA_AUTOSCALE_DOWN_THRESHOLD").unwrap_or(d.down_threshold);
        let cooldown = env_parse::<u64>("CLOACINA_AUTOSCALE_COOLDOWN_S")
            .map(Duration::from_secs)
            .unwrap_or(d.cooldown);
        let floor = env_parse("CLOACINA_AUTOSCALE_FLOOR").unwrap_or(d.floor);
        let interval = env_parse::<u64>("CLOACINA_AUTOSCALE_INTERVAL_S")
            .filter(|n| *n > 0)
            .map(Duration::from_secs)
            .unwrap_or(d.interval);
        Self {
            up_threshold,
            down_threshold,
            cooldown,
            floor,
            interval,
        }
    }
}

/// Parse an env var into `T`, returning `None` if unset or unparsable.
fn env_parse<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|s| s.parse::<T>().ok())
}

/// Decide whether to scale a tenant Up / Down / Hold.
///
/// Pure: utilization in, action out. The loop turns `Up`/`Down` into a clamped
/// `desired ± 1` and a `set_desired` write.
///
/// - `Up`   if `util > cfg.up_threshold` **and** `desired < effective_limit`.
/// - `Down` if `util < cfg.down_threshold` **and** `desired > floor`.
/// - `Hold` otherwise — i.e. in the hysteresis band, or already clamped at the
///   effective limit (can't go up) or the floor (can't go down).
///
/// The clamp lives here, not just in the loop: deciding `Up` at the limit (or
/// `Down` at the floor) would burn a cooldown on a no-op write.
pub fn decide(
    util: f64,
    desired: u32,
    effective_limit: u32,
    floor: u32,
    cfg: &ScaleConfig,
) -> ScaleAction {
    if util > cfg.up_threshold && desired < effective_limit {
        ScaleAction::Up
    } else if util < cfg.down_threshold && desired > floor {
        ScaleAction::Down
    } else {
        ScaleAction::Hold
    }
}

/// Cooldown gate: may we act on this tenant now?
///
/// Pure (no DB, clock injected as `now`) so it's testable in isolation. Unlike a
/// monotonic [`Instant`], the last-action time is a **wall-clock**
/// `NaiveDateTime` read back from the DB (`agent_desired_counts.last_autoscaled_at`),
/// so the cooldown is coordinated across replicas: leadership rotates per tick,
/// and a replica leading a later tick still sees a peer's recent scale action
/// (CLOACI-A-0008 refinement). Both times are UTC (`chrono::Utc::now().naive_utc()`
/// and the DB's `now()`).
///
/// - `None` (never autoscaled) → `true`.
/// - Within `cooldown` of the last action → `false`.
/// - At/after `cooldown` has elapsed → `true`.
///
/// A negative elapsed (clock skew between replicas, or `last` slightly ahead of
/// `now`) is treated as "still cooling down" → `false`, the conservative choice.
pub fn should_act_at(
    last: Option<chrono::NaiveDateTime>,
    now: chrono::NaiveDateTime,
    cooldown: Duration,
) -> bool {
    match last {
        None => true,
        Some(t) => match (now - t).to_std() {
            // Non-negative elapsed: gate on the cooldown as usual.
            Ok(elapsed) => elapsed >= cooldown,
            // Negative elapsed (now < last, e.g. cross-replica skew): not yet.
            Err(_) => false,
        },
    }
}

/// Compute per-tenant utilization from a roster snapshot.
///
/// Utilization = Σ`in_flight` / Σ`max_concurrency` over the tenant's agents.
/// Only tenant-scoped agents (`tenant_id = Some(..)`) are counted — a global
/// (`None`) agent has no `desired_count` to scale, so it's excluded. A tenant
/// whose summed `max_concurrency` is `0` (e.g. every live agent advertises zero
/// capacity) maps to `0.0` rather than dividing by zero — we don't scale up a
/// tenant with no usable capacity on the strength of a degenerate ratio.
pub fn tenant_utilizations(records: &[AgentRecord]) -> HashMap<String, f64> {
    // tenant -> (Σ in_flight, Σ max_concurrency)
    let mut sums: HashMap<String, (u64, u64)> = HashMap::new();
    for r in records {
        if let Some(tenant) = &r.tenant_id {
            let e = sums.entry(tenant.clone()).or_insert((0, 0));
            e.0 += u64::from(r.in_flight);
            e.1 += u64::from(r.max_concurrency);
        }
    }
    sums.into_iter()
        .map(|(tenant, (in_flight, max))| {
            let util = if max == 0 {
                0.0
            } else {
                in_flight as f64 / max as f64
            };
            (tenant, util)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn cfg() -> ScaleConfig {
        ScaleConfig::default()
    }

    fn rec(tenant: Option<&str>, max: u32, in_flight: u32) -> AgentRecord {
        AgentRecord {
            agent_id: format!("agent-{}-{}", tenant.unwrap_or("global"), in_flight),
            max_concurrency: max,
            in_flight,
            available_capacity: max.saturating_sub(in_flight),
            target_triple: "aarch64-apple-darwin".to_string(),
            capabilities: vec![],
            last_heartbeat: Instant::now(),
            tenant_id: tenant.map(|s| s.to_string()),
        }
    }

    // ---- decide() — every branch -------------------------------------------

    #[test]
    fn decide_up_when_over_threshold_and_under_limit() {
        // util 0.9 > 0.8, desired 2 < limit 4 → Up
        assert_eq!(decide(0.9, 2, 4, 0, &cfg()), ScaleAction::Up);
    }

    #[test]
    fn decide_hold_when_over_threshold_but_at_limit() {
        // util 0.9 > 0.8 but desired 4 == limit 4 → Hold (clamped at the ceiling)
        assert_eq!(decide(0.9, 4, 4, 0, &cfg()), ScaleAction::Hold);
    }

    #[test]
    fn decide_down_when_under_threshold_and_above_floor() {
        // util 0.1 < 0.2, desired 3 > floor 1 → Down
        assert_eq!(decide(0.1, 3, 4, 1, &cfg()), ScaleAction::Down);
    }

    #[test]
    fn decide_hold_when_under_threshold_but_at_floor() {
        // util 0.1 < 0.2 but desired 1 == floor 1 → Hold (clamped at the floor)
        assert_eq!(decide(0.1, 1, 4, 1, &cfg()), ScaleAction::Hold);
    }

    #[test]
    fn decide_hold_in_hysteresis_band() {
        // util 0.5 is between down (0.2) and up (0.8) → Hold regardless of room
        assert_eq!(decide(0.5, 2, 4, 0, &cfg()), ScaleAction::Hold);
    }

    #[test]
    fn decide_hold_at_exact_thresholds() {
        // Strict comparisons: exactly at up- or down-threshold holds.
        assert_eq!(decide(0.8, 1, 4, 0, &cfg()), ScaleAction::Hold);
        assert_eq!(decide(0.2, 3, 4, 0, &cfg()), ScaleAction::Hold);
    }

    // ---- tenant_utilizations() ---------------------------------------------

    #[test]
    fn utilization_is_per_tenant() {
        // t1: two agents, Σin_flight=3+1=4, Σmax=4+4=8 → 0.5
        // t2: one agent,  Σin_flight=8,    Σmax=8     → 1.0
        let records = vec![
            rec(Some("t1"), 4, 3),
            rec(Some("t1"), 4, 1),
            rec(Some("t2"), 8, 8),
        ];
        let utils = tenant_utilizations(&records);
        assert_eq!(utils.len(), 2);
        assert!((utils["t1"] - 0.5).abs() < f64::EPSILON);
        assert!((utils["t2"] - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn utilization_guards_zero_capacity() {
        // A tenant whose live agents advertise zero max_concurrency → 0.0,
        // not a divide-by-zero / NaN.
        let records = vec![rec(Some("t1"), 0, 0)];
        let utils = tenant_utilizations(&records);
        assert_eq!(utils["t1"], 0.0);
    }

    #[test]
    fn utilization_excludes_global_agents() {
        // A None-tenant (global) agent has no desired_count to scale → excluded.
        let records = vec![rec(None, 4, 4), rec(Some("t1"), 4, 2)];
        let utils = tenant_utilizations(&records);
        assert_eq!(utils.len(), 1);
        assert!((utils["t1"] - 0.5).abs() < f64::EPSILON);
    }

    // ---- should_act_at() — cross-replica (wall-clock) cooldown -------------

    #[test]
    fn should_act_at_when_never_scaled() {
        // None (no last_autoscaled_at row / never autoscaled) → always allowed.
        let now = chrono::Utc::now().naive_utc();
        assert!(should_act_at(None, now, Duration::from_secs(60)));
    }

    #[test]
    fn should_not_act_at_within_cooldown() {
        let now = chrono::Utc::now().naive_utc();
        let last = now - chrono::Duration::seconds(10);
        assert!(!should_act_at(Some(last), now, Duration::from_secs(60)));
    }

    #[test]
    fn should_act_at_after_cooldown() {
        let now = chrono::Utc::now().naive_utc();
        let last = now - chrono::Duration::seconds(90);
        assert!(should_act_at(Some(last), now, Duration::from_secs(60)));
    }

    #[test]
    fn should_act_at_exactly_at_cooldown() {
        // Boundary: elapsed == cooldown is allowed (>=).
        let now = chrono::Utc::now().naive_utc();
        let last = now - chrono::Duration::seconds(60);
        assert!(should_act_at(Some(last), now, Duration::from_secs(60)));
    }

    #[test]
    fn should_not_act_at_with_future_last() {
        // Cross-replica clock skew: last is ahead of now → conservatively wait.
        let now = chrono::Utc::now().naive_utc();
        let last = now + chrono::Duration::seconds(5);
        assert!(!should_act_at(Some(last), now, Duration::from_secs(60)));
    }

    // ---- ScaleConfig::from_env ---------------------------------------------

    #[test]
    fn config_defaults_are_sane() {
        let c = ScaleConfig::default();
        assert_eq!(c.up_threshold, 0.8);
        assert_eq!(c.down_threshold, 0.2);
        assert_eq!(c.cooldown, Duration::from_secs(60));
        assert_eq!(c.floor, 0);
        assert_eq!(c.interval, Duration::from_secs(30));
    }
}
