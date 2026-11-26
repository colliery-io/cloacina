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

//! Kubernetes fleet actuator (CLOACI-T-0814, CLOACI-I-0127).
//!
//! The production sibling of the Docker dev actuator ([`super::docker`]).
//! Reconciles a tenant's running agent count by driving a single
//! `cloacina-agent` `Deployment`'s `replicas` in the **tenant's own
//! namespace** (`cloacina-tenant-<sanitized>`, REQ-007). The agent pods
//! self-register via the existing `cloacina-agent` path using a tenant-scoped
//! key minted by [`KeyMinter`] and delivered through a per-tenant `Secret`
//! referenced as `CLOACINA_API_KEY`.
//!
//! The Kubernetes API and the agent-key mint are abstracted behind [`KubeOps`]
//! and [`KeyMinter`] (the same trait the Docker actuator reuses) so
//! [`KubernetesActuator::reconcile`] is unit-testable — assert the
//! ensure/scale/mint call counts — WITHOUT a live cluster or a database.
//!
//! ## Pools (CLOACINA-T-1021)
//! A tenant's agents are grouped in pools, one `Deployment` per pool in the
//! tenant's namespace. The pool `default` keeps the name `cloacina-agent`;
//! another pool is `cloacina-agent-<pool>`. Each pool's settings (resources,
//! concurrency, image, service account, extra egress) come from
//! [`AgentPoolSource`] on every reconcile and are applied with server-side
//! apply, so a changed setting changes the pod template and Kubernetes rolls
//! the Deployment.
//!
//! ## Tenant isolation (NFR-004 / REQ-007)
//! Every namespace/deployment/secret is keyed by `tenant_namespace(tenant)`, so
//! the actuator only ever touches the requesting tenant's namespace.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, warn};

use cloacina::database::Database;
use cloacina_api_types::{AgentEgressRule, AgentPoolSettings, DEFAULT_AGENT_POOL};

use super::docker::{DalKeyMinter, KeyMinter};
use super::settings::{AgentPoolSource, DalPoolSource, DefaultPoolOnly, fixed_agent_count};
use super::{ActuatorError, FleetActuator, ReconcileOutcome, reconcile_delta};

/// Label marking a workload as actuator-managed.
const LABEL_MANAGED: &str = "cloacina.managed";
/// Label binding a managed workload to its tenant.
const LABEL_TENANT: &str = "cloacina.tenant";
/// Kubernetes app label (matches the chart's convention).
const LABEL_APP: &str = "app.kubernetes.io/name";

/// Label naming the pool of a pool's workload. Not set for the pool `default`,
/// so that its objects stay as they were before pools.
const LABEL_POOL: &str = "cloacina.pool";

/// The pod label keys the actuator sets itself. A pool's `pod_labels` cannot
/// set them: [`LABEL_APP`] is the Deployment's selector, which cannot change
/// after the Deployment is created.
pub const RESERVED_POD_LABELS: [&str; 4] = [LABEL_APP, LABEL_MANAGED, LABEL_TENANT, LABEL_POOL];

/// Name of the agent `Deployment` of the pool `default` (lives in the tenant's
/// namespace, so the name need not encode the tenant). Another pool's
/// Deployment is `cloacina-agent-<pool>`: see [`pool_deployment_name`].
const AGENT_DEPLOYMENT_NAME: &str = "cloacina-agent";
/// Name of the per-tenant `Secret` holding the agent key.
const AGENT_SECRET_NAME: &str = "cloacina-agent-key";
/// Key within [`AGENT_SECRET_NAME`] holding the plaintext agent key.
const AGENT_SECRET_KEY: &str = "api-key";
/// Pod-template annotation naming the id of the API key in the Secret
/// (CLOACINA-T-1055). It tells the actuator which key the tenant's agents
/// hold, and a new key changes the template, so the Deployment rolls.
const KEY_ID_ANNOTATION: &str = "cloacina.io/agent-key-id";
/// Name of the per-tenant agent `NetworkPolicy` (REQ-007, CLOACI-T-0819).
const AGENT_NETWORK_POLICY_NAME: &str = "cloacina-agent-fleet";

/// Prefix of a pool's own `NetworkPolicy`, which carries its extra egress.
const POOL_NETWORK_POLICY_PREFIX: &str = "cloacina-agent-pool-";

/// The name of a pool's agent `Deployment`. It is also the value of the pool's
/// `app.kubernetes.io/name` pod label, so no two pools select the same pods.
pub fn pool_deployment_name(pool: &str) -> String {
    if pool == DEFAULT_AGENT_POOL {
        AGENT_DEPLOYMENT_NAME.to_string()
    } else {
        format!("{AGENT_DEPLOYMENT_NAME}-{pool}")
    }
}

/// The name of a pool's own `NetworkPolicy`.
pub fn pool_network_policy_name(pool: &str) -> String {
    format!("{POOL_NETWORK_POLICY_PREFIX}{pool}")
}

/// uid/gid the agent image runs as (`docker/Dockerfile.agent:70-76`). Reflected
/// in the pod/container `securityContext` so the pod is admissible on a
/// PodSecurity `restricted` cluster.
const AGENT_RUN_AS: i64 = 10001;
/// Server-side-apply field manager — owns the namespace/secret/deployment
/// objects (but NOT `spec.replicas`, which is driven separately so the
/// actuator's apply never fights its own scale).
const FIELD_MANAGER: &str = "cloacina-fleet-actuator";

/// Default namespace prefix (REQ-007 — per-tenant namespace isolation).
const NAMESPACE_PREFIX: &str = "cloacina-tenant-";

/// Maximum length of a DNS-1123 label (Kubernetes namespace/object names).
const DNS_LABEL_MAX: usize = 63;

/// Compute requests + limits for the agent container. Defaults are tuned to the
/// agent's real footprint: it embeds a CPython interpreter (PyO3) and unpacks a
/// `workflow/`+`vendor/` tree plus `dlopen`s cdylibs, so the memory limit is set
/// generously (1Gi) to avoid OOM-killing a Python-packaged workflow with vendored
/// deps; the CPU request is modest (a mostly-idle WS client that bursts on load).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentResources {
    pub cpu_request: String,
    pub memory_request: String,
    pub cpu_limit: String,
    pub memory_limit: String,
}

impl Default for AgentResources {
    fn default() -> Self {
        Self {
            cpu_request: "250m".to_string(),
            memory_request: "256Mi".to_string(),
            cpu_limit: "1".to_string(),
            memory_limit: "1Gi".to_string(),
        }
    }
}

impl AgentResources {
    /// The server-wide resources: `CLOACINA_AGENT_CPU_REQUEST`,
    /// `_MEMORY_REQUEST`, `_CPU_LIMIT`, `_MEMORY_LIMIT`, each with its default.
    pub fn from_env() -> Self {
        let defaults = Self::default();
        Self {
            cpu_request: env_nonempty("CLOACINA_AGENT_CPU_REQUEST").unwrap_or(defaults.cpu_request),
            memory_request: env_nonempty("CLOACINA_AGENT_MEMORY_REQUEST")
                .unwrap_or(defaults.memory_request),
            cpu_limit: env_nonempty("CLOACINA_AGENT_CPU_LIMIT").unwrap_or(defaults.cpu_limit),
            memory_limit: env_nonempty("CLOACINA_AGENT_MEMORY_LIMIT")
                .unwrap_or(defaults.memory_limit),
        }
    }

    /// These resources with a pool's own values in place of the ones it sets.
    pub fn with_pool(&self, pool: &AgentPoolSettings) -> Self {
        let pick = |own: &Option<String>, default: &String| own.clone().unwrap_or(default.clone());
        Self {
            cpu_request: pick(&pool.cpu_request, &self.cpu_request),
            memory_request: pick(&pool.memory_request, &self.memory_request),
            cpu_limit: pick(&pool.cpu_limit, &self.cpu_limit),
            memory_limit: pick(&pool.memory_limit, &self.memory_limit),
        }
    }
}

/// Where the `cloacina-server` lives, so the per-tenant agent `NetworkPolicy`
/// egress can allow agents → server (CLOACI-T-0819). Plumbed from the chart via
/// `CLOACINA_SERVER_NAMESPACE` + `CLOACINA_SERVER_POD_SELECTOR` + `CLOACINA_SERVER_PORT`.
#[derive(Debug, Clone)]
pub struct ServerEndpoint {
    /// Namespace the server pods run in (`.Release.Namespace`).
    pub namespace: String,
    /// Server pod labels (the chart's `selectorLabels`) the egress rule targets.
    pub pod_labels: BTreeMap<String, String>,
    /// Server pod port the agents connect to (containerPort == service port, 8080).
    pub port: u16,
}

/// Kubernetes actuator configuration, read from the environment. Mirrors
/// [`super::docker::DockerConfig`] minus the Docker-network knob (in-cluster
/// pods reach the server by Service DNS).
#[derive(Debug, Clone)]
pub struct KubernetesConfig {
    /// Agent image to run (`CLOACINA_AGENT_IMAGE`, default `cloacina-agent:latest`).
    pub image: String,
    /// Server URL injected as the agent's `CLOACINA_SERVER`
    /// (`CLOACINA_AGENT_SERVER_URL`, default `http://server:8080`).
    pub server_url: String,
    /// Agent container resource requests + limits (`CLOACINA_AGENT_*`).
    pub resources: AgentResources,
    /// Whether to install a per-tenant agent `NetworkPolicy`
    /// (`CLOACINA_FLEET_NETWORK_POLICY`, default ON). REQ-007 defense-in-depth.
    pub network_policy_enabled: bool,
    /// Server endpoint the NetworkPolicy egress allows. `None` when the chart did
    /// not plumb it — the actuator then SKIPS the policy (fail-OPEN for the
    /// policy specifically, so a missing knob never locks agents out of the
    /// server; the server-side ABAC remains the real boundary).
    pub server_endpoint: Option<ServerEndpoint>,
    /// Namespace the cluster DNS service runs in
    /// (`CLOACINA_FLEET_DNS_NAMESPACE`, default `kube-system`).
    pub dns_namespace: String,
}

/// Parse a `k=v,k2=v2` selector string into a label map (empty entries skipped).
fn parse_label_selector(raw: &str) -> BTreeMap<String, String> {
    raw.split(',')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            let (k, v) = (k.trim(), v.trim());
            if k.is_empty() || v.is_empty() {
                None
            } else {
                Some((k.to_string(), v.to_string()))
            }
        })
        .collect()
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

impl KubernetesConfig {
    pub fn from_env() -> Self {
        let image =
            env_nonempty("CLOACINA_AGENT_IMAGE").unwrap_or_else(|| "cloacina-agent:latest".into());
        let server_url = env_nonempty("CLOACINA_AGENT_SERVER_URL")
            .unwrap_or_else(|| "http://server:8080".into());

        let resources = AgentResources::from_env();

        // Default ON: a Kubernetes actuator implies a production cluster where the
        // per-tenant NetworkPolicy is wanted. Set CLOACINA_FLEET_NETWORK_POLICY=false
        // to disable.
        let network_policy_enabled = env_nonempty("CLOACINA_FLEET_NETWORK_POLICY")
            .map(|v| {
                !matches!(
                    v.to_ascii_lowercase().as_str(),
                    "false" | "0" | "no" | "off"
                )
            })
            .unwrap_or(true);

        let dns_namespace =
            env_nonempty("CLOACINA_FLEET_DNS_NAMESPACE").unwrap_or_else(|| "kube-system".into());

        // Only build a ServerEndpoint when the namespace AND a non-empty pod
        // selector are present — otherwise we cannot author a correct egress
        // allow and must skip the policy rather than strand the fleet.
        let server_endpoint = match (
            env_nonempty("CLOACINA_SERVER_NAMESPACE"),
            env_nonempty("CLOACINA_SERVER_POD_SELECTOR").map(|s| parse_label_selector(&s)),
        ) {
            (Some(namespace), Some(pod_labels)) if !pod_labels.is_empty() => Some(ServerEndpoint {
                namespace,
                pod_labels,
                port: env_nonempty("CLOACINA_SERVER_PORT")
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(8080),
            }),
            _ => None,
        };

        Self {
            image,
            server_url,
            resources,
            network_policy_enabled,
            server_endpoint,
            dns_namespace,
        }
    }
}

