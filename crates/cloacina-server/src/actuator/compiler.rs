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

//! A compiler per tenant (CLOACINA-T-1034).
//!
//! A `cloacina-compiler` serves one schema (`--tenant-schema`). For every
//! tenant, `public` included (CLOACINA-T-1063), the fleet actuator runs one
//! compiler, next to the tenant's agents, and only while the tenant's build
//! queue has work:
//!
//! - the fleet loop reads each tenant's queue on every tick
//!   ([`run_compiler_pass`]) and decides with [`compiler_want`];
//! - a queue with a `pending` or `building` row starts the compiler;
//! - a queue that has been empty for the cooldown stops it (0 replicas);
//! - a tenant whose queue was never filled gets nothing at all.
//!
//! Nothing is stored: the decision is computed from the queue rows on each
//! tick, so it does not matter which replica leads the tick (CLOACINA-A-0008).
//!
//! Every package waits in the queue, Python ones too: the compiler runs no
//! cargo for them, but it is the only process that completes the row.
//!
//! A compiler connects with its tenant's build role (CLOACINA-T-1063), which
//! reaches the four build tables of that tenant's schema only. The server
//! sets a new password at each start of the compiler and writes the URL only
//! where that compiler reads it ([`BuildCredentials`]); it keeps no copy.
//!
//! The Kubernetes side is behind [`CompilerKubeOps`] and the Docker side
//! behind [`CompilerContainerOps`], so both are tested without a cluster or a
//! daemon.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::{debug, info, warn};

use super::kubernetes::{AgentResources, tenant_namespace};
use super::{ActuatorError, FleetActuator};

/// Name of a tenant's compiler `Deployment`, of its `NetworkPolicy` and the
/// value of its pods' `app.kubernetes.io/name` label.
pub const COMPILER_NAME: &str = "cloacina-compiler";
/// Name of the `Secret` that holds the compiler's database URL.
pub const COMPILER_SECRET_NAME: &str = "cloacina-compiler-db";
/// Key of the database URL in [`COMPILER_SECRET_NAME`].
pub const COMPILER_SECRET_KEY: &str = "database-url";

const LABEL_APP: &str = "app.kubernetes.io/name";
const LABEL_COMPONENT: &str = "app.kubernetes.io/component";
const LABEL_MANAGED: &str = "cloacina.managed";
const LABEL_TENANT: &str = "cloacina.tenant";

/// uid and gid of a hardened compiler pod (the agents use the same).
const COMPILER_RUN_AS: i64 = 10001;
/// Where a hardened pod mounts its writable volumes.
const HOME_DIR: &str = "/var/cloacina/home";
const CARGO_HOME_DIR: &str = "/var/cloacina/cargo";

/// Private and link-local IPv4 ranges. The `internet` crate egress leaves
/// them out, so a build cannot reach the cluster, the node or a cloud
/// metadata address through that rule.
const NOT_INTERNET: [&str; 5] = [
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "100.64.0.0/10",
    "169.254.0.0/16",
];

/// Docker labels of a tenant's compiler container. Not `cloacina.managed`:
/// the agent reconcile counts every container with that label as an agent.
const DOCKER_LABEL_COMPILER: &str = "cloacina.compiler";
const DOCKER_LABEL_TENANT: &str = "cloacina.compiler-tenant";
/// Docker label of a compiler container that connects with its tenant's
/// build role (CLOACINA-T-1063). A compiler container without it was started
/// by an older server with the server's own URL, and is stopped.
pub const DOCKER_LABEL_CREDENTIAL: &str = "cloacina.compiler-credential";
/// The value of [`DOCKER_LABEL_CREDENTIAL`].
pub const DOCKER_CREDENTIAL_BUILD_ROLE: &str = "build-role";

/// Annotation on the pod template of a tenant's compiler: an id of the build
/// role password in its Secret (CLOACINA-T-1063). A new password comes with a
/// new id, so the pod is replaced and reads the new Secret. A Deployment
/// without it was made by an older server, with the server's own URL.
pub const BUILD_CREDENTIAL_ANNOTATION: &str = "cloacina.io/build-credential";

// ---------------------------------------------------------------------------
// The build role's credential (CLOACINA-T-1063)
// ---------------------------------------------------------------------------

/// Gives a tenant's build role a new password. The compiler side calls it
/// each time it starts a tenant's compiler, and writes the URL only where
/// that compiler reads it. Nothing keeps the password.
#[async_trait]
pub trait BuildCredentials: Send + Sync {
    async fn issue(
        &self,
        tenant_id: &str,
    ) -> Result<cloacina::database::BuildCredential, ActuatorError>;
}

/// [`BuildCredentials`] over the server's admin database.
pub struct AdminBuildCredentials {
    admin: cloacina::database::Database,
}

impl AdminBuildCredentials {
    pub fn new(admin: cloacina::database::Database) -> Self {
        Self { admin }
    }
}

#[async_trait]
impl BuildCredentials for AdminBuildCredentials {
    async fn issue(
        &self,
        tenant_id: &str,
    ) -> Result<cloacina::database::BuildCredential, ActuatorError> {
        cloacina::database::DatabaseAdmin::new(self.admin.clone())
            .issue_build_password(tenant_id)
            .await
            .map_err(|e| {
                ActuatorError::Substrate(format!(
                    "build role password for tenant '{tenant_id}': {e}"
                ))
            })
    }
}

/// The URL a tenant's compiler connects with: the compilers' database URL
/// (where they reach the database) with the build role as its user.
pub fn build_role_url(
    database_url: &str,
    credential: &cloacina::database::BuildCredential,
) -> Result<String, ActuatorError> {
    cloacina::database::connection::inject_credentials(
        database_url,
        &credential.role,
        &credential.password,
    )
    .ok_or_else(|| {
        ActuatorError::Substrate(
            "the compilers' database URL cannot take the build role's credentials".to_string(),
        )
    })
}

// ---------------------------------------------------------------------------
// The scale decision
// ---------------------------------------------------------------------------

/// What the loop wants of a tenant's compiler on one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerWant {
    /// The queue has work: one compiler.
    Run,
    /// The queue is empty but was not empty for the whole cooldown: leave the
    /// compiler as it is. This never starts one.
    Keep,
    /// The queue has been empty for the cooldown: no compiler.
    Stop,
}

/// What one [`FleetActuator::reconcile_compiler`] call did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompilerOutcome {
    /// A compiler was started (from none, or from 0 replicas).
    pub started: bool,
    /// A running compiler was stopped.
    pub stopped: bool,
}

/// A tenant's build queue, as far as the decision needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildQueue {
    /// Rows `pending`, not superseded: what the compiler would claim.
    pub pending: u64,
    /// Rows `building`, not superseded.
    pub building: u64,
    /// The newest of: last successful build, last failed build, last
    /// heartbeat. `None`: the queue never held a build.
    pub last_activity: Option<DateTime<Utc>>,
}

impl BuildQueue {
    /// Whether a compiler has something to do.
    pub fn has_work(&self) -> bool {
        self.pending > 0 || self.building > 0
    }
}

impl From<&cloacina::registry::workflow_registry::BuildQueueStats> for BuildQueue {
    fn from(s: &cloacina::registry::workflow_registry::BuildQueueStats) -> Self {
        Self {
            pending: s.pending,
            building: s.building,
            last_activity: [s.last_success_at, s.last_failure_at, s.heartbeat_at]
                .into_iter()
                .flatten()
                .max(),
        }
    }
}

/// Decide what a tenant's compiler should be. Pure.
///
/// Work → [`CompilerWant::Run`]. No work and the last build activity is
/// younger than `cooldown` → [`CompilerWant::Keep`]. Else
/// [`CompilerWant::Stop`]. A tenant that never had a build is `Stop`, which
/// creates nothing.
pub fn compiler_want(queue: &BuildQueue, now: DateTime<Utc>, cooldown: Duration) -> CompilerWant {
    if queue.has_work() {
        return CompilerWant::Run;
    }
    let cooldown = chrono::Duration::from_std(cooldown).unwrap_or(chrono::Duration::MAX);
    match queue.last_activity {
        Some(at) if now.signed_duration_since(at) < cooldown => CompilerWant::Keep,
        _ => CompilerWant::Stop,
    }
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// Where a build may fetch crates from, in the compiler's `NetworkPolicy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrateEgress {
    /// TCP 443 to every public IPv4 address (private and link-local ranges
    /// left out). The default.
    Internet,
    /// Every destination and port.
    All,
    /// Nothing more than DNS, the database and the listed CIDRs: for an
    /// install that builds from a vendor directory or a mirror.
    None,
}

/// Server-wide settings of the per-tenant compilers, read from the
/// environment ([`CompilerSettings::from_env`]).
#[derive(Clone)]
pub struct CompilerSettings {
    /// Compiler image (`CLOACINA_TENANT_COMPILER_IMAGE`).
    pub image: String,
    /// The database URL the compilers connect with
    /// (`CLOACINA_TENANT_COMPILER_DATABASE_URL`, default: the server's own).
    pub database_url: String,
    /// The whole cargo flag list (`CLOACINA_TENANT_COMPILER_CARGO_FLAGS`,
    /// separated by spaces), passed as `--cargo-flags-replace`. Empty: the
    /// compiler's own default (`build --release --lib --frozen --offline`).
    pub cargo_flags: Vec<String>,
    /// `--dev-workspace`: a path in the image
    /// (`CLOACINA_TENANT_COMPILER_DEV_WORKSPACE`). Development only.
    pub dev_workspace: Option<String>,
    /// `--vendor-dir`: a path in the image
    /// (`CLOACINA_TENANT_COMPILER_VENDOR_DIR`).
    pub vendor_dir: Option<String>,
    /// `--cargo-target-dir` (`CLOACINA_TENANT_COMPILER_TARGET_DIR`, default
    /// `/workspace/target`). A hardened pod mounts an emptyDir there.
    pub target_dir: String,
    /// `--build-timeout-s` (`CLOACINA_TENANT_COMPILER_BUILD_TIMEOUT_S`).
    /// `None`: the compiler's default.
    pub build_timeout_s: Option<u64>,
    /// `--poll-interval-ms` (`CLOACINA_TENANT_COMPILER_POLL_INTERVAL_MS`,
    /// default 1000).
    pub poll_interval_ms: u64,
    /// Requests and limits of the compiler container
    /// (`CLOACINA_TENANT_COMPILER_CPU_REQUEST`, `_MEMORY_REQUEST`,
    /// `_CPU_LIMIT`, `_MEMORY_LIMIT`).
    pub resources: AgentResources,
    /// Size limit of the emptyDirs for the target directory and the cargo
    /// home (`CLOACINA_TENANT_COMPILER_CACHE_SIZE`, default `8Gi`).
    pub cache_size: String,
    /// Pod hardening (`CLOACINA_TENANT_COMPILER_HARDENED`, default on): not
    /// root, read-only root filesystem, no capabilities, writable paths on
    /// emptyDir volumes. Off: the pod runs as the image is built, with the
    /// image's own filesystem. For a development image only.
    pub hardened: bool,
    /// How long the queue must be empty before the compiler is stopped
    /// (`CLOACINA_TENANT_COMPILER_COOLDOWN_S`, default 300).
    pub cooldown: Duration,
    /// Crate egress of the NetworkPolicy
    /// (`CLOACINA_TENANT_COMPILER_EGRESS`: `internet`, `all` or `none`).
    pub crate_egress: CrateEgress,
    /// More egress CIDRs, every port
    /// (`CLOACINA_TENANT_COMPILER_EGRESS_CIDRS`, separated by commas): a
    /// registry mirror, a git host, a proxy.
    pub egress_cidrs: Vec<String>,
    /// CIDRs of the database (`CLOACINA_TENANT_COMPILER_DATABASE_CIDRS`).
    /// Empty: the database port is allowed to every destination.
    pub database_cidrs: Vec<String>,
    /// Docker network of a compiler container (`CLOACINA_AGENT_NETWORK`, the
    /// one the agents use). Docker actuator only.
    pub docker_network: Option<String>,
}

