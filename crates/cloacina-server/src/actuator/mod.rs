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

//! Pluggable fleet actuator (CLOACI-T-0810, CLOACI-I-0127).
//!
//! The control plane (T-0809) owns each tenant's `desired_count`; the
//! **actuator** reconciles *actual → desired* by spawning/stopping
//! `cloacina-agent` runtimes. Two substrates ship: a Docker-container actuator
//! for dev (see [`docker`]) and a Kubernetes actuator for production (see
//! [`kubernetes`], CLOACI-T-0814) that scales a per-tenant agent `Deployment`
//! in the tenant's own namespace.
//!
//! Substrate selection is explicit + validated at boot, fail-closed, by the
//! [`guard`] module (REQ-008): a misconfigured actuator REFUSES to start rather
//! than silently scaling the wrong thing.

pub mod compiler;
pub mod docker;
pub mod guard;
pub mod kubernetes;
pub mod settings;

use async_trait::async_trait;
use cloacina::task_logs::LogFormat;

/// The server setting for the log format of the agents the actuator starts
/// (CLOACINA-T-1101). It is also the name of the variable an agent reads, and
/// the actuator sets it on each agent it starts.
pub const AGENT_LOG_FORMAT_ENV: &str = "CLOACINA_AGENT_LOG_FORMAT";

/// The log format of the agents the actuator starts, from the server's
/// environment ([`AGENT_LOG_FORMAT_ENV`]): `text` or `json`. Not set, or
/// empty: `text`. Another value is an error: the server must not start with
/// it.
pub fn agent_log_format_from_env() -> Result<LogFormat, String> {
    agent_log_format_from(std::env::var(AGENT_LOG_FORMAT_ENV).ok().as_deref())
}

/// [`agent_log_format_from_env`] over a given value, so tests need no process
/// environment.
pub fn agent_log_format_from(raw: Option<&str>) -> Result<LogFormat, String> {
    match raw.map(str::trim).filter(|v| !v.is_empty()) {
        None => Ok(LogFormat::default()),
        Some(value) => value
            .parse::<LogFormat>()
            .map_err(|e| format!("{AGENT_LOG_FORMAT_ENV}: {e}")),
    }
}

/// The value of [`AGENT_LOG_FORMAT_ENV`] for an agent the actuator starts.
/// `None` for `text`, the agent's own default: the environment of the agent
/// is then as it was before the setting, and an upgrade rolls no pod.
pub fn agent_log_format_env_value(format: LogFormat) -> Option<&'static str> {
    match format {
        LogFormat::Text => None,
        LogFormat::Json => Some("json"),
    }
}

/// The expiry and rotation rule of the agent keys the Kubernetes actuator
/// mints (CLOACINA-T-1083). A key expires `lifetime` after it is minted. When
/// less than `rotate_before` is left, the key is no longer live for the
/// actuator: the next reconcile mints a new key and rolls the pods onto it.
/// Docker actuator keys have no expiry: the Docker actuator is single user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentKeyExpiry {
    pub lifetime: chrono::Duration,
    pub rotate_before: chrono::Duration,
}

impl AgentKeyExpiry {
    /// Default lifetime of an agent key, in days.
    pub const DEFAULT_LIFETIME_DAYS: u32 = 30;
    /// Default days left at which an agent key is rotated.
    pub const DEFAULT_ROTATE_DAYS: u32 = 10;

    /// The rule from the server settings. The rotation window must be at
    /// least a day and shorter than the lifetime: else every key would be due
    /// at once.
    pub fn from_days(lifetime_days: u32, rotate_days: u32) -> Result<Self, String> {
        if rotate_days == 0 {
            return Err("--agent-key-rotate-days must be at least 1".to_string());
        }
        if rotate_days >= lifetime_days {
            return Err(format!(
                "--agent-key-rotate-days ({rotate_days}) must be less than \
                 --agent-key-lifetime-days ({lifetime_days})"
            ));
        }
        Ok(Self {
            lifetime: chrono::Duration::days(i64::from(lifetime_days)),
            rotate_before: chrono::Duration::days(i64::from(rotate_days)),
        })
    }

    /// The expiry of a key minted at `now`.
    pub fn expires_at(&self, now: chrono::DateTime<chrono::Utc>) -> chrono::DateTime<chrono::Utc> {
        now + self.lifetime
    }

    /// True when a key that expires at `expires_at` must be replaced now:
    /// less than `rotate_before` is left. A key with no expiry is not due.
    pub fn is_due(
        &self,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        now: chrono::DateTime<chrono::Utc>,
    ) -> bool {
        expires_at.is_some_and(|at| at - now < self.rotate_before)
    }
}