/// Sanitize a tenant id into the DNS-1123-label segment of its namespace.
///
/// Tenant ids may contain characters invalid in a Kubernetes name (uppercase,
/// `_`, etc.). Lowercase, map every non-alphanumeric run to a single `-`, trim
/// leading/trailing `-`, and truncate to fit within the 63-char label limit
/// once the `cloacina-tenant-` prefix is applied.
pub fn tenant_namespace(tenant_id: &str) -> String {
    let budget = DNS_LABEL_MAX - NAMESPACE_PREFIX.len();
    let mut segment = String::with_capacity(tenant_id.len());
    let mut prev_dash = false;
    for c in tenant_id.to_ascii_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            segment.push(c);
            prev_dash = false;
        } else if !prev_dash {
            segment.push('-');
            prev_dash = true;
        }
    }
    let segment = segment.trim_matches('-');
    let segment: String = segment.chars().take(budget).collect();
    let segment = segment.trim_end_matches('-');
    // A wholly-invalid tenant id collapses to empty; fall back to a stable
    // marker so the namespace name is still a valid label.
    let segment = if segment.is_empty() {
        "unnamed"
    } else {
        segment
    };
    format!("{NAMESPACE_PREFIX}{segment}")
}

/// Everything [`KubeOps::ensure_deployment`] needs to materialize the agent
/// `Deployment` of one pool of a tenant (replica count is driven separately by
/// `scale_deployment` so it is intentionally absent here).
#[derive(Debug, Clone)]
pub struct AgentDeployment {
    pub namespace: String,
    /// [`pool_deployment_name`] of the pool.
    pub name: String,
    pub tenant_id: String,
    /// The pool. `default` adds nothing to the manifest; another pool adds the
    /// `cloacina.pool` label and `CLOACINA_AGENT_POOL`, which the agent
    /// registers as the capability `pool:<name>`.
    pub pool: String,
    /// The agent's max concurrency (`CLOACINA_AGENT_MAX_CONCURRENCY`). `None`
    /// leaves the variable out: the agent uses its own default.
    pub max_concurrency: Option<u32>,
    /// ServiceAccount the pods run as. `None` leaves `serviceAccountName` out:
    /// the pods run as the namespace's `default` ServiceAccount.
    pub service_account_name: Option<String>,
    /// Labels the pool adds to the pod template (not to the selector). A key
    /// in [`RESERVED_POD_LABELS`] is skipped.
    pub pod_labels: BTreeMap<String, String>,
    /// Annotations the pool adds to the pod template.
    pub pod_annotations: BTreeMap<String, String>,
    /// The pod's `automountServiceAccountToken`. `None` leaves it out.
    pub automount_service_account_token: Option<bool>,
    pub image: String,
    pub server_url: String,
    /// Secret the agent key is read from (referenced as `CLOACINA_API_KEY`).
    pub secret_name: String,
    pub secret_key: String,
    /// The id of the API key in the Secret: the pod-template annotation
    /// [`KEY_ID_ANNOTATION`]. `None` leaves the annotation out.
    pub key_id: Option<String>,
    /// Resource requests + limits for the agent container.
    pub resources: AgentResources,
}

/// Everything [`KubeOps::ensure_network_policy`] needs to materialize one
/// tenant's agent `NetworkPolicy` (CLOACI-T-0819, REQ-007 defense-in-depth).
#[derive(Debug, Clone)]
pub struct NetworkPolicySpec {
    /// Tenant namespace the policy lives in (and whose pods it governs).
    pub namespace: String,
    /// Server endpoint the egress rule allows the agents to reach.
    pub server: ServerEndpoint,
    /// Namespace the cluster DNS service runs in (DNS egress is allowed there).
    pub dns_namespace: String,
}

/// Everything [`KubeOps::ensure_pool_network_policy`] needs for the
/// `NetworkPolicy` of one pool: the pool's extra egress rules, for the pool's
/// pods only. NetworkPolicies add up, so the pool's pods get the tenant
/// policy's allows (DNS and the server) plus these.
#[derive(Debug, Clone)]
pub struct PoolNetworkPolicySpec {
    pub namespace: String,
    /// [`pool_network_policy_name`] of the pool.
    pub name: String,
    /// The pool's `app.kubernetes.io/name` pod label ([`pool_deployment_name`]).
    pub pod_app_label: String,
    /// The extra egress rules. Empty: the policy allows nothing more.
    pub extra_egress: Vec<AgentEgressRule>,
}

/// The Kubernetes API operations the actuator needs. Abstracted so `reconcile`
/// can be unit-tested against a mock (assert ensure/scale/mint counts) without
/// a cluster.
#[async_trait]
pub trait KubeOps: Send + Sync {
    /// Ensure the tenant's namespace exists (REQ-007). Idempotent.
    async fn ensure_namespace(&self, namespace: &str) -> Result<(), ActuatorError>;
    /// Ensure the per-tenant agent-key `Secret` exists with `api_key`. Idempotent.
    async fn ensure_secret(
        &self,
        namespace: &str,
        name: &str,
        key: &str,
        api_key: &str,
    ) -> Result<(), ActuatorError>;
    /// Ensure a `ServiceAccount` exists in the tenant namespace with exactly
    /// these annotations (for a cloud workload identity). Idempotent.
    async fn ensure_service_account(
        &self,
        namespace: &str,
        name: &str,
        annotations: &BTreeMap<String, String>,
    ) -> Result<(), ActuatorError>;
    /// Ensure a pool's agent `Deployment` exists (does NOT set replicas).
    /// Idempotent. A changed pod template rolls the Deployment.
    async fn ensure_deployment(&self, spec: &AgentDeployment) -> Result<(), ActuatorError>;
    /// Ensure a pool's own `NetworkPolicy` exists with its extra egress rules.
    /// Idempotent.
    async fn ensure_pool_network_policy(
        &self,
        spec: &PoolNetworkPolicySpec,
    ) -> Result<(), ActuatorError>;
    /// Delete a `Deployment`. Absent is not an error.
    async fn delete_deployment(&self, namespace: &str, name: &str) -> Result<(), ActuatorError>;
    /// Delete a `NetworkPolicy`. Absent is not an error.
    async fn delete_network_policy(&self, namespace: &str, name: &str)
    -> Result<(), ActuatorError>;
    /// Ensure the tenant's agent `NetworkPolicy` exists (REQ-007 defense-in-depth,
    /// CLOACI-T-0819). Default-deny ingress; egress to DNS + the server only.
    /// Idempotent.
    async fn ensure_network_policy(&self, spec: &NetworkPolicySpec) -> Result<(), ActuatorError>;
    /// Set the agent `Deployment`'s replica count.
    async fn scale_deployment(
        &self,
        namespace: &str,
        name: &str,
        replicas: u32,
    ) -> Result<(), ActuatorError>;
    /// Current ready replica count for the agent `Deployment` (0 if absent).
    async fn count_ready_replicas(&self, namespace: &str, name: &str)
    -> Result<u32, ActuatorError>;
    /// The id of the API key a `Deployment`'s pods are started with: its
    /// pod-template annotation [`KEY_ID_ANNOTATION`]. `None` when the
    /// Deployment is absent or was applied before CLOACINA-T-1055.
    async fn deployment_key_id(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<Option<String>, ActuatorError>;
}

/// Kubernetes fleet actuator. Generic over its API substrate ([`KubeOps`]) and
/// key mint ([`KeyMinter`]) for testability; production wiring uses
/// [`KubeApiOps`] + [`DalKeyMinter`] via [`KubernetesActuator::from_env`].
pub struct KubernetesActuator {
    ops: Arc<dyn KubeOps>,
    minter: Arc<dyn KeyMinter>,
    pools: Arc<dyn AgentPoolSource>,
    config: KubernetesConfig,
}

impl KubernetesActuator {
    /// Construct with explicit collaborators (used by tests). Every tenant has
    /// only the pool `default` on the server-wide defaults until
    /// [`Self::with_pools`] gives a source.
    pub fn new(
        ops: Arc<dyn KubeOps>,
        minter: Arc<dyn KeyMinter>,
        config: KubernetesConfig,
    ) -> Self {
        Self {
            ops,
            minter,
            pools: Arc::new(DefaultPoolOnly),
            config,
        }
    }

    /// Read each tenant's pools from `pools`.
    pub fn with_pools(mut self, pools: Arc<dyn AgentPoolSource>) -> Self {
        self.pools = pools;
        self
    }

    /// Production constructor: an in-cluster kube `Client` + DAL-backed key
    /// mint, configured from the environment.
    ///
    /// Client construction uses the **in-cluster** config (service-account
    /// token + `KUBERNETES_SERVICE_HOST`); this is synchronous and lazy (no API
    /// round-trip here — the substrate guard has already confirmed we are
    /// in-cluster, fail-closed). Constructing outside a cluster errors, which
    /// the guard maps to a fatal boot failure.
    pub fn from_env(database: Database) -> Result<Self, ActuatorError> {
        let ops = KubeApiOps::in_cluster()?;
        Ok(Self {
            ops: Arc::new(ops),
            minter: Arc::new(DalKeyMinter::new(database.clone())),
            pools: Arc::new(DalPoolSource::new(database)),
            config: KubernetesConfig::from_env(),
        })
    }

    /// Whether the actuator writes NetworkPolicies: enabled, and the chart
    /// plumbed the server endpoint.
    fn network_policy_active(&self) -> bool {
        self.config.network_policy_enabled && self.config.server_endpoint.is_some()
    }