impl std::fmt::Debug for CompilerSettings {
    /// The database URL holds a password: it is never printed.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompilerSettings")
            .field("image", &self.image)
            .field("database_url", &"<redacted>")
            .field("cargo_flags", &self.cargo_flags)
            .field("dev_workspace", &self.dev_workspace)
            .field("vendor_dir", &self.vendor_dir)
            .field("target_dir", &self.target_dir)
            .field("build_timeout_s", &self.build_timeout_s)
            .field("poll_interval_ms", &self.poll_interval_ms)
            .field("resources", &self.resources)
            .field("cache_size", &self.cache_size)
            .field("hardened", &self.hardened)
            .field("cooldown", &self.cooldown)
            .field("crate_egress", &self.crate_egress)
            .field("egress_cidrs", &self.egress_cidrs)
            .field("database_cidrs", &self.database_cidrs)
            .field("docker_network", &self.docker_network)
            .finish()
    }
}

impl CompilerSettings {
    /// The defaults, with the given database URL.
    pub fn with_database_url(database_url: &str) -> Self {
        Self {
            image: "cloacina-compiler:latest".to_string(),
            database_url: database_url.to_string(),
            cargo_flags: Vec::new(),
            dev_workspace: None,
            vendor_dir: None,
            target_dir: "/workspace/target".to_string(),
            build_timeout_s: None,
            poll_interval_ms: 1000,
            resources: AgentResources {
                cpu_request: "500m".to_string(),
                memory_request: "1Gi".to_string(),
                cpu_limit: "2".to_string(),
                memory_limit: "4Gi".to_string(),
            },
            cache_size: "8Gi".to_string(),
            hardened: true,
            cooldown: Duration::from_secs(300),
            crate_egress: CrateEgress::Internet,
            egress_cidrs: Vec::new(),
            database_cidrs: Vec::new(),
            docker_network: None,
        }
    }

    /// Read the settings from the process environment. `Ok(None)`: per-tenant
    /// compilers are off (`CLOACINA_TENANT_COMPILERS` is not true), which is
    /// the default. `server_database_url` is used when no own URL is set.
    /// A value that cannot be used is an error: the server must not start
    /// with half the settings.
    pub fn from_env(server_database_url: &str) -> Result<Option<Self>, String> {
        Self::from_lookup(server_database_url, |key| std::env::var(key).ok())
    }

    /// [`Self::from_env`] over any lookup, so tests need no process
    /// environment.
    pub fn from_lookup(
        server_database_url: &str,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Result<Option<Self>, String> {
        let get = |key: &str| {
            lookup(key)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let flag = |key: &str, default: bool| -> Result<bool, String> {
            match get(key).map(|v| v.to_ascii_lowercase()) {
                None => Ok(default),
                Some(v) if matches!(v.as_str(), "true" | "1" | "yes" | "on") => Ok(true),
                Some(v) if matches!(v.as_str(), "false" | "0" | "no" | "off") => Ok(false),
                Some(v) => Err(format!("{key}: '{v}' is not true or false")),
            }
        };
        let number = |key: &str| -> Result<Option<u64>, String> {
            get(key)
                .map(|v| {
                    v.parse::<u64>()
                        .map_err(|_| format!("{key}: '{v}' is not a whole number"))
                })
                .transpose()
        };
        let cidrs = |key: &str| -> Result<Vec<String>, String> {
            let mut out = Vec::new();
            for part in get(key).unwrap_or_default().split(',') {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                if !valid_cidr(part) {
                    return Err(format!("{key}: '{part}' is not a CIDR block"));
                }
                out.push(part.to_string());
            }
            Ok(out)
        };

        if !flag("CLOACINA_TENANT_COMPILERS", false)? {
            return Ok(None);
        }

        let mut s = Self::with_database_url(
            &get("CLOACINA_TENANT_COMPILER_DATABASE_URL")
                .unwrap_or_else(|| server_database_url.to_string()),
        );
        if let Some(image) = get("CLOACINA_TENANT_COMPILER_IMAGE") {
            s.image = image;
        }
        s.cargo_flags = get("CLOACINA_TENANT_COMPILER_CARGO_FLAGS")
            .map(|v| v.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default();
        s.dev_workspace = get("CLOACINA_TENANT_COMPILER_DEV_WORKSPACE");
        s.vendor_dir = get("CLOACINA_TENANT_COMPILER_VENDOR_DIR");
        if let Some(dir) = get("CLOACINA_TENANT_COMPILER_TARGET_DIR") {
            if !dir.starts_with('/') {
                return Err(format!(
                    "CLOACINA_TENANT_COMPILER_TARGET_DIR: '{dir}' is not an absolute path"
                ));
            }
            s.target_dir = dir;
        }
        s.build_timeout_s = number("CLOACINA_TENANT_COMPILER_BUILD_TIMEOUT_S")?;
        if let Some(ms) = number("CLOACINA_TENANT_COMPILER_POLL_INTERVAL_MS")? {
            s.poll_interval_ms = ms;
        }
        if let Some(v) = get("CLOACINA_TENANT_COMPILER_CPU_REQUEST") {
            s.resources.cpu_request = v;
        }
        if let Some(v) = get("CLOACINA_TENANT_COMPILER_MEMORY_REQUEST") {
            s.resources.memory_request = v;
        }
        if let Some(v) = get("CLOACINA_TENANT_COMPILER_CPU_LIMIT") {
            s.resources.cpu_limit = v;
        }
        if let Some(v) = get("CLOACINA_TENANT_COMPILER_MEMORY_LIMIT") {
            s.resources.memory_limit = v;
        }
        if let Some(v) = get("CLOACINA_TENANT_COMPILER_CACHE_SIZE") {
            s.cache_size = v;
        }
        s.hardened = flag("CLOACINA_TENANT_COMPILER_HARDENED", true)?;
        if let Some(secs) = number("CLOACINA_TENANT_COMPILER_COOLDOWN_S")? {
            s.cooldown = Duration::from_secs(secs);
        }
        s.crate_egress = match get("CLOACINA_TENANT_COMPILER_EGRESS")
            .map(|v| v.to_ascii_lowercase())
            .as_deref()
        {
            None | Some("internet") => CrateEgress::Internet,
            Some("all") => CrateEgress::All,
            Some("none") => CrateEgress::None,
            Some(other) => {
                return Err(format!(
                    "CLOACINA_TENANT_COMPILER_EGRESS: '{other}' is not internet, all or none"
                ));
            }
        };
        s.egress_cidrs = cidrs("CLOACINA_TENANT_COMPILER_EGRESS_CIDRS")?;
        s.database_cidrs = cidrs("CLOACINA_TENANT_COMPILER_DATABASE_CIDRS")?;
        s.docker_network = get("CLOACINA_AGENT_NETWORK");
        Ok(Some(s))
    }

    /// The compiler's arguments. The database URL is not among them: it is in
    /// the environment, from the Secret.
    pub fn args(&self, tenant_id: &str) -> Vec<String> {
        let mut args = vec![
            // Loopback: nothing probes the compiler, so it listens on no
            // address another pod can reach.
            "--bind".to_string(),
            "127.0.0.1:9000".to_string(),
            "--poll-interval-ms".to_string(),
            self.poll_interval_ms.to_string(),
            "--tenant-schema".to_string(),
            tenant_id.to_string(),
            "--cargo-target-dir".to_string(),
            self.target_dir.clone(),
        ];
        if self.hardened {
            args.push("--home".to_string());
            args.push(HOME_DIR.to_string());
        }
        for flag in &self.cargo_flags {
            args.push(format!("--cargo-flags-replace={flag}"));
        }
        if let Some(secs) = self.build_timeout_s {
            args.push("--build-timeout-s".to_string());
            args.push(secs.to_string());
        }
        args
    }

    /// The compiler's environment, without the database URL: `(name, value)`.
    pub fn plain_env(&self) -> Vec<(String, String)> {
        let mut env = Vec::new();
        if let Some(dir) = &self.dev_workspace {
            env.push(("CLOACINA_COMPILER_DEV_WORKSPACE".to_string(), dir.clone()));
        }
        if let Some(dir) = &self.vendor_dir {
            env.push(("CLOACINA_COMPILER_VENDOR_DIR".to_string(), dir.clone()));
        }
        if self.hardened {
            env.push(("HOME".to_string(), HOME_DIR.to_string()));
            env.push(("CARGO_HOME".to_string(), CARGO_HOME_DIR.to_string()));
        }
        env
    }
}

/// Whether `raw` is `address/prefix` with a prefix in range.
fn valid_cidr(raw: &str) -> bool {
    let Some((addr, prefix)) = raw.split_once('/') else {
        return false;
    };
    let Ok(prefix) = prefix.parse::<u8>() else {
        return false;
    };
    match addr.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(_)) => prefix <= 32,
        Ok(std::net::IpAddr::V6(_)) => prefix <= 128,
        Err(_) => false,
    }
}

/// The port of a Postgres URL; 5432 when it names none.
pub fn database_port(url: &str) -> u16 {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let authority = rest.split(['/', '?']).next().unwrap_or("");
    let host = authority
        .rsplit_once('@')
        .map(|(_, h)| h)
        .unwrap_or(authority);
    // An IPv6 host is in brackets: the port is after the closing one.
    let port = match host.rsplit_once(']') {
        Some((_, after)) => after.strip_prefix(':'),
        None => host.rsplit_once(':').map(|(_, p)| p),
    };
    port.and_then(|p| p.parse().ok()).unwrap_or(5432)
}

// ---------------------------------------------------------------------------
// Kubernetes
// ---------------------------------------------------------------------------

/// The kinds of object the compiler side applies or deletes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KubeKind {
    Namespace,
    Secret,
    NetworkPolicy,
    Deployment,
}

/// The Kubernetes operations the compiler side needs. Its own trait, apart
/// from the agents' `KubeOps`, so the two do not change together.
#[async_trait]
pub trait CompilerKubeOps: Send + Sync {
    /// Server-side apply of one object. A namespace has an empty `namespace`.
    async fn apply(
        &self,
        kind: KubeKind,
        namespace: &str,
        name: &str,
        manifest: &serde_json::Value,
    ) -> Result<(), ActuatorError>;
    /// Delete one object. Absent is not an error.
    async fn delete(
        &self,
        kind: KubeKind,
        namespace: &str,
        name: &str,
    ) -> Result<(), ActuatorError>;
    /// `spec.replicas` of a Deployment. `None`: there is no such Deployment.
    async fn deployment_replicas(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<Option<u32>, ActuatorError>;
    /// Set `spec.replicas` of a Deployment.
    async fn scale(&self, namespace: &str, name: &str, replicas: u32) -> Result<(), ActuatorError>;
    /// The [`BUILD_CREDENTIAL_ANNOTATION`] of a Deployment's pod template.
    /// `None`: no such Deployment, or none with the annotation.
    async fn build_credential_marker(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<Option<String>, ActuatorError>;
}

/// Put the build credential's id on the pod template of a compiler
/// Deployment (CLOACINA-T-1063).
pub fn with_build_credential_marker(
    mut deployment: serde_json::Value,
    marker: &str,
) -> serde_json::Value {
    let template = &mut deployment["spec"]["template"]["metadata"];
    if !template["annotations"].is_object() {
        template["annotations"] = serde_json::json!({});
    }
    template["annotations"][BUILD_CREDENTIAL_ANNOTATION] = serde_json::json!(marker);
    deployment
}

/// The tenant namespace manifest: the same object the agent side applies.
fn namespace_manifest(namespace: &str) -> serde_json::Value {
    serde_json::json!({
        "apiVersion": "v1",
        "kind": "Namespace",
        "metadata": { "name": namespace, "labels": { LABEL_MANAGED: "true" } }
    })
}

/// The Secret with the compiler's database URL. `None` writes an empty
/// value: that is how the credential is taken away when the tenant goes (the
/// server has no right to delete Secrets).
pub fn database_secret_manifest(namespace: &str, database_url: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "apiVersion": "v1",
        "kind": "Secret",
        "metadata": {
            "name": COMPILER_SECRET_NAME,
            "namespace": namespace,
            "labels": { LABEL_MANAGED: "true" },
        },
        "type": "Opaque",
        "stringData": { COMPILER_SECRET_KEY: database_url.unwrap_or("") },
    })
}

