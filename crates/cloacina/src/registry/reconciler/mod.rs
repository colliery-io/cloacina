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

//! # Registry Reconciler
//!
//! The Registry Reconciler is responsible for synchronizing the persistent workflow registry
//! state with the in-memory task and workflow registries. It ensures that:
//!
//! - Packages registered in the database are loaded into the global registries
//! - Packages removed from the database are unloaded from the global registries
//! - System restarts properly restore all registered packages
//! - Dynamic package loading/unloading works seamlessly
//!
//! ## Key Components
//!
//! - `RegistryReconciler`: Main reconciliation service
//! - `ReconcilerConfig`: Configuration for reconciliation behavior
//! - `ReconcileResult`: Result of a reconciliation operation
//! - `PackageState`: Tracking loaded package state

mod from_document;
mod loading;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::{Interval, interval};
use tracing::{debug, error, info, warn};

use crate::computation_graph::scheduler::ComputationGraphScheduler;
use crate::registry::error::RegistryError;
use crate::registry::loader::package_loader::PackageLoader;
use crate::registry::loader::task_registrar::TaskRegistrar;
use crate::registry::traits::WorkflowRegistry;
use crate::registry::types::{WorkflowMetadata, WorkflowPackageId};
use crate::task::TaskNamespace;

/// Configuration for the Registry Reconciler
#[derive(Debug, Clone)]
pub struct ReconcilerConfig {
    /// How often to run reconciliation
    pub reconcile_interval: Duration,

    /// Whether to perform startup reconciliation
    pub enable_startup_reconciliation: bool,

    /// Maximum time to wait for a single package load/unload operation
    pub package_operation_timeout: Duration,

    /// Whether to continue reconciliation if individual package operations fail
    pub continue_on_package_error: bool,

    /// Default tenant ID to use for package loading
    pub default_tenant_id: String,

    /// When true, the reconciler verifies the stored signature of each
    /// package before it loads it (CLOACI-T-0571, CLOACINA-T-1062): a
    /// `package_signatures` row stored for the tenant's org or the server
    /// org, a live trusted key of one of them, and a signature that
    /// verifies. A package that does not pass is not loaded. The upload
    /// route makes the same check; this one also covers a package that did
    /// not come through it, and a signer revoked since the upload.
    pub require_signatures: bool,

    /// The server-wide organization (CLOACINA-A-0018). At load the signer
    /// is looked up in the org of `default_tenant_id` first
    /// ([`crate::security::tenant_org_id`]), then in this one if it is set.
    /// Mirrors `cloacina-server`'s `--verification-org-id` CLI flag.
    pub verification_org_id: Option<crate::UniversalUuid>,

    /// The database that holds the trusted keys, when it is not the
    /// registry's own (CLOACINA-T-1062). A tenant's runner reads packages
    /// and signature rows from the tenant's schema; the trusted keys are in
    /// the admin schema. `None` reads them from the registry's database.
    pub signature_trust_database: Option<crate::Database>,

    /// How a package is loaded (CLOACINA-T-1075). The default,
    /// [`PackageLoadMode::InProcess`], opens the package's library or
    /// imports its Python module in this process. `cloacina-server` sets
    /// [`PackageLoadMode::MetadataOnly`] unless it runs with single trust.
    pub package_load_mode: PackageLoadMode,
}

impl ReconcilerConfig {
    /// The organizations whose trusted keys count at load, in the order
    /// they are asked: the org of the tenant, then the server org if set.
    pub fn signature_org_ids(&self) -> Vec<crate::UniversalUuid> {
        let tenant_org = crate::security::tenant_org_id(&self.default_tenant_id);
        let mut orgs = vec![tenant_org];
        if let Some(server_org) = self.verification_org_id
            && server_org != tenant_org
        {
            orgs.push(server_org);
        }
        orgs
    }
}

/// How the reconciler loads a package (CLOACINA-T-1075, CLOACINA-A-0019).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PackageLoadMode {
    /// Open the package's library, or import its Python module, in this
    /// process. The package's code runs here: its load-time code, its
    /// triggers, its graph and, with the in-process executor, its tasks. The
    /// mode of the embedded library and of `cloacinactl daemon`.
    #[default]
    InProcess,
    /// Register the package from the metadata document its compiler stored
    /// (CLOACINA-T-1074). No library is opened and no module is imported:
    /// the tasks are entries with no code, which an executor that sends work
    /// to agents runs there. A poll trigger is polled on an agent
    /// (CLOACINA-T-1077). A package with no document does not load; a
    /// package that needs its code in this process (see
    /// [`crate::registry::in_process`]) is refused with the reason.
    MetadataOnly,
}

impl Default for ReconcilerConfig {
    fn default() -> Self {
        Self {
            reconcile_interval: Duration::from_secs(5),
            enable_startup_reconciliation: true,
            package_operation_timeout: Duration::from_secs(30),
            continue_on_package_error: true,
            default_tenant_id: "public".to_string(),
            require_signatures: false,
            verification_org_id: None,
            signature_trust_database: None,
            package_load_mode: PackageLoadMode::InProcess,
        }
    }
}

/// Result of a reconciliation operation
#[derive(Debug, Clone)]
pub struct ReconcileResult {
    /// Packages that were loaded during this reconciliation
    pub packages_loaded: Vec<WorkflowPackageId>,

    /// Packages that were unloaded during this reconciliation
    pub packages_unloaded: Vec<WorkflowPackageId>,

    /// Packages that failed to load/unload
    pub packages_failed: Vec<(WorkflowPackageId, String)>,

    /// Total packages currently tracked
    pub total_packages_tracked: usize,

