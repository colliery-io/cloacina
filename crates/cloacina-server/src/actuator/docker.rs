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
//! ## Pools (CLOACINA-T-1021)
//! A tenant's agents are grouped in pools. A container carries its pool in the
//! `cloacina.pool` label (none: the pool `default`) and the pool settings it
//! was started with in `cloacina.agent-settings`. Of a pool's settings this
//! actuator applies the image, the memory and CPU limits and the max
//! concurrency; requests, the service account and the egress rules have no
//! Docker counterpart and are ignored. A container whose settings label
//! differs from the pool's current settings is replaced.
//!
//! ## Tenant isolation (NFR-004)
//! Every list/spawn/stop is scoped by the `cloacina.tenant` label, so the
//! actuator never observes or targets another tenant's containers.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use bollard::Docker;
use bollard::models::{ContainerCreateBody, HostConfig};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, ListContainersOptionsBuilder, RemoveContainerOptionsBuilder,
    StopContainerOptions,
};
use tracing::{debug, info, warn};
use uuid::Uuid;

use cloacina::database::Database;
use cloacina_api_types::{AgentPoolSettings, DEFAULT_AGENT_POOL};

use super::settings::{
    AgentPoolSource, DalPoolSource, DefaultPoolOnly, Quantity, fixed_agent_count,
};
use super::{ActuatorError, FleetActuator, ReconcileOutcome, reconcile_delta};

/// The role minted agent keys carry (CLOACINA-T-1055): `agent` holds the
/// permission to act as an agent and to get a WebSocket ticket, and nothing
/// else (see `routes/authz.rs`). No other role may register an agent.
pub(crate) const AGENT_KEY_ROLE: &str = "agent";

/// The name of a key the actuator mints for `tenant_id`: `agent:<tenant>:<uuid>`.
/// [`KeyMinter::revoke_others`] finds the actuator's keys by this prefix.
fn agent_key_name_prefix(tenant_id: &str) -> String {
    format!("agent:{tenant_id}:")
}

/// Label marking a container as actuator-managed.
const LABEL_MANAGED: &str = "cloacina.managed";
/// Label binding a managed container to its tenant.
const LABEL_TENANT: &str = "cloacina.tenant";

/// Label naming a container's pool. Absent: the pool `default`.
const LABEL_POOL: &str = "cloacina.pool";
/// Label holding the pool settings a container was started with
/// ([`DockerPoolSettings::label`]). Absent: no settings.
const LABEL_SETTINGS: &str = "cloacina.agent-settings";
/// Label naming the id of the API key a managed container was started with,
/// so the actuator can revoke the key when the container goes.
const LABEL_KEY_ID: &str = "cloacina.agent-key-id";

/// The part of a pool's settings that maps onto a Docker container.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DockerPoolSettings {
    /// Image override. `None`: the server-wide image.
    pub image: Option<String>,
    /// Memory limit in bytes (`HostConfig.Memory`).
    pub memory_bytes: Option<i64>,
    /// CPU limit in units of 1e-9 CPUs (`HostConfig.NanoCpus`).
    pub nano_cpus: Option<i64>,
    /// The agent's max concurrency (`CLOACINA_AGENT_MAX_CONCURRENCY`).
    pub max_concurrency: Option<u32>,
}

impl DockerPoolSettings {
    /// Take what maps from a pool's settings. The quantities were validated
    /// at the API; one that does not parse here is an error, not a default.
    pub fn from_pool(settings: &AgentPoolSettings) -> Result<Self, ActuatorError> {
        let quantity = |field: &str, raw: &Option<String>| {
            raw.as_deref()
                .map(|r| {
                    Quantity::parse(r).map_err(|e| ActuatorError::Settings(format!("{field}: {e}")))
                })
                .transpose()
        };
        let to_i64 = |field: &str, v: i128| {
            i64::try_from(v).map_err(|_| ActuatorError::Settings(format!("{field}: too large")))
        };
        let memory_bytes = quantity("memory_limit", &settings.memory_limit)?
            .map(|q| to_i64("memory_limit", q.units_ceil()))
            .transpose()?;
        let nano_cpus = quantity("cpu_limit", &settings.cpu_limit)?
            .map(|q| to_i64("cpu_limit", q.nanos()))
            .transpose()?;
        Ok(Self {
            image: settings.image.clone(),
            memory_bytes,
            nano_cpus,
            max_concurrency: settings.max_concurrency,
        })
    }

    /// The value of the `cloacina.agent-settings` label: empty with no
    /// settings (so a container from before pools counts as current), else the
    /// set values in a fixed order.
    pub fn label(&self) -> String {
        let mut parts = Vec::new();
        if let Some(image) = &self.image {
            parts.push(format!("image={image}"));
        }
        if let Some(m) = self.memory_bytes {
            parts.push(format!("memory={m}"));
        }
        if let Some(c) = self.nano_cpus {
            parts.push(format!("nano_cpus={c}"));
        }
        if let Some(n) = self.max_concurrency {
            parts.push(format!("max_concurrency={n}"));
        }
        parts.join(";")
    }
}

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
    /// Log format of the agents (`CLOACINA_AGENT_LOG_FORMAT`, default
    /// `text`), set on each agent container as the same variable
    /// (CLOACINA-T-1101). The server checks the value at start and gives it
    /// here: [`DockerConfig::from_env`] leaves the default.
    pub agent_log_format: cloacina::task_logs::LogFormat,
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
            agent_log_format: Default::default(),
        }
    }
}

