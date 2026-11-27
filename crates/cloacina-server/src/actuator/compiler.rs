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
//! A `cloacina-compiler` serves one schema (`--tenant-schema`). The chart's
//! own compiler serves `public`. For every other tenant the fleet actuator
//! runs one compiler, next to the tenant's agents, and only while the
//! tenant's build queue has work:
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
/// Key of the Python package index URL in [`COMPILER_SECRET_NAME`]
/// (CLOACINA-T-1117). Present only when an index URL is set: it can hold a
/// password, so it is never in the pod spec.
pub const COMPILER_SECRET_INDEX_KEY: &str = "python-index-url";

const LABEL_APP: &str = "app.kubernetes.io/name";
const LABEL_COMPONENT: &str = "app.kubernetes.io/component";
const LABEL_MANAGED: &str = "cloacina.managed";
const LABEL_TENANT: &str = "cloacina.tenant";

/// uid and gid of a hardened compiler pod (the agents use the same).
const COMPILER_RUN_AS: i64 = 10001;
/// Where a hardened pod mounts its writable volumes.
const HOME_DIR: &str = "/var/cloacina/home";
const CARGO_HOME_DIR: &str = "/var/cloacina/cargo";
/// The `uv` cache of a hardened pod: its own emptyDir, so the cache does not
/// fill the home, where the build folder is (CLOACINA-T-1117).
const UV_CACHE_DIR: &str = "/var/cloacina/uv-cache";
/// Where a hardened pod mounts its cache claim (CLOACINA-T-1117). The cargo
/// home, the target directory and the `uv` cache are folders in it, made by
/// the compiler's own user.
const CACHE_CLAIM_DIR: &str = "/var/cloacina/cache";
/// Name of a tenant compiler's PersistentVolumeClaim.
pub const COMPILER_CACHE_CLAIM_NAME: &str = "cloacina-compiler-cache";

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
    /// Size limit of the home emptyDir of a hardened pod, where the build
    /// folder is (`CLOACINA_TENANT_COMPILER_HOME_SIZE`, default `4Gi`).
    pub home_size: String,
    /// The Python settings of the compiler (CLOACINA-T-1117).
    pub python: TenantPythonSettings,
    /// The cache claim of a hardened pod (CLOACINA-T-1117).
    pub cache_claim: CacheClaimSettings,
}

/// A PersistentVolumeClaim per tenant compiler that holds its cargo home, its
/// target directory and its `uv` cache, so a start from 0 replicas finds the
/// caches of the last run (CLOACINA-T-1117). Hardened pods only: an
/// unhardened pod runs on the image's own filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheClaimSettings {
    /// On by default (`CLOACINA_TENANT_COMPILER_CACHE_CLAIM`). Off: the three
    /// caches are emptyDirs, as before.
    pub enabled: bool,
    /// Requested size (`CLOACINA_TENANT_COMPILER_CACHE_CLAIM_SIZE`, default
    /// `20Gi`: the sum of the emptyDir limits it replaces, 8Gi cargo home,
    /// 8Gi target directory and 4Gi `uv` cache).
    pub size: String,
    /// Storage class (`CLOACINA_TENANT_COMPILER_CACHE_CLAIM_STORAGE_CLASS`).
    /// `None`: no `storageClassName`, the cluster's default class.
    pub storage_class: Option<String>,
}

impl Default for CacheClaimSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            size: "20Gi".to_string(),
            storage_class: None,
        }
    }
}

/// Whether `raw` is a Kubernetes quantity of storage above zero: a number
/// with an optional binary or decimal suffix (`20Gi`, `500M`, `1.5Ti`).
fn valid_storage_quantity(raw: &str) -> bool {
    const SUFFIXES: [&str; 12] = [
        "Ki", "Mi", "Gi", "Ti", "Pi", "Ei", "k", "M", "G", "T", "P", "E",
    ];
    let number = SUFFIXES
        .iter()
        .find_map(|s| raw.strip_suffix(s))
        .unwrap_or(raw);
    let mut parts = number.splitn(2, '.');
    let whole = parts.next().unwrap_or("");
    let fraction = parts.next();
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    digits(whole)
        && fraction.is_none_or(digits)
        && number.chars().any(|c| c.is_ascii_digit() && c != '0')
}

/// Whether `raw` is a valid name of a storage class (a DNS subdomain).
fn valid_storage_class(raw: &str) -> bool {
    raw.len() <= 253
        && raw
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
        && raw.starts_with(|c: char| c.is_ascii_alphanumeric())
        && raw.ends_with(|c: char| c.is_ascii_alphanumeric())
}

/// The Python vendoring settings of a tenant's compiler (CLOACINA-T-1117),
/// from `CLOACINA_TENANT_COMPILER_PYTHON_*`. Each one that is set is given to
/// the compiler as its own `CLOACINA_COMPILER_PYTHON_*` setting; one that is
/// not set leaves the compiler's default.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct TenantPythonSettings {
    /// Platforms to vendor for (`_PYTHON_PLATFORMS`, comma-separated).
    /// Empty: the compiler's default, `aarch64-linux` and `x86_64-linux`.
    pub platforms: Vec<String>,
    /// Python version, `major.minor` (`_PYTHON_VERSION`). `None`: 3.11.
    pub version: Option<String>,
    /// Package index URL (`_PYTHON_INDEX_URL`). `None`: PyPI. Can hold a
    /// password: it goes in the Secret, never in the pod spec or a log.
    pub index_url: Option<String>,
    /// A folder of wheels in the compiler image (`_PYTHON_WHEEL_DIR`).
    pub wheel_dir: Option<String>,
    /// Vendor with no network (`_PYTHON_OFFLINE`).
    pub offline: bool,
    /// The `uv` cache folder (`_PYTHON_UV_CACHE_DIR`). `None`: in a
    /// hardened pod, its own emptyDir; else `uv`'s default.
    pub uv_cache_dir: Option<String>,
    /// Size limit of the `uv` cache emptyDir of a hardened pod
    /// (`_PYTHON_CACHE_SIZE`, default `4Gi`).
    pub cache_size: String,
}