    /// Duration of the reconciliation operation
    pub reconciliation_duration: Duration,
}

impl ReconcileResult {
    /// Check if the reconciliation had any changes
    pub fn has_changes(&self) -> bool {
        !self.packages_loaded.is_empty() || !self.packages_unloaded.is_empty()
    }

    /// Check if the reconciliation had any failures
    pub fn has_failures(&self) -> bool {
        !self.packages_failed.is_empty()
    }
}

/// Tracks the state of loaded packages
#[derive(Debug, Clone)]
pub(super) struct PackageState {
    /// Package metadata
    pub(super) metadata: WorkflowMetadata,

    /// Task namespaces registered for this package
    pub(super) task_namespaces: Vec<TaskNamespace>,

    /// Workflow name registered for this package
    pub(super) workflow_name: Option<String>,

    /// Trigger names registered for this package
    pub(super) trigger_names: Vec<String>,

    /// Computation graph name loaded for this package (if any)
    pub(super) graph_name: Option<String>,

    /// Reactor names this package owns (declared via `#[reactor]` or
    /// `#[computation_graph]`'s bundled reactor). Used by the reverse-order
    /// unload pipeline (T-0554 Phase 2): the package's own reactors are
    /// torn down via `scheduler.unload_reactor` after subscribers have
    /// been unbound. Cross-package subscribers (graphs that bind to a
    /// reactor owned by another package) do NOT appear here.
    pub(super) reactor_names: Vec<String>,

    /// Schedule IDs created when the reconciler registered this package's
    /// trigger declarations through an attached `CronWorkflowRegistrar` —
    /// both `#[trigger(cron = ...)]` (cron rows) and `#[trigger(poll_interval
    /// = ...)]` (poll rows, CLOACI-I-0124 / WS-6). Empty when no registrar is
    /// attached (e.g. the standalone daemon path registers out-of-band). Used
    /// by `unload_package` to drop the schedules when the package is removed
    /// (both kinds are deleted by id).
    pub(super) cron_schedule_ids: Vec<String>,

    /// Trigger-less graph names registered through the FFI bridge for
    /// this package (T-0553 follow-up — Trigger-less CG FFI bridge).
    /// Populated by `step_load_triggerless_cgs` for cdylib packages;
    /// empty for in-process / Python loads. `unload_package` drops
    /// each name from the runtime.
    pub(super) triggerless_graph_names: Vec<String>,
}

/// Trait the reconciler uses to register and unregister cron workflow
/// schedules at package load/unload time. Implementations live on the
/// runner side (the standalone daemon and the embedded
/// `cloacina-server` runner both implement this against their cron
/// scheduler / DAL). Decoupling this from the reconciler lets cron
/// registration ride the same `reconcile()` lifecycle as every other
/// primitive, instead of relying on the daemon's bespoke post-reconcile
/// hook (which never fired in server mode).
#[async_trait::async_trait]
pub trait CronWorkflowRegistrar: Send + Sync {
    /// Create a cron schedule for a workflow. Returns an opaque
    /// schedule ID the reconciler hands back to
    /// [`unregister_cron_workflow`](Self::unregister_cron_workflow) on unload.
    async fn register_cron_workflow(
        &self,
        workflow_name: &str,
        cron_expression: &str,
        timezone: &str,
    ) -> Result<String, String>;

    /// Create a cron schedule with what its trigger declares for it
    /// (CLOACINA-T-1020): the catch-up policy and the limit on active runs.
    /// The reconciler calls this one.
    /// The default ignores the options, so a registrar written before them
    /// still compiles; `DalCronRegistrar` applies them.
    async fn register_cron_workflow_with_options(
        &self,
        workflow_name: &str,
        cron_expression: &str,
        timezone: &str,
        options: &crate::models::schedule::CronScheduleOptions,
    ) -> Result<String, String> {
        let _ = options;
        self.register_cron_workflow(workflow_name, cron_expression, timezone)
            .await
    }

    /// Drop a cron schedule by the ID returned from
    /// [`register_cron_workflow`](Self::register_cron_workflow).
    async fn unregister_cron_workflow(&self, schedule_id: &str) -> Result<(), String>;

    /// The ids of all schedule rows that exist now, cron and poll.
    /// The reconciler reads them before a load (CLOACINA-T-1057): the
    /// register calls are upserts, so a re-load gets back the id of a row
    /// that already exists, and a failed load must delete only the rows it
    /// made. The default is the empty set, so a registrar written before
    /// this still compiles; with it, a failed load deletes every id the
    /// load got. `DalCronRegistrar` reads the rows.
    async fn existing_schedule_ids(&self) -> Result<std::collections::HashSet<String>, String> {
        Ok(std::collections::HashSet::new())
    }

    /// Create a `trigger`-type schedule row for a custom-poll (non-cron)
    /// trigger. The in-memory `Trigger` impl is registered separately by
    /// the reconciler's runtime step; this persists the row the trigger
    /// scheduler polls (`get_enabled_triggers`) and the read API surfaces
    /// in the Triggers view (CLOACI-I-0124 / WS-6). Returns an opaque
    /// schedule ID dropped via [`unregister_cron_workflow`](Self::unregister_cron_workflow) on unload
    /// (both schedule kinds are deleted by id). The default impl degrades
    /// loudly so a registrar without trigger support fails visibly rather
    /// than silently dropping poll triggers.
    async fn register_poll_trigger(
        &self,
        trigger_name: &str,
        workflow_name: &str,
        poll_interval_ms: i32,
        allow_concurrent: bool,
    ) -> Result<String, String> {
        let _ = (
            trigger_name,
            workflow_name,
            poll_interval_ms,
            allow_concurrent,
        );
        Err("poll-trigger scheduling not supported by this registrar".to_string())
    }
}