    /// Reconcile one pool of a tenant: its ServiceAccount, its NetworkPolicy,
    /// its Deployment and the replica count.
    async fn reconcile_pool(
        &self,
        tenant_id: &str,
        namespace: &str,
        pool: &str,
        settings: &AgentPoolSettings,
        desired: u32,
        key_id: &str,
    ) -> Result<ReconcileOutcome, ActuatorError> {
        let name = pool_deployment_name(pool);

        // The pool's ServiceAccount, with its annotations (workload identity).
        // Applied before the Deployment so the pods can start.
        if let Some(sa) = &settings.service_account_name {
            self.ops
                .ensure_service_account(namespace, sa, &settings.service_account_annotations)
                .await?;
        }

        // The pool's extra egress. Written for every pool, also with no rules,
        // so that rules taken out of the settings go away. Not written when the
        // tenant policy is not: then nothing is denied.
        if self.network_policy_active() {
            self.ops
                .ensure_pool_network_policy(&PoolNetworkPolicySpec {
                    namespace: namespace.to_string(),
                    name: pool_network_policy_name(pool),
                    pod_app_label: name.clone(),
                    extra_egress: settings.extra_egress.clone(),
                })
                .await?;
        }

        // Ready replicas are our notion of "running" capacity. Absent
        // Deployment → 0 (treated as a cold start).
        let running = self.ops.count_ready_replicas(namespace, &name).await?;
        let delta = reconcile_delta(running, desired);

        // Ensure the Deployment object exists (idempotent; never sets replicas).
        // The pool's settings override the server-wide defaults.
        let spec = AgentDeployment {
            namespace: namespace.to_string(),
            name: name.clone(),
            tenant_id: tenant_id.to_string(),
            pool: pool.to_string(),
            max_concurrency: settings.max_concurrency,
            service_account_name: settings.service_account_name.clone(),
            pod_labels: settings.pod_labels.clone(),
            pod_annotations: settings.pod_annotations.clone(),
            automount_service_account_token: settings.automount_service_account_token,
            image: settings
                .image
                .clone()
                .unwrap_or_else(|| self.config.image.clone()),
            server_url: self.config.server_url.clone(),
            secret_name: AGENT_SECRET_NAME.to_string(),
            secret_key: AGENT_SECRET_KEY.to_string(),
            key_id: Some(key_id.to_string()),
            resources: self.config.resources.with_pool(settings),
        };
        self.ops.ensure_deployment(&spec).await?;

        // Drive replicas to desired.
        self.ops.scale_deployment(namespace, &name, desired).await?;

        let spawned = delta.to_spawn;
        let stopped = delta.to_stop;
        if spawned > 0 || stopped > 0 {
            info!(
                tenant = %tenant_id,
                namespace = %namespace,
                pool = %pool,
                desired,
                running_before = running,
                spawned,
                stopped,
                "fleet actuator reconciled tenant agents (kubernetes)"
            );
        }

        Ok(ReconcileOutcome {
            spawned,
            stopped,
            running: running + spawned - stopped,
        })
    }
}

#[async_trait]
impl FleetActuator for KubernetesActuator {
    async fn reconcile(
        &self,
        tenant_id: &str,
        desired: u32,
    ) -> Result<ReconcileOutcome, ActuatorError> {
        let namespace = tenant_namespace(tenant_id);

        // The tenant's pools, `default` first. A failed read stops here: the
        // agents stay as they are instead of being resized to the defaults.
        let pools = self.pools.pools(tenant_id).await?;

        // REQ-007: the tenant's namespace is the isolation boundary.
        self.ops.ensure_namespace(&namespace).await?;

        // REQ-007 defense-in-depth (CLOACI-T-0819): lock the namespace down with a
        // default-deny NetworkPolicy that only lets agents reach DNS + the server.
        // Skipped (fail-OPEN) when the chart did not plumb the server endpoint, so
        // a missing knob never strands the fleet — the server-side ABAC (NFR-004)
        // remains the real boundary.
        if self.config.network_policy_enabled {
            match &self.config.server_endpoint {
                Some(server) => {
                    self.ops
                        .ensure_network_policy(&NetworkPolicySpec {
                            namespace: namespace.clone(),
                            server: server.clone(),
                            dns_namespace: self.config.dns_namespace.clone(),
                        })
                        .await?;
                }
                None => warn!(
                    tenant = %tenant_id,
                    namespace = %namespace,
                    "fleet NetworkPolicy enabled but server endpoint not plumbed \
                     (CLOACINA_SERVER_NAMESPACE / CLOACINA_SERVER_POD_SELECTOR); \
                     skipping policy to avoid stranding agents"
                ),
            }
        }

        // The tenant's agent key (CLOACINA-T-1055). One Secret backs every
        // pool's Deployment, so all the tenant's agents share one key of role
        // `agent`. The `default` Deployment's pod template names the key in
        // use. With none named (a cold start, or a Deployment from before the
        // `agent` role), or one that is not a live `agent` key (revoked, or a
        // `read` key), a new key is minted and written to the Secret; the
        // annotation then changes on every pool's Deployment, which rolls its
        // pods onto the new key.
        let noted = self
            .ops
            .deployment_key_id(&namespace, &pool_deployment_name(DEFAULT_AGENT_POOL))
            .await?;
        let key_id = match noted {
            Some(id) if self.minter.is_live_agent_key(tenant_id, &id).await? => id,
            _ => {
                let key = self.minter.mint(tenant_id).await?;
                self.ops
                    .ensure_secret(
                        &namespace,
                        AGENT_SECRET_NAME,
                        AGENT_SECRET_KEY,
                        &key.plaintext,
                    )
                    .await?;
                info!(tenant = %tenant_id, namespace = %namespace, "fleet actuator minted the tenant's agent key (kubernetes)");
                key.id
            }
        };

        // One Deployment per pool. `desired` is the count of `default`; another
        // pool runs its own fixed count. A pool that fails does not stop the
        // others; the first error is returned after all were tried.
        let mut total = ReconcileOutcome::default();
        let mut first_error = None;
        for (pool, settings) in &pools {
            let want = if pool == DEFAULT_AGENT_POOL {
                desired
            } else {
                fixed_agent_count(settings)
            };
            match self
                .reconcile_pool(tenant_id, &namespace, pool, settings, want, &key_id)
                .await
            {
                Ok(out) => {
                    total.spawned += out.spawned;
                    total.stopped += out.stopped;
                    total.running += out.running;
                }
                Err(e) => {
                    warn!(tenant = %tenant_id, pool = %pool, error = %e, "fleet actuator: pool reconcile failed");
                    first_error.get_or_insert(e);
                }
            }
        }
        if let Some(e) = first_error {
            return Err(e);
        }
        // Every pool is on `key_id` now: revoke every other key the actuator
        // minted for the tenant (finding 12 of CLOACINA-D-0019). That takes
        // the key just replaced and the `read` keys from before the `agent`
        // role. Not done after a failed pool: its pods may still hold the old
        // key.
        let revoked = self
            .minter
            .revoke_others(tenant_id, std::slice::from_ref(&key_id))
            .await?;
        if revoked > 0 {
            info!(tenant = %tenant_id, revoked, "fleet actuator revoked replaced agent keys (kubernetes)");
        }
        Ok(total)
    }

    async fn remove_pool(&self, tenant_id: &str, pool: &str) -> Result<(), ActuatorError> {
        if pool == DEFAULT_AGENT_POOL {
            return Err(ActuatorError::Settings(
                "the pool 'default' is not removed".to_string(),
            ));
        }
        let namespace = tenant_namespace(tenant_id);
        self.ops
            .delete_deployment(&namespace, &pool_deployment_name(pool))
            .await?;
        self.ops
            .delete_network_policy(&namespace, &pool_network_policy_name(pool))
            .await?;
        // The pool's ServiceAccount stays: another pool may name it, and the
        // actuator has no right to delete ServiceAccounts.
        info!(tenant = %tenant_id, namespace = %namespace, pool = %pool, "fleet actuator removed pool (kubernetes)");
        Ok(())
    }

    fn kind(&self) -> &'static str {
        "kubernetes"
    }
}

// ---------------------------------------------------------------------------
// Real KubeOps over the Kubernetes API (`kube` + `k8s-openapi`).
// ---------------------------------------------------------------------------

use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::{Namespace, Secret, ServiceAccount};
use k8s_openapi::api::networking::v1::NetworkPolicy;
use kube::api::{DeleteParams, Patch, PatchParams};
use kube::{Api, Client, Config};

/// Build the hardened agent `Deployment` manifest (CLOACI-T-0819).
///
/// Pure function so the hardened pod spec — pod/container `securityContext`,
/// `emptyDir` volumes for the readOnlyRootFilesystem writable paths, and the
/// resource requests/limits — is unit-testable WITHOUT a cluster.
///
/// NB: no `spec.replicas` (scaling is owned by `scale_deployment`) and
/// deliberately **no probes** — the agent is a WebSocket client with no health
/// endpoint; the server tracks liveness via heartbeat + eviction
/// (`CLOACINA_AGENT_LIVENESS_MISSES`). Do not add an httpGet probe here.
fn agent_deployment_manifest(spec: &AgentDeployment) -> serde_json::Value {
    let r = &spec.resources;
    // The app label is the Deployment name: `cloacina-agent` for the pool
    // `default` (as before pools), `cloacina-agent-<pool>` for another, so no
    // two pools of a namespace select the same pods.
    let app = spec.name.as_str();
    let mut manifest = serde_json::json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {
            "name": spec.name,
            "namespace": spec.namespace,
            "labels": {
                LABEL_APP: app,
                LABEL_MANAGED: "true",
                LABEL_TENANT: spec.tenant_id,
            },
        },
        "spec": {
            "selector": {
                "matchLabels": { LABEL_APP: app }
            },
            "template": {
                "metadata": {
                    "labels": {
                        LABEL_APP: app,
                        LABEL_MANAGED: "true",
                        LABEL_TENANT: spec.tenant_id,
                    }
                },
                "spec": {
                    // No service account token in the pod (CLOACINA-T-1033):
                    // the agent never calls the Kubernetes API, and task code
                    // runs in the agent process and could read a mounted token.
                    "automountServiceAccountToken": false,
                    // Pod-level securityContext: non-root, the agent image's uid/gid
                    // (Dockerfile.agent), fsGroup so the emptyDirs are group-writable,
                    // and the RuntimeDefault seccomp profile — required for PodSecurity
                    // `restricted` admission.
                    "securityContext": {
                        "runAsNonRoot": true,
                        "runAsUser": AGENT_RUN_AS,
                        "runAsGroup": AGENT_RUN_AS,
                        "fsGroup": AGENT_RUN_AS,
                        "seccompProfile": { "type": "RuntimeDefault" },
                    },
                    "containers": [{
                        "name": "cloacina-agent",
                        "image": spec.image,
                        // Container-level hardening for `restricted` admission.
                        "securityContext": {
                            "allowPrivilegeEscalation": false,
                            "readOnlyRootFilesystem": true,
                            "capabilities": { "drop": ["ALL"] },
                        },
                        "env": [
                            { "name": "CLOACINA_SERVER", "value": spec.server_url },
                            {
                                "name": "CLOACINA_API_KEY",
                                "valueFrom": {
                                    "secretKeyRef": {
                                        "name": spec.secret_name,
                                        "key": spec.secret_key,
                                    }
                                }
                            }
                        ],
                        "resources": {
                            "requests": { "cpu": r.cpu_request, "memory": r.memory_request },
                            "limits": { "cpu": r.cpu_limit, "memory": r.memory_limit },
                        },
                        // readOnlyRootFilesystem → the agent's writable paths must be
                        // emptyDir volumes. The agent unpacks each Python package's
                        // workflow/+vendor/ tree and caches cdylibs under $HOME
                        // (CLOACINA_AGENT_CACHE_DIR=/home/cloacina/.cloacina-agent-cache),
                        // so /home/cloacina is sized generously; /tmp covers CPython
                        // scratch.
                        "volumeMounts": [
                            { "name": "home", "mountPath": "/home/cloacina" },
                            { "name": "tmp", "mountPath": "/tmp" },
                        ],
                    }],
                    "volumes": [
                        { "name": "home", "emptyDir": { "sizeLimit": "2Gi" } },
                        { "name": "tmp", "emptyDir": { "sizeLimit": "1Gi" } },
                    ],
                }
            }
        }
    });

    // What a pool's settings add. Each is left out when not set, so the pool
    // `default` with no settings gives the manifest it gave before pools and
    // an upgrade rolls nothing.
    if spec.pool != DEFAULT_AGENT_POOL {
        manifest["metadata"]["labels"][LABEL_POOL] = spec.pool.clone().into();
        manifest["spec"]["template"]["metadata"]["labels"][LABEL_POOL] = spec.pool.clone().into();
    }
    // The pool's own pod labels and annotations go on the pod template only.
    // The selector stays the app label: a selector cannot change once the
    // Deployment exists. A key the actuator owns is skipped here too (the API
    // refuses it), so a stored value can never reach the selector's label.
    let template_meta = &mut manifest["spec"]["template"]["metadata"];
    for (key, value) in &spec.pod_labels {
        if !RESERVED_POD_LABELS.contains(&key.as_str()) {
            template_meta["labels"][key.as_str()] = value.clone().into();
        }
    }
    if !spec.pod_annotations.is_empty() {
        template_meta["annotations"] = serde_json::json!(spec.pod_annotations);
    }
    // After the pool's annotations, so a stored value cannot replace it.
    if let Some(key_id) = &spec.key_id {
        template_meta["annotations"][KEY_ID_ANNOTATION] = key_id.clone().into();
    }
    let pod = &mut manifest["spec"]["template"]["spec"];
    if let Some(sa) = &spec.service_account_name {
        pod["serviceAccountName"] = sa.clone().into();
    }
    if let Some(automount) = spec.automount_service_account_token {
        pod["automountServiceAccountToken"] = automount.into();
    }
    if let Some(env) = pod["containers"][0]["env"].as_array_mut() {
        if spec.pool != DEFAULT_AGENT_POOL {
            env.push(serde_json::json!({ "name": "CLOACINA_AGENT_POOL", "value": spec.pool }));
        }
        if let Some(n) = spec.max_concurrency {
            env.push(serde_json::json!({
                "name": "CLOACINA_AGENT_MAX_CONCURRENCY",
                "value": n.to_string(),
            }));
        }
    }
    manifest
}

/// Build the per-tenant agent `NetworkPolicy` manifest (CLOACI-T-0819, REQ-007).
///
/// Pure function so the egress allow-list is unit-testable WITHOUT a cluster —
/// the riskiest surface, since a wrong egress rule strands the whole fleet.
///
/// - `podSelector: {}` → governs every pod in the tenant namespace.
/// - Ingress: **deny all** (empty rule list) — agents serve no traffic.
/// - Egress: allow ONLY (a) DNS (UDP+TCP 53 into the DNS namespace) so the
///   agent can resolve the server's Service FQDN, and (b) the cloacina-server
///   pods (namespaceSelector + podSelector) on the server port. Everything else
///   is denied.
///
/// The namespace selectors key on `kubernetes.io/metadata.name`, the immutable
/// label the API server stamps on every namespace (GA since k8s 1.21), so no
/// extra labelling of the server/DNS namespaces is required.
fn network_policy_manifest(spec: &NetworkPolicySpec) -> serde_json::Value {
    let server_labels: serde_json::Map<String, serde_json::Value> = spec
        .server
        .pod_labels
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
        .collect();

    serde_json::json!({
        "apiVersion": "networking.k8s.io/v1",
        "kind": "NetworkPolicy",
        "metadata": {
            "name": AGENT_NETWORK_POLICY_NAME,
            "namespace": spec.namespace,
            "labels": { LABEL_MANAGED: "true" },
        },
        "spec": {
            "podSelector": {},
            "policyTypes": ["Ingress", "Egress"],
            // Ingress: empty list → deny all inbound.
            "ingress": [],
            "egress": [
                // (a) DNS — UDP + TCP 53 into the cluster DNS namespace. Restricted
                // to that namespace + port (not a podSelector) so it stays correct
                // across kube-dns / CoreDNS pod-label differences.
                {
                    "to": [{
                        "namespaceSelector": {
                            "matchLabels": { "kubernetes.io/metadata.name": spec.dns_namespace }
                        }
                    }],
                    "ports": [
                        { "protocol": "UDP", "port": 53 },
                        { "protocol": "TCP", "port": 53 },
                    ],
                },
                // (b) the cloacina-server pods, on the server port. This is the
                // single allow the agents need to register/heartbeat/stream work.
                {
                    "to": [{
                        "namespaceSelector": {
                            "matchLabels": { "kubernetes.io/metadata.name": spec.server.namespace }
                        },
                        "podSelector": { "matchLabels": server_labels }
                    }],
                    "ports": [
                        { "protocol": "TCP", "port": spec.server.port }
                    ],
                },
            ],
        }
    })
}