impl std::fmt::Debug for TenantPythonSettings {
    /// The index URL can hold a password: it is never printed.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TenantPythonSettings")
            .field("platforms", &self.platforms)
            .field("version", &self.version)
            .field("index_url", &self.index_url.as_ref().map(|_| "<redacted>"))
            .field("wheel_dir", &self.wheel_dir)
            .field("offline", &self.offline)
            .field("uv_cache_dir", &self.uv_cache_dir)
            .field("cache_size", &self.cache_size)
            .finish()
    }
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
            .field("home_size", &self.home_size)
            .field("python", &self.python)
            .field("cache_claim", &self.cache_claim)
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
            home_size: "4Gi".to_string(),
            python: TenantPythonSettings {
                cache_size: "4Gi".to_string(),
                ..TenantPythonSettings::default()
            },
            cache_claim: CacheClaimSettings::default(),
        }
    }

    /// Whether the pod mounts the cache claim: on, and hardened.
    pub fn uses_cache_claim(&self) -> bool {
        self.hardened && self.cache_claim.enabled
    }

    /// The cargo target directory the compiler is given.
    fn effective_target_dir(&self) -> String {
        if self.uses_cache_claim() {
            format!("{CACHE_CLAIM_DIR}/target")
        } else {
            self.target_dir.clone()
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
        if let Some(v) = get("CLOACINA_TENANT_COMPILER_HOME_SIZE") {
            s.home_size = v;
        }
        // CLOACINA-T-1117: the cache claim.
        s.cache_claim.enabled = flag("CLOACINA_TENANT_COMPILER_CACHE_CLAIM", true)?;
        if let Some(v) = get("CLOACINA_TENANT_COMPILER_CACHE_CLAIM_SIZE") {
            if !valid_storage_quantity(&v) {
                return Err(format!(
                    "CLOACINA_TENANT_COMPILER_CACHE_CLAIM_SIZE: '{v}' is not a storage size \
                     above zero, for example 20Gi"
                ));
            }
            s.cache_claim.size = v;
        }
        if let Some(v) = get("CLOACINA_TENANT_COMPILER_CACHE_CLAIM_STORAGE_CLASS") {
            if !valid_storage_class(&v) {
                return Err(format!(
                    "CLOACINA_TENANT_COMPILER_CACHE_CLAIM_STORAGE_CLASS: '{v}' is not a storage \
                     class name (lower-case letters, digits, '-' and '.')"
                ));
            }
            s.cache_claim.storage_class = Some(v);
        }

        // CLOACINA-T-1117: the Python settings. Checked here, so a value the
        // compiler would refuse stops the server and not each compiler pod.
        let absolute = |key: &str| -> Result<Option<String>, String> {
            match get(key) {
                Some(dir) if !dir.starts_with('/') => {
                    Err(format!("{key}: '{dir}' is not an absolute path"))
                }
                other => Ok(other),
            }
        };
        let known = cloacina::cloacina_workflow_plugin::python_env::SUPPORTED_PLATFORMS;
        for platform in get("CLOACINA_TENANT_COMPILER_PYTHON_PLATFORMS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
        {
            if !known.contains(&platform) {
                return Err(format!(
                    "CLOACINA_TENANT_COMPILER_PYTHON_PLATFORMS: '{platform}' is not one of {}",
                    known.join(", ")
                ));
            }
            if !s.python.platforms.iter().any(|p| p == platform) {
                s.python.platforms.push(platform.to_string());
            }
        }
        if let Some(v) = get("CLOACINA_TENANT_COMPILER_PYTHON_VERSION") {
            let ok = v.split_once('.').is_some_and(|(major, minor)| {
                !major.is_empty()
                    && !minor.is_empty()
                    && major.chars().all(|c| c.is_ascii_digit())
                    && minor.chars().all(|c| c.is_ascii_digit())
            });
            if !ok {
                return Err(format!(
                    "CLOACINA_TENANT_COMPILER_PYTHON_VERSION: '{v}' is not major.minor, for example 3.11"
                ));
            }
            s.python.version = Some(v);
        }
        s.python.index_url = get("CLOACINA_TENANT_COMPILER_PYTHON_INDEX_URL");
        s.python.wheel_dir = absolute("CLOACINA_TENANT_COMPILER_PYTHON_WHEEL_DIR")?;
        s.python.offline = flag("CLOACINA_TENANT_COMPILER_PYTHON_OFFLINE", false)?;
        s.python.uv_cache_dir = absolute("CLOACINA_TENANT_COMPILER_PYTHON_UV_CACHE_DIR")?;
        if let Some(v) = get("CLOACINA_TENANT_COMPILER_PYTHON_CACHE_SIZE") {
            s.python.cache_size = v;
        }
        // A hardened pod starts with an empty `uv` cache: offline, only a
        // wheel folder in the image can hold the wheels.
        if s.python.offline && s.hardened && s.python.wheel_dir.is_none() {
            return Err(
                "CLOACINA_TENANT_COMPILER_PYTHON_OFFLINE: a hardened tenant compiler starts with \
                 an empty uv cache, so offline it needs CLOACINA_TENANT_COMPILER_PYTHON_WHEEL_DIR \
                 (a folder of wheels in the image)"
                    .to_string(),
            );
        }
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
            self.effective_target_dir(),
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
            let cargo_home = if self.uses_cache_claim() {
                format!("{CACHE_CLAIM_DIR}/cargo")
            } else {
                CARGO_HOME_DIR.to_string()
            };
            env.push(("CARGO_HOME".to_string(), cargo_home));
        }
        // CLOACINA-T-1117: the Python settings, as the compiler's own env
        // vars. The index URL is not here: it comes from the Secret.
        let py = &self.python;
        if !py.platforms.is_empty() {
            env.push((
                "CLOACINA_COMPILER_PYTHON_PLATFORMS".to_string(),
                py.platforms.join(","),
            ));
        }
        if let Some(v) = &py.version {
            env.push(("CLOACINA_COMPILER_PYTHON_VERSION".to_string(), v.clone()));
        }
        if let Some(dir) = &py.wheel_dir {
            env.push((
                "CLOACINA_COMPILER_PYTHON_WHEEL_DIR".to_string(),
                dir.clone(),
            ));
        }
        if py.offline {
            env.push((
                "CLOACINA_COMPILER_PYTHON_OFFLINE".to_string(),
                "true".to_string(),
            ));
        }
        let uv_cache = match (&py.uv_cache_dir, self.hardened) {
            (Some(dir), _) => Some(dir.clone()),
            (None, true) if self.uses_cache_claim() => Some(format!("{CACHE_CLAIM_DIR}/uv")),
            (None, true) => Some(UV_CACHE_DIR.to_string()),
            (None, false) => None,
        };
        if let Some(dir) = uv_cache {
            env.push(("CLOACINA_COMPILER_PYTHON_UV_CACHE_DIR".to_string(), dir));
        }
        env
    }
}

/// The value of `CLOACINA_MAX_UNPACKED_BYTES` for a tenant's compiler: the
/// server's own unpack limit, when it is not the default (CLOACINA-T-1117).
/// The compiler refuses a vendored set that a loader with this limit would
/// refuse.
fn unpack_limit_env() -> Option<(String, String)> {
    cloacina::cloacina_workflow_plugin::unpack_limit::child_env_value().map(|bytes| {
        (
            cloacina::cloacina_workflow_plugin::unpack_limit::MAX_UNPACKED_BYTES_ENV.to_string(),
            bytes,
        )
    })
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
    /// Merge `patch` into a Secret that exists. `Ok(false)`: there is no such
    /// Secret (or no such namespace), and none is made. Unlike [`Self::apply`],
    /// which makes the object when it is absent.
    async fn patch_secret_if_present(
        &self,
        namespace: &str,
        name: &str,
        patch: &serde_json::Value,
    ) -> Result<bool, ActuatorError>;
    /// Make a PersistentVolumeClaim when there is none of that name
    /// (CLOACINA-T-1117). A claim that exists is left as it is: its storage
    /// class cannot change, and its data is the cache. `Ok(true)`: made now.
    async fn ensure_claim(
        &self,
        namespace: &str,
        name: &str,
        manifest: &serde_json::Value,
    ) -> Result<bool, ActuatorError>;
    /// Delete a PersistentVolumeClaim. Absent is not an error.
    async fn delete_claim(&self, namespace: &str, name: &str) -> Result<(), ActuatorError>;
}