/// Status information about the reconciler
#[derive(Debug, Clone)]
pub struct ReconcilerStatus {
    /// Number of packages currently loaded
    pub packages_loaded: usize,

    /// Details about each loaded package
    pub package_details: Vec<PackageStatusDetail>,
}

/// Detailed status information about a loaded package
#[derive(Debug, Clone)]
pub struct PackageStatusDetail {
    /// Package name
    pub package_name: String,

    /// Package version
    pub version: String,

    /// Number of tasks registered
    pub task_count: usize,

    /// Whether a workflow was registered
    pub has_workflow: bool,
}

/// Registry Reconciler for synchronizing database state with in-memory registries
pub struct RegistryReconciler {
    /// Reference to the workflow registry for database operations
    pub(super) registry: Arc<dyn WorkflowRegistry>,

    /// Configuration for reconciliation behavior
    pub(super) config: ReconcilerConfig,

    /// Optional runtime handle. When set, the reconciler pushes
    /// newly-loaded/unloaded registrations through the runtime so executors
    /// looking up tasks/workflows/triggers/CGs/stream backends stay in sync
    /// with dynamic package loads.
    pub(super) runtime: Option<Arc<crate::Runtime>>,

    /// Tracking of currently loaded packages
    pub(super) loaded_packages: Arc<tokio::sync::RwLock<HashMap<WorkflowPackageId, PackageState>>>,

    /// Package loader for extracting metadata from .so files
    pub(super) package_loader: PackageLoader,

    /// Task registrar for managing dynamic task registration
    pub(super) task_registrar: TaskRegistrar,

    /// Shutdown signal receiver
    shutdown_rx: watch::Receiver<bool>,

    /// Reconciliation interval timer
    interval: Interval,

    /// Optional graph scheduler for computation graph packages.
    /// Shared reference so it can be set after construction.
    graph_scheduler: Arc<tokio::sync::RwLock<Option<Arc<ComputationGraphScheduler>>>>,

    /// Optional cron registrar. When attached, the reconciler registers
    /// each `#[trigger(cron = ...)]` declaration in the package at load
    /// time and deregisters at unload. Without it, cron triggers are a
    /// no-op (the standalone daemon historically did this out-of-band;
    /// server mode had no cron registration at all). Closes the gap
    /// where packaged cron triggers never fired under cloacina-server.
    pub(super) cron_registrar: Option<Arc<dyn CronWorkflowRegistrar>>,

    /// CLOACI-T-0835: packages this process already asked the compiler to
    /// rebuild after a stale-artifact load failure. One request per package
    /// per process — if the rebuilt artifact STILL fails (e.g. the compiler
    /// itself is the stale side), we do not ping-pong the row back to
    /// `pending` forever.
    recompile_requested: std::sync::Mutex<std::collections::HashSet<WorkflowPackageId>>,

    /// CLOACINA-T-1027: the check of a package's task placements (agent pool,
    /// CPU and memory requests) at load. `None` loads every package. Taken
    /// from the process-wide registration
    /// ([`crate::registry::task_placement_check`]) when the reconciler is
    /// made; the server registers one at start.
    pub(super) placement_check: Option<Arc<dyn crate::registry::TaskPlacementCheck>>,

    /// CLOACINA-T-1077: where a poll trigger of a package loaded from its
    /// metadata document sends its polls. Shared with the runner, which sets
    /// the dispatcher after it is made.
    pub(super) poll_dispatcher: crate::trigger::PollDispatcherSlot,

    /// CLOACINA-T-1119: the signature each loaded package passed the load
    /// check with, by package. Its hash is what the check after a revoke
    /// reads. Empty with signatures not required.
    pub(super) loaded_signatures:
        tokio::sync::RwLock<HashMap<WorkflowPackageId, crate::registry::traits::LoadSignature>>,

    /// CLOACINA-T-1119: the revoke marker of the last pass whose check of
    /// the loaded packages ran to its end. `None` before the first one.
    revoke_marker: tokio::sync::Mutex<Option<crate::registry::traits::SignerRevokeMarker>>,
}

/// The codes of a check after a revoke that refuse a loaded package
/// (CLOACINA-T-1119): the signer's key is revoked, or no org of the list
/// trusts it any more. Any other answer leaves the package loaded.
const REVOKE_REFUSAL_CODES: [&str; 2] = ["revoked_signer", "untrusted_signer"];

/// The audit action of a package refused after a revoke (CLOACINA-T-1119).
pub const ACTION_PACKAGE_SIGNER_REVOKED: &str = "package.signer_revoked";