/// The compiler `Deployment` of one tenant. Pure. No `spec.replicas`: the
/// count is set apart, as for the agents.
///
/// No probes: nothing reaches the compiler's port (it listens on loopback),
/// and the build queue shows whether it works.
pub fn compiler_deployment_manifest(
    namespace: &str,
    tenant_id: &str,
    settings: &CompilerSettings,
) -> serde_json::Value {
    let r = &settings.resources;
    let mut env = vec![serde_json::json!({
        // From the Secret, never on the command line: an argument is in the
        // pod spec and in /proc/<pid>/cmdline.
        "name": "DATABASE_URL",
        "valueFrom": {
            "secretKeyRef": { "name": COMPILER_SECRET_NAME, "key": COMPILER_SECRET_KEY }
        }
    })];
    for (name, value) in settings.plain_env() {
        env.push(serde_json::json!({ "name": name, "value": value }));
    }

    let mut manifest = serde_json::json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {
            "name": COMPILER_NAME,
            "namespace": namespace,
            "labels": {
                LABEL_APP: COMPILER_NAME,
                LABEL_COMPONENT: "compiler",
                LABEL_MANAGED: "true",
                LABEL_TENANT: tenant_id,
            },
        },
        "spec": {
            // One compiler, and its caches are its own: the old pod goes
            // before the new one comes.
            "strategy": { "type": "Recreate" },
            "selector": { "matchLabels": { LABEL_APP: COMPILER_NAME } },
            "template": {
                "metadata": {
                    "labels": {
                        LABEL_APP: COMPILER_NAME,
                        LABEL_COMPONENT: "compiler",
                        LABEL_MANAGED: "true",
                        LABEL_TENANT: tenant_id,
                    }
                },
                "spec": {
                    // The compiler never calls the Kubernetes API, and build
                    // code could read a mounted token.
                    "automountServiceAccountToken": false,
                    "containers": [{
                        "name": COMPILER_NAME,
                        "image": settings.image,
                        "args": settings.args(tenant_id),
                        "env": env,
                        "resources": {
                            "requests": { "cpu": r.cpu_request, "memory": r.memory_request },
                            "limits": { "cpu": r.cpu_limit, "memory": r.memory_limit },
                        },
                    }],
                }
            }
        }
    });

    if settings.hardened {
        let pod = &mut manifest["spec"]["template"]["spec"];
        pod["securityContext"] = serde_json::json!({
            "runAsNonRoot": true,
            "runAsUser": COMPILER_RUN_AS,
            "runAsGroup": COMPILER_RUN_AS,
            "fsGroup": COMPILER_RUN_AS,
            "seccompProfile": { "type": "RuntimeDefault" },
        });
        pod["containers"][0]["securityContext"] = serde_json::json!({
            "allowPrivilegeEscalation": false,
            "readOnlyRootFilesystem": true,
            "capabilities": { "drop": ["ALL"] },
        });
        // A build writes: the unpacked source and the logs (home), cargo's
        // registry and git checkouts (cargo home), the build output (target)
        // and scratch files (/tmp). With a read-only root each is an emptyDir,
        // so a compiler that starts again starts with empty caches.
        pod["containers"][0]["volumeMounts"] = serde_json::json!([
            { "name": "home", "mountPath": HOME_DIR },
            { "name": "cargo-home", "mountPath": CARGO_HOME_DIR },
            { "name": "target", "mountPath": settings.target_dir },
            { "name": "tmp", "mountPath": "/tmp" },
        ]);
        pod["volumes"] = serde_json::json!([
            { "name": "home", "emptyDir": { "sizeLimit": "2Gi" } },
            { "name": "cargo-home", "emptyDir": { "sizeLimit": settings.cache_size } },
            { "name": "target", "emptyDir": { "sizeLimit": settings.cache_size } },
            { "name": "tmp", "emptyDir": { "sizeLimit": "1Gi" } },
        ]);
    }
    manifest
}

/// The `NetworkPolicy` of a tenant's compiler pods. Pure.
///
/// - Ingress: none. Nothing connects to a compiler.
/// - Egress: DNS; the database port; and crates, by
///   [`CompilerSettings::crate_egress`] and
///   [`CompilerSettings::egress_cidrs`].
///
/// The policy selects the compiler pods only. Policies add up, so the pods
/// also get what the tenant's agent policy allows (DNS and the server).
pub fn compiler_network_policy_manifest(
    namespace: &str,
    dns_namespace: &str,
    settings: &CompilerSettings,
) -> serde_json::Value {
    let ip_blocks = |cidrs: &[String]| -> Vec<serde_json::Value> {
        cidrs
            .iter()
            .map(|c| serde_json::json!({ "ipBlock": { "cidr": c } }))
            .collect()
    };

    let mut egress = vec![serde_json::json!({
        "to": [{
            "namespaceSelector": {
                "matchLabels": { "kubernetes.io/metadata.name": dns_namespace }
            }
        }],
        "ports": [
            { "protocol": "UDP", "port": 53 },
            { "protocol": "TCP", "port": 53 },
        ],
    })];

    // The database. A NetworkPolicy cannot name a host, so without CIDRs the
    // rule is the port alone, to every destination.
    let mut database = serde_json::json!({
        "ports": [{ "protocol": "TCP", "port": database_port(&settings.database_url) }],
    });
    if !settings.database_cidrs.is_empty() {
        database["to"] = ip_blocks(&settings.database_cidrs).into();
    }
    egress.push(database);

    match settings.crate_egress {
        CrateEgress::Internet => egress.push(serde_json::json!({
            "to": [{ "ipBlock": { "cidr": "0.0.0.0/0", "except": NOT_INTERNET } }],
            "ports": [{ "protocol": "TCP", "port": 443 }],
        })),
        // An empty rule allows every destination and port.
        CrateEgress::All => egress.push(serde_json::json!({})),
        CrateEgress::None => {}
    }
    if !settings.egress_cidrs.is_empty() {
        egress.push(serde_json::json!({ "to": ip_blocks(&settings.egress_cidrs) }));
    }

    serde_json::json!({
        "apiVersion": "networking.k8s.io/v1",
        "kind": "NetworkPolicy",
        "metadata": {
            "name": COMPILER_NAME,
            "namespace": namespace,
            "labels": { LABEL_MANAGED: "true" },
        },
        "spec": {
            "podSelector": { "matchLabels": { LABEL_APP: COMPILER_NAME } },
            "policyTypes": ["Ingress", "Egress"],
            "ingress": [],
            "egress": egress,
        }
    })
}

/// The compiler side of the Kubernetes actuator.
pub struct KubeCompilers {
    ops: Arc<dyn CompilerKubeOps>,
    settings: CompilerSettings,
    /// Whether to write the compiler's NetworkPolicy
    /// (`CLOACINA_FLEET_NETWORK_POLICY`, the agents' switch).
    network_policy: bool,
    dns_namespace: String,
    /// Issues the build role's password at each start (CLOACINA-T-1063).
    credentials: Arc<dyn BuildCredentials>,
}

impl KubeCompilers {
    pub fn new(
        ops: Arc<dyn CompilerKubeOps>,
        settings: CompilerSettings,
        network_policy: bool,
        dns_namespace: String,
        credentials: Arc<dyn BuildCredentials>,
    ) -> Self {
        Self {
            ops,
            settings,
            network_policy,
            dns_namespace,
            credentials,
        }
    }

    pub fn cooldown(&self) -> Duration {
        self.settings.cooldown
    }

    /// Move one tenant's compiler toward `want`.
    pub async fn reconcile(
        &self,
        tenant_id: &str,
        want: CompilerWant,
    ) -> Result<CompilerOutcome, ActuatorError> {
        if want == CompilerWant::Keep {
            return Ok(CompilerOutcome::default());
        }
        let namespace = tenant_namespace(tenant_id);
        let current = self
            .ops
            .deployment_replicas(&namespace, COMPILER_NAME)
            .await?;

        if want == CompilerWant::Stop {
            // No Deployment, or one at 0: nothing to do, and nothing is made
            // for a tenant that never had a build.
            if current.unwrap_or(0) == 0 {
                return Ok(CompilerOutcome::default());
            }
            self.ops.scale(&namespace, COMPILER_NAME, 0).await?;
            info!(tenant = %tenant_id, namespace = %namespace, "tenant compiler stopped: build queue empty for the cooldown");
            return Ok(CompilerOutcome {
                started: false,
                stopped: true,
            });
        }

        // Run. The namespace, the policy and the Deployment are applied on
        // each tick with work, so a changed server setting reaches the
        // compiler. The Secret and the policy come before the Deployment: the
        // pod needs the first to start and must not run without the second.
        self.ops
            .apply(
                KubeKind::Namespace,
                "",
                &namespace,
                &namespace_manifest(&namespace),
            )
            .await?;
        // CLOACINA-T-1063: the Secret holds the build role's URL with a
        // password set at this start. It is written only when no pod runs
        // (no Deployment, or 0 replicas), or when the Deployment has no
        // marker (made by an older server with the server's own URL: the new
        // marker replaces the pod). Else the running pod's password stays.
        let marker = self
            .ops
            .build_credential_marker(&namespace, COMPILER_NAME)
            .await?;
        let marker = match marker {
            Some(marker) if current.unwrap_or(0) > 0 => marker,
            _ => {
                let credential = self.credentials.issue(tenant_id).await?;
                let url = build_role_url(&self.settings.database_url, &credential)?;
                self.ops
                    .apply(
                        KubeKind::Secret,
                        &namespace,
                        COMPILER_SECRET_NAME,
                        &database_secret_manifest(&namespace, Some(&url)),
                    )
                    .await?;
                info!(tenant = %tenant_id, namespace = %namespace, role = %credential.role, "tenant compiler: new build role password written to its Secret");
                uuid::Uuid::new_v4().simple().to_string()
            }
        };
        if self.network_policy {
            self.ops
                .apply(
                    KubeKind::NetworkPolicy,
                    &namespace,
                    COMPILER_NAME,
                    &compiler_network_policy_manifest(
                        &namespace,
                        &self.dns_namespace,
                        &self.settings,
                    ),
                )
                .await?;
        }
        self.ops
            .apply(
                KubeKind::Deployment,
                &namespace,
                COMPILER_NAME,
                &with_build_credential_marker(
                    compiler_deployment_manifest(&namespace, tenant_id, &self.settings),
                    &marker,
                ),
            )
            .await?;
        if current != Some(1) {
            self.ops.scale(&namespace, COMPILER_NAME, 1).await?;
        }
        let started = current.unwrap_or(0) == 0;
        if started {
            info!(tenant = %tenant_id, namespace = %namespace, "tenant compiler started: build queue has work");
        }
        Ok(CompilerOutcome {
            started,
            stopped: false,
        })
    }