/// A managed container the actuator knows about.
#[derive(Debug, Clone)]
pub struct ContainerRef {
    pub id: String,
    /// The container's pool (`cloacina.pool`; `default` when it has no label).
    pub pool: String,
    /// The settings it was started with (`cloacina.agent-settings`; empty when
    /// it has no label).
    pub settings: String,
    /// The id of the API key the container was started with
    /// (`cloacina.agent-key-id`). `None` for a container started before
    /// CLOACINA-T-1055.
    pub key_id: Option<String>,
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
    /// The id of `api_key`: the container's `cloacina.agent-key-id` label.
    pub key_id: String,
    /// The pool the agent is in. Other than `default` it is injected as
    /// `CLOACINA_AGENT_POOL` and set as the `cloacina.pool` label.
    pub pool: String,
    /// The pool settings that map onto a container (`image` is already applied
    /// to [`SpawnSpec::image`]).
    pub settings: DockerPoolSettings,
    /// The agent's log format (`CLOACINA_AGENT_LOG_FORMAT`). `text` leaves
    /// the variable out: the agent's own default.
    pub log_format: cloacina::task_logs::LogFormat,
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

/// A key the actuator minted: its id, and the plaintext to give the agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintedKey {
    pub id: String,
    pub plaintext: String,
}

/// The agent keys of the fleet: mint, check and revoke.
#[async_trait]
pub trait KeyMinter: Send + Sync {
    /// Mint a key of role `agent` for the tenant.
    async fn mint(&self, tenant_id: &str) -> Result<MintedKey, ActuatorError>;
    /// True when `key_id` is a key of the tenant, of role `agent`, not
    /// revoked. A key of any other role cannot act as an agent: the agents
    /// that hold it are replaced.
    async fn is_live_agent_key(&self, tenant_id: &str, key_id: &str)
    -> Result<bool, ActuatorError>;
    /// Revoke one key. A key that is gone already is not an error.
    async fn revoke(&self, key_id: &str) -> Result<(), ActuatorError>;
    /// Revoke every key the actuator minted for the tenant except `keep`:
    /// the keys of agents that are gone, and the `read` keys minted before
    /// CLOACINA-T-1055. Gives the number revoked.
    async fn revoke_others(&self, tenant_id: &str, keep: &[String]) -> Result<u32, ActuatorError>;
}

/// Real [`KeyMinter`] backed by the api-keys DAL.
///
/// The Docker actuator mints one key for each container and revokes it when
/// the container is stopped. The Kubernetes actuator keeps one key for each
/// tenant. A revoked key can still pass for up to 30 seconds on a replica
/// that has it in its key cache.
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
    async fn mint(&self, tenant_id: &str) -> Result<MintedKey, ActuatorError> {
        let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
        let name = format!("{}{}", agent_key_name_prefix(tenant_id), Uuid::new_v4());
        let dal = cloacina::dal::DAL::new(self.database.clone());
        let info = dal
            .api_keys()
            .create_key(&hash, &name, Some(tenant_id), false, AGENT_KEY_ROLE)
            .await
            .map_err(|e| ActuatorError::KeyMint(e.to_string()))?;
        Ok(MintedKey {
            id: info.id.to_string(),
            plaintext,
        })
    }

    async fn is_live_agent_key(
        &self,
        tenant_id: &str,
        key_id: &str,
    ) -> Result<bool, ActuatorError> {
        let Ok(id) = Uuid::parse_str(key_id) else {
            return Ok(false);
        };
        let dal = cloacina::dal::DAL::new(self.database.clone());
        let info = dal
            .api_keys()
            .get_key(id)
            .await
            .map_err(|e| ActuatorError::KeyMint(e.to_string()))?;
        Ok(info.is_some_and(|k| {
            !k.revoked
                && k.permissions == AGENT_KEY_ROLE
                && k.tenant_id.as_deref() == Some(tenant_id)
        }))
    }

    async fn revoke(&self, key_id: &str) -> Result<(), ActuatorError> {
        let Ok(id) = Uuid::parse_str(key_id) else {
            return Ok(());
        };
        let dal = cloacina::dal::DAL::new(self.database.clone());
        dal.api_keys()
            .revoke_key(id)
            .await
            .map(|_| ())
            .map_err(|e| ActuatorError::KeyMint(e.to_string()))
    }

    async fn revoke_others(&self, tenant_id: &str, keep: &[String]) -> Result<u32, ActuatorError> {
        let dal = cloacina::dal::DAL::new(self.database.clone());
        let keys = dal
            .api_keys()
            .list_keys_for_tenant(tenant_id)
            .await
            .map_err(|e| ActuatorError::KeyMint(e.to_string()))?;
        let prefix = agent_key_name_prefix(tenant_id);
        let mut revoked = 0;
        for key in keys {
            let id = key.id.to_string();
            if key.revoked || !key.name.starts_with(&prefix) || keep.contains(&id) {
                continue;
            }
            dal.api_keys()
                .revoke_key(key.id)
                .await
                .map_err(|e| ActuatorError::KeyMint(e.to_string()))?;
            revoked += 1;
        }
        Ok(revoked)
    }
}