/// The cache claim of one tenant's compiler (CLOACINA-T-1117). Pure.
pub fn compiler_cache_claim_manifest(
    namespace: &str,
    tenant_id: &str,
    claim: &CacheClaimSettings,
) -> serde_json::Value {
    let mut manifest = serde_json::json!({
        "apiVersion": "v1",
        "kind": "PersistentVolumeClaim",
        "metadata": {
            "name": COMPILER_CACHE_CLAIM_NAME,
            "namespace": namespace,
            "labels": {
                LABEL_APP: COMPILER_NAME,
                LABEL_COMPONENT: "compiler",
                LABEL_MANAGED: "true",
                LABEL_TENANT: tenant_id,
            },
        },
        "spec": {
            // One pod at a time (the Deployment is Recreate).
            "accessModes": ["ReadWriteOnce"],
            "resources": { "requests": { "storage": claim.size } },
        },
    });
    if let Some(class) = &claim.storage_class {
        manifest["spec"]["storageClassName"] = serde_json::json!(class);
    }
    manifest
}

/// The tenant namespace manifest: the same object the agent side applies.
fn namespace_manifest(namespace: &str) -> serde_json::Value {
    serde_json::json!({
        "apiVersion": "v1",
        "kind": "Namespace",
        "metadata": { "name": namespace, "labels": { LABEL_MANAGED: "true" } }
    })
}

/// The Secret with the compiler's database URL.
pub fn database_secret_manifest(namespace: &str, database_url: &str) -> serde_json::Value {
    serde_json::json!({
        "apiVersion": "v1",
        "kind": "Secret",
        "metadata": {
            "name": COMPILER_SECRET_NAME,
            "namespace": namespace,
            "labels": { LABEL_MANAGED: "true" },
        },
        "type": "Opaque",
        "stringData": { COMPILER_SECRET_KEY: database_url },
    })
}

/// The Secret a running tenant compiler reads: the database URL and, when
/// one is set, the Python package index URL (CLOACINA-T-1117), both of which
/// can hold a password.
pub fn compiler_secret_manifest(namespace: &str, settings: &CompilerSettings) -> serde_json::Value {
    let mut secret = database_secret_manifest(namespace, &settings.database_url);
    if let Some(url) = &settings.python.index_url {
        secret["stringData"][COMPILER_SECRET_INDEX_KEY] = serde_json::json!(url);
    }
    secret
}

