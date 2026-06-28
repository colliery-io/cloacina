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

//! Docker-container fleet actuator (CLOACI-T-0810, CLOACI-I-0127).
//!
//! Reconciles a tenant's running agent count by spawning/stopping
//! `cloacina-agent` containers labelled `cloacina.tenant=<t>` +
//! `cloacina.managed=true`. A spawned agent self-registers via the existing
//! `cloacina-agent` path using an injected tenant-scoped key (`CLOACINA_API_KEY`).
//!
//! The Docker Engine API (via `bollard`) and the agent-key mint are abstracted
//! behind [`ContainerOps`] and [`KeyMinter`] so [`DockerActuator::reconcile`] is
//! unit-testable (assert the spawn/stop call counts) without a live daemon or a
//! database.
//!
//! ## Tenant isolation (NFR-004)
//! Every list/spawn/stop is scoped by the `cloacina.tenant` label, so the
//! actuator never observes or targets another tenant's containers.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use bollard::models::{ContainerCreateBody, HostConfig};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, ListContainersOptionsBuilder, RemoveContainerOptionsBuilder,
    StopContainerOptions,
};
use bollard::Docker;
use tracing::{info, warn};
use uuid::Uuid;

use cloacina::database::Database;

use super::{reconcile_delta, ActuatorError, FleetActuator, ReconcileOutcome};

/// The role minted agent keys carry. `POST /v1/agent/register` is authorized at
/// `Access::any(Level::Read)` (see `routes/authz.rs`), so a tenant-scoped `read`
/// key is the minimal credential an agent needs to self-register.
const AGENT_KEY_ROLE: &str = "read";

/// Label marking a container as actuator-managed.
const LABEL_MANAGED: &str = "cloacina.managed";
/// Label binding a managed container to its tenant.
const LABEL_TENANT: &str = "cloacina.tenant";

/// Docker actuator configuration, read from the environment.
#[derive(Debug, Clone)]
pub struct DockerConfig {
    /// Agent image to run (`CLOACINA_AGENT_IMAGE`, default `cloacina-agent:latest`).
    pub image: String,
    /// Optional Docker network to attach so the agent can reach the server
    /// (`CLOACINA_AGENT_NETWORK`, e.g. the compose network).
    pub network: Option<String>,
    /// Server URL injected as the agent's `CLOACINA_SERVER`
    /// (`CLOACINA_AGENT_SERVER_URL`, default `http://server:8080`).
    pub server_url: String,
}

impl DockerConfig {
    pub fn from_env() -> Self {
        let image = std::env::var("CLOACINA_AGENT_IMAGE")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "cloacina-agent:latest".to_string());
        let network = std::env::var("CLOACINA_AGENT_NETWORK")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let server_url = std::env::var("CLOACINA_AGENT_SERVER_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "http://server:8080".to_string());
        Self {
            image,
            network,
            server_url,
        }
    }
}

/// A managed container the actuator knows about.
#[derive(Debug, Clone)]
pub struct ContainerRef {
    pub id: String,
}

/// Everything needed to spawn one tenant agent container.
#[derive(Debug, Clone)]
pub struct SpawnSpec {
    pub tenant_id: String,
    pub image: String,
    pub network: Option<String>,
    pub server_url: String,
    /// Plaintext tenant-scoped agent key, injected as `CLOACINA_API_KEY`.
    pub api_key: String,
}

/// The Docker Engine operations the actuator needs. Abstracted so `reconcile`
/// can be unit-tested against a mock (assert spawn/stop counts) without a
/// daemon.
#[async_trait]
pub trait ContainerOps: Send + Sync {
    /// List the actuator-managed containers for `tenant_id`
    /// (`cloacina.tenant=<t>` AND `cloacina.managed=true`).
    async fn list_managed(&self, tenant_id: &str) -> Result<Vec<ContainerRef>, ActuatorError>;
    /// Create + start one agent container per `spec`.
    async fn spawn(&self, spec: SpawnSpec) -> Result<(), ActuatorError>;
    /// Stop + remove the container with `id`.
    async fn stop(&self, id: &str) -> Result<(), ActuatorError>;
}