/// Docker-container fleet actuator. Generic over its substrate ([`ContainerOps`])
/// and key mint ([`KeyMinter`]) for testability; production wiring uses
/// [`BollardOps`] + [`DalKeyMinter`] via [`DockerActuator::from_env`].
pub struct DockerActuator {
    ops: Arc<dyn ContainerOps>,
    minter: Arc<dyn KeyMinter>,
    pools: Arc<dyn AgentPoolSource>,
    config: DockerConfig,
    /// The per-tenant compilers (CLOACINA-T-1034). `None`: off.
    compilers: Option<super::compiler::DockerCompilers>,
}

impl DockerActuator {
    /// Construct with explicit collaborators (used by tests). Every tenant has
    /// only the pool `default` with no settings until [`Self::with_pools`]
    /// gives a source.
    pub fn new(
        ops: Arc<dyn ContainerOps>,
        minter: Arc<dyn KeyMinter>,
        config: DockerConfig,
    ) -> Self {
        Self {
            ops,
            minter,
            pools: Arc::new(DefaultPoolOnly),
            config,
            compilers: None,
        }
    }

    /// Read each tenant's pools from `pools`.
    pub fn with_pools(mut self, pools: Arc<dyn AgentPoolSource>) -> Self {
        self.pools = pools;
        self
    }

    /// Run a compiler container per tenant (CLOACINA-T-1034) through `ops`.
    pub fn with_compilers(
        mut self,
        ops: Arc<dyn super::compiler::CompilerContainerOps>,
        settings: super::compiler::CompilerSettings,
    ) -> Self {
        self.compilers = Some(super::compiler::DockerCompilers::new(ops, settings));
        self
    }

    /// Production constructor: a lazily-connected bollard client + DAL-backed
    /// key mint, configured from the environment. Connecting is lazy (no daemon
    /// round-trip here); the substrate guard has already probed the socket.
    pub fn from_env(
        database: Database,
        compilers: Option<super::compiler::CompilerSettings>,
        agent_log_format: cloacina::task_logs::LogFormat,
    ) -> Result<Self, ActuatorError> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| ActuatorError::Substrate(format!("docker connect failed: {e}")))?;
        let config = DockerConfig {
            agent_log_format,
            ..DockerConfig::from_env()
        };
        let ops = Arc::new(BollardOps::new(docker));
        let actuator = Self {
            ops: ops.clone(),
            minter: Arc::new(DalKeyMinter::new(database.clone())),
            pools: Arc::new(DalPoolSource::new(database)),
            config,
            compilers: None,
        };
        Ok(match compilers {
            Some(settings) => actuator.with_compilers(ops, settings),
            None => actuator,
        })
    }
}