impl RegistryReconciler {
    /// Create a new Registry Reconciler
    pub fn new(
        registry: Arc<dyn WorkflowRegistry>,
        config: ReconcilerConfig,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Result<Self, RegistryError> {
        let interval = interval(config.reconcile_interval);

        let package_loader = PackageLoader::new().map_err(RegistryError::Loader)?;
        let shared_cache = package_loader.handle_cache();

        let task_registrar =
            TaskRegistrar::with_handle_cache(shared_cache).map_err(RegistryError::Loader)?;

        Ok(Self {
            registry,
            config,
            runtime: None,
            loaded_packages: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            package_loader,
            task_registrar,
            shutdown_rx,
            interval,
            graph_scheduler: Arc::new(tokio::sync::RwLock::new(None)),
            cron_registrar: None,
            recompile_requested: std::sync::Mutex::new(std::collections::HashSet::new()),
            placement_check: crate::registry::task_placement_check(),
            poll_dispatcher: crate::trigger::empty_poll_dispatcher_slot(),
            loaded_signatures: tokio::sync::RwLock::new(HashMap::new()),
            revoke_marker: tokio::sync::Mutex::new(None),
        })
    }

    /// Share the runner's poll dispatcher slot (CLOACINA-T-1077), so a
    /// dispatcher set on the runner later reaches the poll triggers this
    /// reconciler registers.
    pub fn set_poll_dispatcher_slot(&mut self, slot: crate::trigger::PollDispatcherSlot) {
        self.poll_dispatcher = slot;
    }

    /// Set the check of a package's task placements at load
    /// (CLOACINA-T-1027), in place of the process-wide registration.
    pub fn with_placement_check(
        mut self,
        check: Arc<dyn crate::registry::TaskPlacementCheck>,
    ) -> Self {
        self.placement_check = Some(check);
        self
    }

    /// The tenant scope this reconciler loads and unloads under
    /// (CLOACI-T-0924).
    ///
    /// The `ComputationGraphScheduler` is a single `Arc` shared by every
    /// tenant's runner, so unlike the `Runtime` (whose handle carries a scope)
    /// its name-taking methods need the tenant passed explicitly. Package loads
    /// already stamp `decl.tenant_id = config.default_tenant_id`, so this is
    /// exactly the scope the entries were claimed under. Embedded runners leave
    /// `default_tenant_id` at `"public"` and are self-consistent.
    pub(super) fn tenant_scope(&self) -> crate::tenant_scope::TenantScope<'_> {
        crate::tenant_scope::TenantScope::tenant(&self.config.default_tenant_id)
    }