/// Mints a tenant-scoped agent key, returning the plaintext to inject.
#[async_trait]
pub trait KeyMinter: Send + Sync {
    async fn mint(&self, tenant_id: &str) -> Result<String, ActuatorError>;
}

/// Real [`KeyMinter`] backed by the api-keys DAL.
///
/// DECISION (CLOACI-T-0810, flagged): mint one fresh key per spawned container.
/// Key-reuse and revoke-on-stop are deferred — keys accumulate in `api_keys`
/// across spawns (each is a tenant-scoped `read` key). A future task should
/// reclaim/revoke a stopped container's key.
pub struct DalKeyMinter {
    database: Database,
}

impl DalKeyMinter {
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl KeyMinter for DalKeyMinter {
    async fn mint(&self, tenant_id: &str) -> Result<String, ActuatorError> {
        let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
        let name = format!("agent:{tenant_id}:{}", Uuid::new_v4());
        let dal = cloacina::dal::DAL::new(self.database.clone());
        dal.api_keys()
            .create_key(&hash, &name, Some(tenant_id), false, AGENT_KEY_ROLE)
            .await
            .map_err(|e| ActuatorError::KeyMint(e.to_string()))?;
        Ok(plaintext)
    }
}

/// Docker-container fleet actuator. Generic over its substrate ([`ContainerOps`])
/// and key mint ([`KeyMinter`]) for testability; production wiring uses
/// [`BollardOps`] + [`DalKeyMinter`] via [`DockerActuator::from_env`].
pub struct DockerActuator {
    ops: Arc<dyn ContainerOps>,
    minter: Arc<dyn KeyMinter>,
    config: DockerConfig,
}

impl DockerActuator {
    /// Construct with explicit collaborators (used by tests).
    pub fn new(
        ops: Arc<dyn ContainerOps>,
        minter: Arc<dyn KeyMinter>,
        config: DockerConfig,
    ) -> Self {
        Self {
            ops,
            minter,
            config,
        }
    }

    /// Production constructor: a lazily-connected bollard client + DAL-backed
    /// key mint, configured from the environment. Connecting is lazy (no daemon
    /// round-trip here); the substrate guard has already probed the socket.
    pub fn from_env(database: Database) -> Result<Self, ActuatorError> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| ActuatorError::Substrate(format!("docker connect failed: {e}")))?;
        let config = DockerConfig::from_env();
        Ok(Self {
            ops: Arc::new(BollardOps::new(docker)),
            minter: Arc::new(DalKeyMinter::new(database)),
            config,
        })
    }
}

#[async_trait]
impl FleetActuator for DockerActuator {
    async fn reconcile(
        &self,
        tenant_id: &str,
        desired: u32,
    ) -> Result<ReconcileOutcome, ActuatorError> {
        let managed = self.ops.list_managed(tenant_id).await?;
        let running = managed.len() as u32;
        let delta = reconcile_delta(running, desired);

        let mut spawned = 0u32;
        for _ in 0..delta.to_spawn {
            // Mint one fresh tenant-scoped key per container (flagged decision).
            let api_key = self.minter.mint(tenant_id).await?;
            let spec = SpawnSpec {
                tenant_id: tenant_id.to_string(),
                image: self.config.image.clone(),
                network: self.config.network.clone(),
                server_url: self.config.server_url.clone(),
                api_key,
            };
            self.ops.spawn(spec).await?;
            spawned += 1;
        }

        let mut stopped = 0u32;
        // Stop surplus: pick the first `to_stop` managed containers for the tenant.
        for c in managed.iter().take(delta.to_stop as usize) {
            self.ops.stop(&c.id).await?;
            stopped += 1;
        }

        if spawned > 0 || stopped > 0 {
            info!(
                tenant = %tenant_id,
                desired,
                running_before = running,
                spawned,
                stopped,
                "fleet actuator reconciled tenant agents (docker)"
            );
        }

        Ok(ReconcileOutcome {
            spawned,
            stopped,
            running: running + spawned - stopped,
        })
    }

    fn kind(&self) -> &'static str {
        "docker"
    }
}

/// Real [`ContainerOps`] over the Docker Engine API (`bollard`).
pub struct BollardOps {
    docker: Docker,
}