impl DockerActuator {
    /// Stop a container and revoke the key it was started with.
    async fn stop_and_revoke(&self, c: &ContainerRef) -> Result<(), ActuatorError> {
        self.ops.stop(&c.id).await?;
        if let Some(key_id) = &c.key_id {
            self.minter.revoke(key_id).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl FleetActuator for DockerActuator {
    async fn reconcile(
        &self,
        tenant_id: &str,
        desired: u32,
    ) -> Result<ReconcileOutcome, ActuatorError> {
        // The tenant's pools, `default` first. A failed read stops here: the
        // agents stay as they are.
        let pools = self.pools.pools(tenant_id).await?;
        let managed = self.ops.list_managed(tenant_id).await?;

        let mut total = ReconcileOutcome::default();
        // The keys of the containers that stay or start: every other key the
        // actuator minted for the tenant is revoked at the end.
        let mut live_keys: Vec<String> = Vec::new();

        // A container of a pool that no longer exists is stopped.
        for c in managed
            .iter()
            .filter(|c| !pools.iter().any(|(name, _)| *name == c.pool))
        {
            self.stop_and_revoke(c).await?;
            total.stopped += 1;
        }

        for (pool, settings) in &pools {
            let desired = if pool == DEFAULT_AGENT_POOL {
                desired
            } else {
                fixed_agent_count(settings)
            };
            let applied = DockerPoolSettings::from_pool(settings)?;
            if settings.cpu_request.is_some()
                || settings.memory_request.is_some()
                || settings.service_account_name.is_some()
                || !settings.service_account_annotations.is_empty()
                || !settings.pod_labels.is_empty()
                || !settings.pod_annotations.is_empty()
                || settings.automount_service_account_token.is_some()
                || !settings.extra_egress.is_empty()
            {
                debug!(
                    tenant = %tenant_id,
                    pool = %pool,
                    "docker actuator ignores the pool's resource requests, service account, pod \
                     labels and annotations and extra egress: they have no Docker counterpart"
                );
            }
            let label = applied.label();

            // A container started with other settings is replaced: stop it
            // here, and the spawn below makes up the count. So is a container
            // whose key cannot act as an agent (CLOACINA-T-1055): one started
            // before the `agent` role on a `read` key, or one whose key is
            // revoked.
            let mut current: Vec<&ContainerRef> = Vec::new();
            let mut stale: Vec<&ContainerRef> = Vec::new();
            for c in managed.iter().filter(|c| c.pool == *pool) {
                let key_ok = match &c.key_id {
                    Some(id) => self.minter.is_live_agent_key(tenant_id, id).await?,
                    None => false,
                };
                if c.settings == label && key_ok {
                    current.push(c);
                } else {
                    stale.push(c);
                }
            }
            let mut stopped = 0u32;
            for c in &stale {
                self.stop_and_revoke(c).await?;
                stopped += 1;
            }

            let running = current.len() as u32;
            let delta = reconcile_delta(running, desired);

            let mut spawned = 0u32;
            for _ in 0..delta.to_spawn {
                // One key of role `agent` for each container: it is revoked
                // when the container is stopped.
                let key = self.minter.mint(tenant_id).await?;
                live_keys.push(key.id.clone());
                let spec = SpawnSpec {
                    tenant_id: tenant_id.to_string(),
                    image: applied
                        .image
                        .clone()
                        .unwrap_or_else(|| self.config.image.clone()),
                    network: self.config.network.clone(),
                    server_url: self.config.server_url.clone(),
                    api_key: key.plaintext,
                    key_id: key.id,
                    pool: pool.clone(),
                    settings: applied.clone(),
                    log_format: self.config.agent_log_format,
                };
                self.ops.spawn(spec).await?;
                spawned += 1;
            }

            // Stop surplus: pick the first `to_stop` current containers of the pool.
            for c in current.iter().take(delta.to_stop as usize) {
                self.stop_and_revoke(c).await?;
                stopped += 1;
            }
            live_keys.extend(
                current
                    .iter()
                    .skip(delta.to_stop as usize)
                    .filter_map(|c| c.key_id.clone()),
            );

            if spawned > 0 || stopped > 0 {
                info!(
                    tenant = %tenant_id,
                    pool = %pool,
                    desired,
                    running_before = running,
                    spawned,
                    stopped,
                    replaced = stale.len(),
                    "fleet actuator reconciled tenant agents (docker)"
                );
            }

            total.spawned += spawned;
            total.stopped += stopped;
            total.running += running + spawned - delta.to_stop;
        }

        // Finding 12 of CLOACINA-D-0019: no key outlives its agent. This also
        // revokes the `read` keys the actuator minted before the `agent` role.
        let revoked = self.minter.revoke_others(tenant_id, &live_keys).await?;
        if revoked > 0 {
            info!(tenant = %tenant_id, revoked, "fleet actuator revoked agent keys with no agent (docker)");
        }

        Ok(total)
    }

    async fn remove_pool(&self, tenant_id: &str, pool: &str) -> Result<(), ActuatorError> {
        if pool == DEFAULT_AGENT_POOL {
            return Err(ActuatorError::Settings(
                "the pool 'default' is not removed".to_string(),
            ));
        }
        for c in self
            .ops
            .list_managed(tenant_id)
            .await?
            .iter()
            .filter(|c| c.pool == pool)
        {
            self.stop_and_revoke(c).await?;
        }
        Ok(())
    }

    /// Tenant removal (CLOACINA-T-1095): stop every managed container of the
    /// tenant, in every pool, and revoke its key. The list is by label, so a
    /// second call finds none and succeeds.
    async fn remove_agents(&self, tenant_id: &str) -> Result<(), ActuatorError> {
        let managed = self.ops.list_managed(tenant_id).await?;
        for c in &managed {
            self.stop_and_revoke(c).await?;
        }
        if !managed.is_empty() {
            info!(tenant = %tenant_id, stopped = managed.len(), "fleet actuator removed the tenant's agents (docker)");
        }
        Ok(())
    }

    async fn reconcile_compiler(
        &self,
        tenant_id: &str,
        want: super::compiler::CompilerWant,
    ) -> Result<super::compiler::CompilerOutcome, ActuatorError> {
        match &self.compilers {
            Some(compilers) => compilers.reconcile(tenant_id, want).await,
            None => Ok(Default::default()),
        }
    }

    async fn remove_compiler(&self, tenant_id: &str) -> Result<(), ActuatorError> {
        match &self.compilers {
            Some(compilers) => compilers.remove(tenant_id).await,
            None => Ok(()),
        }
    }

    fn compiler_cooldown(&self) -> Option<std::time::Duration> {
        self.compilers.as_ref().map(|c| c.cooldown())
    }

    fn kind(&self) -> &'static str {
        "docker"
    }
}

/// The environment of a spawned agent container.
fn spawn_env(spec: &SpawnSpec) -> Vec<String> {
    let mut env = vec![
        format!("CLOACINA_SERVER={}", spec.server_url),
        format!("CLOACINA_API_KEY={}", spec.api_key),
    ];
    if spec.pool != DEFAULT_AGENT_POOL {
        env.push(format!("CLOACINA_AGENT_POOL={}", spec.pool));
    }
    if let Some(n) = spec.settings.max_concurrency {
        env.push(format!("CLOACINA_AGENT_MAX_CONCURRENCY={n}"));
    }
    // CLOACINA-T-1027: the totals the agent reports are the container's
    // limits. CPU in millicores, rounded up; memory in bytes.
    if let Some(nanos) = spec.settings.nano_cpus.filter(|n| *n > 0) {
        let millis = nanos / 1_000_000 + i64::from(nanos % 1_000_000 != 0);
        env.push(format!("CLOACINA_AGENT_CPU={millis}m"));
    }
    if let Some(bytes) = spec.settings.memory_bytes.filter(|b| *b > 0) {
        env.push(format!("CLOACINA_AGENT_MEMORY={bytes}"));
    }
    // The agent unpacks packages with the server's limit (CLOACINA-T-1048).
    // Not set for the default, so the environment is as before.
    if let Some(bytes) = cloacina::cloacina_workflow_plugin::unpack_limit::child_env_value() {
        env.push(format!("CLOACINA_MAX_UNPACKED_BYTES={bytes}"));
    }
    // The agent's log format (CLOACINA-T-1101). Not set for `text`, so the
    // environment is as before.
    if let Some(format) = super::agent_log_format_env_value(spec.log_format) {
        env.push(format!("{}={format}", super::AGENT_LOG_FORMAT_ENV));
    }
    env
}

/// The host config of a spawned agent container: the network, and the pool's
/// memory and CPU limits. `None` when nothing is set.
fn spawn_host_config(spec: &SpawnSpec) -> Option<HostConfig> {
    if spec.network.is_none()
        && spec.settings.memory_bytes.is_none()
        && spec.settings.nano_cpus.is_none()
    {
        return None;
    }
    Some(HostConfig {
        network_mode: spec.network.clone(),
        memory: spec.settings.memory_bytes,
        nano_cpus: spec.settings.nano_cpus,
        ..Default::default()
    })
}

/// Real [`ContainerOps`] over the Docker Engine API (`bollard`).
pub struct BollardOps {
    pub(super) docker: Docker,
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
            .filter_map(|s| {
                let label = |key: &str| s.labels.as_ref().and_then(|l| l.get(key)).cloned();
                Some(ContainerRef {
                    id: s.id.clone()?,
                    pool: label(LABEL_POOL).unwrap_or_else(|| DEFAULT_AGENT_POOL.to_string()),
                    settings: label(LABEL_SETTINGS).unwrap_or_default(),
                    key_id: label(LABEL_KEY_ID),
                })
            })
            .collect())
    }