impl Default for AgentKeyExpiry {
    fn default() -> Self {
        Self::from_days(Self::DEFAULT_LIFETIME_DAYS, Self::DEFAULT_ROTATE_DAYS)
            .expect("the default agent key expiry is valid")
    }
}

static AGENT_KEY_EXPIRY: std::sync::OnceLock<AgentKeyExpiry> = std::sync::OnceLock::new();

/// Set the agent key expiry rule from the server settings. Called once at
/// start, before the actuator is built. A second call keeps the first value.
pub fn set_agent_key_expiry(expiry: AgentKeyExpiry) {
    let _ = AGENT_KEY_EXPIRY.set(expiry);
}

/// The agent key expiry rule: the server setting, or the default.
pub fn agent_key_expiry() -> AgentKeyExpiry {
    AGENT_KEY_EXPIRY.get().copied().unwrap_or_default()
}

/// Tells every server that keys were revoked, so each clears its key cache
/// (CLOACINA-T-1128). The actuator sends it after it revokes agent keys.
#[async_trait]
pub trait KeyRevocationNotice: Send + Sync {
    async fn announce(&self);
}

/// What a single [`FleetActuator::reconcile`] pass did for one tenant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReconcileOutcome {
    /// Agent runtimes started this pass.
    pub spawned: u32,
    /// Agent runtimes stopped this pass.
    pub stopped: u32,
    /// Agent runtimes running for the tenant *after* this pass.
    pub running: u32,
}

/// Errors surfaced by an actuator while reconciling.
#[derive(Debug)]
pub enum ActuatorError {
    /// The container substrate (Docker daemon / engine API) failed.
    Substrate(String),
    /// Minting the per-tenant agent key failed.
    KeyMint(String),
    /// Reading the tenant's agent pools failed. The reconcile stops: it must
    /// not fall back to the defaults, which would resize the agents.
    Settings(String),
}

impl std::fmt::Display for ActuatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActuatorError::Substrate(m) => write!(f, "container substrate error: {m}"),
            ActuatorError::KeyMint(m) => write!(f, "agent-key mint error: {m}"),
            ActuatorError::Settings(m) => write!(f, "agent pool settings error: {m}"),
        }
    }
}

impl std::error::Error for ActuatorError {}

/// Reconciles a tenant's *actual* agent count toward its *desired* count.
///
/// Implementations are substrate-specific (Docker for dev, Kubernetes for
/// production). `reconcile` is idempotent: called repeatedly by the server's
/// background loop with the current desired count, it actuates only the delta.
///
/// A tenant's agents are grouped in pools (CLOACINA-T-1021). `desired` is the
/// count of the pool `default`; the actuator reads the tenant's other pools
/// and their fixed counts itself, and reconciles each pool.
#[async_trait]
pub trait FleetActuator: Send + Sync {
    /// Drive the tenant's running agents toward what is wanted: `desired`
    /// agents in the pool `default`, and each other pool at its own count.
    /// The outcome sums the pools. Must never touch another tenant's runtimes
    /// (NFR-004).
    async fn reconcile(
        &self,
        tenant_id: &str,
        desired: u32,
    ) -> Result<ReconcileOutcome, ActuatorError>;

    /// Remove what the actuator made for one pool of a tenant (its agents and
    /// the objects that belong to the pool alone). The caller has checked that
    /// the pool has no agents. Idempotent; never called for `default`.
    async fn remove_pool(&self, _tenant_id: &str, _pool: &str) -> Result<(), ActuatorError> {
        Ok(())
    }

    /// Remove every agent the actuator made for a tenant, in each pool, and
    /// the objects that hold its agent key (CLOACINA-T-1095). Called when the
    /// tenant is removed, before its pool rows are deleted. Idempotent: a
    /// second call, or a call for a tenant with no agents, succeeds and makes
    /// nothing. The default does nothing: an actuator that runs no agents.
    async fn remove_agents(&self, _tenant_id: &str) -> Result<(), ActuatorError> {
        Ok(())
    }

    /// Move the tenant's compiler toward `want` (CLOACINA-T-1034): one
    /// compiler while the tenant's build queue has work, none after the queue
    /// has been empty for the cooldown. Idempotent. The default does nothing:
    /// an actuator with no per-tenant compilers.
    async fn reconcile_compiler(
        &self,
        _tenant_id: &str,
        _want: compiler::CompilerWant,
    ) -> Result<compiler::CompilerOutcome, ActuatorError> {
        Ok(compiler::CompilerOutcome::default())
    }

    /// Remove what the actuator made for a tenant's compiler. Called when the
    /// tenant is removed. Idempotent.
    async fn remove_compiler(&self, _tenant_id: &str) -> Result<(), ActuatorError> {
        Ok(())
    }