    /// Take away what was made for a tenant's compiler: the Deployment, the
    /// NetworkPolicy, and the database URL in the Secret. Idempotent.
    pub async fn remove(&self, tenant_id: &str) -> Result<(), ActuatorError> {
        let namespace = tenant_namespace(tenant_id);
        // Nothing was ever made for a tenant with no Deployment: do not make
        // a namespace or a Secret for it now.
        let existed = self
            .ops
            .deployment_replicas(&namespace, COMPILER_NAME)
            .await?
            .is_some();
        self.ops
            .delete(KubeKind::Deployment, &namespace, COMPILER_NAME)
            .await?;
        self.ops
            .delete(KubeKind::NetworkPolicy, &namespace, COMPILER_NAME)
            .await?;
        if existed {
            self.ops
                .apply(
                    KubeKind::Secret,
                    &namespace,
                    COMPILER_SECRET_NAME,
                    &database_secret_manifest(&namespace, None),
                )
                .await?;
            info!(tenant = %tenant_id, namespace = %namespace, "tenant compiler removed (kubernetes)");
        }
        Ok(())
    }
}

mod kube_api {
    //! [`CompilerKubeOps`] over the real Kubernetes API, on the client the
    //! agent side uses.

    use async_trait::async_trait;
    use k8s_openapi::api::apps::v1::Deployment;
    use k8s_openapi::api::core::v1::{Namespace, Secret};
    use k8s_openapi::api::networking::v1::NetworkPolicy;
    use kube::Api;
    use kube::api::{DeleteParams, Patch, PatchParams};

    use super::super::ActuatorError;
    use super::super::kubernetes::KubeApiOps;
    use super::{CompilerKubeOps, KubeKind};

    fn failed(what: &str, e: impl std::fmt::Display) -> ActuatorError {
        ActuatorError::Substrate(format!("compiler {what} failed: {e}"))
    }

    /// Absent is fine for a delete.
    fn deleted<T>(what: &str, result: Result<T, kube::Error>) -> Result<(), ActuatorError> {
        match result {
            Ok(_) => Ok(()),
            Err(kube::Error::Api(e)) if e.code == 404 => Ok(()),
            Err(e) => Err(failed(what, e)),
        }
    }

    #[async_trait]
    impl CompilerKubeOps for KubeApiOps {
        async fn apply(
            &self,
            kind: KubeKind,
            namespace: &str,
            name: &str,
            manifest: &serde_json::Value,
        ) -> Result<(), ActuatorError> {
            let params = Self::apply_params();
            let patch = Patch::Apply(manifest);
            let client = self.client.clone();
            match kind {
                KubeKind::Namespace => Api::<Namespace>::all(client)
                    .patch(name, &params, &patch)
                    .await
                    .map(|_| ())
                    .map_err(|e| failed("namespace apply", e)),
                KubeKind::Secret => Api::<Secret>::namespaced(client, namespace)
                    .patch(name, &params, &patch)
                    .await
                    .map(|_| ())
                    .map_err(|e| failed("secret apply", e)),
                KubeKind::NetworkPolicy => Api::<NetworkPolicy>::namespaced(client, namespace)
                    .patch(name, &params, &patch)
                    .await
                    .map(|_| ())
                    .map_err(|e| failed("network policy apply", e)),
                KubeKind::Deployment => Api::<Deployment>::namespaced(client, namespace)
                    .patch(name, &params, &patch)
                    .await
                    .map(|_| ())
                    .map_err(|e| failed("deployment apply", e)),
            }
        }

        async fn delete(
            &self,
            kind: KubeKind,
            namespace: &str,
            name: &str,
        ) -> Result<(), ActuatorError> {
            let params = DeleteParams::default();
            let client = self.client.clone();
            match kind {
                KubeKind::Deployment => deleted(
                    "deployment delete",
                    Api::<Deployment>::namespaced(client, namespace)
                        .delete(name, &params)
                        .await,
                ),
                KubeKind::NetworkPolicy => deleted(
                    "network policy delete",
                    Api::<NetworkPolicy>::namespaced(client, namespace)
                        .delete(name, &params)
                        .await,
                ),
                // The server has no right to delete these, and never does.
                KubeKind::Namespace | KubeKind::Secret => Err(ActuatorError::Substrate(
                    "the compiler side deletes no namespace and no secret".to_string(),
                )),
            }
        }

        async fn deployment_replicas(
            &self,
            namespace: &str,
            name: &str,
        ) -> Result<Option<u32>, ActuatorError> {
            let found = Api::<Deployment>::namespaced(self.client.clone(), namespace)
                .get_opt(name)
                .await
                .map_err(|e| failed("deployment read", e))?;
            Ok(found.map(|d| {
                // A Deployment with no replicas field runs one pod.
                d.spec.and_then(|s| s.replicas).unwrap_or(1).max(0) as u32
            }))
        }

        async fn scale(
            &self,
            namespace: &str,
            name: &str,
            replicas: u32,
        ) -> Result<(), ActuatorError> {
            let patch = serde_json::json!({ "spec": { "replicas": replicas } });
            Api::<Deployment>::namespaced(self.client.clone(), namespace)
                .patch(name, &PatchParams::default(), &Patch::Merge(&patch))
                .await
                .map(|_| ())
                .map_err(|e| failed("scale", e))
        }

        async fn build_credential_marker(
            &self,
            namespace: &str,
            name: &str,
        ) -> Result<Option<String>, ActuatorError> {
            let found = Api::<Deployment>::namespaced(self.client.clone(), namespace)
                .get_opt(name)
                .await
                .map_err(|e| failed("deployment read", e))?;
            Ok(found
                .and_then(|d| d.spec)
                .and_then(|s| s.template.metadata)
                .and_then(|m| m.annotations)
                .and_then(|a| a.get(super::BUILD_CREDENTIAL_ANNOTATION).cloned()))
        }
    }
}

// ---------------------------------------------------------------------------
// Docker
// ---------------------------------------------------------------------------

/// Everything needed to start one tenant's compiler container.
#[derive(Clone)]
pub struct CompilerContainer {
    pub tenant_id: String,
    pub image: String,
    pub args: Vec<String>,
    /// `NAME=value` pairs. Holds the database URL: never logged.
    pub env: Vec<String>,
    pub network: Option<String>,
}

/// The Docker Engine operations the compiler side needs.
#[async_trait]
pub trait CompilerContainerOps: Send + Sync {
    /// Ids of the running compiler containers of a tenant.
    async fn list_compilers(&self, tenant_id: &str) -> Result<Vec<String>, ActuatorError>;
    /// Create and start one compiler container.
    async fn start_compiler(&self, spec: CompilerContainer) -> Result<(), ActuatorError>;
    /// Stop and remove a container.
    async fn stop_compiler(&self, id: &str) -> Result<(), ActuatorError>;
    /// Ids of the tenant's running compiler containers that lack
    /// [`DOCKER_LABEL_CREDENTIAL`]: started by an older server, with the
    /// server's own URL (CLOACINA-T-1063).
    async fn list_compilers_without_build_role(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<String>, ActuatorError>;
}

/// The compiler side of the Docker actuator: one container per tenant with
/// work, none without. Of the settings it applies the image, the arguments,
/// the database URL and the network. The resources, the hardening, the cache
/// volumes and the NetworkPolicy have no counterpart here.
pub struct DockerCompilers {
    ops: Arc<dyn CompilerContainerOps>,
    settings: CompilerSettings,
    /// Issues the build role's password at each start (CLOACINA-T-1063).
    credentials: Arc<dyn BuildCredentials>,
}

impl DockerCompilers {
    pub fn new(
        ops: Arc<dyn CompilerContainerOps>,
        settings: CompilerSettings,
        credentials: Arc<dyn BuildCredentials>,
    ) -> Self {
        Self {
            ops,
            settings,
            credentials,
        }
    }

    pub fn cooldown(&self) -> Duration {
        self.settings.cooldown
    }

    /// Stop the tenant's compiler containers that an older server started
    /// with its own URL (CLOACINA-T-1063). Returns whether there were any.
    async fn stop_legacy(&self, tenant_id: &str) -> Result<bool, ActuatorError> {
        let legacy = self
            .ops
            .list_compilers_without_build_role(tenant_id)
            .await?;
        for id in &legacy {
            self.ops.stop_compiler(id).await?;
        }
        if !legacy.is_empty() {
            info!(tenant = %tenant_id, stopped = legacy.len(), "tenant compiler with the server's URL stopped (docker)");
        }
        Ok(!legacy.is_empty())
    }

    /// The container a tenant's compiler runs as, connecting with
    /// `database_url` (the build role's).
    pub fn container(&self, tenant_id: &str, database_url: &str) -> CompilerContainer {
        // The image's own filesystem is used: the hardened paths are volumes
        // of the Kubernetes pod only.
        let plain = CompilerSettings {
            hardened: false,
            ..self.settings.clone()
        };
        let mut env = vec![format!("DATABASE_URL={database_url}")];
        env.extend(
            plain
                .plain_env()
                .into_iter()
                .map(|(k, v)| format!("{k}={v}")),
        );
        CompilerContainer {
            tenant_id: tenant_id.to_string(),
            image: plain.image.clone(),
            args: plain.args(tenant_id),
            env,
            network: plain.docker_network.clone(),
        }
    }

    pub async fn reconcile(
        &self,
        tenant_id: &str,
        want: CompilerWant,
    ) -> Result<CompilerOutcome, ActuatorError> {
        if want == CompilerWant::Keep {
            return Ok(CompilerOutcome::default());
        }
        let stopped_legacy = self.stop_legacy(tenant_id).await?;
        let running = self.ops.list_compilers(tenant_id).await?;
        if want == CompilerWant::Stop {
            for id in &running {
                self.ops.stop_compiler(id).await?;
            }
            return Ok(CompilerOutcome {
                started: false,
                stopped: stopped_legacy || !running.is_empty(),
            });
        }
        if !running.is_empty() {
            return Ok(CompilerOutcome::default());
        }
        debug!(
            tenant = %tenant_id,
            "docker actuator ignores the compiler's resources, hardening, cache volumes and \
             network policy: they have no Docker counterpart here"
        );
        // CLOACINA-T-1063: a new build role password at each start, in the
        // container's environment only.
        let credential = self.credentials.issue(tenant_id).await?;
        let url = build_role_url(&self.settings.database_url, &credential)?;
        self.ops
            .start_compiler(self.container(tenant_id, &url))
            .await?;
        info!(tenant = %tenant_id, role = %credential.role, "tenant compiler started: build queue has work (docker)");
        Ok(CompilerOutcome {
            started: true,
            stopped: false,
        })
    }

    pub async fn remove(&self, tenant_id: &str) -> Result<(), ActuatorError> {
        self.stop_legacy(tenant_id).await?;
        for id in self.ops.list_compilers(tenant_id).await? {
            self.ops.stop_compiler(&id).await?;
        }
        Ok(())
    }
}

mod docker_api {
    //! [`CompilerContainerOps`] over the Docker Engine API, on the client the
    //! agent side uses.

    use std::collections::HashMap;

    use async_trait::async_trait;
    use bollard::models::{ContainerCreateBody, HostConfig};
    use bollard::query_parameters::{
        CreateContainerOptionsBuilder, ListContainersOptionsBuilder, RemoveContainerOptionsBuilder,
        StartContainerOptions, StopContainerOptions,
    };
    use uuid::Uuid;

    use super::super::ActuatorError;
    use super::super::docker::BollardOps;
    use super::{
        CompilerContainer, CompilerContainerOps, DOCKER_CREDENTIAL_BUILD_ROLE,
        DOCKER_LABEL_COMPILER, DOCKER_LABEL_CREDENTIAL, DOCKER_LABEL_TENANT,
    };

    fn failed(what: &str, e: impl std::fmt::Display) -> ActuatorError {
        ActuatorError::Substrate(format!("compiler {what} failed: {e}"))
    }

