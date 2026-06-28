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
//! ## Tenant isolation (NFR-004 / REQ-007)
//! Every namespace/deployment/secret is keyed by `tenant_namespace(tenant)`, so
//! the actuator only ever touches the requesting tenant's namespace.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, warn};

use cloacina::database::Database;

use super::docker::{DalKeyMinter, KeyMinter};
use super::{reconcile_delta, ActuatorError, FleetActuator, ReconcileOutcome};

/// Label marking a workload as actuator-managed.
const LABEL_MANAGED: &str = "cloacina.managed";
/// Label binding a managed workload to its tenant.
const LABEL_TENANT: &str = "cloacina.tenant";
/// Kubernetes app label (matches the chart's convention).
const LABEL_APP: &str = "app.kubernetes.io/name";

/// Name of the single per-tenant agent `Deployment` (lives in the tenant's
/// namespace, so the name need not encode the tenant).
const AGENT_DEPLOYMENT_NAME: &str = "cloacina-agent";
/// Name of the per-tenant `Secret` holding the agent key.
const AGENT_SECRET_NAME: &str = "cloacina-agent-key";
/// Key within [`AGENT_SECRET_NAME`] holding the plaintext agent key.
const AGENT_SECRET_KEY: &str = "api-key";
/// Name of the per-tenant agent `NetworkPolicy` (REQ-007, CLOACI-T-0819).
const AGENT_NETWORK_POLICY_NAME: &str = "cloacina-agent-fleet";

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
#[derive(Debug, Clone)]
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

        let defaults = AgentResources::default();
        let resources = AgentResources {
            cpu_request: env_nonempty("CLOACINA_AGENT_CPU_REQUEST").unwrap_or(defaults.cpu_request),
            memory_request: env_nonempty("CLOACINA_AGENT_MEMORY_REQUEST")
                .unwrap_or(defaults.memory_request),
            cpu_limit: env_nonempty("CLOACINA_AGENT_CPU_LIMIT").unwrap_or(defaults.cpu_limit),
            memory_limit: env_nonempty("CLOACINA_AGENT_MEMORY_LIMIT")
                .unwrap_or(defaults.memory_limit),
        };

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

/// Everything [`KubeOps::ensure_deployment`] needs to materialize one tenant's
/// agent `Deployment` (replica count is driven separately by `scale_deployment`
/// so it is intentionally absent here).
#[derive(Debug, Clone)]
pub struct AgentDeployment {
    pub namespace: String,
    pub name: String,
    pub tenant_id: String,
    pub image: String,
    pub server_url: String,
    /// Secret the agent key is read from (referenced as `CLOACINA_API_KEY`).
    pub secret_name: String,
    pub secret_key: String,
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
    /// Ensure the tenant's agent `Deployment` exists (does NOT set replicas).
    /// Idempotent.
    async fn ensure_deployment(&self, spec: &AgentDeployment) -> Result<(), ActuatorError>;
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
}

/// Kubernetes fleet actuator. Generic over its API substrate ([`KubeOps`]) and
/// key mint ([`KeyMinter`]) for testability; production wiring uses
/// [`KubeApiOps`] + [`DalKeyMinter`] via [`KubernetesActuator::from_env`].
pub struct KubernetesActuator {
    ops: Arc<dyn KubeOps>,
    minter: Arc<dyn KeyMinter>,
    config: KubernetesConfig,
}

