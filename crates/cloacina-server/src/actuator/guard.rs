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

//! Substrate guard (CLOACI-T-0810, REQ-008).
//!
//! Actuator selection is **explicit** (`CLOACINA_FLEET_ACTUATOR`) and validated
//! at boot, **fail-closed**: the Docker actuator REFUSES to start when
//! Kubernetes is detected (so we never scale Docker containers on a host whose
//! real substrate is a cluster) and refuses when no Docker socket is reachable.
//! A misconfigured actuator must produce a loud boot error, never silent
//! wrong-scaling.
//!
//! The Kubernetes / Docker-socket probes are abstracted behind the [`Substrate`]
//! trait so the decision logic ([`evaluate`]) is unit-testable without touching
//! the real host.

use std::sync::Arc;
use std::path::Path;

use cloacina::database::Database;

use super::docker::DockerActuator;
use super::{FleetActuator, NoopActuator};

/// In-cluster Kubernetes service-account token mount. Its presence (or the
/// `KUBERNETES_SERVICE_HOST` env var) means we are running inside a pod.
const K8S_SA_TOKEN_PATH: &str = "/var/run/secrets/kubernetes.io/serviceaccount/token";

/// Default Docker daemon unix socket.
const DOCKER_SOCK_PATH: &str = "/var/run/docker.sock";

/// Errors from building the actuator at boot. Every variant is fatal — the
/// server must refuse to start (fail-closed) when `build_actuator` returns one.
#[derive(Debug)]
pub enum GuardError {
    /// The selected actuator is unsafe on the detected substrate (e.g. Docker
    /// actuator selected while running inside Kubernetes).
    Refused(String),
    /// The selected substrate is unreachable (e.g. no Docker socket).
    Unavailable(String),
    /// The selected actuator is recognized but not implemented here.
    NotImplemented(String),
    /// `CLOACINA_FLEET_ACTUATOR` held an unrecognized value.
    Unknown(String),
}

impl std::fmt::Display for GuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GuardError::Refused(m) => write!(f, "fleet actuator refused (fail-closed): {m}"),
            GuardError::Unavailable(m) => write!(f, "fleet actuator substrate unavailable: {m}"),
            GuardError::NotImplemented(m) => write!(f, "fleet actuator not implemented: {m}"),
            GuardError::Unknown(m) => write!(
                f,
                "unknown CLOACINA_FLEET_ACTUATOR value {m:?} (expected one of: none, docker, kubernetes)"
            ),
        }
    }
}

impl std::error::Error for GuardError {}

/// Probes the runtime substrate. Injectable so [`evaluate`] is testable without
/// depending on the real host's env / filesystem / Docker daemon.
pub trait Substrate {
    /// Whether we appear to be running inside Kubernetes (SA token mount present
    /// or `KUBERNETES_SERVICE_HOST` set).
    fn kubernetes_detected(&self) -> bool;
    /// Whether a Docker daemon socket appears reachable.
    fn docker_socket_reachable(&self) -> bool;
}

/// The real host substrate: reads env vars + the filesystem.
pub struct HostSubstrate;

impl Substrate for HostSubstrate {
    fn kubernetes_detected(&self) -> bool {
        std::env::var_os("KUBERNETES_SERVICE_HOST").is_some()
            || Path::new(K8S_SA_TOKEN_PATH).exists()
    }

    fn docker_socket_reachable(&self) -> bool {
        // An explicit DOCKER_HOST (tcp:// or unix://) is an operator-declared
        // daemon endpoint; otherwise fall back to the default unix socket.
        std::env::var_os("DOCKER_HOST").is_some() || Path::new(DOCKER_SOCK_PATH).exists()
    }
}

/// The validated outcome of [`evaluate`]: which actuator to construct. Kept
/// separate from construction so the decision can be unit-tested without a
/// Docker client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Actuation off — build a `NoopActuator`.
    Noop,
    /// Build the Docker actuator.
    Docker,
}