impl BollardOps {
    pub fn new(docker: Docker) -> Self {
        Self { docker }
    }
}

#[async_trait]
impl ContainerOps for BollardOps {
    async fn list_managed(&self, tenant_id: &str) -> Result<Vec<ContainerRef>, ActuatorError> {
        // Filter by BOTH labels so we only ever see this tenant's managed
        // containers (NFR-004 tenant isolation).
        let mut filters: HashMap<String, Vec<String>> = HashMap::new();
        filters.insert(
            "label".to_string(),
            vec![
                format!("{LABEL_TENANT}={tenant_id}"),
                format!("{LABEL_MANAGED}=true"),
            ],
        );
        let options = ListContainersOptionsBuilder::default()
            // Running agents only — an exited/dead container is not live capacity.
            .all(false)
            .filters(&filters)
            .build();

        let summaries = self
            .docker
            .list_containers(Some(options))
            .await
            .map_err(|e| ActuatorError::Substrate(format!("list_containers failed: {e}")))?;

        Ok(summaries
            .into_iter()
            .filter_map(|s| s.id.map(|id| ContainerRef { id }))
            .collect())
    }

    async fn spawn(&self, spec: SpawnSpec) -> Result<(), ActuatorError> {
        let mut labels: HashMap<String, String> = HashMap::new();
        labels.insert(LABEL_TENANT.to_string(), spec.tenant_id.clone());
        labels.insert(LABEL_MANAGED.to_string(), "true".to_string());

        let env = vec![
            format!("CLOACINA_SERVER={}", spec.server_url),
            format!("CLOACINA_API_KEY={}", spec.api_key),
        ];

        let host_config = spec.network.as_ref().map(|net| HostConfig {
            network_mode: Some(net.clone()),
            ..Default::default()
        });

        let body = ContainerCreateBody {
            image: Some(spec.image.clone()),
            env: Some(env),
            labels: Some(labels),
            host_config,
            ..Default::default()
        };

        // Unique name per container; tenant lives in the label, not the name
        // (tenant ids may contain characters invalid in a container name).
        let name = format!("cloacina-agent-{}", Uuid::new_v4());
        let create_opts = CreateContainerOptionsBuilder::default().name(&name).build();

        let created = self
            .docker
            .create_container(Some(create_opts), body)
            .await
            .map_err(|e| ActuatorError::Substrate(format!("create_container failed: {e}")))?;

        self.docker
            .start_container(
                &created.id,
                None::<bollard::query_parameters::StartContainerOptions>,
            )
            .await
            .map_err(|e| ActuatorError::Substrate(format!("start_container failed: {e}")))?;

        info!(container = %created.id, name = %name, tenant = %spec.tenant_id, "spawned agent container");
        Ok(())
    }

