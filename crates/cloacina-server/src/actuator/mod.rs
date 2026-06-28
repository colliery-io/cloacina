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

pub mod docker;
pub mod guard;
pub mod kubernetes;

use async_trait::async_trait;

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
}

impl std::fmt::Display for ActuatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActuatorError::Substrate(m) => write!(f, "container substrate error: {m}"),
            ActuatorError::KeyMint(m) => write!(f, "agent-key mint error: {m}"),
        }
    }
}

impl std::error::Error for ActuatorError {}

/// Reconciles a tenant's *actual* agent count toward its *desired* count.
///
/// Implementations are substrate-specific (Docker today; Kubernetes later).
/// `reconcile` is idempotent: called repeatedly by the server's background loop
/// with the current desired count, it actuates only the delta.
#[async_trait]
pub trait FleetActuator: Send + Sync {
    /// Drive the tenant's running agent count toward `desired`, returning what
    /// changed. Must never touch another tenant's runtimes (NFR-004).
    async fn reconcile(
        &self,
        tenant_id: &str,
        desired: u32,
    ) -> Result<ReconcileOutcome, ActuatorError>;

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