    /// Attach a Runtime to this reconciler. Package load/unload operations
    /// will push registrations through the runtime so executors see the
    /// same view as the reconciler.
    pub fn with_runtime(mut self, runtime: Arc<crate::Runtime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Set the graph scheduler for computation graph package routing.
    pub fn with_graph_scheduler(self, scheduler: Arc<ComputationGraphScheduler>) -> Self {
        // Use try_write since this is called during initialization (not async)
        if let Ok(mut lock) = self.graph_scheduler.try_write() {
            *lock = Some(scheduler);
        }
        self
    }

    /// Replace the graph scheduler slot with a shared reference from the runner.
    /// This allows the runner to inject the scheduler after construction.
    pub fn set_graph_scheduler_slot(
        &mut self,
        slot: Arc<tokio::sync::RwLock<Option<Arc<ComputationGraphScheduler>>>>,
    ) {
        self.graph_scheduler = slot;
    }

    /// Attach a cron registrar that the reconciler will use to install
    /// cron schedules for each `#[trigger(cron = ...)]` declaration in
    /// loaded packages, and to drop them on unload. Builder-style
    /// counterpart for callers that wire the reconciler in one chained
    /// expression.
    pub fn with_cron_registrar(mut self, registrar: Arc<dyn CronWorkflowRegistrar>) -> Self {
        self.cron_registrar = Some(registrar);
        self
    }

    /// Inject a cron registrar after construction (mirrors
    /// `set_graph_scheduler_slot`). Used by `DefaultRunner` setup
    /// because the runner needs to construct the reconciler before it
    /// can build the cron registrar (the registrar holds runner-owned
    /// resources).
    pub fn set_cron_registrar(&mut self, registrar: Arc<dyn CronWorkflowRegistrar>) {
        self.cron_registrar = Some(registrar);
    }

    /// Start the background reconciliation loop
    pub async fn start_reconciliation_loop(mut self) -> Result<(), RegistryError> {
        info!(
            "Starting Registry Reconciler with interval {:?}",
            self.config.reconcile_interval
        );

        // Perform startup reconciliation if enabled
        if self.config.enable_startup_reconciliation {
            info!("Performing startup reconciliation");
            match self.reconcile().await {
                Ok(result) => {
                    info!(
                        "Startup reconciliation completed: {} loaded, {} unloaded, {} failed",
                        result.packages_loaded.len(),
                        result.packages_unloaded.len(),
                        result.packages_failed.len()
                    );
                }
                Err(e) => {
                    error!("Startup reconciliation failed: {}", e);
                    if !self.config.continue_on_package_error {
                        return Err(e);
                    }
                }
            }
        }

        // Main reconciliation loop
        loop {
            tokio::select! {
                _ = self.interval.tick() => {
                    debug!("Running periodic reconciliation");
                    match self.reconcile().await {
                        Ok(result) => {
                            if result.has_changes() {
                                info!(
                                    "Reconciliation completed: {} loaded, {} unloaded",
                                    result.packages_loaded.len(),
                                    result.packages_unloaded.len()
                                );
                            } else {
                                debug!("Reconciliation completed with no changes");
                            }

                            if result.has_failures() {
                                warn!("Reconciliation had {} failures", result.packages_failed.len());
                                for (package_id, error) in &result.packages_failed {
                                    warn!("Package {} failed: {}", package_id, error);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Reconciliation failed: {}", e);
                            if !self.config.continue_on_package_error {
                                return Err(e);
                            }
                        }
                    }
                }
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        info!("Registry Reconciler shutdown requested");
                        break;
                    }
                }
            }
        }

        // Perform cleanup on shutdown
        info!("Registry Reconciler shutting down");
        self.shutdown_cleanup().await?;

        Ok(())
    }

    /// Perform a single reconciliation operation
    pub async fn reconcile(&self) -> Result<ReconcileResult, RegistryError> {
        let start_time = std::time::Instant::now();

        // CLOACINA-T-1119: a key revoked since the last pass. Before the
        // list, so a package refused here is not in it.
        let refused_after_revoke = self.recheck_after_revoke().await;

        // Get all packages from the database
        let db_packages = self.registry.list_workflows().await?;
        let db_package_ids: HashSet<WorkflowPackageId> = db_packages.iter().map(|p| p.id).collect();

        // Get currently loaded packages
        let loaded_packages = self.loaded_packages.read().await;
        let loaded_package_ids: HashSet<WorkflowPackageId> =
            loaded_packages.keys().cloned().collect();
        drop(loaded_packages);

        // Determine what needs to be loaded and unloaded.
        //
        // T-0553 follow-up: sort `packages_to_load` by registration
        // timestamp (`created_at`) so cross-package binding order is
        // deterministic. The HashSet difference produces an arbitrary
        // iteration order, which broke cross-package fan-out (subscriber
        // loading before publisher → "no such reactor is loaded"). For
        // unloads, sort REVERSE by created_at so dependents tear down
        // before publishers — this complements the per-package reverse
        // step pipeline (workflows → CGs → reactors → triggers → tasks)
        // by also reversing across packages.
        let mut packages_to_load: Vec<_> = db_package_ids
            .difference(&loaded_package_ids)
            .cloned()
            .collect();
        packages_to_load.sort_by_key(|id| {
            db_packages
                .iter()
                .find(|p| p.id == *id)
                .map(|p| p.created_at)
                .unwrap_or_else(chrono::Utc::now)
        });

        let mut packages_to_unload: Vec<_> = loaded_package_ids
            .difference(&db_package_ids)
            .cloned()
            .collect();
        // Best-effort reverse-creation-order unload using whatever metadata
        // the loaded_packages map still holds; fall back to package_id as
        // a stable tiebreaker.
        {
            let snapshot = self.loaded_packages.read().await;
            packages_to_unload.sort_by(|a, b| {
                let a_t = snapshot.get(a).map(|s| s.metadata.created_at);
                let b_t = snapshot.get(b).map(|s| s.metadata.created_at);
                b_t.cmp(&a_t).then_with(|| b.cmp(a))
            });
        }

        debug!(
            "Reconciliation: {} packages to load, {} to unload",
            packages_to_load.len(),
            packages_to_unload.len()
        );

        let mut result = ReconcileResult {
            packages_loaded: Vec::new(),
            packages_unloaded: refused_after_revoke,
            packages_failed: Vec::new(),
            total_packages_tracked: 0,
            reconciliation_duration: Duration::ZERO,
        };

        // Unload packages that are no longer in the database
        for package_id in packages_to_unload {
            match self.unload_package(package_id).await {
                Ok(()) => {
                    result.packages_unloaded.push(package_id);
                    info!("Unloaded package: {}", package_id);
                }
                Err(e) => {
                    let error_msg = format!("Failed to unload package {}: {}", package_id, e);
                    error!("{}", error_msg);
                    result.packages_failed.push((package_id, error_msg));

                    if !self.config.continue_on_package_error {
                        return Err(e);
                    }
                }
            }
        }

        // Load packages that are new in the database
        info!(
            "Reconciler: {} package(s) to load: {:?}",
            packages_to_load.len(),
            packages_to_load
        );
        for (pkg_idx, package_id) in packages_to_load.iter().enumerate() {
            info!(
                "Reconciler: starting package {}/{} (id={})",
                pkg_idx + 1,
                packages_to_load.len(),
                package_id
            );
            // Find the package metadata in db_packages
            if let Some(package_metadata) = db_packages.iter().find(|p| p.id == *package_id) {
                info!(
                    "Reconciler: loading {} v{} (id={})",
                    package_metadata.package_name, package_metadata.version, package_id
                );
                match self.load_package(package_metadata.clone()).await {
                    Ok(()) => {
                        result.packages_loaded.push(*package_id);
                        info!(
                            "Loaded package: {} v{}",
                            package_metadata.package_name, package_metadata.version
                        );
                    }
                    Err(e) => {
                        let error_msg = format!(
                            "Failed to load package {} ({}:{}): {}",
                            package_id, package_metadata.package_name, package_metadata.version, e
                        );
                        error!("{}", error_msg);

                        // CLOACI-T-0835: a STALE-ARTIFACT failure (the cdylib
                        // was built against an older plugin ABI / interface
                        // version than this host expects) is fixable by a
                        // rebuild from retained source — signal the compiler
                        // by flipping the row back to `pending`. Once per
                        // package per process, so a rebuild that comes back
                        // still-stale doesn't ping-pong forever.
                        if is_stale_artifact_error(&error_msg)
                            && self.recompile_requested.lock().unwrap().insert(*package_id)
                        {
                            match self.registry.request_recompile(*package_id).await {
                                Ok(true) => warn!(
                                    "Package {} v{} has a STALE compiled artifact (ABI/interface \
                                     mismatch) — requested a recompile from retained source; it \
                                     will reload once the compiler rebuilds it",
                                    package_metadata.package_name, package_metadata.version
                                ),
                                Ok(false) => debug!(
                                    "Stale artifact for {} v{}, but this registry cannot \
                                     schedule recompiles",
                                    package_metadata.package_name, package_metadata.version
                                ),
                                Err(req_err) => warn!(
                                    "Failed to request recompile for {} v{}: {}",
                                    package_metadata.package_name,
                                    package_metadata.version,
                                    req_err
                                ),
                            }
                        }

                        result.packages_failed.push((*package_id, error_msg));

                        if !self.config.continue_on_package_error {
                            return Err(e);
                        }
                    }
                }
            } else {
                let error_msg = format!("Package {} not found in database during load", package_id);
                error!("{}", error_msg);
                result.packages_failed.push((*package_id, error_msg));
            }
        }

        // Update total packages tracked
        let loaded_packages = self.loaded_packages.read().await;
        result.total_packages_tracked = loaded_packages.len();
        drop(loaded_packages);

        result.reconciliation_duration = start_time.elapsed();

        Ok(result)
    }

    /// Check the loaded packages again when a trusted key was revoked
    /// (CLOACINA-T-1119). Returns the packages refused and unloaded.
    ///
    /// With signatures required, each pass reads the revoke marker of the
    /// tenant's org and the server org. When it is not the one of the last
    /// full check (and on the first pass), each loaded package is checked
    /// again by the hash its load check computed. A package refused with
    /// [`REVOKE_REFUSAL_CODES`] is refused for good by
    /// [`Self::refuse_after_revoke`]. A check that cannot run refuses nothing,
    /// and the marker is kept unread, so the next pass checks again.
    async fn recheck_after_revoke(&self) -> Vec<WorkflowPackageId> {
        if !self.config.require_signatures {
            return Vec::new();
        }
        let org_ids = self.config.signature_org_ids();
        let trust = self.config.signature_trust_database.as_ref();
        let marker = match self.registry.signer_revoke_marker(&org_ids, trust).await {
            Ok(Some(marker)) => marker,
            Ok(None) => return Vec::new(),
            Err(e) => {
                warn!(
                    tenant_id = %self.config.default_tenant_id,
                    "could not read the revoke marker of the trusted keys; the loaded \
                     packages are checked at the next pass: {e}"
                );
                return Vec::new();
            }
        };
        let mut last = self.revoke_marker.lock().await;
        if last.as_ref() == Some(&marker) {
            return Vec::new();
        }

        let to_check: Vec<(
            WorkflowPackageId,
            WorkflowMetadata,
            Option<String>,
            crate::registry::traits::LoadSignature,
        )> = {
            let signatures = self.loaded_signatures.read().await;
            let loaded = self.loaded_packages.read().await;
            loaded
                .iter()
                .filter_map(|(id, state)| {
                    signatures.get(id).map(|sig| {
                        (
                            *id,
                            state.metadata.clone(),
                            state.workflow_name.clone(),
                            sig.clone(),
                        )
                    })
                })
                .collect()
        };

        let mut refused = Vec::new();
        let mut complete = true;
        for (id, metadata, workflow_name, signature) in to_check {
            match self
                .registry
                .recheck_package_signature(&signature.package_hash, &org_ids, trust)
                .await
            {
                Ok(Ok(_)) => {}
                Ok(Err(refusal)) if REVOKE_REFUSAL_CODES.contains(&refusal.code) => {
                    self.refuse_after_revoke(
                        id,
                        &metadata,
                        workflow_name.as_deref(),
                        &signature,
                        &refusal,
                    )
                    .await;
                    refused.push(id);
                }
                Ok(Err(refusal)) => warn!(
                    package_name = %metadata.package_name,
                    package_version = %metadata.version,
                    code = refusal.code,
                    "the check after a revoke did not pass, with a code that does not \
                     refuse a loaded package; it stays loaded: {}",
                    refusal.reason
                ),
                Err(e) => {
                    complete = false;
                    warn!(
                        package_name = %metadata.package_name,
                        package_version = %metadata.version,
                        "the check after a revoke could not run; it runs again at the \
                         next pass: {e}"
                    );
                }
            }
        }
        if complete {
            *last = Some(marker);
        }
        refused
    }

    /// Refuse a loaded package whose signer is no longer trusted
    /// (CLOACINA-T-1119). In order: the row goes from `success` to `failed`
    /// with the reason (so no replica lists it, and agents no longer fetch
    /// its code); the runs of the version that have not ended are cancelled;
    /// the package is unloaded here, with its schedules. The replica whose
    /// update changed the row writes the warn line and the audit row, so there
    /// is one of each; the others unload and cancel only.
    async fn refuse_after_revoke(
        &self,
        id: WorkflowPackageId,
        metadata: &WorkflowMetadata,
        workflow_name: Option<&str>,
        signature: &crate::registry::traits::LoadSignature,
        refusal: &crate::registry::traits::LoadSignatureRefusal,
    ) {
        let fingerprint = signature.signer_fingerprint.as_str();
        let reason = format!(
            "package {} v{} is refused: the key {} that signed it is no longer trusted ({}): \
             {}. Its runs that had not ended are cancelled and its schedules removed; \
             publish a new version signed by a live trusted key",
            metadata.package_name, metadata.version, fingerprint, refusal.code, refusal.reason
        );
        let changed = match self.registry.refuse_package(id, &reason).await {
            Ok(changed) => changed,
            Err(e) => {
                warn!(
                    package_name = %metadata.package_name,
                    package_version = %metadata.version,
                    "could not record the refusal of the package after a revoke; it is \
                     unloaded here: {e}"
                );
                false
            }
        };

        let cancel_reason = format!(
            "cancelled: the key {} that signed package {} v{} is no longer trusted ({})",
            fingerprint, metadata.package_name, metadata.version, refusal.code
        );
        let cancelled = match self
            .registry
            .cancel_package_runs(
                &metadata.package_name,
                &metadata.version,
                workflow_name,
                &cancel_reason,
            )
            .await
        {
            Ok(n) => n,
            Err(e) => {
                warn!(
                    package_name = %metadata.package_name,
                    package_version = %metadata.version,
                    "could not cancel the runs of the package after a revoke: {e}"
                );
                0
            }
        };

        if let Err(e) = self.unload_package(id).await {
            warn!(
                package_name = %metadata.package_name,
                package_version = %metadata.version,
                "the unload after a revoke did not finish; it is tried again at the next \
                 pass: {e}"
            );
        }

        if changed {
            warn!(
                package_name = %metadata.package_name,
                package_version = %metadata.version,
                tenant_id = %self.config.default_tenant_id,
                fingerprint = %fingerprint,
                code = refusal.code,
                cancelled_runs = cancelled,
                "{}",
                reason
            );
            crate::security::audit::log_package_load_failure(
                signature.trusted_by,
                &format!("{} v{}", metadata.package_name, metadata.version),
                &reason,
                refusal.code,
            );
            let event = crate::dal::unified::audit_events::NewAuditEvent {
                tenant_id: Some(self.config.default_tenant_id.clone()),
                actor_key_id: "server".to_string(),
                actor_key_name: "server".to_string(),
                actor_identity: None,
                action: ACTION_PACKAGE_SIGNER_REVOKED.to_string(),
                target_kind: "package".to_string(),
                target: Some(id.to_string()),
                outcome: crate::dal::unified::audit_events::OUTCOME_DENIED.to_string(),
                detail: serde_json::json!({
                    "package": metadata.package_name,
                    "version": metadata.version,
                    "fingerprint": fingerprint,
                    "code": refusal.code,
                    "cancelled_runs": cancelled,
                }),
            };
            if let Err(e) = self
                .registry
                .write_audit_event(event, self.config.signature_trust_database.as_ref())
                .await
            {
                warn!(
                    package_name = %metadata.package_name,
                    "could not write the audit row of the refusal after a revoke: {e}"
                );
            }
        } else {
            info!(
                package_name = %metadata.package_name,
                package_version = %metadata.version,
                fingerprint = %fingerprint,
                cancelled_runs = cancelled,
                "package unloaded after a revoke; the refusal was recorded by another replica"
            );
        }
    }

    /// Perform cleanup operations during shutdown
    async fn shutdown_cleanup(&self) -> Result<(), RegistryError> {
        info!("Performing Registry Reconciler shutdown cleanup");

        // Optionally unload all packages during shutdown
        // For now, we'll just log the current state
        let loaded_packages = self.loaded_packages.read().await;
        if !loaded_packages.is_empty() {
            info!(
                "Shutdown with {} packages still loaded",
                loaded_packages.len()
            );
            for (package_id, state) in loaded_packages.iter() {
                debug!(
                    "Loaded package on shutdown: {} - {} v{}",
                    package_id, state.metadata.package_name, state.metadata.version
                );
            }
        }

        Ok(())
    }

    /// Get the current reconciliation status
    pub async fn get_status(&self) -> ReconcilerStatus {
        let loaded_packages = self.loaded_packages.read().await;

        ReconcilerStatus {
            packages_loaded: loaded_packages.len(),
            package_details: loaded_packages
                .values()
                .map(|state| PackageStatusDetail {
                    package_name: state.metadata.package_name.clone(),
                    version: state.metadata.version.clone(),
                    task_count: state.task_namespaces.len(),
                    has_workflow: state.workflow_name.is_some(),
                })
                .collect(),
        }
    }
}

/// CLOACI-T-0835: does this load failure mean the compiled artifact is STALE
/// (built against an older plugin ABI / interface version than this host
/// expects) — i.e. a rebuild from source would fix it? Matches the two gates a
/// version bump trips: fidius's ABI check ("incompatible ABI version") and its
/// per-interface hash check ("interface hash mismatch"). Message-based because
/// the typed fidius error is stringly-flattened by the loader layers; both
/// strings are fidius-host error display texts, stable within a pinned fidius.
fn is_stale_artifact_error(message: &str) -> bool {
    message.contains("incompatible ABI version") || message.contains("interface hash mismatch")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_artifact_classifier_matches_abi_and_interface_hash() {
        assert!(is_stale_artifact_error(
            "Failed to load library at /tmp/x.so: incompatible ABI version: got 200, expected 500"
        ));
        assert!(is_stale_artifact_error(
            "interface hash mismatch: got 0xdead, expected 0xbeef"
        ));
        // Ordinary failures must NOT trigger a rebuild.
        assert!(!is_stale_artifact_error("No space left on device"));
        assert!(!is_stale_artifact_error(
            "declares 1 constructor node(s) but has NO bundled providers"
        ));
    }
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn test_reconciler_config_default() {
        let config = ReconcilerConfig::default();
        assert_eq!(config.reconcile_interval, Duration::from_secs(5));
        assert!(config.enable_startup_reconciliation);
        assert_eq!(config.package_operation_timeout, Duration::from_secs(30));
        assert!(config.continue_on_package_error);
        assert_eq!(config.default_tenant_id, "public");
    }

    #[test]
    fn test_reconcile_result_methods() {
        let result = ReconcileResult {
            packages_loaded: vec![Uuid::new_v4()],
            packages_unloaded: vec![],
            packages_failed: vec![],
            total_packages_tracked: 1,
            reconciliation_duration: Duration::from_millis(100),
        };

        assert!(result.has_changes());
        assert!(!result.has_failures());

        let result_no_changes = ReconcileResult {
            packages_loaded: vec![],
            packages_unloaded: vec![],
            packages_failed: vec![(Uuid::new_v4(), "error".to_string())],
            total_packages_tracked: 0,
            reconciliation_duration: Duration::from_millis(50),
        };

        assert!(!result_no_changes.has_changes());
        assert!(result_no_changes.has_failures());
    }

    #[test]
    fn test_reconciler_status() {
        let status = ReconcilerStatus {
            packages_loaded: 2,
            package_details: vec![
                PackageStatusDetail {
                    package_name: "pkg1".to_string(),
                    version: "1.0.0".to_string(),
                    task_count: 3,
                    has_workflow: true,
                },
                PackageStatusDetail {
                    package_name: "pkg2".to_string(),
                    version: "2.0.0".to_string(),
                    task_count: 1,
                    has_workflow: false,
                },
            ],
        };

        assert_eq!(status.packages_loaded, 2);
        assert_eq!(status.package_details.len(), 2);
        assert_eq!(status.package_details[0].package_name, "pkg1");
        assert!(status.package_details[0].has_workflow);
        assert!(!status.package_details[1].has_workflow);
    }

    #[test]
    fn test_reconciler_config_custom_values() {
        let config = ReconcilerConfig {
            reconcile_interval: Duration::from_secs(60),
            enable_startup_reconciliation: false,
            package_operation_timeout: Duration::from_secs(120),
            continue_on_package_error: false,
            default_tenant_id: "tenant-42".to_string(),
            require_signatures: false,
            verification_org_id: None,
            signature_trust_database: None,
            package_load_mode: PackageLoadMode::MetadataOnly,
        };

        // CLOACINA-T-1062: the tenant's org is asked first, then the server
        // org if one is set; the same org is not asked twice.
        let tenant_org = crate::security::tenant_org_id("tenant-42");
        assert_eq!(config.signature_org_ids(), vec![tenant_org]);
        let server_org = crate::UniversalUuid::new_v4();
        let with_server_org = ReconcilerConfig {
            verification_org_id: Some(server_org),
            ..config.clone()
        };
        assert_eq!(
            with_server_org.signature_org_ids(),
            vec![tenant_org, server_org]
        );
        let same_org = ReconcilerConfig {
            verification_org_id: Some(tenant_org),
            ..config.clone()
        };
        assert_eq!(same_org.signature_org_ids(), vec![tenant_org]);

        assert_eq!(config.reconcile_interval, Duration::from_secs(60));
        assert!(!config.enable_startup_reconciliation);
        assert_eq!(config.package_operation_timeout, Duration::from_secs(120));
        assert!(!config.continue_on_package_error);
        assert_eq!(config.default_tenant_id, "tenant-42");
    }

    #[test]
    fn test_reconcile_result_no_changes_no_failures() {
        let result = ReconcileResult {
            packages_loaded: vec![],
            packages_unloaded: vec![],
            packages_failed: vec![],
            total_packages_tracked: 5,
            reconciliation_duration: Duration::from_millis(10),
        };

        assert!(!result.has_changes());
        assert!(!result.has_failures());
        assert_eq!(result.total_packages_tracked, 5);
    }

    #[test]
    fn test_reconcile_result_unloaded_counts_as_change() {
        let result = ReconcileResult {
            packages_loaded: vec![],
            packages_unloaded: vec![Uuid::new_v4()],
            packages_failed: vec![],
            total_packages_tracked: 0,
            reconciliation_duration: Duration::from_millis(20),
        };

        assert!(result.has_changes());
        assert!(!result.has_failures());
    }

    #[test]
    fn test_reconcile_result_both_loaded_and_unloaded() {
        let result = ReconcileResult {
            packages_loaded: vec![Uuid::new_v4(), Uuid::new_v4()],
            packages_unloaded: vec![Uuid::new_v4()],
            packages_failed: vec![(Uuid::new_v4(), "timeout".to_string())],
            total_packages_tracked: 3,
            reconciliation_duration: Duration::from_secs(2),
        };

        assert!(result.has_changes());
        assert!(result.has_failures());
        assert_eq!(result.packages_loaded.len(), 2);
        assert_eq!(result.packages_unloaded.len(), 1);
        assert_eq!(result.packages_failed.len(), 1);
    }

    #[test]
    fn test_package_status_detail_fields() {
        let detail = PackageStatusDetail {
            package_name: "my-workflow".to_string(),
            version: "2.3.1".to_string(),
            task_count: 7,
            has_workflow: true,
        };

        assert_eq!(detail.package_name, "my-workflow");
        assert_eq!(detail.version, "2.3.1");
        assert_eq!(detail.task_count, 7);
        assert!(detail.has_workflow);
    }

    #[test]
    fn test_reconciler_status_empty() {
        let status = ReconcilerStatus {
            packages_loaded: 0,
            package_details: vec![],
        };

        assert_eq!(status.packages_loaded, 0);
        assert!(status.package_details.is_empty());
    }

    #[test]
    fn test_reconciler_config_clone() {
        let config = ReconcilerConfig::default();
        let cloned = config.clone();
        assert_eq!(config.reconcile_interval, cloned.reconcile_interval);
        assert_eq!(
            config.enable_startup_reconciliation,
            cloned.enable_startup_reconciliation
        );
        assert_eq!(config.default_tenant_id, cloned.default_tenant_id);
    }

    #[test]
    fn test_reconcile_result_clone() {
        let id = Uuid::new_v4();
        let result = ReconcileResult {
            packages_loaded: vec![id],
            packages_unloaded: vec![],
            packages_failed: vec![],
            total_packages_tracked: 1,
            reconciliation_duration: Duration::from_millis(50),
        };

        let cloned = result.clone();
        assert_eq!(cloned.packages_loaded, vec![id]);
        assert_eq!(cloned.reconciliation_duration, Duration::from_millis(50));
    }
}