    async fn stop(&self, id: &str) -> Result<(), ActuatorError> {
        // Best-effort stop, then force-remove. A stop failure (already stopped)
        // shouldn't block removal.
        if let Err(e) = self
            .docker
            .stop_container(id, None::<StopContainerOptions>)
            .await
        {
            warn!(container = %id, error = %e, "stop_container failed (continuing to remove)");
        }
        let remove_opts = RemoveContainerOptionsBuilder::default().force(true).build();
        self.docker
            .remove_container(id, Some(remove_opts))
            .await
            .map_err(|e| ActuatorError::Substrate(format!("remove_container failed: {e}")))?;
        info!(container = %id, "stopped + removed agent container");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex;

    /// Mock that records spawn/stop calls and reports a fixed running set.
    struct MockOps {
        running: Vec<ContainerRef>,
        spawns: AtomicU32,
        stops: Mutex<Vec<String>>,
    }

    impl MockOps {
        fn with_running(n: usize) -> Self {
            Self {
                running: (0..n)
                    .map(|i| ContainerRef {
                        id: format!("c{i}"),
                    })
                    .collect(),
                spawns: AtomicU32::new(0),
                stops: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ContainerOps for MockOps {
        async fn list_managed(&self, _tenant_id: &str) -> Result<Vec<ContainerRef>, ActuatorError> {
            Ok(self.running.clone())
        }
        async fn spawn(&self, _spec: SpawnSpec) -> Result<(), ActuatorError> {
            self.spawns.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn stop(&self, id: &str) -> Result<(), ActuatorError> {
            self.stops.lock().unwrap().push(id.to_string());
            Ok(())
        }
    }

    /// Mock minter that counts and hands back a deterministic plaintext.
    struct MockMinter {
        minted: AtomicU32,
    }

    #[async_trait]
    impl KeyMinter for MockMinter {
        async fn mint(&self, _tenant_id: &str) -> Result<String, ActuatorError> {
            let n = self.minted.fetch_add(1, Ordering::SeqCst);
            Ok(format!("clk_test_{n}"))
        }
    }

    fn config() -> DockerConfig {
        DockerConfig {
            image: "cloacina-agent:latest".to_string(),
            network: Some("cloacina_net".to_string()),
            server_url: "http://server:8080".to_string(),
        }
    }

    fn actuator(ops: Arc<MockOps>, minter: Arc<MockMinter>) -> DockerActuator {
        DockerActuator::new(ops, minter, config())
    }

    #[tokio::test]
    async fn reconcile_spawns_the_deficit_and_mints_one_key_each() {
        let ops = Arc::new(MockOps::with_running(1));
        let minter = Arc::new(MockMinter {
            minted: AtomicU32::new(0),
        });
        let act = actuator(ops.clone(), minter.clone());

        let out = act.reconcile("acme", 4).await.unwrap();

        assert_eq!(out.spawned, 3);
        assert_eq!(out.stopped, 0);
        assert_eq!(out.running, 4);
        assert_eq!(ops.spawns.load(Ordering::SeqCst), 3);
        assert!(ops.stops.lock().unwrap().is_empty());
        // One key minted per spawned container (flagged decision).
        assert_eq!(minter.minted.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn reconcile_stops_the_surplus() {
        let ops = Arc::new(MockOps::with_running(5));
        let minter = Arc::new(MockMinter {
            minted: AtomicU32::new(0),
        });
        let act = actuator(ops.clone(), minter.clone());

        let out = act.reconcile("acme", 2).await.unwrap();

        assert_eq!(out.spawned, 0);
        assert_eq!(out.stopped, 3);
        assert_eq!(out.running, 2);
        assert_eq!(ops.spawns.load(Ordering::SeqCst), 0);
        assert_eq!(ops.stops.lock().unwrap().len(), 3);
        // No keys minted when only stopping.
        assert_eq!(minter.minted.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn reconcile_is_noop_when_converged() {
        let ops = Arc::new(MockOps::with_running(3));
        let minter = Arc::new(MockMinter {
            minted: AtomicU32::new(0),
        });
        let act = actuator(ops.clone(), minter.clone());

        let out = act.reconcile("acme", 3).await.unwrap();

        assert_eq!(out.spawned, 0);
        assert_eq!(out.stopped, 0);
        assert_eq!(out.running, 3);
        assert_eq!(ops.spawns.load(Ordering::SeqCst), 0);
        assert!(ops.stops.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn reconcile_scales_from_zero() {
        let ops = Arc::new(MockOps::with_running(0));
        let minter = Arc::new(MockMinter {
            minted: AtomicU32::new(0),
        });
        let act = actuator(ops.clone(), minter.clone());

        let out = act.reconcile("acme", 2).await.unwrap();

        assert_eq!(out.spawned, 2);
        assert_eq!(out.running, 2);
        assert_eq!(ops.spawns.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn kind_is_docker() {
        let ops = Arc::new(MockOps::with_running(0));
        let minter = Arc::new(MockMinter {
            minted: AtomicU32::new(0),
        });
        assert_eq!(actuator(ops, minter).kind(), "docker");
    }

    #[test]
    fn config_defaults() {
        // Clear so defaults apply regardless of ambient env.
        std::env::remove_var("CLOACINA_AGENT_IMAGE");
        std::env::remove_var("CLOACINA_AGENT_NETWORK");
        std::env::remove_var("CLOACINA_AGENT_SERVER_URL");
        let c = DockerConfig::from_env();
        assert_eq!(c.image, "cloacina-agent:latest");
        assert_eq!(c.network, None);
        assert_eq!(c.server_url, "http://server:8080");
    }
}