/// Build the `NetworkPolicy` of one pool (CLOACINA-T-1021): the pool's extra
/// egress rules, for the pool's pods only.
///
/// Pure function, unit-testable without a cluster. The policy selects the
/// pool's pods by their app label and has the `Egress` type only, so it adds
/// allows to the tenant policy and takes none away. Each rule is one CIDR
/// block (`ipBlock`); a rule with no ports allows every port. With no rules the
/// policy allows nothing more.
fn pool_network_policy_manifest(spec: &PoolNetworkPolicySpec) -> serde_json::Value {
    let egress: Vec<serde_json::Value> = spec
        .extra_egress
        .iter()
        .map(|rule| {
            let mut out = serde_json::json!({ "to": [{ "ipBlock": { "cidr": rule.cidr } }] });
            if !rule.ports.is_empty() {
                let ports: Vec<serde_json::Value> = rule
                    .ports
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "protocol": p.protocol.as_deref().unwrap_or("TCP"),
                            "port": p.port,
                        })
                    })
                    .collect();
                out["ports"] = ports.into();
            }
            out
        })
        .collect();

    serde_json::json!({
        "apiVersion": "networking.k8s.io/v1",
        "kind": "NetworkPolicy",
        "metadata": {
            "name": spec.name,
            "namespace": spec.namespace,
            "labels": { LABEL_MANAGED: "true" },
        },
        "spec": {
            "podSelector": { "matchLabels": { LABEL_APP: spec.pod_app_label } },
            "policyTypes": ["Egress"],
            "egress": egress,
        }
    })
}

/// Build a pool's `ServiceAccount` manifest (CLOACINA-T-1021). The annotations
/// are the whole set: server-side apply removes one that was taken out of the
/// settings. `automountServiceAccountToken` is not set, as on the pod.
fn service_account_manifest(
    namespace: &str,
    name: &str,
    annotations: &BTreeMap<String, String>,
) -> serde_json::Value {
    let mut manifest = serde_json::json!({
        "apiVersion": "v1",
        "kind": "ServiceAccount",
        "metadata": {
            "name": name,
            "namespace": namespace,
            "labels": { LABEL_MANAGED: "true" },
        }
    });
    if !annotations.is_empty() {
        manifest["metadata"]["annotations"] = serde_json::json!(annotations);
    }
    manifest
}

/// Real [`KubeOps`] backed by an in-cluster kube `Client`.
///
/// Namespace/secret/deployment objects are reconciled with **server-side
/// apply** (idempotent create-or-update under a single field manager).
/// `spec.replicas` is deliberately omitted from the applied Deployment and
/// driven by a separate merge patch in [`KubeApiOps::scale_deployment`], so the
/// actuator's apply never conflicts with its own scaling.
pub struct KubeApiOps {
    client: Client,
}

impl KubeApiOps {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Build from the in-cluster service-account config. Synchronous + lazy.
    pub fn in_cluster() -> Result<Self, ActuatorError> {
        let config = Config::incluster().map_err(|e| {
            ActuatorError::Substrate(format!("in-cluster kube config unavailable: {e}"))
        })?;
        let client = Client::try_from(config).map_err(|e| {
            ActuatorError::Substrate(format!("kube client construction failed: {e}"))
        })?;
        Ok(Self::new(client))
    }

    fn apply_params() -> PatchParams {
        // `force` so the actuator always wins ownership of the fields it
        // manages (it is the sole owner of these objects).
        PatchParams::apply(FIELD_MANAGER).force()
    }
}

#[async_trait]
impl KubeOps for KubeApiOps {
    async fn ensure_namespace(&self, namespace: &str) -> Result<(), ActuatorError> {
        let api: Api<Namespace> = Api::all(self.client.clone());
        let ns = serde_json::json!({
            "apiVersion": "v1",
            "kind": "Namespace",
            "metadata": {
                "name": namespace,
                "labels": { LABEL_MANAGED: "true" },
            }
        });
        api.patch(namespace, &Self::apply_params(), &Patch::Apply(&ns))
            .await
            .map_err(|e| ActuatorError::Substrate(format!("ensure_namespace failed: {e}")))?;
        Ok(())
    }

    async fn ensure_secret(
        &self,
        namespace: &str,
        name: &str,
        key: &str,
        api_key: &str,
    ) -> Result<(), ActuatorError> {
        let api: Api<Secret> = Api::namespaced(self.client.clone(), namespace);
        let secret = serde_json::json!({
            "apiVersion": "v1",
            "kind": "Secret",
            "metadata": {
                "name": name,
                "namespace": namespace,
                "labels": { LABEL_MANAGED: "true" },
            },
            "type": "Opaque",
            "stringData": { key: api_key },
        });
        api.patch(name, &Self::apply_params(), &Patch::Apply(&secret))
            .await
            .map_err(|e| ActuatorError::Substrate(format!("ensure_secret failed: {e}")))?;
        Ok(())
    }

    async fn ensure_service_account(
        &self,
        namespace: &str,
        name: &str,
        annotations: &BTreeMap<String, String>,
    ) -> Result<(), ActuatorError> {
        let api: Api<ServiceAccount> = Api::namespaced(self.client.clone(), namespace);
        let sa = service_account_manifest(namespace, name, annotations);
        api.patch(name, &Self::apply_params(), &Patch::Apply(&sa))
            .await
            .map_err(|e| ActuatorError::Substrate(format!("ensure_service_account failed: {e}")))?;
        Ok(())
    }

    async fn ensure_pool_network_policy(
        &self,
        spec: &PoolNetworkPolicySpec,
    ) -> Result<(), ActuatorError> {
        let api: Api<NetworkPolicy> = Api::namespaced(self.client.clone(), &spec.namespace);
        let policy = pool_network_policy_manifest(spec);
        api.patch(&spec.name, &Self::apply_params(), &Patch::Apply(&policy))
            .await
            .map_err(|e| {
                ActuatorError::Substrate(format!("ensure_pool_network_policy failed: {e}"))
            })?;
        Ok(())
    }

    async fn delete_deployment(&self, namespace: &str, name: &str) -> Result<(), ActuatorError> {
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
        match api.delete(name, &DeleteParams::default()).await {
            Ok(_) => Ok(()),
            Err(kube::Error::Api(e)) if e.code == 404 => Ok(()),
            Err(e) => Err(ActuatorError::Substrate(format!(
                "delete_deployment failed: {e}"
            ))),
        }
    }