    #[async_trait]
    impl CompilerContainerOps for BollardOps {
        async fn list_compilers(&self, tenant_id: &str) -> Result<Vec<String>, ActuatorError> {
            let mut filters: HashMap<String, Vec<String>> = HashMap::new();
            filters.insert(
                "label".to_string(),
                vec![
                    format!("{DOCKER_LABEL_TENANT}={tenant_id}"),
                    format!("{DOCKER_LABEL_COMPILER}=true"),
                ],
            );
            let options = ListContainersOptionsBuilder::default()
                .all(false)
                .filters(&filters)
                .build();
            let found = self
                .docker
                .list_containers(Some(options))
                .await
                .map_err(|e| failed("list_containers", e))?;
            Ok(found.into_iter().filter_map(|c| c.id).collect())
        }

        async fn list_compilers_without_build_role(
            &self,
            tenant_id: &str,
        ) -> Result<Vec<String>, ActuatorError> {
            let mut filters: HashMap<String, Vec<String>> = HashMap::new();
            filters.insert(
                "label".to_string(),
                vec![
                    format!("{DOCKER_LABEL_TENANT}={tenant_id}"),
                    format!("{DOCKER_LABEL_COMPILER}=true"),
                ],
            );
            let options = ListContainersOptionsBuilder::default()
                .all(false)
                .filters(&filters)
                .build();
            let found = self
                .docker
                .list_containers(Some(options))
                .await
                .map_err(|e| failed("list_containers", e))?;
            Ok(found
                .into_iter()
                .filter(|c| {
                    c.labels
                        .as_ref()
                        .and_then(|l| l.get(DOCKER_LABEL_CREDENTIAL))
                        != Some(&DOCKER_CREDENTIAL_BUILD_ROLE.to_string())
                })
                .filter_map(|c| c.id)
                .collect())
        }

        async fn start_compiler(&self, spec: CompilerContainer) -> Result<(), ActuatorError> {
            let mut labels: HashMap<String, String> = HashMap::new();
            labels.insert(DOCKER_LABEL_COMPILER.to_string(), "true".to_string());
            labels.insert(DOCKER_LABEL_TENANT.to_string(), spec.tenant_id.clone());
            labels.insert(
                DOCKER_LABEL_CREDENTIAL.to_string(),
                DOCKER_CREDENTIAL_BUILD_ROLE.to_string(),
            );
            let body = ContainerCreateBody {
                image: Some(spec.image.clone()),
                cmd: Some(spec.args.clone()),
                env: Some(spec.env.clone()),
                labels: Some(labels),
                host_config: spec.network.clone().map(|network| HostConfig {
                    network_mode: Some(network),
                    ..Default::default()
                }),
                ..Default::default()
            };
            let name = format!("cloacina-compiler-{}", Uuid::new_v4());
            let options = CreateContainerOptionsBuilder::default().name(&name).build();
            let created = self
                .docker
                .create_container(Some(options), body)
                .await
                .map_err(|e| failed("create_container", e))?;
            self.docker
                .start_container(&created.id, None::<StartContainerOptions>)
                .await
                .map_err(|e| failed("start_container", e))?;
            Ok(())
        }

        async fn stop_compiler(&self, id: &str) -> Result<(), ActuatorError> {
            // A container that stopped by itself is still removed below.
            let _ = self
                .docker
                .stop_container(id, None::<StopContainerOptions>)
                .await;
            let options = RemoveContainerOptionsBuilder::default().force(true).build();
            self.docker
                .remove_container(id, Some(options))
                .await
                .map_err(|e| failed("remove_container", e))
        }
    }
}

// ---------------------------------------------------------------------------
// The pass of the fleet loop
// ---------------------------------------------------------------------------

/// Where the fleet loop reads each tenant's build queue from.
#[async_trait]
pub trait BuildQueueSource: Send + Sync {
    /// Every tenant, `public` included, with its queue. A tenant whose queue
    /// could not be read is left out: its compiler stays as it is.
    async fn queues(&self) -> Result<Vec<(String, BuildQueue)>, String>;
}

/// [`BuildQueueSource`] over the database: the tenant schemas, and in each
/// the counts the compiler itself claims from.
pub struct DalBuildQueues {
    admin: cloacina::database::Database,
    tenants: Arc<crate::TenantDatabaseCache>,
}

impl DalBuildQueues {
    pub fn new(
        admin: cloacina::database::Database,
        tenants: Arc<crate::TenantDatabaseCache>,
    ) -> Self {
        Self { admin, tenants }
    }
}

#[async_trait]
impl BuildQueueSource for DalBuildQueues {
    async fn queues(&self) -> Result<Vec<(String, BuildQueue)>, String> {
        let schemas = cloacina::database::DatabaseAdmin::new(self.admin.clone())
            .list_tenant_schemas()
            .await
            .map_err(|e| e.to_string())?;
        let mut out = Vec::with_capacity(schemas.len() + 1);
        // `public` is a tenant like the others (CLOACINA-T-1063, decision 4
        // of CLOACINA-A-0019): its compiler is made here too, and the chart
        // renders none while the fleet compilers are on. Its queue is in the
        // admin schema.
        match cloacina::registry::workflow_registry::build_queue_stats(&self.admin).await {
            Ok(stats) => out.push(("public".to_string(), BuildQueue::from(&stats))),
            Err(e) => {
                warn!(tenant = "public", error = %e, "tenant compilers: build queue read failed; skipped");
            }
        }
        for tenant in schemas {
            let db = match self.tenants.resolve(&tenant, &self.admin).await {
                Ok(db) => db,
                Err(e) => {
                    warn!(tenant = %tenant, error = %e, "tenant compilers: no database for the tenant; skipped");
                    continue;
                }
            };
            match cloacina::registry::workflow_registry::build_queue_stats(&db).await {
                Ok(stats) => out.push((tenant, BuildQueue::from(&stats))),
                Err(e) => {
                    warn!(tenant = %tenant, error = %e, "tenant compilers: build queue read failed; skipped");
                }
            }
        }
        Ok(out)
    }
}

/// Describe the metrics of the pass. Called once at server start.
pub fn describe_metrics() {
    metrics::describe_counter!(
        "cloacina_tenant_compiler_starts_total",
        "Per-tenant compilers started by the fleet loop: one for each time a \
         tenant's build queue got work while its compiler was not running \
         (CLOACINA-T-1034). Each start is a cold start: the compiler's cargo \
         caches are empty."
    );
    metrics::describe_gauge!(
        "cloacina_tenant_compiler_tenants_pending",
        "Tenants whose build queue has a `pending` or `building` package, as \
         read on the last tick of the fleet loop (CLOACINA-T-1034). `public` \
         is counted like any tenant (CLOACINA-T-1063). Set only by the \
         replica that led the tick."
    );
}

/// One pass over every tenant: read the queue, decide, tell the actuator.
/// Runs inside the fleet leader lock. Returns how many compilers it started.
pub async fn run_compiler_pass(
    actuator: &dyn FleetActuator,
    queues: &dyn BuildQueueSource,
    now: DateTime<Utc>,
) -> u32 {
    let Some(cooldown) = actuator.compiler_cooldown() else {
        return 0;
    };
    let queues = match queues.queues().await {
        Ok(q) => q,
        Err(e) => {
            warn!(error = %e, "tenant compilers: tenant list read failed; pass skipped");
            return 0;
        }
    };
    let with_work = queues.iter().filter(|(_, q)| q.has_work()).count();
    metrics::gauge!("cloacina_tenant_compiler_tenants_pending").set(with_work as f64);

    let mut started = 0;
    for (tenant, queue) in queues {
        let want = compiler_want(&queue, now, cooldown);
        match actuator.reconcile_compiler(&tenant, want).await {
            Ok(outcome) => {
                if outcome.started {
                    started += 1;
                    metrics::counter!("cloacina_tenant_compiler_starts_total").increment(1);
                }
            }
            Err(e) => warn!(
                tenant = %tenant,
                want = ?want,
                error = %e,
                "tenant compilers: reconcile failed"
            ),
        }
    }
    started
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    type Applied = (KubeKind, String, String, serde_json::Value);

    /// Fake [`CompilerKubeOps`]: one Deployment's replica count, and a record
    /// of every call.
    #[derive(Default)]
    struct FakeKube {
        /// `None`: no Deployment.
        replicas: Mutex<Option<u32>>,
        /// The build credential marker of the Deployment's pod template.
        marker: Mutex<Option<String>>,
        applied: Mutex<Vec<Applied>>,
        deleted: Mutex<Vec<(KubeKind, String, String)>>,
        scaled: Mutex<Vec<(String, String, u32)>>,
    }

    impl FakeKube {
        fn with_replicas(replicas: Option<u32>) -> Arc<Self> {
            Arc::new(Self {
                replicas: Mutex::new(replicas),
                ..Default::default()
            })
        }
        /// A Deployment made by this server version: it has a marker.
        fn running_with_marker(replicas: u32, marker: &str) -> Arc<Self> {
            Arc::new(Self {
                replicas: Mutex::new(Some(replicas)),
                marker: Mutex::new(Some(marker.to_string())),
                ..Default::default()
            })
        }
        fn applied_kinds(&self) -> Vec<KubeKind> {
            self.applied.lock().unwrap().iter().map(|a| a.0).collect()
        }
        fn last(&self, kind: KubeKind) -> serde_json::Value {
            self.applied
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|a| a.0 == kind)
                .map(|a| a.3.clone())
                .expect("no such object applied")
        }
    }

    #[async_trait]
    impl CompilerKubeOps for FakeKube {
        async fn apply(
            &self,
            kind: KubeKind,
            namespace: &str,
            name: &str,
            manifest: &serde_json::Value,
        ) -> Result<(), ActuatorError> {
            self.applied.lock().unwrap().push((
                kind,
                namespace.to_string(),
                name.to_string(),
                manifest.clone(),
            ));
            if kind == KubeKind::Deployment {
                // A new Deployment with no replicas field runs one pod.
                let mut replicas = self.replicas.lock().unwrap();
                if replicas.is_none() {
                    *replicas = Some(1);
                }
                *self.marker.lock().unwrap() =
                    manifest["spec"]["template"]["metadata"]["annotations"]
                        [BUILD_CREDENTIAL_ANNOTATION]
                        .as_str()
                        .map(str::to_string);
            }
            Ok(())
        }
        async fn build_credential_marker(
            &self,
            _namespace: &str,
            _name: &str,
        ) -> Result<Option<String>, ActuatorError> {
            if self.replicas.lock().unwrap().is_none() {
                return Ok(None);
            }
            Ok(self.marker.lock().unwrap().clone())
        }
        async fn delete(
            &self,
            kind: KubeKind,
            namespace: &str,
            name: &str,
        ) -> Result<(), ActuatorError> {
            self.deleted
                .lock()
                .unwrap()
                .push((kind, namespace.to_string(), name.to_string()));
            if kind == KubeKind::Deployment {
                *self.replicas.lock().unwrap() = None;
            }
            Ok(())
        }
        async fn deployment_replicas(
            &self,
            _namespace: &str,
            _name: &str,
        ) -> Result<Option<u32>, ActuatorError> {
            Ok(*self.replicas.lock().unwrap())
        }
        async fn scale(
            &self,
            namespace: &str,
            name: &str,
            replicas: u32,
        ) -> Result<(), ActuatorError> {
            self.scaled
                .lock()
                .unwrap()
                .push((namespace.to_string(), name.to_string(), replicas));
            *self.replicas.lock().unwrap() = Some(replicas);
            Ok(())
        }
    }

    const URL: &str = "postgres://cloacina:s3cret@db.example:6432/cloacina";

    /// Fake [`BuildCredentials`]: `<tenant>$build` and a numbered password.
    #[derive(Default)]
    struct FakeCredentials {
        issued: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl BuildCredentials for FakeCredentials {
        async fn issue(
            &self,
            tenant_id: &str,
        ) -> Result<cloacina::database::BuildCredential, ActuatorError> {
            let mut issued = self.issued.lock().unwrap();
            issued.push(tenant_id.to_string());
            Ok(cloacina::database::BuildCredential {
                role: cloacina::database::build_role_name(tenant_id),
                password: format!("buildpw{}", issued.len()),
            })
        }
    }