    /// The cooldown of the per-tenant compilers. `None`: this actuator runs
    /// none, and the fleet loop reads no build queue.
    fn compiler_cooldown(&self) -> Option<std::time::Duration> {
        None
    }

    /// A short, stable identifier for the actuator kind (`"docker"`, `"none"`,
    /// …). `"none"` signals the background reconcile loop should not run.
    fn kind(&self) -> &'static str;
}

/// The number of runtimes to start or stop to move `running` toward `desired`.
///
/// Exactly one of the two is non-zero (or both zero when already converged).
/// Pure + saturating so it can be unit-tested in isolation from any substrate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconcileDelta {
    pub to_spawn: u32,
    pub to_stop: u32,
}

/// Compute the spawn/stop delta to move `running` toward `desired`.
///
/// `desired > running` → spawn the difference; `running > desired` → stop the
/// difference; equal → no-op. Saturating subtraction throughout.
pub fn reconcile_delta(running: u32, desired: u32) -> ReconcileDelta {
    if desired >= running {
        ReconcileDelta {
            to_spawn: desired - running,
            to_stop: 0,
        }
    } else {
        ReconcileDelta {
            to_spawn: 0,
            to_stop: running - desired,
        }
    }
}

/// The default actuator when actuation is off (`CLOACINA_FLEET_ACTUATOR=none`).
///
/// `reconcile` is a no-op that just echoes the desired count back as `running`;
/// `kind()` is `"none"`, which the server uses to skip the reconcile loop.
pub struct NoopActuator;

#[async_trait]
impl FleetActuator for NoopActuator {
    async fn reconcile(
        &self,
        _tenant_id: &str,
        desired: u32,
    ) -> Result<ReconcileOutcome, ActuatorError> {
        Ok(ReconcileOutcome {
            spawned: 0,
            stopped: 0,
            running: desired,
        })
    }

    fn kind(&self) -> &'static str {
        "none"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_agent_log_format_is_text_unless_json_is_set() {
        assert_eq!(agent_log_format_from(None), Ok(LogFormat::Text));
        assert_eq!(agent_log_format_from(Some("")), Ok(LogFormat::Text));
        assert_eq!(agent_log_format_from(Some("  ")), Ok(LogFormat::Text));
        assert_eq!(agent_log_format_from(Some("text")), Ok(LogFormat::Text));
        assert_eq!(agent_log_format_from(Some("json")), Ok(LogFormat::Json));
        assert_eq!(agent_log_format_from(Some(" JSON ")), Ok(LogFormat::Json));
    }

    /// The server stops at start on this error: it names the setting.
    #[test]
    fn an_agent_log_format_that_is_not_text_or_json_is_an_error() {
        for bad in ["yaml", "true", "text,json"] {
            let err = agent_log_format_from(Some(bad)).unwrap_err();
            assert!(err.starts_with("CLOACINA_AGENT_LOG_FORMAT: "), "{err}");
            assert!(err.contains("`text` or `json`"), "{err}");
        }
    }

    #[test]
    fn the_agent_gets_the_variable_only_for_json() {
        assert_eq!(agent_log_format_env_value(LogFormat::Text), None);
        assert_eq!(agent_log_format_env_value(LogFormat::Json), Some("json"));
    }

    #[test]
    fn delta_spawns_when_under_desired() {
        assert_eq!(
            reconcile_delta(1, 4),
            ReconcileDelta {
                to_spawn: 3,
                to_stop: 0
            }
        );
    }

    #[test]
    fn delta_stops_when_over_desired() {
        assert_eq!(
            reconcile_delta(5, 2),
            ReconcileDelta {
                to_spawn: 0,
                to_stop: 3
            }
        );
    }

    #[test]
    fn delta_noop_when_converged() {
        assert_eq!(
            reconcile_delta(3, 3),
            ReconcileDelta {
                to_spawn: 0,
                to_stop: 0
            }
        );
    }

    #[test]
    fn delta_from_zero_running() {
        assert_eq!(
            reconcile_delta(0, 2),
            ReconcileDelta {
                to_spawn: 2,
                to_stop: 0
            }
        );
    }

    #[test]
    fn delta_drain_to_zero() {
        assert_eq!(
            reconcile_delta(3, 0),
            ReconcileDelta {
                to_spawn: 0,
                to_stop: 3
            }
        );
    }

    #[tokio::test]
    async fn noop_actuator_is_inert() {
        let a = NoopActuator;
        assert_eq!(a.kind(), "none");
        let out = a.reconcile("acme", 7).await.unwrap();
        assert_eq!(out.spawned, 0);
        assert_eq!(out.stopped, 0);
        assert_eq!(out.running, 7);
    }
}