    async fn delete_network_policy(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<(), ActuatorError> {
        let api: Api<NetworkPolicy> = Api::namespaced(self.client.clone(), namespace);
        match api.delete(name, &DeleteParams::default()).await {
            Ok(_) => Ok(()),
            Err(kube::Error::Api(e)) if e.code == 404 => Ok(()),
            Err(e) => Err(ActuatorError::Substrate(format!(
                "delete_network_policy failed: {e}"
            ))),
        }
    }

    async fn ensure_deployment(&self, spec: &AgentDeployment) -> Result<(), ActuatorError> {
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), &spec.namespace);
        let deployment = agent_deployment_manifest(spec);
        api.patch(
            &spec.name,
            &Self::apply_params(),
            &Patch::Apply(&deployment),
        )
        .await
        .map_err(|e| ActuatorError::Substrate(format!("ensure_deployment failed: {e}")))?;
        Ok(())
    }

    async fn ensure_network_policy(&self, spec: &NetworkPolicySpec) -> Result<(), ActuatorError> {
        let api: Api<NetworkPolicy> = Api::namespaced(self.client.clone(), &spec.namespace);
        let policy = network_policy_manifest(spec);
        api.patch(
            AGENT_NETWORK_POLICY_NAME,
            &Self::apply_params(),
            &Patch::Apply(&policy),
        )
        .await
        .map_err(|e| ActuatorError::Substrate(format!("ensure_network_policy failed: {e}")))?;
        Ok(())
    }

    async fn scale_deployment(
        &self,
        namespace: &str,
        name: &str,
        replicas: u32,
    ) -> Result<(), ActuatorError> {
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
        let patch = serde_json::json!({ "spec": { "replicas": replicas } });
        api.patch(name, &PatchParams::default(), &Patch::Merge(&patch))
            .await
            .map_err(|e| ActuatorError::Substrate(format!("scale_deployment failed: {e}")))?;
        Ok(())
    }

    async fn count_ready_replicas(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<u32, ActuatorError> {
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
        let found = api
            .get_opt(name)
            .await
            .map_err(|e| ActuatorError::Substrate(format!("get deployment failed: {e}")))?;
        let ready = found
            .and_then(|d| d.status)
            .and_then(|s| s.ready_replicas)
            .unwrap_or(0);
        Ok(ready.max(0) as u32)
    }

    async fn deployment_key_id(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<Option<String>, ActuatorError> {
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
        let found = api
            .get_opt(name)
            .await
            .map_err(|e| ActuatorError::Substrate(format!("get deployment failed: {e}")))?;
        Ok(found
            .and_then(|d| d.spec)
            .and_then(|s| s.template.metadata)
            .and_then(|m| m.annotations)
            .and_then(|a| a.get(KEY_ID_ANNOTATION).cloned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// `(namespace, name, annotations)` of one applied ServiceAccount.
    type AppliedServiceAccount = (String, String, BTreeMap<String, String>);

    /// Mock that records ensure/scale calls and reports a fixed ready count.
    struct MockOps {
        ready: u32,
        namespaces: Mutex<Vec<String>>,
        secrets: AtomicU32,
        deployments: AtomicU32,
        scales: Mutex<Vec<(String, u32)>>,
        network_policies: Mutex<Vec<NetworkPolicySpec>>,
        /// Every Deployment spec applied, in order.
        deployment_specs: Mutex<Vec<AgentDeployment>>,
        /// `(deployment name, replicas)` of every scale, in order.
        scaled: Mutex<Vec<(String, u32)>>,
        service_accounts: Mutex<Vec<AppliedServiceAccount>>,
        pool_policies: Mutex<Vec<PoolNetworkPolicySpec>>,
        /// `kind/namespace/name` of every delete, in order.
        deleted: Mutex<Vec<String>>,
        /// Deployment name whose apply fails, if any.
        fail_deployment: Option<String>,
        /// The key id the `default` Deployment's pod template names.
        key_id: Mutex<Option<String>>,
    }

    impl MockOps {
        fn with_ready(ready: u32) -> Self {
            Self {
                ready,
                namespaces: Mutex::new(Vec::new()),
                secrets: AtomicU32::new(0),
                deployments: AtomicU32::new(0),
                scales: Mutex::new(Vec::new()),
                network_policies: Mutex::new(Vec::new()),
                deployment_specs: Mutex::new(Vec::new()),
                scaled: Mutex::new(Vec::new()),
                service_accounts: Mutex::new(Vec::new()),
                pool_policies: Mutex::new(Vec::new()),
                deleted: Mutex::new(Vec::new()),
                fail_deployment: None,
                // Running agents are on a live `agent` key; with none running
                // there is no Deployment yet.
                key_id: Mutex::new((ready > 0).then(|| "minted-before".to_string())),
            }
        }
    }

    #[async_trait]
    impl KubeOps for MockOps {
        async fn ensure_namespace(&self, namespace: &str) -> Result<(), ActuatorError> {
            self.namespaces.lock().unwrap().push(namespace.to_string());
            Ok(())
        }
        async fn ensure_secret(
            &self,
            _namespace: &str,
            _name: &str,
            _key: &str,
            _api_key: &str,
        ) -> Result<(), ActuatorError> {
            self.secrets.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn ensure_service_account(
            &self,
            namespace: &str,
            name: &str,
            annotations: &BTreeMap<String, String>,
        ) -> Result<(), ActuatorError> {
            self.service_accounts.lock().unwrap().push((
                namespace.to_string(),
                name.to_string(),
                annotations.clone(),
            ));
            Ok(())
        }
        async fn ensure_deployment(&self, spec: &AgentDeployment) -> Result<(), ActuatorError> {
            if self.fail_deployment.as_deref() == Some(spec.name.as_str()) {
                return Err(ActuatorError::Substrate("apply refused".to_string()));
            }
            self.deployments.fetch_add(1, Ordering::SeqCst);
            self.deployment_specs.lock().unwrap().push(spec.clone());
            if spec.name == pool_deployment_name(DEFAULT_AGENT_POOL) {
                *self.key_id.lock().unwrap() = spec.key_id.clone();
            }
            Ok(())
        }
        async fn ensure_pool_network_policy(
            &self,
            spec: &PoolNetworkPolicySpec,
        ) -> Result<(), ActuatorError> {
            self.pool_policies.lock().unwrap().push(spec.clone());
            Ok(())
        }
        async fn delete_deployment(
            &self,
            namespace: &str,
            name: &str,
        ) -> Result<(), ActuatorError> {
            self.deleted
                .lock()
                .unwrap()
                .push(format!("deployment/{namespace}/{name}"));
            Ok(())
        }
        async fn delete_network_policy(
            &self,
            namespace: &str,
            name: &str,
        ) -> Result<(), ActuatorError> {
            self.deleted
                .lock()
                .unwrap()
                .push(format!("networkpolicy/{namespace}/{name}"));
            Ok(())
        }
        async fn ensure_network_policy(
            &self,
            spec: &NetworkPolicySpec,
        ) -> Result<(), ActuatorError> {
            self.network_policies.lock().unwrap().push(spec.clone());
            Ok(())
        }
        async fn scale_deployment(
            &self,
            namespace: &str,
            name: &str,
            replicas: u32,
        ) -> Result<(), ActuatorError> {
            self.scales
                .lock()
                .unwrap()
                .push((namespace.to_string(), replicas));
            self.scaled
                .lock()
                .unwrap()
                .push((name.to_string(), replicas));
            Ok(())
        }
        async fn count_ready_replicas(
            &self,
            _namespace: &str,
            _name: &str,
        ) -> Result<u32, ActuatorError> {
            Ok(self.ready)
        }
        async fn deployment_key_id(
            &self,
            _namespace: &str,
            _name: &str,
        ) -> Result<Option<String>, ActuatorError> {
            Ok(self.key_id.lock().unwrap().clone())
        }
    }

    /// Mock minter: counts, hands back a deterministic key, and records the
    /// keys it was told to keep. A key id that starts with `minted-` is a
    /// live `agent` key; any other id is not.
    #[derive(Default)]
    struct MockMinter {
        minted: AtomicU32,
        kept: Mutex<Vec<Vec<String>>>,
    }

    #[async_trait]
    impl KeyMinter for MockMinter {
        async fn mint(
            &self,
            _tenant_id: &str,
        ) -> Result<super::super::docker::MintedKey, ActuatorError> {
            let n = self.minted.fetch_add(1, Ordering::SeqCst);
            Ok(super::super::docker::MintedKey {
                id: format!("minted-{n}"),
                plaintext: format!("clk_test_{n}"),
            })
        }
        async fn is_live_agent_key(
            &self,
            _tenant_id: &str,
            key_id: &str,
        ) -> Result<bool, ActuatorError> {
            Ok(key_id.starts_with("minted-"))
        }
        async fn revoke(&self, _key_id: &str) -> Result<(), ActuatorError> {
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

    fn server_endpoint() -> ServerEndpoint {
        let mut pod_labels = BTreeMap::new();
        pod_labels.insert(
            "app.kubernetes.io/name".to_string(),
            "cloacina-server".to_string(),
        );
        pod_labels.insert("app.kubernetes.io/instance".to_string(), "rel".to_string());
        ServerEndpoint {
            namespace: "cloacina-system".to_string(),
            pod_labels,
            port: 8080,
        }
    }

    fn config() -> KubernetesConfig {
        KubernetesConfig {
            image: "cloacina-agent:latest".to_string(),
            server_url: "http://server:8080".to_string(),
            resources: AgentResources::default(),
            network_policy_enabled: true,
            server_endpoint: Some(server_endpoint()),
            dns_namespace: "kube-system".to_string(),
        }
    }

    fn actuator(ops: Arc<MockOps>, minter: Arc<MockMinter>) -> KubernetesActuator {
        KubernetesActuator::new(ops, minter, config())
    }

    #[tokio::test]
    async fn reconcile_scales_from_zero_and_mints_one_key() {
        let ops = Arc::new(MockOps::with_ready(0));
        let minter = Arc::new(MockMinter::default());
        let act = actuator(ops.clone(), minter.clone());

        let out = act.reconcile("acme", 3).await.unwrap();

        assert_eq!(out.spawned, 3);
        assert_eq!(out.stopped, 0);
        assert_eq!(out.running, 3);
        // Namespace ensured (REQ-007), secret minted+upserted once, deployment
        // ensured, scaled to the desired replica count.
        assert_eq!(
            ops.namespaces.lock().unwrap().as_slice(),
            &["cloacina-tenant-acme".to_string()]
        );
        assert_eq!(minter.minted.load(Ordering::SeqCst), 1);
        assert_eq!(ops.secrets.load(Ordering::SeqCst), 1);
        assert_eq!(ops.deployments.load(Ordering::SeqCst), 1);
        assert_eq!(
            ops.scales.lock().unwrap().as_slice(),
            &[("cloacina-tenant-acme".to_string(), 3u32)]
        );
        // REQ-007 defense-in-depth (T-0819): a per-tenant NetworkPolicy is ensured
        // for the tenant namespace, carrying the plumbed server endpoint.
        let nps = ops.network_policies.lock().unwrap();
        assert_eq!(nps.len(), 1);
        assert_eq!(nps[0].namespace, "cloacina-tenant-acme");
        assert_eq!(nps[0].server.namespace, "cloacina-system");
        assert_eq!(nps[0].dns_namespace, "kube-system");
    }

    #[tokio::test]
    async fn reconcile_skips_network_policy_when_disabled_or_unplumbed() {
        // Enabled but server endpoint not plumbed → policy skipped (fail-OPEN),
        // and reconcile still succeeds (fleet not stranded).
        let mut cfg = config();
        cfg.server_endpoint = None;
        let ops = Arc::new(MockOps::with_ready(0));
        let minter = Arc::new(MockMinter::default());
        let act = KubernetesActuator::new(ops.clone(), minter, cfg);
        act.reconcile("acme", 1).await.unwrap();
        assert!(ops.network_policies.lock().unwrap().is_empty());

        // Explicitly disabled → policy skipped even when plumbed.
        let mut cfg = config();
        cfg.network_policy_enabled = false;
        let ops = Arc::new(MockOps::with_ready(0));
        let minter = Arc::new(MockMinter::default());
        let act = KubernetesActuator::new(ops.clone(), minter, cfg);
        act.reconcile("acme", 1).await.unwrap();
        assert!(ops.network_policies.lock().unwrap().is_empty());
    }

    #[test]
    fn agent_deployment_manifest_is_pod_security_restricted() {
        let spec = AgentDeployment {
            namespace: "cloacina-tenant-acme".to_string(),
            name: "cloacina-agent".to_string(),
            tenant_id: "acme".to_string(),
            pool: DEFAULT_AGENT_POOL.to_string(),
            max_concurrency: None,
            service_account_name: None,
            pod_labels: BTreeMap::new(),
            pod_annotations: BTreeMap::new(),
            automount_service_account_token: None,
            image: "cloacina-agent:latest".to_string(),
            server_url: "http://server:8080".to_string(),
            secret_name: AGENT_SECRET_NAME.to_string(),
            secret_key: AGENT_SECRET_KEY.to_string(),
            key_id: None,
            resources: AgentResources::default(),
        };
        let m = agent_deployment_manifest(&spec);
        let pod = &m["spec"]["template"]["spec"];

        // No service account token is mounted (CLOACINA-T-1033).
        assert_eq!(
            pod["automountServiceAccountToken"],
            serde_json::json!(false)
        );

        // The key reaches the agent as an environment variable from the
        // Secret, and no volume holds it: the agent takes the variable out of
        // its environment at start, and a mounted file would stay readable.
        let key_env = pod["containers"][0]["env"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["name"] == "CLOACINA_API_KEY")
            .expect("CLOACINA_API_KEY is set");
        assert_eq!(
            key_env["valueFrom"]["secretKeyRef"]["name"],
            serde_json::json!(AGENT_SECRET_NAME)
        );
        assert!(
            pod["volumes"]
                .as_array()
                .unwrap()
                .iter()
                .all(|v| v.get("secret").is_none())
        );

        // Pod securityContext (restricted admission).
        let psc = &pod["securityContext"];
        assert_eq!(psc["runAsNonRoot"], serde_json::json!(true));
        assert_eq!(psc["runAsUser"], serde_json::json!(10001));
        assert_eq!(psc["runAsGroup"], serde_json::json!(10001));
        assert_eq!(psc["fsGroup"], serde_json::json!(10001));
        assert_eq!(
            psc["seccompProfile"]["type"],
            serde_json::json!("RuntimeDefault")
        );

        // Container securityContext.
        let c = &pod["containers"][0];
        let csc = &c["securityContext"];
        assert_eq!(csc["allowPrivilegeEscalation"], serde_json::json!(false));
        assert_eq!(csc["readOnlyRootFilesystem"], serde_json::json!(true));
        assert_eq!(csc["capabilities"]["drop"], serde_json::json!(["ALL"]));

        // Resources from AgentResources defaults.
        assert_eq!(c["resources"]["requests"]["cpu"], serde_json::json!("250m"));
        assert_eq!(
            c["resources"]["requests"]["memory"],
            serde_json::json!("256Mi")
        );
        assert_eq!(c["resources"]["limits"]["cpu"], serde_json::json!("1"));
        assert_eq!(c["resources"]["limits"]["memory"], serde_json::json!("1Gi"));

        // emptyDir volumes + mounts for the writable paths under RO rootfs.
        let mounts = c["volumeMounts"].as_array().unwrap();
        let mount_paths: Vec<&str> = mounts
            .iter()
            .map(|mnt| mnt["mountPath"].as_str().unwrap())
            .collect();
        assert!(mount_paths.contains(&"/home/cloacina"));
        assert!(mount_paths.contains(&"/tmp"));
        let vols = pod["volumes"].as_array().unwrap();
        assert_eq!(vols.len(), 2);
        assert!(vols.iter().all(|v| v.get("emptyDir").is_some()));

        // NO probes (agent is a WS client; liveness via server heartbeat).
        assert!(c.get("livenessProbe").is_none());
        assert!(c.get("readinessProbe").is_none());
        assert!(c.get("startupProbe").is_none());
        // NO replicas (scaling is owned by scale_deployment).
        assert!(m["spec"].get("replicas").is_none());
    }

    #[test]
    fn network_policy_manifest_denies_ingress_and_allows_only_dns_and_server() {
        let np = network_policy_manifest(&NetworkPolicySpec {
            namespace: "cloacina-tenant-acme".to_string(),
            server: server_endpoint(),
            dns_namespace: "kube-system".to_string(),
        });
        let spec = &np["spec"];

        // Governs all pods; both policy types.
        assert_eq!(spec["podSelector"], serde_json::json!({}));
        assert_eq!(
            spec["policyTypes"],
            serde_json::json!(["Ingress", "Egress"])
        );
        // Ingress deny-all (empty list).
        assert_eq!(spec["ingress"], serde_json::json!([]));

        let egress = spec["egress"].as_array().unwrap();
        assert_eq!(egress.len(), 2, "exactly DNS + server egress allows");

        // (a) DNS rule: UDP+TCP 53 into the DNS namespace.
        let dns = &egress[0];
        assert_eq!(
            dns["to"][0]["namespaceSelector"]["matchLabels"]["kubernetes.io/metadata.name"],
            serde_json::json!("kube-system")
        );
        assert_eq!(
            dns["ports"],
            serde_json::json!([
                { "protocol": "UDP", "port": 53 },
                { "protocol": "TCP", "port": 53 },
            ])
        );

        // (b) server rule: server ns + server pod labels on the server port.
        let srv = &egress[1];
        assert_eq!(
            srv["to"][0]["namespaceSelector"]["matchLabels"]["kubernetes.io/metadata.name"],
            serde_json::json!("cloacina-system")
        );
        assert_eq!(
            srv["to"][0]["podSelector"]["matchLabels"]["app.kubernetes.io/name"],
            serde_json::json!("cloacina-server")
        );
        assert_eq!(
            srv["to"][0]["podSelector"]["matchLabels"]["app.kubernetes.io/instance"],
            serde_json::json!("rel")
        );
        assert_eq!(
            srv["ports"],
            serde_json::json!([{ "protocol": "TCP", "port": 8080 }])
        );
    }

    // ── pools and their settings (CLOACINA-T-1021) ──────────────────────────

    use super::super::settings::test_support::{BrokenPools, FixedPools};
    use cloacina_api_types::AgentEgressPort;

    fn minter() -> Arc<MockMinter> {
        Arc::new(MockMinter::default())
    }

    fn default_spec() -> AgentDeployment {
        AgentDeployment {
            namespace: "cloacina-tenant-acme".to_string(),
            name: pool_deployment_name(DEFAULT_AGENT_POOL),
            tenant_id: "acme".to_string(),
            pool: DEFAULT_AGENT_POOL.to_string(),
            max_concurrency: None,
            service_account_name: None,
            pod_labels: BTreeMap::new(),
            pod_annotations: BTreeMap::new(),
            automount_service_account_token: None,
            image: "cloacina-agent:latest".to_string(),
            server_url: "http://server:8080".to_string(),
            secret_name: AGENT_SECRET_NAME.to_string(),
            secret_key: AGENT_SECRET_KEY.to_string(),
            key_id: None,
            resources: AgentResources::default(),
        }
    }

    fn big_pool() -> AgentPoolSettings {
        AgentPoolSettings {
            cpu_request: Some("2".into()),
            memory_request: Some("16Gi".into()),
            cpu_limit: Some("8".into()),
            memory_limit: Some("32Gi".into()),
            max_concurrency: Some(1),
            max_agents: Some(2),
            ..Default::default()
        }
    }

    /// The manifest of the pool `default` with no settings, written out in
    /// full: it is what the actuator applied before pools. If this test fails,
    /// an upgrade rolls every tenant's Deployment.
    #[test]
    fn default_pool_manifest_is_unchanged_from_before_pools() {
        let m = agent_deployment_manifest(&default_spec());
        let expected = serde_json::json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": {
                "name": "cloacina-agent",
                "namespace": "cloacina-tenant-acme",
                "labels": {
                    "app.kubernetes.io/name": "cloacina-agent",
                    "cloacina.managed": "true",
                    "cloacina.tenant": "acme",
                },
            },
            "spec": {
                "selector": {
                    "matchLabels": { "app.kubernetes.io/name": "cloacina-agent" }
                },
                "template": {
                    "metadata": {
                        "labels": {
                            "app.kubernetes.io/name": "cloacina-agent",
                            "cloacina.managed": "true",
                            "cloacina.tenant": "acme",
                        }
                    },
                    "spec": {
                        "securityContext": {
                            "runAsNonRoot": true,
                            "runAsUser": 10001,
                            "runAsGroup": 10001,
                            "fsGroup": 10001,
                            "seccompProfile": { "type": "RuntimeDefault" },
                        },
                        "containers": [{
                            "name": "cloacina-agent",
                            "image": "cloacina-agent:latest",
                            "securityContext": {
                                "allowPrivilegeEscalation": false,
                                "readOnlyRootFilesystem": true,
                                "capabilities": { "drop": ["ALL"] },
                            },
                            "env": [
                                { "name": "CLOACINA_SERVER", "value": "http://server:8080" },
                                {
                                    "name": "CLOACINA_API_KEY",
                                    "valueFrom": {
                                        "secretKeyRef": {
                                            "name": "cloacina-agent-key",
                                            "key": "api-key",
                                        }
                                    }
                                }
                            ],
                            "resources": {
                                "requests": { "cpu": "250m", "memory": "256Mi" },
                                "limits": { "cpu": "1", "memory": "1Gi" },
                            },
                            "volumeMounts": [
                                { "name": "home", "mountPath": "/home/cloacina" },
                                { "name": "tmp", "mountPath": "/tmp" },
                            ],
                        }],
                        "volumes": [
                            { "name": "home", "emptyDir": { "sizeLimit": "2Gi" } },
                            { "name": "tmp", "emptyDir": { "sizeLimit": "1Gi" } },
                        ],
                    }
                }
            }
        });
        assert_eq!(m, expected);
    }

    #[test]
    fn manifest_carries_resources_and_image() {
        let mut spec = default_spec();
        spec.image = "registry.example/cloacina-agent:1.2.3".to_string();
        spec.resources = AgentResources::default().with_pool(&big_pool());
        let m = agent_deployment_manifest(&spec);
        let c = &m["spec"]["template"]["spec"]["containers"][0];
        assert_eq!(c["image"], "registry.example/cloacina-agent:1.2.3");
        assert_eq!(
            c["resources"],
            serde_json::json!({
                "requests": { "cpu": "2", "memory": "16Gi" },
                "limits": { "cpu": "8", "memory": "32Gi" },
            })
        );
    }

    #[test]
    fn resources_keep_the_default_for_each_value_a_pool_does_not_set() {
        let r = AgentResources::default().with_pool(&AgentPoolSettings {
            memory_limit: Some("32Gi".into()),
            ..Default::default()
        });
        assert_eq!(r.memory_limit, "32Gi");
        assert_eq!(r.memory_request, "256Mi");
        assert_eq!(r.cpu_request, "250m");
        assert_eq!(r.cpu_limit, "1");
    }

    fn env_value<'a>(m: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
        m["spec"]["template"]["spec"]["containers"][0]["env"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["name"] == name)
            .map(|e| &e["value"])
    }

    #[test]
    fn manifest_sets_max_concurrency_env_only_when_set() {
        let m = agent_deployment_manifest(&default_spec());
        assert!(env_value(&m, "CLOACINA_AGENT_MAX_CONCURRENCY").is_none());

        let mut spec = default_spec();
        spec.max_concurrency = Some(1);
        let m = agent_deployment_manifest(&spec);
        assert_eq!(
            env_value(&m, "CLOACINA_AGENT_MAX_CONCURRENCY"),
            Some(&serde_json::json!("1"))
        );
    }

    #[test]
    fn manifest_sets_service_account_name_only_when_set() {
        let m = agent_deployment_manifest(&default_spec());
        let pod = &m["spec"]["template"]["spec"];
        assert!(pod.get("serviceAccountName").is_none());
        // Token mount is left to the cluster default, as before.
        assert!(pod.get("automountServiceAccountToken").is_none());

        let mut spec = default_spec();
        spec.service_account_name = Some("team-data".to_string());
        let m = agent_deployment_manifest(&spec);
        let pod = &m["spec"]["template"]["spec"];
        assert_eq!(pod["serviceAccountName"], "team-data");
        assert!(pod.get("automountServiceAccountToken").is_none());
    }

    fn string_map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn manifest_adds_pod_labels_to_the_template_and_not_to_the_selector() {
        let mut spec = default_spec();
        spec.pod_labels = string_map(&[("azure.workload.identity/use", "true"), ("team", "data")]);
        let m = agent_deployment_manifest(&spec);
        let labels = &m["spec"]["template"]["metadata"]["labels"];
        assert_eq!(labels["azure.workload.identity/use"], "true");
        assert_eq!(labels["team"], "data");
        // The actuator's own labels are still there.
        assert_eq!(labels["app.kubernetes.io/name"], "cloacina-agent");
        assert_eq!(labels["cloacina.managed"], "true");
        assert_eq!(labels["cloacina.tenant"], "acme");

        // A selector cannot change after the Deployment is created: it is the
        // same with and without the labels, and with other labels.
        let plain = agent_deployment_manifest(&default_spec());
        assert_eq!(m["spec"]["selector"], plain["spec"]["selector"]);
        assert_eq!(
            m["spec"]["selector"],
            serde_json::json!({ "matchLabels": { "app.kubernetes.io/name": "cloacina-agent" } })
        );
        spec.pod_labels = string_map(&[("team", "other")]);
        let changed = agent_deployment_manifest(&spec);
        assert_eq!(changed["spec"]["selector"], plain["spec"]["selector"]);
        // The Deployment's own labels do not take the pod labels.
        assert!(m["metadata"]["labels"].get("team").is_none());
    }

    /// A changed label or annotation changes the pod template, which is what
    /// makes Kubernetes roll the Deployment.
    #[test]
    fn a_changed_pod_label_or_annotation_changes_the_pod_template() {
        let template =
            |spec: &AgentDeployment| agent_deployment_manifest(spec)["spec"]["template"].clone();
        let base = template(&default_spec());

        let mut labelled = default_spec();
        labelled.pod_labels = string_map(&[("team", "data")]);
        assert_ne!(template(&labelled), base);
        let mut relabelled = default_spec();
        relabelled.pod_labels = string_map(&[("team", "other")]);
        assert_ne!(template(&relabelled), template(&labelled));

        let mut annotated = default_spec();
        annotated.pod_annotations = string_map(&[("example.com/note", "a")]);
        assert_ne!(template(&annotated), base);

        let mut automount = default_spec();
        automount.automount_service_account_token = Some(false);
        assert_ne!(template(&automount), base);
    }

    /// The API refuses a label key the actuator owns. If one is stored all the
    /// same, the manifest keeps the actuator's value.
    #[test]
    fn manifest_never_lets_a_pod_label_replace_one_the_actuator_owns() {
        let mut spec = default_spec();
        spec.pod_labels = string_map(&[
            ("app.kubernetes.io/name", "something-else"),
            ("cloacina.tenant", "globex"),
            ("cloacina.managed", "false"),
            ("cloacina.pool", "big"),
        ]);
        let m = agent_deployment_manifest(&spec);
        assert_eq!(m, agent_deployment_manifest(&default_spec()));
    }

    #[test]
    fn manifest_adds_pod_annotations_only_when_set() {
        let m = agent_deployment_manifest(&default_spec());
        assert!(
            m["spec"]["template"]["metadata"]
                .get("annotations")
                .is_none()
        );

        let mut spec = default_spec();
        spec.pod_annotations = string_map(&[("example.com/note", "free text")]);
        let m = agent_deployment_manifest(&spec);
        assert_eq!(
            m["spec"]["template"]["metadata"]["annotations"],
            serde_json::json!({ "example.com/note": "free text" })
        );
        // On the pod template, not on the Deployment itself.
        assert!(m["metadata"].get("annotations").is_none());
    }

    #[test]
    fn manifest_sets_automount_service_account_token_only_when_set() {
        let pod = |v: Option<bool>| {
            let mut spec = default_spec();
            spec.automount_service_account_token = v;
            agent_deployment_manifest(&spec)["spec"]["template"]["spec"].clone()
        };
        assert!(pod(None).get("automountServiceAccountToken").is_none());
        assert_eq!(pod(Some(false))["automountServiceAccountToken"], false);
        assert_eq!(pod(Some(true))["automountServiceAccountToken"], true);
    }

    /// What an injected identity token needs from the pod: it is a mounted
    /// volume, so the read-only root filesystem and the dropped capabilities
    /// do not matter, and the non-root user reads it through `fsGroup`. The
    /// pool settings leave these as they are.
    #[test]
    fn identity_settings_keep_the_pod_hardening_and_fs_group() {
        let mut spec = default_spec();
        spec.service_account_name = Some("team-data".to_string());
        spec.pod_labels = string_map(&[("azure.workload.identity/use", "true")]);
        spec.automount_service_account_token = Some(false);
        let m = agent_deployment_manifest(&spec);
        let pod = &m["spec"]["template"]["spec"];
        assert_eq!(pod["securityContext"]["runAsUser"], 10001);
        assert_eq!(pod["securityContext"]["fsGroup"], 10001);
        assert_eq!(pod["securityContext"]["runAsNonRoot"], true);
        let c = &pod["containers"][0];
        assert_eq!(c["securityContext"]["readOnlyRootFilesystem"], true);
        assert_eq!(
            c["securityContext"]["capabilities"]["drop"],
            serde_json::json!(["ALL"])
        );
    }

    #[tokio::test]
    async fn reconcile_passes_pod_metadata_and_automount_to_the_deployment() {
        let settings = AgentPoolSettings {
            service_account_name: Some("team-data".into()),
            pod_labels: string_map(&[("azure.workload.identity/use", "true")]),
            pod_annotations: string_map(&[("example.com/note", "a")]),
            automount_service_account_token: Some(false),
            ..Default::default()
        };
        let ops = Arc::new(MockOps::with_ready(0));
        let act = actuator(ops.clone(), minter())
            .with_pools(Arc::new(FixedPools::default_pool(settings.clone())));
        act.reconcile("acme", 1).await.unwrap();
        {
            let specs = ops.deployment_specs.lock().unwrap();
            assert_eq!(specs[0].pod_labels, settings.pod_labels);
            assert_eq!(specs[0].pod_annotations, settings.pod_annotations);
            assert_eq!(specs[0].automount_service_account_token, Some(false));
        }

        // With no settings none of them is set.
        let ops = Arc::new(MockOps::with_ready(0));
        let act = actuator(ops.clone(), minter());
        act.reconcile("acme", 1).await.unwrap();
        let specs = ops.deployment_specs.lock().unwrap();
        assert!(specs[0].pod_labels.is_empty());
        assert!(specs[0].pod_annotations.is_empty());
        assert_eq!(specs[0].automount_service_account_token, None);
    }

    #[test]
    fn manifest_of_another_pool_has_its_own_name_selector_and_pool_env() {
        let mut spec = default_spec();
        spec.pool = "big".to_string();
        spec.name = pool_deployment_name("big");
        let m = agent_deployment_manifest(&spec);
        assert_eq!(m["metadata"]["name"], "cloacina-agent-big");
        // Its selector matches its own pods only, not the default pool's.
        assert_eq!(
            m["spec"]["selector"]["matchLabels"],
            serde_json::json!({ "app.kubernetes.io/name": "cloacina-agent-big" })
        );
        let labels = &m["spec"]["template"]["metadata"]["labels"];
        assert_eq!(labels["app.kubernetes.io/name"], "cloacina-agent-big");
        assert_eq!(labels["cloacina.pool"], "big");
        assert_eq!(labels["cloacina.tenant"], "acme");
        assert_eq!(m["metadata"]["labels"]["cloacina.pool"], "big");
        // The agent learns its pool from the env and registers `pool:big`.
        assert_eq!(
            env_value(&m, "CLOACINA_AGENT_POOL"),
            Some(&serde_json::json!("big"))
        );
        // The default pool sets neither.
        let d = agent_deployment_manifest(&default_spec());
        assert!(env_value(&d, "CLOACINA_AGENT_POOL").is_none());
        assert!(
            d["spec"]["template"]["metadata"]["labels"]
                .get("cloacina.pool")
                .is_none()
        );
    }

    #[test]
    fn pool_object_names() {
        assert_eq!(pool_deployment_name("default"), "cloacina-agent");
        assert_eq!(pool_deployment_name("big"), "cloacina-agent-big");
        assert_eq!(
            pool_network_policy_name("default"),
            "cloacina-agent-pool-default"
        );
        // The longest pool name still gives valid object names.
        let longest = "x".repeat(super::super::settings::MAX_POOL_NAME_LEN);
        assert!(pool_deployment_name(&longest).len() <= DNS_LABEL_MAX);
        assert!(pool_network_policy_name(&longest).len() <= DNS_LABEL_MAX);
    }

    #[test]
    fn service_account_manifest_carries_annotations() {
        let mut annotations = BTreeMap::new();
        annotations.insert(
            "eks.amazonaws.com/role-arn".to_string(),
            "arn:aws:iam::000000000000:role/example-team".to_string(),
        );
        let m = service_account_manifest("cloacina-tenant-acme", "team-data", &annotations);
        assert_eq!(m["kind"], "ServiceAccount");
        assert_eq!(m["metadata"]["name"], "team-data");
        assert_eq!(m["metadata"]["namespace"], "cloacina-tenant-acme");
        assert_eq!(m["metadata"]["labels"]["cloacina.managed"], "true");
        assert_eq!(
            m["metadata"]["annotations"],
            serde_json::json!({
                "eks.amazonaws.com/role-arn": "arn:aws:iam::000000000000:role/example-team"
            })
        );
        assert!(m.get("automountServiceAccountToken").is_none());
        // No annotations: the key is left out.
        let m = service_account_manifest("cloacina-tenant-acme", "team-data", &BTreeMap::new());
        assert!(m["metadata"].get("annotations").is_none());
    }

    #[test]
    fn pool_network_policy_adds_only_the_extra_egress_for_the_pool_pods() {
        let np = pool_network_policy_manifest(&PoolNetworkPolicySpec {
            namespace: "cloacina-tenant-acme".to_string(),
            name: pool_network_policy_name("big"),
            pod_app_label: pool_deployment_name("big"),
            extra_egress: vec![
                AgentEgressRule {
                    cidr: "198.51.100.0/24".to_string(),
                    ports: vec![
                        AgentEgressPort {
                            port: 443,
                            protocol: None,
                        },
                        AgentEgressPort {
                            port: 4317,
                            protocol: Some("UDP".to_string()),
                        },
                    ],
                },
                AgentEgressRule {
                    cidr: "192.0.2.7/32".to_string(),
                    ports: vec![],
                },
            ],
        });
        assert_eq!(np["metadata"]["name"], "cloacina-agent-pool-big");
        assert_eq!(np["metadata"]["namespace"], "cloacina-tenant-acme");
        let spec = &np["spec"];
        // Only the pool's pods; egress only (ingress stays denied by the
        // tenant policy).
        assert_eq!(
            spec["podSelector"],
            serde_json::json!({ "matchLabels": { "app.kubernetes.io/name": "cloacina-agent-big" } })
        );
        assert_eq!(spec["policyTypes"], serde_json::json!(["Egress"]));
        assert!(spec.get("ingress").is_none());
        assert_eq!(
            spec["egress"],
            serde_json::json!([
                {
                    "to": [{ "ipBlock": { "cidr": "198.51.100.0/24" } }],
                    "ports": [
                        { "protocol": "TCP", "port": 443 },
                        { "protocol": "UDP", "port": 4317 },
                    ],
                },
                // No ports: every port of the block.
                { "to": [{ "ipBlock": { "cidr": "192.0.2.7/32" } }] },
            ])
        );
    }

    #[test]
    fn pool_network_policy_with_no_rules_allows_nothing_more() {
        let np = pool_network_policy_manifest(&PoolNetworkPolicySpec {
            namespace: "cloacina-tenant-acme".to_string(),
            name: pool_network_policy_name(DEFAULT_AGENT_POOL),
            pod_app_label: pool_deployment_name(DEFAULT_AGENT_POOL),
            extra_egress: vec![],
        });
        assert_eq!(np["spec"]["egress"], serde_json::json!([]));
        assert_eq!(np["spec"]["policyTypes"], serde_json::json!(["Egress"]));
    }

    #[tokio::test]
    async fn reconcile_with_no_stored_pools_touches_only_the_default_pool() {
        let ops = Arc::new(MockOps::with_ready(0));
        let act = actuator(ops.clone(), minter());
        act.reconcile("acme", 2).await.unwrap();

        let specs = ops.deployment_specs.lock().unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name, "cloacina-agent");
        assert_eq!(specs[0].pool, "default");
        assert_eq!(specs[0].image, "cloacina-agent:latest");
        assert_eq!(specs[0].resources, AgentResources::default());
        assert_eq!(specs[0].max_concurrency, None);
        assert_eq!(specs[0].service_account_name, None);
        assert!(ops.service_accounts.lock().unwrap().is_empty());
        // The tenant policy is unchanged; the default pool's own policy has no
        // rules.
        assert_eq!(ops.network_policies.lock().unwrap().len(), 1);
        let pool_policies = ops.pool_policies.lock().unwrap();
        assert_eq!(pool_policies.len(), 1);
        assert_eq!(pool_policies[0].name, "cloacina-agent-pool-default");
        assert_eq!(pool_policies[0].pod_app_label, "cloacina-agent");
        assert!(pool_policies[0].extra_egress.is_empty());
    }

    #[tokio::test]
    async fn reconcile_applies_default_pool_settings() {
        let mut annotations = BTreeMap::new();
        annotations.insert("example.com/role".to_string(), "team-data".to_string());
        let settings = AgentPoolSettings {
            memory_limit: Some("32Gi".into()),
            memory_request: Some("16Gi".into()),
            max_concurrency: Some(2),
            image: Some("registry.example/cloacina-agent:1.2.3".into()),
            service_account_name: Some("team-data".into()),
            service_account_annotations: annotations.clone(),
            extra_egress: vec![AgentEgressRule {
                cidr: "198.51.100.0/24".into(),
                ports: vec![AgentEgressPort {
                    port: 443,
                    protocol: None,
                }],
            }],
            ..Default::default()
        };
        let ops = Arc::new(MockOps::with_ready(0));
        let act = actuator(ops.clone(), minter())
            .with_pools(Arc::new(FixedPools::default_pool(settings.clone())));
        act.reconcile("acme", 1).await.unwrap();

        let specs = ops.deployment_specs.lock().unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name, "cloacina-agent");
        assert_eq!(specs[0].image, "registry.example/cloacina-agent:1.2.3");
        assert_eq!(specs[0].resources.memory_limit, "32Gi");
        assert_eq!(specs[0].resources.memory_request, "16Gi");
        // Not set by the pool: the server-wide default.
        assert_eq!(specs[0].resources.cpu_limit, "1");
        assert_eq!(specs[0].max_concurrency, Some(2));
        assert_eq!(specs[0].service_account_name.as_deref(), Some("team-data"));
        assert_eq!(
            ops.service_accounts.lock().unwrap().as_slice(),
            &[(
                "cloacina-tenant-acme".to_string(),
                "team-data".to_string(),
                annotations
            )]
        );
        let pool_policies = ops.pool_policies.lock().unwrap();
        assert_eq!(pool_policies[0].extra_egress, settings.extra_egress);
    }

    #[tokio::test]
    async fn reconcile_runs_one_deployment_per_pool_at_its_own_count() {
        let ops = Arc::new(MockOps::with_ready(0));
        let act = actuator(ops.clone(), minter())
            .with_pools(Arc::new(FixedPools(vec![("big".to_string(), big_pool())])));
        // 3 is the desired count of the pool `default`; `big` runs its own 2.
        let out = act.reconcile("acme", 3).await.unwrap();
        assert_eq!(out.spawned, 5);
        assert_eq!(out.running, 5);

        assert_eq!(
            ops.scaled.lock().unwrap().as_slice(),
            &[
                ("cloacina-agent".to_string(), 3u32),
                ("cloacina-agent-big".to_string(), 2u32),
            ]
        );
        let specs = ops.deployment_specs.lock().unwrap();
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].pool, "default");
        assert_eq!(specs[0].resources, AgentResources::default());
        assert_eq!(specs[1].pool, "big");
        assert_eq!(specs[1].name, "cloacina-agent-big");
        assert_eq!(specs[1].resources.memory_limit, "32Gi");
        assert_eq!(specs[1].max_concurrency, Some(1));
        // Every pool is in the tenant's one namespace.
        assert!(specs.iter().all(|s| s.namespace == "cloacina-tenant-acme"));
        assert_eq!(
            ops.namespaces.lock().unwrap().as_slice(),
            &["cloacina-tenant-acme".to_string()]
        );
        let names: Vec<String> = ops
            .pool_policies
            .lock()
            .unwrap()
            .iter()
            .map(|p| p.name.clone())
            .collect();
        assert_eq!(
            names,
            vec!["cloacina-agent-pool-default", "cloacina-agent-pool-big"]
        );
    }

    #[tokio::test]
    async fn reconcile_writes_no_pool_policy_when_the_tenant_policy_is_off() {
        let mut cfg = config();
        cfg.network_policy_enabled = false;
        let ops = Arc::new(MockOps::with_ready(0));
        let act = KubernetesActuator::new(ops.clone(), minter(), cfg);
        act.reconcile("acme", 1).await.unwrap();
        assert!(ops.pool_policies.lock().unwrap().is_empty());

        let mut cfg = config();
        cfg.server_endpoint = None;
        let ops = Arc::new(MockOps::with_ready(0));
        let act = KubernetesActuator::new(ops.clone(), minter(), cfg);
        act.reconcile("acme", 1).await.unwrap();
        assert!(ops.pool_policies.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn reconcile_changes_nothing_when_the_pools_cannot_be_read() {
        let ops = Arc::new(MockOps::with_ready(2));
        let act = actuator(ops.clone(), minter()).with_pools(Arc::new(BrokenPools));
        let err = act.reconcile("acme", 2).await.unwrap_err();
        assert!(matches!(err, ActuatorError::Settings(_)));
        // Not resized to the defaults: nothing was applied or scaled.
        assert!(ops.namespaces.lock().unwrap().is_empty());
        assert!(ops.deployment_specs.lock().unwrap().is_empty());
        assert!(ops.scaled.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_failing_pool_does_not_stop_the_other_pools() {
        let mut ops = MockOps::with_ready(0);
        ops.fail_deployment = Some("cloacina-agent".to_string());
        let ops = Arc::new(ops);
        let act = actuator(ops.clone(), minter())
            .with_pools(Arc::new(FixedPools(vec![("big".to_string(), big_pool())])));
        let err = act.reconcile("acme", 3).await.unwrap_err();
        assert!(matches!(err, ActuatorError::Substrate(_)));
        // `big` was still applied and scaled.
        assert_eq!(
            ops.scaled.lock().unwrap().as_slice(),
            &[("cloacina-agent-big".to_string(), 2u32)]
        );
    }

    #[tokio::test]
    async fn remove_pool_deletes_the_pool_deployment_and_policy() {
        let ops = Arc::new(MockOps::with_ready(0));
        let act = actuator(ops.clone(), minter());
        act.remove_pool("acme", "big").await.unwrap();
        assert_eq!(
            ops.deleted.lock().unwrap().as_slice(),
            &[
                "deployment/cloacina-tenant-acme/cloacina-agent-big".to_string(),
                "networkpolicy/cloacina-tenant-acme/cloacina-agent-pool-big".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn remove_pool_refuses_the_default_pool() {
        let ops = Arc::new(MockOps::with_ready(0));
        let act = actuator(ops.clone(), minter());
        assert!(act.remove_pool("acme", "default").await.is_err());
        assert!(ops.deleted.lock().unwrap().is_empty());
    }

    #[test]
    fn parse_label_selector_handles_pairs_and_junk() {
        let m = parse_label_selector("a=1, b=2 ,,c=,=d,e=3");
        assert_eq!(m.get("a"), Some(&"1".to_string()));
        assert_eq!(m.get("b"), Some(&"2".to_string()));
        assert_eq!(m.get("e"), Some(&"3".to_string()));
        // empty value / empty key entries dropped.
        assert!(!m.contains_key("c"));
        assert_eq!(m.len(), 3);
    }

    /// CLOACINA-T-1055: agents from before the `agent` role (no key named on
    /// the Deployment), or on a key that is not a live `agent` key, get a new
    /// key: the Secret is written, every pool's pod template names the new
    /// key (so it rolls), and every other key of the tenant is revoked.
    #[tokio::test]
    async fn agents_on_an_old_key_are_moved_to_a_new_agent_key() {
        for old in [None, Some("read-key-from-before".to_string())] {
            let ops = Arc::new(MockOps::with_ready(2));
            *ops.key_id.lock().unwrap() = old;
            let minter = Arc::new(MockMinter::default());
            let act = actuator(ops.clone(), minter.clone());

            // Converged on the count: only the key is wrong.
            let out = act.reconcile("acme", 2).await.unwrap();
            assert_eq!(out.spawned, 0);
            assert_eq!(minter.minted.load(Ordering::SeqCst), 1);
            assert_eq!(ops.secrets.load(Ordering::SeqCst), 1);
            let named = ops.deployment_specs.lock().unwrap()[0].key_id.clone();
            assert_eq!(named.as_deref(), Some("minted-0"));
            let kept = minter.kept.lock().unwrap().clone();
            assert_eq!(kept, vec![vec!["minted-0".to_string()]]);

            // The next reconcile finds the new key named and mints nothing.
            act.reconcile("acme", 2).await.unwrap();
            assert_eq!(minter.minted.load(Ordering::SeqCst), 1);
            assert_eq!(ops.secrets.load(Ordering::SeqCst), 1);
        }
    }

    /// Adding agents does not mint a key: the tenant keeps the one it has.
    #[tokio::test]
    async fn a_scale_up_keeps_the_tenants_key() {
        let ops = Arc::new(MockOps::with_ready(1));
        let minter = Arc::new(MockMinter::default());
        let act = actuator(ops.clone(), minter.clone());
        let out = act.reconcile("acme", 3).await.unwrap();
        assert_eq!(out.spawned, 2);
        assert_eq!(minter.minted.load(Ordering::SeqCst), 0);
        assert_eq!(ops.secrets.load(Ordering::SeqCst), 0);
        assert_eq!(
            ops.deployment_specs.lock().unwrap()[0].key_id.as_deref(),
            Some("minted-before")
        );
    }

    /// A pool that fails keeps the old keys: its pods may still hold one.
    #[tokio::test]
    async fn a_failed_pool_revokes_nothing() {
        let mut ops = MockOps::with_ready(0);
        ops.fail_deployment = Some(pool_deployment_name(DEFAULT_AGENT_POOL));
        let minter = Arc::new(MockMinter::default());
        let act = actuator(Arc::new(ops), minter.clone());
        assert!(act.reconcile("acme", 1).await.is_err());
        assert!(minter.kept.lock().unwrap().is_empty());
    }

    #[test]
    fn the_manifest_names_the_key_in_the_pod_template() {
        let mut spec = AgentDeployment {
            namespace: "cloacina-tenant-acme".to_string(),
            name: "cloacina-agent".to_string(),
            tenant_id: "acme".to_string(),
            pool: DEFAULT_AGENT_POOL.to_string(),
            max_concurrency: None,
            service_account_name: None,
            pod_labels: BTreeMap::new(),
            pod_annotations: string_map(&[(KEY_ID_ANNOTATION, "from-the-pool")]),
            automount_service_account_token: None,
            image: "cloacina-agent:latest".to_string(),
            server_url: "http://server:8080".to_string(),
            secret_name: AGENT_SECRET_NAME.to_string(),
            secret_key: AGENT_SECRET_KEY.to_string(),
            key_id: Some("key-1".to_string()),
            resources: config().resources,
        };
        let m = agent_deployment_manifest(&spec);
        // The actuator's value wins over a pool annotation of the same name.
        assert_eq!(
            m["spec"]["template"]["metadata"]["annotations"][KEY_ID_ANNOTATION],
            "key-1"
        );
        spec.key_id = None;
        spec.pod_annotations = BTreeMap::new();
        let m = agent_deployment_manifest(&spec);
        assert!(
            m["spec"]["template"]["metadata"]
                .get("annotations")
                .is_none()
        );
    }

    #[tokio::test]
    async fn reconcile_scales_down_without_minting() {
        let ops = Arc::new(MockOps::with_ready(5));
        let minter = Arc::new(MockMinter::default());
        let act = actuator(ops.clone(), minter.clone());

        let out = act.reconcile("acme", 2).await.unwrap();

        assert_eq!(out.spawned, 0);
        assert_eq!(out.stopped, 3);
        assert_eq!(out.running, 2);
        // Scale-down mints no key and touches no Secret.
        assert_eq!(minter.minted.load(Ordering::SeqCst), 0);
        assert_eq!(ops.secrets.load(Ordering::SeqCst), 0);
        assert_eq!(
            ops.scales.lock().unwrap().as_slice(),
            &[("cloacina-tenant-acme".to_string(), 2u32)]
        );
    }

    #[tokio::test]
    async fn reconcile_is_noop_when_converged() {
        let ops = Arc::new(MockOps::with_ready(3));
        let minter = Arc::new(MockMinter::default());
        let act = actuator(ops.clone(), minter.clone());

        let out = act.reconcile("acme", 3).await.unwrap();

        assert_eq!(out.spawned, 0);
        assert_eq!(out.stopped, 0);
        assert_eq!(out.running, 3);
        assert_eq!(minter.minted.load(Ordering::SeqCst), 0);
        assert_eq!(ops.secrets.load(Ordering::SeqCst), 0);
        // Deployment is still ensured (idempotent) and replicas reasserted.
        assert_eq!(ops.deployments.load(Ordering::SeqCst), 1);
        assert_eq!(
            ops.scales.lock().unwrap().as_slice(),
            &[("cloacina-tenant-acme".to_string(), 3u32)]
        );
    }

    #[tokio::test]
    async fn reconcile_scales_to_zero_without_minting() {
        let ops = Arc::new(MockOps::with_ready(2));
        let minter = Arc::new(MockMinter::default());
        let act = actuator(ops.clone(), minter.clone());

        let out = act.reconcile("acme", 0).await.unwrap();

        assert_eq!(out.spawned, 0);
        assert_eq!(out.stopped, 2);
        assert_eq!(out.running, 0);
        assert_eq!(minter.minted.load(Ordering::SeqCst), 0);
        assert_eq!(
            ops.scales.lock().unwrap().as_slice(),
            &[("cloacina-tenant-acme".to_string(), 0u32)]
        );
    }

    #[test]
    fn kind_is_kubernetes() {
        let ops = Arc::new(MockOps::with_ready(0));
        let minter = Arc::new(MockMinter::default());
        assert_eq!(actuator(ops, minter).kind(), "kubernetes");
    }

    #[test]
    fn namespace_is_sanitized_and_prefixed() {
        assert_eq!(tenant_namespace("acme"), "cloacina-tenant-acme");
        // Uppercase + underscores → lowercase + single dashes.
        assert_eq!(tenant_namespace("Acme_Corp"), "cloacina-tenant-acme-corp");
        // UUID-style ids stay valid labels.
        assert_eq!(
            tenant_namespace("11111111-2222-3333-4444-555555555555"),
            "cloacina-tenant-11111111-2222-3333-4444-555555555555"
        );
    }

    #[test]
    fn namespace_trims_and_collapses_invalid_runs() {
        // Leading/trailing junk trimmed; internal invalid runs collapse to one dash.
        assert_eq!(tenant_namespace("__a..b__"), "cloacina-tenant-a-b");
        // Wholly-invalid id collapses to the stable fallback.
        assert_eq!(tenant_namespace("___"), "cloacina-tenant-unnamed");
    }

    #[test]
    fn namespace_respects_dns_label_limit() {
        let long = "x".repeat(200);
        let ns = tenant_namespace(&long);
        assert!(ns.len() <= 63, "namespace too long: {} ({})", ns.len(), ns);
        assert!(ns.starts_with("cloacina-tenant-"));
    }

    #[test]
    fn config_defaults() {
        for k in [
            "CLOACINA_AGENT_IMAGE",
            "CLOACINA_AGENT_SERVER_URL",
            "CLOACINA_AGENT_CPU_REQUEST",
            "CLOACINA_AGENT_MEMORY_REQUEST",
            "CLOACINA_AGENT_CPU_LIMIT",
            "CLOACINA_AGENT_MEMORY_LIMIT",
            "CLOACINA_FLEET_NETWORK_POLICY",
            "CLOACINA_FLEET_DNS_NAMESPACE",
            "CLOACINA_SERVER_NAMESPACE",
            "CLOACINA_SERVER_POD_SELECTOR",
            "CLOACINA_SERVER_PORT",
        ] {
            // FIXME: Audit that the environment access only happens in single-threaded code.
            unsafe { std::env::remove_var(k) };
        }
        let c = KubernetesConfig::from_env();
        assert_eq!(c.image, "cloacina-agent:latest");
        assert_eq!(c.server_url, "http://server:8080");
        assert_eq!(c.resources.cpu_request, "250m");
        assert_eq!(c.resources.memory_limit, "1Gi");
        // Default ON, but skipped at reconcile when the server endpoint is absent.
        assert!(c.network_policy_enabled);
        assert!(c.server_endpoint.is_none());
        assert_eq!(c.dns_namespace, "kube-system");

        // Second phase (kept in ONE test to avoid an env-var race with the
        // defaults phase above): plumbing the server endpoint env builds it.
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var("CLOACINA_SERVER_NAMESPACE", "cloacina-system") };
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe {
            std::env::set_var(
                "CLOACINA_SERVER_POD_SELECTOR",
                "app.kubernetes.io/name=cloacina-server,app.kubernetes.io/instance=rel",
            )
        };
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var("CLOACINA_SERVER_PORT", "8080") };
        let c = KubernetesConfig::from_env();
        let ep = c.server_endpoint.expect("endpoint built from env");
        assert_eq!(ep.namespace, "cloacina-system");
        assert_eq!(ep.port, 8080);
        assert_eq!(
            ep.pod_labels.get("app.kubernetes.io/name"),
            Some(&"cloacina-server".to_string())
        );
        for k in [
            "CLOACINA_SERVER_NAMESPACE",
            "CLOACINA_SERVER_POD_SELECTOR",
            "CLOACINA_SERVER_PORT",
        ] {
            // FIXME: Audit that the environment access only happens in single-threaded code.
            unsafe { std::env::remove_var(k) };
        }
    }
}