    fn settings() -> CompilerSettings {
        CompilerSettings::with_database_url(URL)
    }

    fn compilers(ops: Arc<FakeKube>, settings: CompilerSettings) -> KubeCompilers {
        compilers_with(ops, settings, Arc::new(FakeCredentials::default()))
    }

    fn compilers_with(
        ops: Arc<FakeKube>,
        settings: CompilerSettings,
        credentials: Arc<FakeCredentials>,
    ) -> KubeCompilers {
        KubeCompilers::new(ops, settings, true, "kube-system".to_string(), credentials)
    }

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_800_000_000 + secs, 0).unwrap()
    }

    const COOLDOWN: Duration = Duration::from_secs(300);

    // -- the decision -------------------------------------------------------

    #[test]
    fn work_in_the_queue_wants_a_compiler() {
        let pending = BuildQueue {
            pending: 1,
            ..Default::default()
        };
        assert_eq!(compiler_want(&pending, at(0), COOLDOWN), CompilerWant::Run);
        // A row a dead compiler left in `building` also counts: the new
        // compiler's sweeper puts it back in the queue.
        let building = BuildQueue {
            building: 1,
            last_activity: Some(at(-10_000)),
            ..Default::default()
        };
        assert_eq!(compiler_want(&building, at(0), COOLDOWN), CompilerWant::Run);
    }

    #[test]
    fn an_empty_queue_keeps_the_compiler_until_the_cooldown_is_over() {
        let queue = BuildQueue {
            last_activity: Some(at(0)),
            ..Default::default()
        };
        assert_eq!(compiler_want(&queue, at(1), COOLDOWN), CompilerWant::Keep);
        assert_eq!(compiler_want(&queue, at(299), COOLDOWN), CompilerWant::Keep);
        assert_eq!(compiler_want(&queue, at(300), COOLDOWN), CompilerWant::Stop);
        assert_eq!(
            compiler_want(&queue, at(9_000), COOLDOWN),
            CompilerWant::Stop
        );
    }

    #[test]
    fn a_queue_that_never_held_a_build_wants_no_compiler() {
        assert_eq!(
            compiler_want(&BuildQueue::default(), at(0), COOLDOWN),
            CompilerWant::Stop
        );
    }

    #[test]
    fn the_last_activity_is_the_newest_of_the_three_timestamps() {
        let stats = cloacina::registry::workflow_registry::BuildQueueStats {
            pending: 2,
            building: 1,
            last_success_at: Some(at(10)),
            last_failure_at: Some(at(30)),
            heartbeat_at: Some(at(20)),
        };
        let queue = BuildQueue::from(&stats);
        assert_eq!((queue.pending, queue.building), (2, 1));
        assert_eq!(queue.last_activity, Some(at(30)));
    }

    // -- the Kubernetes side ------------------------------------------------

    #[tokio::test]
    async fn work_appears_and_the_compiler_is_made_with_one_replica() {
        let ops = FakeKube::with_replicas(None);
        let c = compilers(ops.clone(), settings());

        let out = c.reconcile("acme", CompilerWant::Run).await.unwrap();

        assert!(out.started && !out.stopped);
        // Namespace, Secret and policy before the Deployment.
        assert_eq!(
            ops.applied_kinds(),
            vec![
                KubeKind::Namespace,
                KubeKind::Secret,
                KubeKind::NetworkPolicy,
                KubeKind::Deployment
            ]
        );
        let applied = ops.applied.lock().unwrap();
        assert!(
            applied
                .iter()
                .skip(1)
                .all(|a| a.1 == "cloacina-tenant-acme")
        );
        assert_eq!(*ops.replicas.lock().unwrap(), Some(1));
    }

    #[tokio::test]
    async fn work_appears_and_a_stopped_compiler_is_scaled_to_one() {
        let ops = FakeKube::with_replicas(Some(0));
        let c = compilers(ops.clone(), settings());

        let out = c.reconcile("acme", CompilerWant::Run).await.unwrap();

        assert!(out.started);
        assert_eq!(
            ops.scaled.lock().unwrap().as_slice(),
            &[(
                "cloacina-tenant-acme".to_string(),
                COMPILER_NAME.to_string(),
                1
            )]
        );
    }

    #[tokio::test]
    async fn a_running_compiler_with_work_is_not_counted_as_a_start() {
        let ops = FakeKube::running_with_marker(1, "m1");
        let creds = Arc::new(FakeCredentials::default());
        let c = compilers_with(ops.clone(), settings(), creds.clone());

        let out = c.reconcile("acme", CompilerWant::Run).await.unwrap();

        assert_eq!(out, CompilerOutcome::default());
        assert!(ops.scaled.lock().unwrap().is_empty());
        // The objects are applied again, so a changed setting gets through;
        // the Secret is not: the running pod keeps its password
        // (CLOACINA-T-1063).
        assert_eq!(
            ops.applied_kinds(),
            vec![
                KubeKind::Namespace,
                KubeKind::NetworkPolicy,
                KubeKind::Deployment
            ]
        );
        assert!(creds.issued.lock().unwrap().is_empty());
        assert_eq!(ops.marker.lock().unwrap().as_deref(), Some("m1"));
    }

    // -- the build role's credential (CLOACINA-T-1063) ----------------------

    #[tokio::test]
    async fn a_start_writes_the_build_role_url_and_never_the_servers() {
        let ops = FakeKube::with_replicas(None);
        let creds = Arc::new(FakeCredentials::default());
        let c = compilers_with(ops.clone(), settings(), creds.clone());

        c.reconcile("acme", CompilerWant::Run).await.unwrap();

        let secret = ops.last(KubeKind::Secret);
        let url = secret["stringData"][COMPILER_SECRET_KEY].as_str().unwrap();
        // The user is the build role (its `$` may be percent-encoded); host,
        // port and database are the compilers' URL's.
        assert_eq!(
            url.replace("%24", "$"),
            "postgres://acme$build:buildpw1@db.example:6432/cloacina"
        );
        assert!(!secret.to_string().contains("s3cret"), "no server password");
        assert_eq!(
            creds.issued.lock().unwrap().as_slice(),
            &["acme".to_string()]
        );
        // The pod template carries the marker of this password.
        let deployment = ops.last(KubeKind::Deployment);
        let marker = deployment["spec"]["template"]["metadata"]["annotations"]
            [BUILD_CREDENTIAL_ANNOTATION]
            .as_str()
            .unwrap();
        assert!(!marker.is_empty());
        assert!(
            !deployment.to_string().contains("buildpw"),
            "no password in the pod spec"
        );
    }

    #[tokio::test]
    async fn each_start_from_zero_sets_a_new_password_and_a_new_marker() {
        let ops = FakeKube::running_with_marker(0, "old");
        let creds = Arc::new(FakeCredentials::default());
        let c = compilers_with(ops.clone(), settings(), creds.clone());

        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        let first = ops.marker.lock().unwrap().clone().unwrap();
        assert_ne!(first, "old");
        // Running: nothing new.
        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        assert_eq!(creds.issued.lock().unwrap().len(), 1);
        // Stopped, then started again: a new password.
        c.reconcile("acme", CompilerWant::Stop).await.unwrap();
        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        assert_eq!(creds.issued.lock().unwrap().len(), 2);
        let second = ops.marker.lock().unwrap().clone().unwrap();
        assert_ne!(first, second);
        let secret = ops.last(KubeKind::Secret);
        assert!(secret.to_string().contains("buildpw2"));
    }

    #[tokio::test]
    async fn a_running_compiler_of_an_older_server_is_moved_to_the_build_role() {
        // Made by an older server: running, no marker, the server's URL in
        // its Secret.
        let ops = FakeKube::with_replicas(Some(1));
        let creds = Arc::new(FakeCredentials::default());
        let c = compilers_with(ops.clone(), settings(), creds.clone());

        let out = c.reconcile("acme", CompilerWant::Run).await.unwrap();

        assert!(!out.started, "a running compiler is not counted as a start");
        assert_eq!(creds.issued.lock().unwrap().len(), 1);
        let secret = ops.last(KubeKind::Secret);
        assert!(
            secret
                .to_string()
                .replace("%24", "$")
                .contains("acme$build")
        );
        assert!(!secret.to_string().contains("s3cret"));
        // The new marker on the pod template replaces the pod.
        assert!(ops.marker.lock().unwrap().is_some());
    }

    #[tokio::test]
    async fn a_failed_password_issue_writes_nothing_and_starts_nothing() {
        struct Refusing;
        #[async_trait]
        impl BuildCredentials for Refusing {
            async fn issue(
                &self,
                _tenant_id: &str,
            ) -> Result<cloacina::database::BuildCredential, ActuatorError> {
                Err(ActuatorError::Substrate("no role".to_string()))
            }
        }
        let ops = FakeKube::with_replicas(None);
        let c = KubeCompilers::new(
            ops.clone(),
            settings(),
            true,
            "kube-system".to_string(),
            Arc::new(Refusing),
        );
        assert!(c.reconcile("acme", CompilerWant::Run).await.is_err());
        assert_eq!(ops.applied_kinds(), vec![KubeKind::Namespace]);
        assert_eq!(*ops.replicas.lock().unwrap(), None);
    }

    #[tokio::test]
    async fn public_gets_a_compiler_like_a_tenant() {
        let ops = FakeKube::with_replicas(None);
        let c = compilers(ops.clone(), settings());
        c.reconcile("public", CompilerWant::Run).await.unwrap();
        let deployment = ops.last(KubeKind::Deployment);
        assert_eq!(
            deployment["metadata"]["namespace"],
            "cloacina-tenant-public"
        );
        assert!(
            deployment
                .to_string()
                .contains("\"--tenant-schema\",\"public\"")
        );
        let secret = ops.last(KubeKind::Secret);
        assert!(
            secret
                .to_string()
                .replace("%24", "$")
                .contains("public$build")
        );
    }

    #[tokio::test]
    async fn an_empty_queue_past_the_cooldown_scales_the_compiler_to_zero() {
        let ops = FakeKube::with_replicas(Some(1));
        let c = compilers(ops.clone(), settings());

        let out = c.reconcile("acme", CompilerWant::Stop).await.unwrap();

        assert!(out.stopped && !out.started);
        assert_eq!(*ops.replicas.lock().unwrap(), Some(0));
        // Scaled, not deleted: the next start is one patch.
        assert!(ops.deleted.lock().unwrap().is_empty());
        assert!(ops.applied.lock().unwrap().is_empty());

        // The next tick finds it at 0 and does nothing.
        let again = c.reconcile("acme", CompilerWant::Stop).await.unwrap();
        assert_eq!(again, CompilerOutcome::default());
        assert_eq!(ops.scaled.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn keep_changes_nothing() {
        let ops = FakeKube::with_replicas(Some(1));
        let c = compilers(ops.clone(), settings());
        let out = c.reconcile("acme", CompilerWant::Keep).await.unwrap();
        assert_eq!(out, CompilerOutcome::default());
        assert!(ops.applied.lock().unwrap().is_empty());
        assert!(ops.scaled.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_tenant_with_an_empty_queue_never_gets_a_compiler() {
        let ops = FakeKube::with_replicas(None);
        let c = compilers(ops.clone(), settings());
        for want in [CompilerWant::Stop, CompilerWant::Keep, CompilerWant::Stop] {
            let out = c.reconcile("quiet", want).await.unwrap();
            assert_eq!(out, CompilerOutcome::default());
        }
        assert!(ops.applied.lock().unwrap().is_empty());
        assert!(ops.scaled.lock().unwrap().is_empty());
        assert_eq!(*ops.replicas.lock().unwrap(), None);
    }

    #[tokio::test]
    async fn a_removed_tenant_loses_its_compiler_and_the_database_url() {
        let ops = FakeKube::with_replicas(Some(1));
        let c = compilers(ops.clone(), settings());

        c.remove("acme").await.unwrap();

        let ns = "cloacina-tenant-acme".to_string();
        assert_eq!(
            ops.deleted.lock().unwrap().as_slice(),
            &[
                (KubeKind::Deployment, ns.clone(), COMPILER_NAME.to_string()),
                (KubeKind::NetworkPolicy, ns, COMPILER_NAME.to_string()),
            ]
        );
        // The Secret is written over with an empty value.
        let secret = ops.last(KubeKind::Secret);
        assert_eq!(secret["stringData"][COMPILER_SECRET_KEY], "");
        assert!(!secret.to_string().contains("s3cret"));
    }

    #[tokio::test]
    async fn removing_a_tenant_that_never_had_a_compiler_creates_nothing() {
        let ops = FakeKube::with_replicas(None);
        let c = compilers(ops.clone(), settings());
        c.remove("quiet").await.unwrap();
        assert!(ops.applied.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn the_policy_is_not_written_when_fleet_policies_are_off() {
        let ops = FakeKube::with_replicas(None);
        let c = KubeCompilers::new(
            ops.clone(),
            settings(),
            false,
            "kube-system".to_string(),
            Arc::new(FakeCredentials::default()),
        );
        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        assert!(!ops.applied_kinds().contains(&KubeKind::NetworkPolicy));
    }

    // -- the manifests ------------------------------------------------------

    #[test]
    fn the_deployment_is_the_tenants_compiler_and_hardened() {
        let mut s = settings();
        s.image = "registry.example/cloacina-compiler:1.2.3".to_string();
        s.cargo_flags = vec!["build".to_string(), "--lib".to_string()];
        s.build_timeout_s = Some(900);
        s.vendor_dir = Some("/opt/vendor".to_string());
        let m = compiler_deployment_manifest("cloacina-tenant-acme", "acme", &s);

        assert_eq!(m["metadata"]["name"], "cloacina-compiler");
        assert_eq!(m["metadata"]["namespace"], "cloacina-tenant-acme");
        assert_eq!(m["metadata"]["labels"]["cloacina.tenant"], "acme");
        assert_eq!(
            m["spec"]["selector"]["matchLabels"]["app.kubernetes.io/name"],
            "cloacina-compiler"
        );
        // The count is set apart.
        assert!(m["spec"].get("replicas").is_none());

        let pod = &m["spec"]["template"]["spec"];
        assert_eq!(pod["automountServiceAccountToken"], false);
        assert_eq!(pod["securityContext"]["runAsNonRoot"], true);
        assert_eq!(pod["securityContext"]["runAsUser"], 10001);
        assert_eq!(
            pod["securityContext"]["seccompProfile"]["type"],
            "RuntimeDefault"
        );

        let container = &pod["containers"][0];
        assert_eq!(
            container["image"],
            "registry.example/cloacina-compiler:1.2.3"
        );
        let sc = &container["securityContext"];
        assert_eq!(sc["allowPrivilegeEscalation"], false);
        assert_eq!(sc["readOnlyRootFilesystem"], true);
        assert_eq!(sc["capabilities"]["drop"][0], "ALL");

        let args: Vec<&str> = container["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap())
            .collect();
        let pair = |flag: &str| {
            let i = args.iter().position(|a| *a == flag).expect(flag);
            args[i + 1]
        };
        assert_eq!(pair("--tenant-schema"), "acme");
        assert_eq!(pair("--bind"), "127.0.0.1:9000");
        assert_eq!(pair("--cargo-target-dir"), "/workspace/target");
        assert_eq!(pair("--home"), "/var/cloacina/home");
        assert_eq!(pair("--build-timeout-s"), "900");
        assert!(args.contains(&"--cargo-flags-replace=build"));
        assert!(args.contains(&"--cargo-flags-replace=--lib"));

        // Every writable path is a volume.
        let mounts: Vec<&str> = container["volumeMounts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["mountPath"].as_str().unwrap())
            .collect();
        assert_eq!(
            mounts,
            vec![
                "/var/cloacina/home",
                "/var/cloacina/cargo",
                "/workspace/target",
                "/tmp"
            ]
        );
        let volumes = pod["volumes"].as_array().unwrap();
        assert_eq!(volumes.len(), 4);
        assert!(volumes.iter().all(|v| v.get("emptyDir").is_some()));
        assert_eq!(volumes[2]["emptyDir"]["sizeLimit"], "8Gi");

        let env = container["env"].as_array().unwrap();
        let value = |name: &str| {
            env.iter()
                .find(|e| e["name"] == name)
                .map(|e| e["value"].clone())
                .unwrap_or_else(|| panic!("no env {name}"))
        };
        assert_eq!(value("CARGO_HOME"), "/var/cloacina/cargo");
        assert_eq!(value("HOME"), "/var/cloacina/home");
        assert_eq!(value("CLOACINA_COMPILER_VENDOR_DIR"), "/opt/vendor");
        assert_eq!(container["resources"]["limits"]["memory"], "4Gi");
    }

    #[test]
    fn the_database_url_is_only_in_the_secret() {
        let s = settings();
        let deployment = compiler_deployment_manifest("cloacina-tenant-acme", "acme", &s);
        let text = deployment.to_string();
        assert!(!text.contains("s3cret"), "the URL is in the Deployment");
        assert!(!text.contains("--database-url"));
        let env = &deployment["spec"]["template"]["spec"]["containers"][0]["env"][0];
        assert_eq!(env["name"], "DATABASE_URL");
        assert_eq!(
            env["valueFrom"]["secretKeyRef"]["name"],
            COMPILER_SECRET_NAME
        );
        assert_eq!(env["valueFrom"]["secretKeyRef"]["key"], COMPILER_SECRET_KEY);

        let secret = database_secret_manifest("cloacina-tenant-acme", Some(URL));
        assert_eq!(secret["metadata"]["namespace"], "cloacina-tenant-acme");
        assert_eq!(secret["stringData"][COMPILER_SECRET_KEY], URL);

        let policy = compiler_network_policy_manifest("cloacina-tenant-acme", "kube-system", &s);
        assert!(!policy.to_string().contains("s3cret"));
        assert!(!format!("{s:?}").contains("s3cret"));
    }

    #[test]
    fn an_unhardened_deployment_uses_the_image_as_it_is() {
        let mut s = settings();
        s.hardened = false;
        let m = compiler_deployment_manifest("cloacina-tenant-acme", "acme", &s);
        let pod = &m["spec"]["template"]["spec"];
        assert!(pod.get("securityContext").is_none());
        assert!(pod.get("volumes").is_none());
        assert!(pod["containers"][0].get("volumeMounts").is_none());
        assert!(!m.to_string().contains("--home"));
        // The token stays out either way.
        assert_eq!(pod["automountServiceAccountToken"], false);
    }

    #[test]
    fn the_default_policy_allows_dns_the_database_port_and_the_internet_on_443() {
        let m =
            compiler_network_policy_manifest("cloacina-tenant-acme", "kube-system", &settings());
        let spec = &m["spec"];
        assert_eq!(
            spec["podSelector"]["matchLabels"]["app.kubernetes.io/name"],
            "cloacina-compiler"
        );
        assert_eq!(
            spec["policyTypes"],
            serde_json::json!(["Ingress", "Egress"])
        );
        assert_eq!(spec["ingress"], serde_json::json!([]));

        let egress = spec["egress"].as_array().unwrap();
        assert_eq!(egress.len(), 3);
        assert_eq!(
            egress[0]["to"][0]["namespaceSelector"]["matchLabels"]["kubernetes.io/metadata.name"],
            "kube-system"
        );
        assert_eq!(egress[0]["ports"][0]["port"], 53);
        // The database: the URL's port, every destination.
        assert_eq!(
            egress[1]["ports"],
            serde_json::json!([{ "protocol": "TCP", "port": 6432 }])
        );
        assert!(egress[1].get("to").is_none());
        // Crates: the public internet on 443, no private range.
        let block = &egress[2]["to"][0]["ipBlock"];
        assert_eq!(block["cidr"], "0.0.0.0/0");
        let except = block["except"].as_array().unwrap();
        for range in [
            "10.0.0.0/8",
            "172.16.0.0/12",
            "192.168.0.0/16",
            "169.254.0.0/16",
        ] {
            assert!(except.iter().any(|e| e == range), "{range} is not left out");
        }
        assert_eq!(
            egress[2]["ports"],
            serde_json::json!([{ "protocol": "TCP", "port": 443 }])
        );
    }

    #[test]
    fn the_policy_follows_the_egress_settings() {
        let mut s = settings();
        s.crate_egress = CrateEgress::None;
        s.database_cidrs = vec!["10.1.2.3/32".to_string()];
        s.egress_cidrs = vec!["10.9.0.0/24".to_string(), "203.0.113.7/32".to_string()];
        let m = compiler_network_policy_manifest("ns", "kube-system", &s);
        let egress = m["spec"]["egress"].as_array().unwrap();
        // DNS, the database, the listed CIDRs. No internet rule.
        assert_eq!(egress.len(), 3);
        assert_eq!(egress[1]["to"][0]["ipBlock"]["cidr"], "10.1.2.3/32");
        assert_eq!(egress[2]["to"][0]["ipBlock"]["cidr"], "10.9.0.0/24");
        assert_eq!(egress[2]["to"][1]["ipBlock"]["cidr"], "203.0.113.7/32");
        assert!(egress[2].get("ports").is_none());
        assert!(!m.to_string().contains("0.0.0.0/0"));

        s.crate_egress = CrateEgress::All;
        s.egress_cidrs.clear();
        let m = compiler_network_policy_manifest("ns", "kube-system", &s);
        let egress = m["spec"]["egress"].as_array().unwrap();
        assert_eq!(egress[2], serde_json::json!({}));
        // Ingress stays closed.
        assert_eq!(m["spec"]["ingress"], serde_json::json!([]));
    }

    // -- the settings -------------------------------------------------------

    fn lookup(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn compilers_are_off_unless_switched_on() {
        assert!(
            CompilerSettings::from_lookup(URL, lookup(&[]))
                .unwrap()
                .is_none()
        );
        assert!(
            CompilerSettings::from_lookup(URL, lookup(&[("CLOACINA_TENANT_COMPILERS", "false")]))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn the_defaults_are_the_servers_url_five_minutes_and_internet_egress() {
        let s =
            CompilerSettings::from_lookup(URL, lookup(&[("CLOACINA_TENANT_COMPILERS", "true")]))
                .unwrap()
                .unwrap();
        assert_eq!(s.database_url, URL);
        assert_eq!(s.cooldown, Duration::from_secs(300));
        assert_eq!(s.crate_egress, CrateEgress::Internet);
        assert!(s.hardened);
        assert!(s.cargo_flags.is_empty());
        assert_eq!(s.target_dir, "/workspace/target");
        assert_eq!(s.resources.memory_limit, "4Gi");
        // No cargo flags given: the compiler keeps its own default list.
        assert!(
            !s.args("acme")
                .iter()
                .any(|a| a.starts_with("--cargo-flags"))
        );
    }

    #[test]
    fn the_settings_are_read_from_the_environment() {
        let s = CompilerSettings::from_lookup(
            URL,
            lookup(&[
                ("CLOACINA_TENANT_COMPILERS", "true"),
                ("CLOACINA_TENANT_COMPILER_IMAGE", "registry.example/c:1"),
                (
                    "CLOACINA_TENANT_COMPILER_DATABASE_URL",
                    "postgres://builder@db/c",
                ),
                ("CLOACINA_TENANT_COMPILER_CARGO_FLAGS", "build  --lib"),
                ("CLOACINA_TENANT_COMPILER_COOLDOWN_S", "60"),
                ("CLOACINA_TENANT_COMPILER_EGRESS", "none"),
                (
                    "CLOACINA_TENANT_COMPILER_EGRESS_CIDRS",
                    "10.9.0.0/24, 203.0.113.7/32",
                ),
                ("CLOACINA_TENANT_COMPILER_HARDENED", "false"),
                ("CLOACINA_TENANT_COMPILER_BUILD_TIMEOUT_S", "1200"),
                ("CLOACINA_TENANT_COMPILER_DEV_WORKSPACE", "/workspace"),
            ]),
        )
        .unwrap()
        .unwrap();
        assert_eq!(s.image, "registry.example/c:1");
        assert_eq!(s.database_url, "postgres://builder@db/c");
        assert_eq!(s.cargo_flags, vec!["build", "--lib"]);
        assert_eq!(s.cooldown, Duration::from_secs(60));
        assert_eq!(s.crate_egress, CrateEgress::None);
        assert_eq!(s.egress_cidrs, vec!["10.9.0.0/24", "203.0.113.7/32"]);
        assert!(!s.hardened);
        assert_eq!(s.build_timeout_s, Some(1200));
        assert_eq!(
            s.plain_env(),
            vec![(
                "CLOACINA_COMPILER_DEV_WORKSPACE".to_string(),
                "/workspace".to_string()
            )]
        );
    }

    #[test]
    fn a_value_that_cannot_be_used_is_an_error_that_names_the_setting() {
        for (key, value) in [
            ("CLOACINA_TENANT_COMPILER_EGRESS", "open"),
            ("CLOACINA_TENANT_COMPILER_EGRESS_CIDRS", "10.0.0.0"),
            ("CLOACINA_TENANT_COMPILER_DATABASE_CIDRS", "10.0.0.0/40"),
            ("CLOACINA_TENANT_COMPILER_COOLDOWN_S", "soon"),
            ("CLOACINA_TENANT_COMPILER_HARDENED", "maybe"),
            ("CLOACINA_TENANT_COMPILER_TARGET_DIR", "target"),
        ] {
            let err = CompilerSettings::from_lookup(URL, move |k| {
                if k == "CLOACINA_TENANT_COMPILERS" {
                    Some("true".to_string())
                } else if k == key {
                    Some(value.to_string())
                } else {
                    None
                }
            })
            .unwrap_err();
            assert!(err.contains(key), "{err}");
        }
    }

    #[test]
    fn the_database_port_is_read_from_the_url() {
        assert_eq!(database_port("postgres://u:p@db:6432/c"), 6432);
        assert_eq!(database_port("postgres://u:p@db/c"), 5432);
        assert_eq!(
            database_port("postgresql://u:p%40x@db.example:5433/c?sslmode=require"),
            5433
        );
        assert_eq!(database_port("postgres://u:p@[2001:db8::1]:7000/c"), 7000);
        assert_eq!(database_port("postgres://u:p@[2001:db8::1]/c"), 5432);
        assert_eq!(database_port("postgres://db"), 5432);
    }

    // -- the Docker side ----------------------------------------------------

    #[derive(Default)]
    struct FakeDocker {
        running: Mutex<Vec<String>>,
        /// Running containers of an older server: no credential label.
        legacy: Mutex<Vec<String>>,
        started: Mutex<Vec<CompilerContainer>>,
        stopped: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl CompilerContainerOps for FakeDocker {
        async fn list_compilers(&self, _tenant_id: &str) -> Result<Vec<String>, ActuatorError> {
            let mut all = self.running.lock().unwrap().clone();
            all.extend(self.legacy.lock().unwrap().iter().cloned());
            Ok(all)
        }
        async fn list_compilers_without_build_role(
            &self,
            _tenant_id: &str,
        ) -> Result<Vec<String>, ActuatorError> {
            Ok(self.legacy.lock().unwrap().clone())
        }
        async fn start_compiler(&self, spec: CompilerContainer) -> Result<(), ActuatorError> {
            self.running
                .lock()
                .unwrap()
                .push(format!("c-{}", spec.tenant_id));
            self.started.lock().unwrap().push(spec);
            Ok(())
        }
        async fn stop_compiler(&self, id: &str) -> Result<(), ActuatorError> {
            self.running.lock().unwrap().retain(|c| c != id);
            self.legacy.lock().unwrap().retain(|c| c != id);
            self.stopped.lock().unwrap().push(id.to_string());
            Ok(())
        }
    }

    fn docker(ops: Arc<FakeDocker>, settings: CompilerSettings) -> DockerCompilers {
        DockerCompilers::new(ops, settings, Arc::new(FakeCredentials::default()))
    }

    #[tokio::test]
    async fn docker_runs_one_container_while_there_is_work() {
        let ops = Arc::new(FakeDocker::default());
        let mut s = settings();
        s.docker_network = Some("cloacina".to_string());
        let c = docker(ops.clone(), s);

        // No work, no container.
        assert_eq!(
            c.reconcile("acme", CompilerWant::Stop).await.unwrap(),
            CompilerOutcome::default()
        );
        assert!(ops.started.lock().unwrap().is_empty());

        // Work: one container, and only one on the next tick.
        assert!(
            c.reconcile("acme", CompilerWant::Run)
                .await
                .unwrap()
                .started
        );
        assert!(
            !c.reconcile("acme", CompilerWant::Run)
                .await
                .unwrap()
                .started
        );
        {
            let started = ops.started.lock().unwrap();
            assert_eq!(started.len(), 1);
            let spec = &started[0];
            assert_eq!(spec.network.as_deref(), Some("cloacina"));
            // The build role's URL (CLOACINA-T-1063), never the server's.
            let url = spec
                .env
                .iter()
                .find_map(|e| e.strip_prefix("DATABASE_URL="))
                .unwrap()
                .replace("%24", "$");
            assert_eq!(
                url,
                "postgres://acme$build:buildpw1@db.example:6432/cloacina"
            );
            assert!(!spec.env.iter().any(|e| e.contains("s3cret")));
            // The URL is in the environment, not in the arguments.
            assert!(!spec.args.iter().any(|a| a.contains("buildpw")));
            let i = spec
                .args
                .iter()
                .position(|a| a == "--tenant-schema")
                .unwrap();
            assert_eq!(spec.args[i + 1], "acme");
            // The image's own filesystem: no hardened paths.
            assert!(!spec.args.iter().any(|a| a == "--home"));
        }

        // Keep leaves it; Stop removes it.
        c.reconcile("acme", CompilerWant::Keep).await.unwrap();
        assert!(ops.stopped.lock().unwrap().is_empty());
        assert!(
            c.reconcile("acme", CompilerWant::Stop)
                .await
                .unwrap()
                .stopped
        );
        assert_eq!(
            ops.stopped.lock().unwrap().as_slice(),
            &["c-acme".to_string()]
        );
    }

    #[tokio::test]
    async fn docker_removes_the_container_of_a_removed_tenant() {
        let ops = Arc::new(FakeDocker::default());
        let c = docker(ops.clone(), settings());
        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        c.remove("acme").await.unwrap();
        assert!(ops.running.lock().unwrap().is_empty());
    }

    /// CLOACINA-T-1063: a compiler container an older server started with
    /// the server's URL is stopped, and a new one starts with the build role.
    #[tokio::test]
    async fn docker_replaces_a_container_that_has_the_servers_url() {
        let ops = Arc::new(FakeDocker::default());
        ops.legacy.lock().unwrap().push("old-acme".to_string());
        let c = docker(ops.clone(), settings());

        let out = c.reconcile("acme", CompilerWant::Run).await.unwrap();

        assert!(out.started);
        assert_eq!(
            ops.stopped.lock().unwrap().as_slice(),
            &["old-acme".to_string()]
        );
        assert_eq!(ops.started.lock().unwrap().len(), 1);

        // Also on remove.
        ops.legacy.lock().unwrap().push("old-acme-2".to_string());
        c.remove("acme").await.unwrap();
        assert!(ops.legacy.lock().unwrap().is_empty());
        assert!(ops.running.lock().unwrap().is_empty());
    }

    // -- the pass -----------------------------------------------------------

    struct FixedQueues(Vec<(String, BuildQueue)>);

    #[async_trait]
    impl BuildQueueSource for FixedQueues {
        async fn queues(&self) -> Result<Vec<(String, BuildQueue)>, String> {
            Ok(self.0.clone())
        }
    }

    /// An actuator that only has the Kubernetes compiler side.
    struct CompilersOnly(Option<KubeCompilers>);

    #[async_trait]
    impl FleetActuator for CompilersOnly {
        async fn reconcile(
            &self,
            _tenant_id: &str,
            _desired: u32,
        ) -> Result<super::super::ReconcileOutcome, ActuatorError> {
            Ok(Default::default())
        }
        async fn reconcile_compiler(
            &self,
            tenant_id: &str,
            want: CompilerWant,
        ) -> Result<CompilerOutcome, ActuatorError> {
            self.0.as_ref().unwrap().reconcile(tenant_id, want).await
        }
        fn compiler_cooldown(&self) -> Option<Duration> {
            self.0.as_ref().map(|c| c.cooldown())
        }
        fn kind(&self) -> &'static str {
            "test"
        }
    }

    fn queue(pending: u64, last_activity: Option<DateTime<Utc>>) -> Vec<(String, BuildQueue)> {
        vec![(
            "acme".to_string(),
            BuildQueue {
                pending,
                building: 0,
                last_activity,
            },
        )]
    }

    #[tokio::test]
    async fn the_pass_starts_keeps_and_stops_a_compiler_as_the_queue_changes() {
        let ops = FakeKube::with_replicas(None);
        let actuator = CompilersOnly(Some(compilers(ops.clone(), settings())));

        // Nothing was ever published: nothing is made.
        let started = run_compiler_pass(&actuator, &FixedQueues(queue(0, None)), at(0)).await;
        assert_eq!(started, 0);
        assert_eq!(*ops.replicas.lock().unwrap(), None);

        // A package is published: one compiler.
        let started = run_compiler_pass(&actuator, &FixedQueues(queue(1, None)), at(30)).await;
        assert_eq!(started, 1);
        assert_eq!(*ops.replicas.lock().unwrap(), Some(1));

        // Built at t=60. Inside the cooldown the compiler stays.
        let built = Some(at(60));
        run_compiler_pass(&actuator, &FixedQueues(queue(0, built)), at(90)).await;
        run_compiler_pass(&actuator, &FixedQueues(queue(0, built)), at(359)).await;
        assert_eq!(*ops.replicas.lock().unwrap(), Some(1));

        // The cooldown is over: 0 replicas.
        run_compiler_pass(&actuator, &FixedQueues(queue(0, built)), at(360)).await;
        assert_eq!(*ops.replicas.lock().unwrap(), Some(0));

        // A new package: started again, and counted.
        let started = run_compiler_pass(&actuator, &FixedQueues(queue(1, built)), at(400)).await;
        assert_eq!(started, 1);
        assert_eq!(*ops.replicas.lock().unwrap(), Some(1));
    }

    #[tokio::test]
    async fn the_pass_reads_no_queue_for_an_actuator_without_compilers() {
        struct Unreadable;
        #[async_trait]
        impl BuildQueueSource for Unreadable {
            async fn queues(&self) -> Result<Vec<(String, BuildQueue)>, String> {
                panic!("the queue was read");
            }
        }
        let started = run_compiler_pass(&CompilersOnly(None), &Unreadable, at(0)).await;
        assert_eq!(started, 0);
        let started = run_compiler_pass(&super::super::NoopActuator, &Unreadable, at(0)).await;
        assert_eq!(started, 0);
    }
}