/// Pure decision logic for the substrate guard (REQ-008). Unit-tested across
/// every branch; performs no construction and no I/O beyond the injected
/// [`Substrate`] probes.
///
/// - `none` → [`Decision::Noop`].
/// - `docker` → **refuse** if Kubernetes is detected; **unavailable** if no
///   Docker socket; else [`Decision::Docker`].
/// - `kubernetes` → recognized but not built here (CLOACI-T-0814): refuse when
///   not in-cluster, else `NotImplemented`.
/// - anything else → [`GuardError::Unknown`].
pub fn evaluate(kind: &str, substrate: &dyn Substrate) -> Result<Decision, GuardError> {
    match kind {
        "none" => Ok(Decision::Noop),
        "docker" => {
            if substrate.kubernetes_detected() {
                return Err(GuardError::Refused(
                    "CLOACINA_FLEET_ACTUATOR=docker but Kubernetes was detected \
                     (KUBERNETES_SERVICE_HOST / service-account token mount). Refusing to \
                     scale Docker containers on a cluster substrate — set the actuator to \
                     'kubernetes' (CLOACI-T-0814) or 'none'."
                        .to_string(),
                ));
            }
            if !substrate.docker_socket_reachable() {
                return Err(GuardError::Unavailable(
                    "CLOACINA_FLEET_ACTUATOR=docker but no Docker daemon socket is reachable \
                     (no DOCKER_HOST and no /var/run/docker.sock). Mount the Docker socket or \
                     set the actuator to 'none'."
                        .to_string(),
                ));
            }
            Ok(Decision::Docker)
        }
        "kubernetes" => {
            if !substrate.kubernetes_detected() {
                Err(GuardError::Refused(
                    "CLOACINA_FLEET_ACTUATOR=kubernetes but not running in-cluster \
                     (no KUBERNETES_SERVICE_HOST / service-account token mount)."
                        .to_string(),
                ))
            } else {
                Err(GuardError::NotImplemented(
                    "kubernetes actuator not yet implemented — CLOACI-T-0814".to_string(),
                ))
            }
        }
        other => Err(GuardError::Unknown(other.to_string())),
    }
}

/// Read the operator-selected actuator kind (`CLOACINA_FLEET_ACTUATOR`),
/// defaulting to `"none"` (actuation off) when unset/empty.
pub fn actuator_kind_from_env() -> String {
    std::env::var("CLOACINA_FLEET_ACTUATOR")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "none".to_string())
}

/// Build the actuator for `kind`, validating it against `substrate`
/// (fail-closed). On `Ok` the server proceeds; on `Err` the caller MUST refuse
/// to start. `database` is used by the Docker actuator to mint per-tenant agent
/// keys at spawn time.
pub fn build_actuator(
    kind: &str,
    substrate: &dyn Substrate,
    database: Database,
) -> Result<Arc<dyn FleetActuator>, GuardError> {
    match evaluate(kind, substrate)? {
        Decision::Noop => Ok(Arc::new(NoopActuator)),
        Decision::Docker => {
            let actuator = DockerActuator::from_env(database)
                .map_err(|e| GuardError::Unavailable(e.to_string()))?;
            Ok(Arc::new(actuator))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeSubstrate {
        k8s: bool,
        docker: bool,
    }

    impl Substrate for FakeSubstrate {
        fn kubernetes_detected(&self) -> bool {
            self.k8s
        }
        fn docker_socket_reachable(&self) -> bool {
            self.docker
        }
    }

    #[test]
    fn none_is_noop() {
        let s = FakeSubstrate {
            k8s: false,
            docker: false,
        };
        assert_eq!(evaluate("none", &s).unwrap(), Decision::Noop);
    }

    #[test]
    fn docker_with_kubernetes_refuses() {
        // Even with a Docker socket present, K8s detection must win (fail-closed).
        let s = FakeSubstrate {
            k8s: true,
            docker: true,
        };
        let err = evaluate("docker", &s).unwrap_err();
        assert!(matches!(err, GuardError::Refused(_)), "got {err:?}");
    }

    #[test]
    fn docker_with_socket_and_no_k8s_builds() {
        let s = FakeSubstrate {
            k8s: false,
            docker: true,
        };
        assert_eq!(evaluate("docker", &s).unwrap(), Decision::Docker);
    }

    #[test]
    fn docker_without_socket_is_unavailable() {
        let s = FakeSubstrate {
            k8s: false,
            docker: false,
        };
        let err = evaluate("docker", &s).unwrap_err();
        assert!(matches!(err, GuardError::Unavailable(_)), "got {err:?}");
    }

    #[test]
    fn kubernetes_not_in_cluster_refuses() {
        let s = FakeSubstrate {
            k8s: false,
            docker: true,
        };
        let err = evaluate("kubernetes", &s).unwrap_err();
        assert!(matches!(err, GuardError::Refused(_)), "got {err:?}");
    }

    #[test]
    fn kubernetes_in_cluster_is_not_implemented() {
        let s = FakeSubstrate {
            k8s: true,
            docker: false,
        };
        let err = evaluate("kubernetes", &s).unwrap_err();
        assert!(matches!(err, GuardError::NotImplemented(_)), "got {err:?}");
    }

    #[test]
    fn unknown_value_errors() {
        let s = FakeSubstrate {
            k8s: false,
            docker: true,
        };
        let err = evaluate("podman", &s).unwrap_err();
        assert!(matches!(err, GuardError::Unknown(_)), "got {err:?}");
    }

    #[test]
    fn default_kind_is_none() {
        // Not asserting on the real env (tests may run anywhere); just that an
        // unset/empty value maps to "none" via the helper's fallback contract.
        std::env::remove_var("CLOACINA_FLEET_ACTUATOR");
        assert_eq!(actuator_kind_from_env(), "none");
    }
}
