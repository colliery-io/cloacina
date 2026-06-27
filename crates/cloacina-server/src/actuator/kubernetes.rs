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

use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;

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
/// Server-side-apply field manager — owns the namespace/secret/deployment
/// objects (but NOT `spec.replicas`, which is driven separately so the
/// actuator's apply never fights its own scale).
const FIELD_MANAGER: &str = "cloacina-fleet-actuator";

/// Default namespace prefix (REQ-007 — per-tenant namespace isolation).
const NAMESPACE_PREFIX: &str = "cloacina-tenant-";

/// Maximum length of a DNS-1123 label (Kubernetes namespace/object names).
const DNS_LABEL_MAX: usize = 63;

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
}

impl KubernetesConfig {
    pub fn from_env() -> Self {
        let image = std::env::var("CLOACINA_AGENT_IMAGE")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "cloacina-agent:latest".to_string());
        let server_url = std::env::var("CLOACINA_AGENT_SERVER_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "http://server:8080".to_string());
        Self { image, server_url }
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
    let segment = if segment.is_empty() { "unnamed" } else { segment };
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
    /// Set the agent `Deployment`'s replica count.
    async fn scale_deployment(
        &self,
        namespace: &str,
        name: &str,
        replicas: u32,
    ) -> Result<(), ActuatorError>;
    /// Current ready replica count for the agent `Deployment` (0 if absent).
    async fn count_ready_replicas(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<u32, ActuatorError>;
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
use kube::api::{Patch, PatchParams};
use kube::{Api, Client, Config};

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
        let client = Client::try_from(config)
            .map_err(|e| ActuatorError::Substrate(format!("kube client construction failed: {e}")))?;
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
        // NB: no `spec.replicas` here — scaling is owned by `scale_deployment`.
        let deployment = serde_json::json!({
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
                        "containers": [{
                            "name": "cloacina-agent",
                            "image": spec.image,
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
                            ]
                        }]
                    }
                }
            }
        });
        api.patch(&spec.name, &Self::apply_params(), &Patch::Apply(&deployment))
            .await
            .map_err(|e| ActuatorError::Substrate(format!("ensure_deployment failed: {e}")))?;
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
    }

    impl MockOps {
        fn with_ready(ready: u32) -> Self {
            Self {
                ready,
                namespaces: Mutex::new(Vec::new()),
                secrets: AtomicU32::new(0),
                deployments: AtomicU32::new(0),
                scales: Mutex::new(Vec::new()),
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

    fn config() -> KubernetesConfig {
        KubernetesConfig {
            image: "cloacina-agent:latest".to_string(),
            server_url: "http://server:8080".to_string(),
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
        assert_eq!(
            tenant_namespace("Acme_Corp"),
            "cloacina-tenant-acme-corp"
        );
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
        std::env::remove_var("CLOACINA_AGENT_IMAGE");
        std::env::remove_var("CLOACINA_AGENT_SERVER_URL");
        let c = KubernetesConfig::from_env();
        assert_eq!(c.image, "cloacina-agent:latest");
        assert_eq!(c.server_url, "http://server:8080");
    }
}