impl KubernetesActuator {
    /// Construct with explicit collaborators (used by tests).
    pub fn new(
        ops: Arc<dyn KubeOps>,
        minter: Arc<dyn KeyMinter>,
        config: KubernetesConfig,
    ) -> Self {
        Self {
            ops,
            minter,
            config,
        }
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
            minter: Arc::new(DalKeyMinter::new(database)),
            config: KubernetesConfig::from_env(),
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
        let name = AGENT_DEPLOYMENT_NAME;

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

        // Ready replicas are our notion of "running" capacity. Absent
        // Deployment → 0 (treated as a cold start).
        let running = self.ops.count_ready_replicas(&namespace, name).await?;
        let delta = reconcile_delta(running, desired);

        // Only (re)mint a key + upsert the Secret when we are adding capacity.
        //
        // DECISION (CLOACI-T-0814, flagged): a single per-tenant Secret backs
        // the whole Deployment, so all replicas share one key (unlike the
        // Docker actuator, which mints one key per container). We re-mint on
        // every scale-UP event (not every reconcile — at convergence
        // `to_spawn == 0`), upserting the Secret in place; existing pods keep
        // their env until rescheduled. Key revocation on scale-down is deferred,
        // same as the Docker actuator.
        if delta.to_spawn > 0 {
            let api_key = self.minter.mint(tenant_id).await?;
            self.ops
                .ensure_secret(&namespace, AGENT_SECRET_NAME, AGENT_SECRET_KEY, &api_key)
                .await?;
        }

        // Ensure the Deployment object exists (idempotent; never sets replicas).
        let spec = AgentDeployment {
            namespace: namespace.clone(),
            name: name.to_string(),
            tenant_id: tenant_id.to_string(),
            image: self.config.image.clone(),
            server_url: self.config.server_url.clone(),
            secret_name: AGENT_SECRET_NAME.to_string(),
            secret_key: AGENT_SECRET_KEY.to_string(),
            resources: self.config.resources.clone(),
        };
        self.ops.ensure_deployment(&spec).await?;

        // Drive replicas to desired.
        self.ops.scale_deployment(&namespace, name, desired).await?;

        let spawned = delta.to_spawn;
        let stopped = delta.to_stop;
        if spawned > 0 || stopped > 0 {
            info!(
                tenant = %tenant_id,
                namespace = %namespace,
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

    fn kind(&self) -> &'static str {
        "kubernetes"
    }
}

// ---------------------------------------------------------------------------
// Real KubeOps over the Kubernetes API (`kube` + `k8s-openapi`).
// ---------------------------------------------------------------------------

use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::{Namespace, Secret};
use k8s_openapi::api::networking::v1::NetworkPolicy;
use kube::api::{Patch, PatchParams};
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
    serde_json::json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {
            "name": spec.name,
            "namespace": spec.namespace,
            "labels": {
                LABEL_APP: "cloacina-agent",
                LABEL_MANAGED: "true",
                LABEL_TENANT: spec.tenant_id,
            },
        },
        "spec": {
            "selector": {
                "matchLabels": { LABEL_APP: "cloacina-agent" }
            },
            "template": {
                "metadata": {
                    "labels": {
                        LABEL_APP: "cloacina-agent",
                        LABEL_MANAGED: "true",
                        LABEL_TENANT: spec.tenant_id,
                    }
                },
                "spec": {
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
    })
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex;

    /// Mock that records ensure/scale calls and reports a fixed ready count.
    struct MockOps {
        ready: u32,
        namespaces: Mutex<Vec<String>>,
        secrets: AtomicU32,
        deployments: AtomicU32,
        scales: Mutex<Vec<(String, u32)>>,
        network_policies: Mutex<Vec<NetworkPolicySpec>>,
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
        async fn ensure_deployment(&self, _spec: &AgentDeployment) -> Result<(), ActuatorError> {
            self.deployments.fetch_add(1, Ordering::SeqCst);
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
            _name: &str,
            replicas: u32,
        ) -> Result<(), ActuatorError> {
            self.scales
                .lock()
                .unwrap()
                .push((namespace.to_string(), replicas));
            Ok(())
        }
        async fn count_ready_replicas(
            &self,
            _namespace: &str,
            _name: &str,
        ) -> Result<u32, ActuatorError> {
            Ok(self.ready)
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
        let minter = Arc::new(MockMinter {
            minted: AtomicU32::new(0),
        });
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
        let minter = Arc::new(MockMinter {
            minted: AtomicU32::new(0),
        });
        let act = KubernetesActuator::new(ops.clone(), minter, cfg);
        act.reconcile("acme", 1).await.unwrap();
        assert!(ops.network_policies.lock().unwrap().is_empty());

        // Explicitly disabled → policy skipped even when plumbed.
        let mut cfg = config();
        cfg.network_policy_enabled = false;
        let ops = Arc::new(MockOps::with_ready(0));
        let minter = Arc::new(MockMinter {
            minted: AtomicU32::new(0),
        });
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
            image: "cloacina-agent:latest".to_string(),
            server_url: "http://server:8080".to_string(),
            secret_name: AGENT_SECRET_NAME.to_string(),
            secret_key: AGENT_SECRET_KEY.to_string(),
            resources: AgentResources::default(),
        };
        let m = agent_deployment_manifest(&spec);
        let pod = &m["spec"]["template"]["spec"];

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

    #[tokio::test]
    async fn reconcile_scales_down_without_minting() {
        let ops = Arc::new(MockOps::with_ready(5));
        let minter = Arc::new(MockMinter {
            minted: AtomicU32::new(0),
        });
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
        let minter = Arc::new(MockMinter {
            minted: AtomicU32::new(0),
        });
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
        let minter = Arc::new(MockMinter {
            minted: AtomicU32::new(0),
        });
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
        let minter = Arc::new(MockMinter {
            minted: AtomicU32::new(0),
        });
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
            std::env::remove_var(k);
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
        std::env::set_var("CLOACINA_SERVER_NAMESPACE", "cloacina-system");
        std::env::set_var(
            "CLOACINA_SERVER_POD_SELECTOR",
            "app.kubernetes.io/name=cloacina-server,app.kubernetes.io/instance=rel",
        );
        std::env::set_var("CLOACINA_SERVER_PORT", "8080");
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
            std::env::remove_var(k);
        }
    }
}