    async fn spawn(&self, spec: SpawnSpec) -> Result<(), ActuatorError> {
        let mut labels: HashMap<String, String> = HashMap::new();
        labels.insert(LABEL_TENANT.to_string(), spec.tenant_id.clone());
        labels.insert(LABEL_MANAGED.to_string(), "true".to_string());
        labels.insert(LABEL_KEY_ID.to_string(), spec.key_id.clone());
        // Labels and env of a pool are added only when set, so a container of
        // the pool `default` with no settings is as it was before pools.
        if spec.pool != DEFAULT_AGENT_POOL {
            labels.insert(LABEL_POOL.to_string(), spec.pool.clone());
        }
        let settings_label = spec.settings.label();
        if !settings_label.is_empty() {
            labels.insert(LABEL_SETTINGS.to_string(), settings_label);
        }

        let env = spawn_env(&spec);
        let host_config = spawn_host_config(&spec);

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
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Mock that records spawn/stop calls and reports a fixed running set.
    struct MockOps {
        running: Vec<ContainerRef>,
        spawns: AtomicU32,
        spawn_specs: Mutex<Vec<SpawnSpec>>,
        stops: Mutex<Vec<String>>,
    }

    impl MockOps {
        fn with_running(n: usize) -> Self {
            Self {
                running: (0..n)
                    .map(|i| container(&format!("c{i}"), DEFAULT_AGENT_POOL, ""))
                    .collect(),
                spawns: AtomicU32::new(0),
                spawn_specs: Mutex::new(Vec::new()),
                stops: Mutex::new(Vec::new()),
            }
        }

        fn with_containers(running: Vec<ContainerRef>) -> Self {
            Self {
                running,
                ..Self::with_running(0)
            }
        }
    }

    fn container(id: &str, pool: &str, settings: &str) -> ContainerRef {
        ContainerRef {
            id: id.to_string(),
            pool: pool.to_string(),
            settings: settings.to_string(),
            // A live `agent` key for the mock minter.
            key_id: Some(format!("k-{id}")),
        }
    }

    #[async_trait]
    impl ContainerOps for MockOps {
        async fn list_managed(&self, _tenant_id: &str) -> Result<Vec<ContainerRef>, ActuatorError> {
            Ok(self.running.clone())
        }
        async fn spawn(&self, spec: SpawnSpec) -> Result<(), ActuatorError> {
            self.spawns.fetch_add(1, Ordering::SeqCst);
            self.spawn_specs.lock().unwrap().push(spec);
            Ok(())
        }
        async fn stop(&self, id: &str) -> Result<(), ActuatorError> {
            self.stops.lock().unwrap().push(id.to_string());
            Ok(())
        }
    }

    /// Mock minter: counts, hands back a deterministic key, and records
    /// what it was asked to revoke. A key id that starts with `k-` or
    /// `minted-` is a live `agent` key; any other id is not.
    #[derive(Default)]
    struct MockMinter {
        minted: AtomicU32,
        revoked: Mutex<Vec<String>>,
        kept: Mutex<Vec<Vec<String>>>,
    }

    #[async_trait]
    impl KeyMinter for MockMinter {
        async fn mint(&self, _tenant_id: &str) -> Result<MintedKey, ActuatorError> {
            let n = self.minted.fetch_add(1, Ordering::SeqCst);
            Ok(MintedKey {
                id: format!("minted-{n}"),
                plaintext: format!("clk_test_{n}"),
            })
        }
        async fn is_live_agent_key(
            &self,
            _tenant_id: &str,
            key_id: &str,
        ) -> Result<bool, ActuatorError> {
            Ok(key_id.starts_with("k-") || key_id.starts_with("minted-"))
        }
        async fn revoke(&self, key_id: &str) -> Result<(), ActuatorError> {
            self.revoked.lock().unwrap().push(key_id.to_string());
            Ok(())
        }
        async fn revoke_others(
            &self,
            _tenant_id: &str,
            keep: &[String],
        ) -> Result<u32, ActuatorError> {
            self.kept.lock().unwrap().push(keep.to_vec());
            Ok(0)
        }
    }

    fn config() -> DockerConfig {
        DockerConfig {
            image: "cloacina-agent:latest".to_string(),
            network: Some("cloacina_net".to_string()),
            server_url: "http://server:8080".to_string(),
            agent_log_format: Default::default(),
        }
    }

    fn actuator(ops: Arc<MockOps>, minter: Arc<MockMinter>) -> DockerActuator {
        DockerActuator::new(ops, minter, config())
    }

    #[tokio::test]
    async fn reconcile_spawns_the_deficit_and_mints_one_key_each() {
        let ops = Arc::new(MockOps::with_running(1));
        let minter = Arc::new(MockMinter::default());
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
        let minter = Arc::new(MockMinter::default());
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
        let minter = Arc::new(MockMinter::default());
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
        let minter = Arc::new(MockMinter::default());
        let act = actuator(ops.clone(), minter.clone());

        let out = act.reconcile("acme", 2).await.unwrap();

        assert_eq!(out.spawned, 2);
        assert_eq!(out.running, 2);
        assert_eq!(ops.spawns.load(Ordering::SeqCst), 2);
    }

    // ── pools and their settings (CLOACINA-T-1021) ──────────────────────────

    use super::super::settings::test_support::FixedPools;

    fn minter() -> Arc<MockMinter> {
        Arc::new(MockMinter::default())
    }

    #[test]
    fn docker_settings_take_what_maps_and_label_it() {
        let none = DockerPoolSettings::from_pool(&AgentPoolSettings::default()).unwrap();
        assert_eq!(none, DockerPoolSettings::default());
        // No settings: an empty label, so a container from before pools counts
        // as current.
        assert_eq!(none.label(), "");

        let applied = DockerPoolSettings::from_pool(&AgentPoolSettings {
            cpu_limit: Some("1500m".into()),
            memory_limit: Some("2Gi".into()),
            max_concurrency: Some(2),
            image: Some("registry.example/cloacina-agent:1.2.3".into()),
            // No Docker counterpart: not in the applied settings or the label.
            cpu_request: Some("1".into()),
            memory_request: Some("1Gi".into()),
            service_account_name: Some("team-data".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(applied.memory_bytes, Some(2 << 30));
        assert_eq!(applied.nano_cpus, Some(1_500_000_000));
        assert_eq!(applied.max_concurrency, Some(2));
        assert_eq!(
            applied.label(),
            "image=registry.example/cloacina-agent:1.2.3;memory=2147483648;\
             nano_cpus=1500000000;max_concurrency=2"
        );

        assert!(matches!(
            DockerPoolSettings::from_pool(&AgentPoolSettings {
                memory_limit: Some("2GB".into()),
                ..Default::default()
            }),
            Err(ActuatorError::Settings(_))
        ));
    }

    #[tokio::test]
    async fn spawn_applies_image_limits_and_concurrency() {
        let settings = AgentPoolSettings {
            cpu_limit: Some("2".into()),
            memory_limit: Some("512Mi".into()),
            max_concurrency: Some(1),
            image: Some("registry.example/cloacina-agent:1.2.3".into()),
            ..Default::default()
        };
        let ops = Arc::new(MockOps::with_running(0));
        let act = actuator(ops.clone(), minter())
            .with_pools(Arc::new(FixedPools::default_pool(settings)));
        act.reconcile("acme", 1).await.unwrap();

        let specs = ops.spawn_specs.lock().unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].image, "registry.example/cloacina-agent:1.2.3");
        let host = spawn_host_config(&specs[0]).unwrap();
        assert_eq!(host.memory, Some(512 << 20));
        assert_eq!(host.nano_cpus, Some(2_000_000_000));
        assert_eq!(host.network_mode.as_deref(), Some("cloacina_net"));
        let env = spawn_env(&specs[0]);
        assert!(env.contains(&"CLOACINA_AGENT_MAX_CONCURRENCY=1".to_string()));
        // CLOACINA-T-1027: the agent reports the container's limits.
        assert!(env.contains(&"CLOACINA_AGENT_CPU=2000m".to_string()));
        assert!(env.contains(&format!("CLOACINA_AGENT_MEMORY={}", 512 << 20)));
        // The pool `default` sets no pool env.
        assert!(!env.iter().any(|e| e.starts_with("CLOACINA_AGENT_POOL=")));
    }

    #[tokio::test]
    async fn spawn_with_no_settings_is_as_before_pools() {
        let ops = Arc::new(MockOps::with_running(0));
        let act = actuator(ops.clone(), minter());
        act.reconcile("acme", 1).await.unwrap();
        let specs = ops.spawn_specs.lock().unwrap();
        assert_eq!(specs[0].image, "cloacina-agent:latest");
        assert_eq!(
            spawn_env(&specs[0]),
            vec![
                "CLOACINA_SERVER=http://server:8080".to_string(),
                "CLOACINA_API_KEY=clk_test_0".to_string(),
            ]
        );
        let host = spawn_host_config(&specs[0]).unwrap();
        assert_eq!(host.memory, None);
        assert_eq!(host.nano_cpus, None);
        assert_eq!(specs[0].settings.label(), "");
    }

    /// CLOACINA-T-1101: with the server's setting at `json`, a started agent
    /// container carries the variable. The default adds nothing: the test
    /// above pins the whole environment.
    #[tokio::test]
    async fn spawn_gives_the_agent_the_log_format_of_the_config() {
        let mut cfg = config();
        cfg.agent_log_format = cloacina::task_logs::LogFormat::Json;
        let ops = Arc::new(MockOps::with_running(0));
        let act = DockerActuator::new(ops.clone(), minter(), cfg);
        act.reconcile("acme", 1).await.unwrap();

        let specs = ops.spawn_specs.lock().unwrap();
        assert_eq!(
            spawn_env(&specs[0]),
            vec![
                "CLOACINA_SERVER=http://server:8080".to_string(),
                "CLOACINA_API_KEY=clk_test_0".to_string(),
                "CLOACINA_AGENT_LOG_FORMAT=json".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn containers_with_old_settings_are_replaced() {
        // Two containers from before the setting, one already current.
        let label = "max_concurrency=2";
        let ops = Arc::new(MockOps::with_containers(vec![
            container("old-1", "default", ""),
            container("old-2", "default", ""),
            container("new-1", "default", label),
        ]));
        let act = actuator(ops.clone(), minter()).with_pools(Arc::new(FixedPools::default_pool(
            AgentPoolSettings {
                max_concurrency: Some(2),
                ..Default::default()
            },
        )));
        let out = act.reconcile("acme", 3).await.unwrap();
        assert_eq!(
            ops.stops.lock().unwrap().as_slice(),
            &["old-1".to_string(), "old-2".to_string()]
        );
        assert_eq!(out.stopped, 2);
        assert_eq!(out.spawned, 2);
        assert_eq!(out.running, 3);
        assert!(
            ops.spawn_specs
                .lock()
                .unwrap()
                .iter()
                .all(|s| s.settings.label() == label)
        );
    }

    #[tokio::test]
    async fn each_pool_runs_its_own_count() {
        let ops = Arc::new(MockOps::with_containers(vec![
            container("d-1", "default", ""),
            container("b-1", "big", "max_concurrency=1"),
            container("b-2", "big", "max_concurrency=1"),
            container("b-3", "big", "max_concurrency=1"),
            // A pool that no longer exists.
            container("gone-1", "gone", ""),
        ]));
        let act = actuator(ops.clone(), minter()).with_pools(Arc::new(FixedPools(vec![(
            "big".to_string(),
            AgentPoolSettings {
                max_concurrency: Some(1),
                max_agents: Some(2),
                ..Default::default()
            },
        )])));
        // 2 is the desired count of `default`; `big` runs its own 2.
        let out = act.reconcile("acme", 2).await.unwrap();
        assert_eq!(
            ops.stops.lock().unwrap().as_slice(),
            &["gone-1".to_string(), "b-1".to_string()]
        );
        let specs = ops.spawn_specs.lock().unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].pool, "default");
        assert_eq!(out.spawned, 1);
        assert_eq!(out.stopped, 2);
        assert_eq!(out.running, 4);
    }

    #[tokio::test]
    async fn a_spawn_in_another_pool_names_the_pool() {
        let ops = Arc::new(MockOps::with_running(0));
        let act = actuator(ops.clone(), minter()).with_pools(Arc::new(FixedPools(vec![(
            "big".to_string(),
            AgentPoolSettings {
                max_agents: Some(1),
                ..Default::default()
            },
        )])));
        act.reconcile("acme", 0).await.unwrap();
        let specs = ops.spawn_specs.lock().unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].pool, "big");
        assert!(spawn_env(&specs[0]).contains(&"CLOACINA_AGENT_POOL=big".to_string()));
    }

    #[tokio::test]
    async fn remove_pool_stops_only_that_pool() {
        let ops = Arc::new(MockOps::with_containers(vec![
            container("d-1", "default", ""),
            container("b-1", "big", ""),
        ]));
        let act = actuator(ops.clone(), minter());
        act.remove_pool("acme", "big").await.unwrap();
        assert_eq!(ops.stops.lock().unwrap().as_slice(), &["b-1".to_string()]);
        assert!(act.remove_pool("acme", "default").await.is_err());
    }

    /// CLOACINA-T-1095: tenant removal stops the containers of every pool.
    #[tokio::test]
    async fn remove_agents_stops_every_pool() {
        let ops = Arc::new(MockOps::with_containers(vec![
            container("d-1", "default", ""),
            container("b-1", "big", ""),
        ]));
        let act = actuator(ops.clone(), minter());
        act.remove_agents("acme").await.unwrap();
        assert_eq!(
            ops.stops.lock().unwrap().as_slice(),
            &["d-1".to_string(), "b-1".to_string()]
        );
        assert_eq!(ops.spawns.load(Ordering::SeqCst), 0);
    }

    /// The role the actuator mints is the one role that may act as an agent.
    #[test]
    fn minted_keys_have_the_agent_role() {
        use crate::routes::authz::{Permission, Role};
        let role = Role::parse(AGENT_KEY_ROLE).expect("a known role");
        assert_eq!(role, Role::Agent);
        assert!(role.grants(Permission::AgentAct));
        assert!(!role.grants(Permission::WorkflowsRead));
        assert_eq!(agent_key_name_prefix("acme"), "agent:acme:");
    }

    /// CLOACINA-T-1055: a container whose key cannot act as an agent (no key
    /// label: started before the `agent` role; or a key that is not a live
    /// `agent` key) is replaced, and its key is revoked.
    #[tokio::test]
    async fn containers_on_keys_that_are_not_agent_keys_are_replaced() {
        let ops = Arc::new(MockOps::with_containers(vec![
            ContainerRef {
                key_id: None,
                ..container("before-upgrade", "default", "")
            },
            ContainerRef {
                key_id: Some("read-key".to_string()),
                ..container("on-read-key", "default", "")
            },
            container("ok", "default", ""),
        ]));
        let m = minter();
        let act = actuator(ops.clone(), m.clone());
        let out = act.reconcile("acme", 3).await.unwrap();
        assert_eq!(
            ops.stops.lock().unwrap().as_slice(),
            &["before-upgrade".to_string(), "on-read-key".to_string()]
        );
        assert_eq!(out.stopped, 2);
        assert_eq!(out.spawned, 2);
        assert_eq!(out.running, 3);
        // The key that is known is revoked by id...
        assert_eq!(
            m.revoked.lock().unwrap().as_slice(),
            &["read-key".to_string()]
        );
        // ...and every key but those of the three agents now running is
        // revoked by the sweep: that takes the key of `before-upgrade`.
        let kept = m.kept.lock().unwrap();
        assert_eq!(kept.len(), 1);
        let mut keep = kept[0].clone();
        keep.sort();
        assert_eq!(keep, vec!["k-ok", "minted-0", "minted-1"]);
        // The new containers are labelled with their key's id.
        let specs = ops.spawn_specs.lock().unwrap();
        assert_eq!(specs[0].key_id, "minted-0");
        assert_eq!(specs[0].api_key, "clk_test_0");
    }

    /// Finding 12: a stopped container's key is revoked.
    #[tokio::test]
    async fn a_stopped_container_loses_its_key() {
        let ops = Arc::new(MockOps::with_running(3));
        let m = minter();
        let act = actuator(ops.clone(), m.clone());
        act.reconcile("acme", 1).await.unwrap();
        assert_eq!(
            m.revoked.lock().unwrap().as_slice(),
            &["k-c0".to_string(), "k-c1".to_string()]
        );
        assert_eq!(m.kept.lock().unwrap()[0], vec!["k-c2".to_string()]);

        // A removed pool: its containers stop and their keys are revoked.
        let ops = Arc::new(MockOps::with_containers(vec![
            container("d-1", "default", ""),
            container("b-1", "big", ""),
        ]));
        let m = minter();
        let act = actuator(ops.clone(), m.clone());
        act.remove_pool("acme", "big").await.unwrap();
        assert_eq!(m.revoked.lock().unwrap().as_slice(), &["k-b-1".to_string()]);
    }

    #[test]
    fn kind_is_docker() {
        let ops = Arc::new(MockOps::with_running(0));
        let minter = Arc::new(MockMinter::default());
        assert_eq!(actuator(ops, minter).kind(), "docker");
    }

    #[test]
    fn config_defaults() {
        // Clear so defaults apply regardless of ambient env.
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var("CLOACINA_AGENT_IMAGE") };
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var("CLOACINA_AGENT_NETWORK") };
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var("CLOACINA_AGENT_SERVER_URL") };
        let c = DockerConfig::from_env();
        assert_eq!(c.image, "cloacina-agent:latest");
        assert_eq!(c.network, None);
        assert_eq!(c.server_url, "http://server:8080");
    }
}