/// The patch that writes an empty value over the database URL and the index
/// URL of the Secret (CLOACINA-T-1105, CLOACINA-T-1117): that is how the
/// credentials are taken away when the tenant goes (the server has no right
/// to delete Secrets). Each key is written, not left out. A patch changes a
/// Secret that exists and makes none.
pub fn empty_database_secret_patch() -> serde_json::Value {
    serde_json::json!({ "stringData": { COMPILER_SECRET_KEY: "", COMPILER_SECRET_INDEX_KEY: "" } })
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
    if settings.python.index_url.is_some() {
        // CLOACINA-T-1117: from the Secret too; the compiler gives it to uv in
        // the environment, not on the command line.
        env.push(serde_json::json!({
            "name": "CLOACINA_COMPILER_PYTHON_INDEX_URL",
            "valueFrom": {
                "secretKeyRef": { "name": COMPILER_SECRET_NAME, "key": COMPILER_SECRET_INDEX_KEY }
            }
        }));
    }
    for (name, value) in settings.plain_env().into_iter().chain(unpack_limit_env()) {
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
        // A build writes: the unpacked source, the logs and the Python
        // vendoring folders (home), cargo's registry and git checkouts (cargo
        // home), the build output (target), scratch files (/tmp) and the
        // wheels uv downloads (uv cache, CLOACINA-T-1117). With a read-only
        // root each is an emptyDir, so a compiler that starts again starts
        // with empty caches.
        if settings.uses_cache_claim() {
            // CLOACINA-T-1117: the three caches are folders of one claim, so
            // a start from 0 replicas finds them. The home and /tmp stay
            // emptyDirs: they hold one build's files.
            pod["containers"][0]["volumeMounts"] = serde_json::json!([
                { "name": "home", "mountPath": HOME_DIR },
                { "name": "tmp", "mountPath": "/tmp" },
                { "name": "cache", "mountPath": CACHE_CLAIM_DIR },
            ]);
            pod["volumes"] = serde_json::json!([
                { "name": "home", "emptyDir": { "sizeLimit": settings.home_size } },
                { "name": "tmp", "emptyDir": { "sizeLimit": "1Gi" } },
                {
                    "name": "cache",
                    "persistentVolumeClaim": { "claimName": COMPILER_CACHE_CLAIM_NAME }
                },
            ]);
        } else {
            pod["containers"][0]["volumeMounts"] = serde_json::json!([
                { "name": "home", "mountPath": HOME_DIR },
                { "name": "cargo-home", "mountPath": CARGO_HOME_DIR },
                { "name": "target", "mountPath": settings.target_dir },
                { "name": "tmp", "mountPath": "/tmp" },
                { "name": "uv-cache", "mountPath": UV_CACHE_DIR },
            ]);
            pod["volumes"] = serde_json::json!([
                { "name": "home", "emptyDir": { "sizeLimit": settings.home_size } },
                { "name": "cargo-home", "emptyDir": { "sizeLimit": settings.cache_size } },
                { "name": "target", "emptyDir": { "sizeLimit": settings.cache_size } },
                { "name": "tmp", "emptyDir": { "sizeLimit": "1Gi" } },
                { "name": "uv-cache", "emptyDir": { "sizeLimit": settings.python.cache_size } },
            ]);
        }
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
}

impl KubeCompilers {
    pub fn new(
        ops: Arc<dyn CompilerKubeOps>,
        settings: CompilerSettings,
        network_policy: bool,
        dns_namespace: String,
    ) -> Self {
        Self {
            ops,
            settings,
            network_policy,
            dns_namespace,
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

        // Run. Everything is applied on each tick with work, so a changed
        // server setting reaches the compiler. The Secret and the policy come
        // before the Deployment: the pod needs the first to start and must
        // not run without the second.
        self.ops
            .apply(
                KubeKind::Namespace,
                "",
                &namespace,
                &namespace_manifest(&namespace),
            )
            .await?;
        self.ops
            .apply(
                KubeKind::Secret,
                &namespace,
                COMPILER_SECRET_NAME,
                &compiler_secret_manifest(&namespace, &self.settings),
            )
            .await?;
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
        // CLOACINA-T-1117: the cache claim, before the pod that mounts it.
        // Made once; a scale to 0 keeps it.
        if self.settings.uses_cache_claim() {
            let made = self
                .ops
                .ensure_claim(
                    &namespace,
                    COMPILER_CACHE_CLAIM_NAME,
                    &compiler_cache_claim_manifest(
                        &namespace,
                        tenant_id,
                        &self.settings.cache_claim,
                    ),
                )
                .await?;
            if made {
                info!(tenant = %tenant_id, namespace = %namespace, "tenant compiler cache claim made");
            }
        }
        self.ops
            .apply(
                KubeKind::Deployment,
                &namespace,
                COMPILER_NAME,
                &compiler_deployment_manifest(&namespace, tenant_id, &self.settings),
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
    /// NetworkPolicy, and the database URL in the Secret. Idempotent: each
    /// step is done on every call, whatever an earlier call left.
    pub async fn remove(&self, tenant_id: &str) -> Result<(), ActuatorError> {
        let namespace = tenant_namespace(tenant_id);
        self.ops
            .delete(KubeKind::Deployment, &namespace, COMPILER_NAME)
            .await?;
        self.ops
            .delete(KubeKind::NetworkPolicy, &namespace, COMPILER_NAME)
            .await?;
        // CLOACINA-T-1117: the cache claim goes with the tenant, whatever the
        // setting is now (it may have been on before). Absent is fine.
        self.ops
            .delete_claim(&namespace, COMPILER_CACHE_CLAIM_NAME)
            .await?;
        // CLOACINA-T-1105: the Secret is emptied whenever it exists. It does
        // not depend on the Deployment: a call that deleted the Deployment
        // and then failed is retried, and the retry finds no Deployment. A
        // patch, so that a tenant that never had a compiler gets no namespace
        // and no Secret now.
        let emptied = self
            .ops
            .patch_secret_if_present(
                &namespace,
                COMPILER_SECRET_NAME,
                &empty_database_secret_patch(),
            )
            .await?;
        if emptied {
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
    use k8s_openapi::api::core::v1::{Namespace, PersistentVolumeClaim, Secret};
    use k8s_openapi::api::networking::v1::NetworkPolicy;
    use kube::Api;
    use kube::api::{DeleteParams, Patch, PatchParams, PostParams};

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

        async fn patch_secret_if_present(
            &self,
            namespace: &str,
            name: &str,
            patch: &serde_json::Value,
        ) -> Result<bool, ActuatorError> {
            // A merge patch, not an apply: it changes a Secret that exists and
            // answers 404 for one that does not. It needs the `patch` verb
            // only; the server has no `get` on Secrets.
            let result = Api::<Secret>::namespaced(self.client.clone(), namespace)
                .patch(name, &PatchParams::default(), &Patch::Merge(patch))
                .await;
            match result {
                Ok(_) => Ok(true),
                Err(kube::Error::Api(e)) if e.code == 404 => Ok(false),
                Err(e) => Err(failed("secret patch", e)),
            }
        }

        async fn ensure_claim(
            &self,
            namespace: &str,
            name: &str,
            manifest: &serde_json::Value,
        ) -> Result<bool, ActuatorError> {
            // `get` then `create`: no `patch` on claims, so a claim that
            // exists is never changed (its storage class cannot be).
            let api = Api::<PersistentVolumeClaim>::namespaced(self.client.clone(), namespace);
            if api
                .get_opt(name)
                .await
                .map_err(|e| failed("claim read", e))?
                .is_some()
            {
                return Ok(false);
            }
            let claim: PersistentVolumeClaim = serde_json::from_value(manifest.clone())
                .map_err(|e| failed("claim manifest", e))?;
            match api.create(&PostParams::default(), &claim).await {
                Ok(_) => Ok(true),
                // Made by another replica between the read and the create.
                Err(kube::Error::Api(e)) if e.code == 409 => Ok(false),
                Err(e) => Err(failed("claim create", e)),
            }
        }

        async fn delete_claim(&self, namespace: &str, name: &str) -> Result<(), ActuatorError> {
            deleted(
                "claim delete",
                Api::<PersistentVolumeClaim>::namespaced(self.client.clone(), namespace)
                    .delete(name, &DeleteParams::default())
                    .await,
            )
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
}

/// The compiler side of the Docker actuator: one container per tenant with
/// work, none without. Of the settings it applies the image, the arguments,
/// the database URL, the Python settings and the network. The resources, the
/// hardening, the cache volumes and the NetworkPolicy have no counterpart
/// here.
pub struct DockerCompilers {
    ops: Arc<dyn CompilerContainerOps>,
    settings: CompilerSettings,
}

impl DockerCompilers {
    pub fn new(ops: Arc<dyn CompilerContainerOps>, settings: CompilerSettings) -> Self {
        Self { ops, settings }
    }

    pub fn cooldown(&self) -> Duration {
        self.settings.cooldown
    }

    /// The container a tenant's compiler runs as.
    pub fn container(&self, tenant_id: &str) -> CompilerContainer {
        // The image's own filesystem is used: the hardened paths are volumes
        // of the Kubernetes pod only.
        let plain = CompilerSettings {
            hardened: false,
            ..self.settings.clone()
        };
        let mut env = vec![format!("DATABASE_URL={}", self.settings.database_url)];
        // CLOACINA-T-1117: next to the database URL, in the environment only.
        if let Some(url) = &self.settings.python.index_url {
            env.push(format!("CLOACINA_COMPILER_PYTHON_INDEX_URL={url}"));
        }
        env.extend(
            plain
                .plain_env()
                .into_iter()
                .chain(unpack_limit_env())
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
        let running = self.ops.list_compilers(tenant_id).await?;
        if want == CompilerWant::Stop {
            for id in &running {
                self.ops.stop_compiler(id).await?;
            }
            return Ok(CompilerOutcome {
                started: false,
                stopped: !running.is_empty(),
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
        self.ops.start_compiler(self.container(tenant_id)).await?;
        info!(tenant = %tenant_id, "tenant compiler started: build queue has work (docker)");
        Ok(CompilerOutcome {
            started: true,
            stopped: false,
        })
    }

    pub async fn remove(&self, tenant_id: &str) -> Result<(), ActuatorError> {
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
        CompilerContainer, CompilerContainerOps, DOCKER_LABEL_COMPILER, DOCKER_LABEL_TENANT,
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

        async fn start_compiler(&self, spec: CompilerContainer) -> Result<(), ActuatorError> {
            let mut labels: HashMap<String, String> = HashMap::new();
            labels.insert(DOCKER_LABEL_COMPILER.to_string(), "true".to_string());
            labels.insert(DOCKER_LABEL_TENANT.to_string(), spec.tenant_id.clone());
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
    /// Every tenant (not `public`) with its queue. A tenant whose queue could
    /// not be read is left out: its compiler stays as it is.
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
        let mut out = Vec::with_capacity(schemas.len());
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
         read on the last tick of the fleet loop (CLOACINA-T-1034). The \
         `public` schema is not counted: the chart's own compiler serves it. \
         Set only by the replica that led the tick."
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
        /// The database URL in the Secret. `None`: no Secret.
        secret: Mutex<Option<String>>,
        applied: Mutex<Vec<Applied>>,
        deleted: Mutex<Vec<(KubeKind, String, String)>>,
        scaled: Mutex<Vec<(String, String, u32)>>,
        /// The next delete of this kind fails, once.
        fail_delete_once: Mutex<Option<KubeKind>>,
        /// The patches applied to a Secret that exists.
        patches: Mutex<Vec<serde_json::Value>>,
        /// The claims that exist: `(namespace, name)` to the manifest.
        claims: Mutex<Vec<(String, String, serde_json::Value)>>,
        /// Calls of `ensure_claim` that made a claim.
        claims_made: Mutex<u32>,
        /// Calls of `delete_claim`.
        claim_deletes: Mutex<u32>,
    }

    impl FakeKube {
        fn with_replicas(replicas: Option<u32>) -> Arc<Self> {
            Arc::new(Self {
                replicas: Mutex::new(replicas),
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
            }
            if kind == KubeKind::Secret {
                // An apply makes the Secret when it is absent.
                let value = manifest["stringData"][COMPILER_SECRET_KEY]
                    .as_str()
                    .unwrap_or_default();
                *self.secret.lock().unwrap() = Some(value.to_string());
            }
            Ok(())
        }
        async fn patch_secret_if_present(
            &self,
            _namespace: &str,
            _name: &str,
            patch: &serde_json::Value,
        ) -> Result<bool, ActuatorError> {
            let mut secret = self.secret.lock().unwrap();
            match secret.as_mut() {
                // A patch makes nothing.
                None => Ok(false),
                Some(value) => {
                    if let Some(new) = patch["stringData"][COMPILER_SECRET_KEY].as_str() {
                        *value = new.to_string();
                    }
                    self.patches.lock().unwrap().push(patch.clone());
                    Ok(true)
                }
            }
        }
        async fn delete(
            &self,
            kind: KubeKind,
            namespace: &str,
            name: &str,
        ) -> Result<(), ActuatorError> {
            {
                let mut fail = self.fail_delete_once.lock().unwrap();
                if *fail == Some(kind) {
                    *fail = None;
                    return Err(ActuatorError::Substrate(
                        "the API server did not answer".to_string(),
                    ));
                }
            }
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
        async fn ensure_claim(
            &self,
            namespace: &str,
            name: &str,
            manifest: &serde_json::Value,
        ) -> Result<bool, ActuatorError> {
            let mut claims = self.claims.lock().unwrap();
            if claims.iter().any(|c| c.0 == namespace && c.1 == name) {
                return Ok(false);
            }
            claims.push((namespace.to_string(), name.to_string(), manifest.clone()));
            *self.claims_made.lock().unwrap() += 1;
            Ok(true)
        }
        async fn delete_claim(&self, namespace: &str, name: &str) -> Result<(), ActuatorError> {
            *self.claim_deletes.lock().unwrap() += 1;
            self.claims
                .lock()
                .unwrap()
                .retain(|c| !(c.0 == namespace && c.1 == name));
            Ok(())
        }
    }

    const URL: &str = "postgres://cloacina:s3cret@db.example:6432/cloacina";

    fn settings() -> CompilerSettings {
        CompilerSettings::with_database_url(URL)
    }

    fn compilers(ops: Arc<FakeKube>, settings: CompilerSettings) -> KubeCompilers {
        KubeCompilers::new(ops, settings, true, "kube-system".to_string())
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
        let ops = FakeKube::with_replicas(Some(1));
        let c = compilers(ops.clone(), settings());

        let out = c.reconcile("acme", CompilerWant::Run).await.unwrap();

        assert_eq!(out, CompilerOutcome::default());
        assert!(ops.scaled.lock().unwrap().is_empty());
        // The objects are applied again, so a changed setting gets through.
        assert_eq!(ops.applied_kinds().len(), 4);
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
        let ops = FakeKube::with_replicas(None);
        let c = compilers(ops.clone(), settings());
        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        assert_eq!(ops.secret.lock().unwrap().as_deref(), Some(URL));

        c.remove("acme").await.unwrap();

        let ns = "cloacina-tenant-acme".to_string();
        assert_eq!(
            ops.deleted.lock().unwrap().as_slice(),
            &[
                (KubeKind::Deployment, ns.clone(), COMPILER_NAME.to_string()),
                (KubeKind::NetworkPolicy, ns, COMPILER_NAME.to_string()),
            ]
        );
        // The Secret is written over with an empty value, by a patch: the
        // last apply of the Secret is the one that made it.
        assert_eq!(ops.secret.lock().unwrap().as_deref(), Some(""));
        assert_eq!(
            ops.last(KubeKind::Secret)["stringData"][COMPILER_SECRET_KEY],
            URL
        );
        assert_eq!(
            empty_database_secret_patch(),
            serde_json::json!({ "stringData": { "database-url": "", "python-index-url": "" } })
        );
    }

    #[tokio::test]
    async fn removing_a_tenant_that_never_had_a_compiler_creates_nothing() {
        let ops = FakeKube::with_replicas(None);
        let c = compilers(ops.clone(), settings());
        c.remove("quiet").await.unwrap();
        assert!(ops.applied.lock().unwrap().is_empty());
        // No Secret was made to hold an empty value.
        assert_eq!(*ops.secret.lock().unwrap(), None);
    }

    /// CLOACINA-T-1105: a removal that fails after the Deployment delete, then
    /// a retry (tenant delete is retried, CLOACINA-T-1068). The retry finds no
    /// Deployment and must still take the database URL out of the Secret.
    #[tokio::test]
    async fn a_retried_removal_blanks_the_secret_when_the_deployment_is_already_gone() {
        let ops = FakeKube::with_replicas(None);
        let c = compilers(ops.clone(), settings());
        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        assert_eq!(ops.secret.lock().unwrap().as_deref(), Some(URL));

        // The first removal deletes the Deployment and then fails.
        *ops.fail_delete_once.lock().unwrap() = Some(KubeKind::NetworkPolicy);
        c.remove("acme").await.expect_err("the first removal fails");
        assert_eq!(
            *ops.replicas.lock().unwrap(),
            None,
            "the Deployment is gone"
        );

        c.remove("acme").await.expect("the retry succeeds");
        assert_eq!(
            ops.secret.lock().unwrap().as_deref(),
            Some(""),
            "the database URL is still in the Secret after the retry"
        );
    }

    /// The Secret is blanked when there never was a Deployment in the cluster
    /// to find: the removal does not read the Deployment to decide.
    #[tokio::test]
    async fn a_removal_blanks_a_secret_that_has_no_deployment() {
        let ops = FakeKube::with_replicas(None);
        *ops.secret.lock().unwrap() = Some(URL.to_string());
        let c = compilers(ops.clone(), settings());

        c.remove("acme").await.unwrap();

        assert_eq!(ops.secret.lock().unwrap().as_deref(), Some(""));
        // Written over, not made again: nothing was applied.
        assert!(ops.applied.lock().unwrap().is_empty());
    }

    /// A removal twice in a row: the second call succeeds and changes nothing.
    #[tokio::test]
    async fn a_second_removal_succeeds() {
        let ops = FakeKube::with_replicas(None);
        let c = compilers(ops.clone(), settings());
        c.reconcile("acme", CompilerWant::Run).await.unwrap();

        c.remove("acme").await.unwrap();
        c.remove("acme").await.unwrap();

        assert_eq!(*ops.replicas.lock().unwrap(), None);
        assert_eq!(ops.secret.lock().unwrap().as_deref(), Some(""));
    }

    #[tokio::test]
    async fn the_policy_is_not_written_when_fleet_policies_are_off() {
        let ops = FakeKube::with_replicas(None);
        let c = KubeCompilers::new(ops.clone(), settings(), false, "kube-system".to_string());
        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        assert!(!ops.applied_kinds().contains(&KubeKind::NetworkPolicy));
    }

    // -- the manifests ------------------------------------------------------

    #[test]
    fn the_deployment_is_the_tenants_compiler_and_hardened() {
        let mut s = settings();
        // The emptyDir layout; the claim has its own tests.
        s.cache_claim.enabled = false;
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
                "/tmp",
                "/var/cloacina/uv-cache",
            ]
        );
        let volumes = pod["volumes"].as_array().unwrap();
        assert_eq!(volumes.len(), 5);
        assert!(volumes.iter().all(|v| v.get("emptyDir").is_some()));
        assert_eq!(volumes[0]["emptyDir"]["sizeLimit"], "4Gi");
        assert_eq!(volumes[2]["emptyDir"]["sizeLimit"], "8Gi");
        assert_eq!(volumes[4]["emptyDir"]["sizeLimit"], "4Gi");

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
        // The uv cache is on its own volume.
        assert_eq!(
            value("CLOACINA_COMPILER_PYTHON_UV_CACHE_DIR"),
            "/var/cloacina/uv-cache"
        );
        assert_eq!(container["resources"]["limits"]["memory"], "4Gi");
    }

    // -- the Python settings (CLOACINA-T-1117) -------------------------------

    fn python_settings() -> CompilerSettings {
        let mut s = settings();
        s.python = TenantPythonSettings {
            platforms: vec!["x86_64-linux".to_string()],
            version: Some("3.12".to_string()),
            index_url: Some("https://robin:idx-pw@pypi.example.com/simple".to_string()),
            wheel_dir: Some("/opt/wheels".to_string()),
            offline: false,
            uv_cache_dir: None,
            cache_size: "6Gi".to_string(),
        };
        s.home_size = "5Gi".to_string();
        s.cache_claim.enabled = false;
        s
    }

    // -- the cache claim (CLOACINA-T-1117) -----------------------------------

    fn mounts_of(m: &serde_json::Value) -> Vec<(String, String)> {
        m["spec"]["template"]["spec"]["containers"][0]["volumeMounts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                (
                    v["name"].as_str().unwrap().to_string(),
                    v["mountPath"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn the_claim_holds_the_three_caches_by_default() {
        let s = settings();
        assert!(s.uses_cache_claim());
        let m = compiler_deployment_manifest("cloacina-tenant-acme", "acme", &s);
        assert_eq!(
            mounts_of(&m),
            vec![
                ("home".to_string(), "/var/cloacina/home".to_string()),
                ("tmp".to_string(), "/tmp".to_string()),
                ("cache".to_string(), "/var/cloacina/cache".to_string()),
            ]
        );
        let volumes = m["spec"]["template"]["spec"]["volumes"].as_array().unwrap();
        assert_eq!(volumes.len(), 3);
        assert_eq!(volumes[0]["emptyDir"]["sizeLimit"], "4Gi");
        assert_eq!(
            volumes[2]["persistentVolumeClaim"]["claimName"],
            COMPILER_CACHE_CLAIM_NAME
        );

        // The caches are folders of the claim.
        let args = s.args("acme");
        let i = args.iter().position(|a| a == "--cargo-target-dir").unwrap();
        assert_eq!(args[i + 1], "/var/cloacina/cache/target");
        let env = s.plain_env();
        assert!(env.contains(&(
            "CARGO_HOME".to_string(),
            "/var/cloacina/cache/cargo".to_string()
        )));
        assert!(env.contains(&(
            "CLOACINA_COMPILER_PYTHON_UV_CACHE_DIR".to_string(),
            "/var/cloacina/cache/uv".to_string()
        )));
    }

    #[test]
    fn the_claim_manifest_uses_the_default_class_unless_one_is_set() {
        let claim = compiler_cache_claim_manifest(
            "cloacina-tenant-acme",
            "acme",
            &CacheClaimSettings::default(),
        );
        assert_eq!(claim["kind"], "PersistentVolumeClaim");
        assert_eq!(claim["metadata"]["name"], COMPILER_CACHE_CLAIM_NAME);
        assert_eq!(claim["metadata"]["namespace"], "cloacina-tenant-acme");
        assert_eq!(claim["metadata"]["labels"]["cloacina.tenant"], "acme");
        assert_eq!(
            claim["spec"]["accessModes"],
            serde_json::json!(["ReadWriteOnce"])
        );
        assert_eq!(claim["spec"]["resources"]["requests"]["storage"], "20Gi");
        assert!(claim["spec"].get("storageClassName").is_none());

        let claim = compiler_cache_claim_manifest(
            "ns",
            "acme",
            &CacheClaimSettings {
                enabled: true,
                size: "50Gi".to_string(),
                storage_class: Some("fast-ssd".to_string()),
            },
        );
        assert_eq!(claim["spec"]["storageClassName"], "fast-ssd");
        assert_eq!(claim["spec"]["resources"]["requests"]["storage"], "50Gi");
    }

    #[test]
    fn with_the_claim_off_or_unhardened_there_is_no_claim() {
        let mut s = settings();
        s.cache_claim.enabled = false;
        let m = compiler_deployment_manifest("ns", "acme", &s);
        assert!(!m.to_string().contains("persistentVolumeClaim"));
        assert_eq!(mounts_of(&m).len(), 5);

        let mut s = settings();
        s.hardened = false;
        assert!(!s.uses_cache_claim());
        let m = compiler_deployment_manifest("ns", "acme", &s);
        assert!(m["spec"]["template"]["spec"].get("volumes").is_none());
        let args = s.args("acme");
        let i = args.iter().position(|a| a == "--cargo-target-dir").unwrap();
        assert_eq!(args[i + 1], "/workspace/target");
    }

    #[tokio::test]
    async fn reconcile_makes_the_claim_once_and_a_stop_keeps_it() {
        let ops = FakeKube::with_replicas(None);
        let c = compilers(ops.clone(), settings());

        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        assert_eq!(*ops.claims_made.lock().unwrap(), 1);
        assert_eq!(ops.claims.lock().unwrap()[0].0, "cloacina-tenant-acme");

        // A tick with work finds the claim and makes none.
        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        assert_eq!(*ops.claims_made.lock().unwrap(), 1);

        // Scale to zero keeps it; the next start uses it.
        c.reconcile("acme", CompilerWant::Stop).await.unwrap();
        assert_eq!(*ops.replicas.lock().unwrap(), Some(0));
        assert_eq!(ops.claims.lock().unwrap().len(), 1);
        assert!(
            c.reconcile("acme", CompilerWant::Run)
                .await
                .unwrap()
                .started
        );
        assert_eq!(*ops.claims_made.lock().unwrap(), 1);
        assert_eq!(*ops.claim_deletes.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn reconcile_with_an_existing_claim_leaves_it_as_it_is() {
        let ops = FakeKube::with_replicas(Some(0));
        let old =
            serde_json::json!({ "spec": { "resources": { "requests": { "storage": "5Gi" } } } });
        ops.claims.lock().unwrap().push((
            "cloacina-tenant-acme".to_string(),
            COMPILER_CACHE_CLAIM_NAME.to_string(),
            old.clone(),
        ));
        let c = compilers(ops.clone(), settings());
        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        assert_eq!(*ops.claims_made.lock().unwrap(), 0);
        assert_eq!(ops.claims.lock().unwrap()[0].2, old);
    }

    #[tokio::test]
    async fn with_the_claim_off_reconcile_makes_no_claim() {
        let ops = FakeKube::with_replicas(None);
        let mut s = settings();
        s.cache_claim.enabled = false;
        let c = compilers(ops.clone(), s);
        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        assert!(ops.claims.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn removal_deletes_the_claim_and_a_second_removal_succeeds() {
        let ops = FakeKube::with_replicas(None);
        let c = compilers(ops.clone(), settings());
        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        assert_eq!(ops.claims.lock().unwrap().len(), 1);

        c.remove("acme").await.unwrap();
        assert!(ops.claims.lock().unwrap().is_empty());
        c.remove("acme").await.unwrap();
        assert_eq!(*ops.claim_deletes.lock().unwrap(), 2);

        // With the setting off now, a claim made before still goes.
        let ops = FakeKube::with_replicas(None);
        ops.claims.lock().unwrap().push((
            "cloacina-tenant-acme".to_string(),
            COMPILER_CACHE_CLAIM_NAME.to_string(),
            serde_json::json!({}),
        ));
        let mut s = settings();
        s.cache_claim.enabled = false;
        compilers(ops.clone(), s).remove("acme").await.unwrap();
        assert!(ops.claims.lock().unwrap().is_empty());
    }

    #[test]
    fn the_claim_settings_are_read_and_checked() {
        let s = CompilerSettings::from_lookup(
            URL,
            lookup(&[
                ("CLOACINA_TENANT_COMPILERS", "true"),
                ("CLOACINA_TENANT_COMPILER_CACHE_CLAIM", "false"),
                ("CLOACINA_TENANT_COMPILER_CACHE_CLAIM_SIZE", "1.5Ti"),
                (
                    "CLOACINA_TENANT_COMPILER_CACHE_CLAIM_STORAGE_CLASS",
                    "fast-ssd",
                ),
            ]),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            s.cache_claim,
            CacheClaimSettings {
                enabled: false,
                size: "1.5Ti".to_string(),
                storage_class: Some("fast-ssd".to_string()),
            }
        );
        let d =
            CompilerSettings::from_lookup(URL, lookup(&[("CLOACINA_TENANT_COMPILERS", "true")]))
                .unwrap()
                .unwrap();
        assert_eq!(d.cache_claim, CacheClaimSettings::default());

        for (key, value) in [
            ("CLOACINA_TENANT_COMPILER_CACHE_CLAIM", "sometimes"),
            ("CLOACINA_TENANT_COMPILER_CACHE_CLAIM_SIZE", "lots"),
            ("CLOACINA_TENANT_COMPILER_CACHE_CLAIM_SIZE", "0Gi"),
            ("CLOACINA_TENANT_COMPILER_CACHE_CLAIM_SIZE", "20GB"),
            (
                "CLOACINA_TENANT_COMPILER_CACHE_CLAIM_STORAGE_CLASS",
                "Fast_SSD",
            ),
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
            assert!(err.contains(key), "{key}={value}: {err}");
        }
        for ok in ["20Gi", "500M", "1", "0.5Ti", "100k"] {
            assert!(valid_storage_quantity(ok), "{ok}");
        }
    }

    fn env_of(m: &serde_json::Value) -> Vec<serde_json::Value> {
        m["spec"]["template"]["spec"]["containers"][0]["env"]
            .as_array()
            .unwrap()
            .clone()
    }

    #[test]
    fn the_compiler_gets_the_python_settings_and_the_index_url_from_the_secret() {
        let s = python_settings();
        let m = compiler_deployment_manifest("cloacina-tenant-acme", "acme", &s);
        let env = env_of(&m);
        let value = |name: &str| {
            env.iter()
                .find(|e| e["name"] == name)
                .map(|e| e["value"].clone())
                .unwrap_or_else(|| panic!("no env {name}"))
        };
        assert_eq!(value("CLOACINA_COMPILER_PYTHON_PLATFORMS"), "x86_64-linux");
        assert_eq!(value("CLOACINA_COMPILER_PYTHON_VERSION"), "3.12");
        assert_eq!(value("CLOACINA_COMPILER_PYTHON_WHEEL_DIR"), "/opt/wheels");
        assert!(
            !env.iter()
                .any(|e| e["name"] == "CLOACINA_COMPILER_PYTHON_OFFLINE")
        );

        // The index URL is read from the Secret, by a key of its own.
        let index = env
            .iter()
            .find(|e| e["name"] == "CLOACINA_COMPILER_PYTHON_INDEX_URL")
            .expect("the index URL env");
        assert!(index.get("value").is_none());
        assert_eq!(
            index["valueFrom"]["secretKeyRef"]["name"],
            COMPILER_SECRET_NAME
        );
        assert_eq!(
            index["valueFrom"]["secretKeyRef"]["key"],
            COMPILER_SECRET_INDEX_KEY
        );
        // Nowhere in the pod spec, and not in the settings' Debug.
        assert!(!m.to_string().contains("idx-pw"));
        assert!(!format!("{s:?}").contains("idx-pw"));

        let secret = compiler_secret_manifest("cloacina-tenant-acme", &s);
        assert_eq!(secret["stringData"][COMPILER_SECRET_KEY], URL);
        assert_eq!(
            secret["stringData"][COMPILER_SECRET_INDEX_KEY],
            "https://robin:idx-pw@pypi.example.com/simple"
        );

        // The sizes of the home and of the uv cache.
        let volumes = m["spec"]["template"]["spec"]["volumes"].as_array().unwrap();
        assert_eq!(volumes[0]["emptyDir"]["sizeLimit"], "5Gi");
        assert_eq!(volumes[4]["name"], "uv-cache");
        assert_eq!(volumes[4]["emptyDir"]["sizeLimit"], "6Gi");
    }

    #[test]
    fn with_no_python_settings_the_compiler_keeps_its_defaults() {
        let m = compiler_deployment_manifest("cloacina-tenant-acme", "acme", &settings());
        let names: Vec<String> = env_of(&m)
            .iter()
            .map(|e| e["name"].as_str().unwrap().to_string())
            .collect();
        // Only the cache folder, which the hardened pod needs on its volume.
        let python: Vec<&String> = names
            .iter()
            .filter(|n| n.starts_with("CLOACINA_COMPILER_PYTHON_"))
            .collect();
        assert_eq!(python, ["CLOACINA_COMPILER_PYTHON_UV_CACHE_DIR"]);
        let secret = compiler_secret_manifest("cloacina-tenant-acme", &settings());
        assert!(
            secret["stringData"]
                .get(COMPILER_SECRET_INDEX_KEY)
                .is_none()
        );

        // Not hardened: uv's own cache folder, and no volume for it.
        let mut s = settings();
        s.hardened = false;
        assert!(
            !s.plain_env()
                .iter()
                .any(|(k, _)| k.starts_with("CLOACINA_COMPILER_PYTHON_"))
        );
    }

    #[test]
    fn offline_and_a_cache_folder_are_passed_on() {
        let mut s = python_settings();
        s.python.offline = true;
        s.python.uv_cache_dir = Some("/var/cloacina/home/uv".to_string());
        let env = s.plain_env();
        assert!(env.contains(&(
            "CLOACINA_COMPILER_PYTHON_OFFLINE".to_string(),
            "true".to_string()
        )));
        assert!(env.contains(&(
            "CLOACINA_COMPILER_PYTHON_UV_CACHE_DIR".to_string(),
            "/var/cloacina/home/uv".to_string()
        )));
    }

    #[tokio::test]
    async fn the_running_compiler_secret_holds_the_index_url_and_the_removal_empties_it() {
        let ops = FakeKube::with_replicas(None);
        let c = compilers(ops.clone(), python_settings());
        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        let secret = ops.last(KubeKind::Secret);
        assert_eq!(
            secret["stringData"][COMPILER_SECRET_INDEX_KEY],
            "https://robin:idx-pw@pypi.example.com/simple"
        );

        c.remove("acme").await.unwrap();
        // Both keys are written over with an empty value, by the patch of
        // CLOACINA-T-1105.
        let removed = ops
            .patches
            .lock()
            .unwrap()
            .last()
            .cloned()
            .expect("the Secret was patched");
        assert!(!removed.to_string().contains("idx-pw"));
        assert!(!removed.to_string().contains("s3cret"));
        assert_eq!(removed["stringData"][COMPILER_SECRET_INDEX_KEY], "");
        assert_eq!(removed["stringData"][COMPILER_SECRET_KEY], "");
    }

    #[test]
    fn the_python_settings_are_read_from_the_environment() {
        let s = CompilerSettings::from_lookup(
            URL,
            lookup(&[
                ("CLOACINA_TENANT_COMPILERS", "true"),
                (
                    "CLOACINA_TENANT_COMPILER_PYTHON_PLATFORMS",
                    "aarch64-linux, x86_64-linux,aarch64-linux",
                ),
                ("CLOACINA_TENANT_COMPILER_PYTHON_VERSION", "3.12"),
                (
                    "CLOACINA_TENANT_COMPILER_PYTHON_INDEX_URL",
                    "https://pypi.example.com/simple",
                ),
                ("CLOACINA_TENANT_COMPILER_PYTHON_WHEEL_DIR", "/opt/wheels"),
                ("CLOACINA_TENANT_COMPILER_PYTHON_OFFLINE", "true"),
                ("CLOACINA_TENANT_COMPILER_PYTHON_UV_CACHE_DIR", "/cache/uv"),
                ("CLOACINA_TENANT_COMPILER_PYTHON_CACHE_SIZE", "10Gi"),
                ("CLOACINA_TENANT_COMPILER_HOME_SIZE", "6Gi"),
            ]),
        )
        .unwrap()
        .unwrap();
        assert_eq!(s.python.platforms, vec!["aarch64-linux", "x86_64-linux"]);
        assert_eq!(s.python.version.as_deref(), Some("3.12"));
        assert_eq!(
            s.python.index_url.as_deref(),
            Some("https://pypi.example.com/simple")
        );
        assert_eq!(s.python.wheel_dir.as_deref(), Some("/opt/wheels"));
        assert!(s.python.offline);
        assert_eq!(s.python.uv_cache_dir.as_deref(), Some("/cache/uv"));
        assert_eq!(s.python.cache_size, "10Gi");
        assert_eq!(s.home_size, "6Gi");

        // The defaults.
        let d =
            CompilerSettings::from_lookup(URL, lookup(&[("CLOACINA_TENANT_COMPILERS", "true")]))
                .unwrap()
                .unwrap();
        assert_eq!(
            d.python,
            TenantPythonSettings {
                cache_size: "4Gi".to_string(),
                ..TenantPythonSettings::default()
            }
        );
        assert_eq!(d.home_size, "4Gi");
    }

    #[test]
    fn a_python_value_that_cannot_be_used_is_an_error_that_names_the_setting() {
        for (key, value) in [
            ("CLOACINA_TENANT_COMPILER_PYTHON_PLATFORMS", "riscv64-linux"),
            ("CLOACINA_TENANT_COMPILER_PYTHON_VERSION", "3"),
            ("CLOACINA_TENANT_COMPILER_PYTHON_VERSION", "3.x"),
            ("CLOACINA_TENANT_COMPILER_PYTHON_WHEEL_DIR", "wheels"),
            ("CLOACINA_TENANT_COMPILER_PYTHON_UV_CACHE_DIR", "cache"),
            ("CLOACINA_TENANT_COMPILER_PYTHON_OFFLINE", "sometimes"),
            // Hardened and offline with no wheel folder: the cache is empty
            // at each start.
            ("CLOACINA_TENANT_COMPILER_PYTHON_OFFLINE", "true"),
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
            assert!(err.contains(key), "{key}={value}: {err}");
        }
        // Not hardened, offline with no wheel folder: the image may carry a
        // cache.
        let s = CompilerSettings::from_lookup(
            URL,
            lookup(&[
                ("CLOACINA_TENANT_COMPILERS", "true"),
                ("CLOACINA_TENANT_COMPILER_HARDENED", "false"),
                ("CLOACINA_TENANT_COMPILER_PYTHON_OFFLINE", "true"),
            ]),
        )
        .unwrap()
        .unwrap();
        assert!(s.python.offline);
    }

    /// CLOACINA-T-1117: the compiler image of the repository holds `uv` and a
    /// Python interpreter, which the compiler needs to vendor.
    #[test]
    fn the_compiler_image_holds_uv_and_python() {
        let dockerfile = include_str!("../../../../docker/Dockerfile.compiler");
        let lines: Vec<&str> = dockerfile.lines().map(str::trim).collect();
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("COPY --from=uv /uv /usr/local/bin/uv")),
            "docker/Dockerfile.compiler does not copy uv"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("FROM ghcr.io/astral-sh/uv:")),
            "docker/Dockerfile.compiler has no uv stage"
        );
        assert!(
            lines.contains(&"python3 \\"),
            "docker/Dockerfile.compiler does not install python3"
        );
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

        let secret = database_secret_manifest("cloacina-tenant-acme", URL);
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
        started: Mutex<Vec<CompilerContainer>>,
        stopped: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl CompilerContainerOps for FakeDocker {
        async fn list_compilers(&self, _tenant_id: &str) -> Result<Vec<String>, ActuatorError> {
            Ok(self.running.lock().unwrap().clone())
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
            self.stopped.lock().unwrap().push(id.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn docker_runs_one_container_while_there_is_work() {
        let ops = Arc::new(FakeDocker::default());
        let mut s = settings();
        s.docker_network = Some("cloacina".to_string());
        let c = DockerCompilers::new(ops.clone(), s);

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
            assert!(spec.env.contains(&format!("DATABASE_URL={URL}")));
            // The URL is in the environment, not in the arguments.
            assert!(!spec.args.iter().any(|a| a.contains("s3cret")));
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
        let c = DockerCompilers::new(ops.clone(), settings());
        c.reconcile("acme", CompilerWant::Run).await.unwrap();
        c.remove("acme").await.unwrap();
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
