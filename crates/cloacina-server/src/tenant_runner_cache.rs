/*
 *  Copyright 2026 Colliery Software
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

/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 */

//! The live `DefaultRunner` of each tenant. CLOACI-T-0580, CLOACINA-T-1031.
//!
//! Each entry is a fully-constructed runner with its own scheduler
//! loop, executor pool, and heartbeat — bound to the tenant's `Database`
//! from `TenantDatabaseCache` so workflow execution writes land in the
//! correct tenant schema.
//!
//! A tenant's runner is that tenant's engine: its reconciler loads the
//! tenant's packages and registers their schedules, and its schedulers fire
//! them and move its executions on. With no runner, nothing of the tenant
//! runs. So every tenant has a runner from server start until the tenant is
//! deleted (CLOACINA-T-1031):
//!
//! - [`spawn`] starts a loop that lists the tenant schemas at start and then
//!   every [`RUNNER_PASS_INTERVAL`], and makes a runner for each tenant that
//!   has none. That covers a restart and a tenant made through another
//!   replica. It also stops the runner of a schema that is gone.
//! - The tenant create route and the package upload route call
//!   [`ensure_runner`], so a new tenant and a first publish do not wait for
//!   the loop.
//! - A runner is never stopped to make room. Stopping it stops the tenant's
//!   schedules and its running work, and closes the connection pool that the
//!   tenant's routes use. The size the cache is built with is a warning
//!   limit only.
//!
//! `Runtime` inventory (`TaskRegistry`, `WorkflowRegistry`,
//! `TriggerRegistry`, `ReactorRegistry`, `GraphRegistry`) is shared by
//! `Arc` across all per-tenant runners — they're inventory-seeded at
//! process start (post-T-0506) and have no per-tenant state.
//!
//! A runner leaves the cache when its tenant is deleted, when its schema is
//! gone, and at server shutdown. Its `shutdown()` is awaited so background
//! tasks join cleanly.

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use cloacina::Database;
use cloacina::Runtime;
use cloacina::computation_graph::scheduler::ComputationGraphScheduler;
use cloacina::{DefaultRunner, DefaultRunnerConfig};

/// Outcome of a bounded-drain eviction. CLOACI-T-0581.
#[derive(Debug, Clone)]
pub enum EvictOutcome {
    /// No runner was cached for this tenant.
    Missing,
    /// Runner drained cleanly within the timeout.
    Drained,
    /// Runner shutdown returned an error (still removed from cache).
    ShutdownError(String),
    /// Drain exceeded the timeout (runner removed; shutdown continues
    /// unawaited in the background).
    Timeout,
}

impl EvictOutcome {
    /// `true` if a runner existed for this tenant (drained, errored,
    /// or timed out). `false` only for `Missing`.
    pub fn was_present(&self) -> bool {
        !matches!(self, EvictOutcome::Missing)
    }
}

/// Hook invoked on each freshly-built per-tenant runner to register the fleet
/// executor on its dispatcher (CLOACI-I-0114 / T-0634). Per-tenant runners each
/// own a dispatcher bound to the tenant's schema, so the fleet executor — like
/// the routing config — must be installed per runner, not just on the global
/// `state.runner` (which is bound to the admin schema and never executes tenant
/// tasks). The closure builds a tenant-scoped `FleetExecutor` (per-tenant DAL +
/// result handler; shared roster/coordinator/wake/runtime) and registers it
/// under the "fleet" key.
pub type FleetRegistrar = Arc<dyn Fn(&DefaultRunner) + Send + Sync>;

/// How often the server looks for tenants with no runner (CLOACINA-T-1031).
/// A tenant made through another replica gets its runner on this replica
/// within this time.
pub const RUNNER_PASS_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

/// The live per-tenant `DefaultRunner` instances, one for each tenant.
pub struct TenantRunnerCache {
    cache: Mutex<HashMap<String, Arc<DefaultRunner>>>,
    /// Held while a runner is built, so two callers cannot build two runners
    /// for one tenant. A runner that lost such a race could not be shut down:
    /// its `shutdown()` closes the pool it shares with the winner.
    build_lock: Mutex<()>,
    /// The server warns when it holds more runners than this
    /// (`--tenant-runner-cache-size`). It stops none of them.
    warn_above: NonZeroUsize,
    /// Shared `Runtime` for every per-tenant runner — inventory-seeded
    /// once at process start, never mutated.
    shared_runtime: Arc<Runtime>,
    /// Base runner config; cloned for each new tenant runner.
    base_config: DefaultRunnerConfig,
    /// CLOACI-T-0634: optional fleet-executor registrar; invoked on every
    /// per-tenant runner at construction so fleet routing actually fires for
    /// tenant workflows.
    fleet_registrar: Option<FleetRegistrar>,
    /// CLOACI-T-0581 follow-up: optional shared `ComputationGraphScheduler`
    /// installed on every per-tenant runner via `set_graph_scheduler` so
    /// the tenant's reconciler can route packaged CGs into it. The
    /// scheduler itself stores `tenant_id` per graph (T-0579), so
    /// cross-tenant filtering at the health-endpoint layer still works
    /// even with a shared scheduler.
    graph_scheduler: Option<Arc<ComputationGraphScheduler>>,
}

impl TenantRunnerCache {
    /// Build a new cache. `warn_above` is the number of runners above which
    /// the server logs a warning; no runner is stopped to stay under it. The
    /// shared runtime is constructed from inventory at this call site;
    /// callers shouldn't construct it separately.
    pub fn new(warn_above: NonZeroUsize, base_config: DefaultRunnerConfig) -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            build_lock: Mutex::new(()),
            warn_above,
            shared_runtime: Arc::new(Runtime::new()),
            base_config,
            graph_scheduler: None,
            fleet_registrar: None,
        }
    }

    /// CLOACI-T-0634: install a fleet-executor registrar. Every per-tenant
    /// runner constructed after this call will have the fleet executor
    /// registered on its dispatcher (which requires the `base_config` to carry
    /// a routing config so a dispatcher exists). Without this, tasks routed to
    /// "fleet" silently fall back to the per-tenant thread executor.
    pub fn with_fleet_registrar(mut self, registrar: FleetRegistrar) -> Self {
        self.fleet_registrar = Some(registrar);
        self
    }

    /// CLOACI-T-0581 follow-up: install a shared graph scheduler. Every
    /// per-tenant runner constructed after this call will have the
    /// scheduler wired via `DefaultRunner::set_graph_scheduler` so the
    /// reconciler can route CG packages. Idempotent — calling again
    /// replaces the scheduler reference, but already-constructed runners
    /// retain whatever they were given at construction time.
    pub fn with_graph_scheduler(mut self, scheduler: Arc<ComputationGraphScheduler>) -> Self {
        self.graph_scheduler = Some(scheduler);
        self
    }

    /// Get the shared `Runtime` so callers can install graph schedulers,
    /// register triggers, etc.
    pub fn shared_runtime(&self) -> Arc<Runtime> {
        self.shared_runtime.clone()
    }

    /// Look up (or construct) the runner for `tenant_id`, bound to
    /// `tenant_database`. The caller is responsible for resolving the
    /// `Database` via `TenantDatabaseCache` (or equivalent) before
    /// calling — this cache only owns runner lifecycle.
    ///
    /// On cache miss: construct a new `DefaultRunner` via
    /// `DefaultRunner::with_database`, sharing the cache's `Runtime`. One
    /// runner is built at a time. No runner is stopped to make room
    /// (CLOACINA-T-1031); above the warning limit the server logs a warning.
    pub async fn get_or_create(
        &self,
        tenant_id: &str,
        tenant_database: Database,
    ) -> Result<Arc<DefaultRunner>, cloacina::WorkflowExecutionError> {
        // Fast path: cached?
        if let Some(runner) = self.cache.lock().await.get(tenant_id) {
            return Ok(runner.clone());
        }

        // Slow path: build a new runner with the shared Runtime. One build at
        // a time; a caller that waited here finds the runner the other made.
        let _build = self.build_lock.lock().await;
        if let Some(runner) = self.cache.lock().await.get(tenant_id) {
            return Ok(runner.clone());
        }
        info!(
            tenant_id = %tenant_id,
            "constructing per-tenant DefaultRunner (CLOACI-T-0580)"
        );
        // CLOACI-T-0781: stamp the tenant on this runner's config so its
        // reconciler namespaces tasks `tenant::pkg::wf::task` — that's what the
        // fleet executor routes on, so the tenant's tasks reach its agents.
        let mut config = self.base_config.clone();
        config.set_tenant_id(tenant_id);
        // CLOACI-T-0890: thread the tenant-scoped secret resolver onto this
        // runner's in-process executor so locally-executed tasks can resolve
        // `{"$secret": …}` bindings. None (KEK unset) fails closed at the task.
        let secret_resolver = crate::secrets::runner_secret_resolver(tenant_id, &tenant_database);
        let runner = DefaultRunner::with_database_secrets(
            tenant_database,
            config,
            Some(self.shared_runtime.clone()),
            secret_resolver,
        )
        .await?;
        // CLOACI-T-0581 follow-up: install the shared graph scheduler if
        // present so the tenant's reconciler can route packaged CGs.
        if let Some(scheduler) = &self.graph_scheduler {
            runner.set_graph_scheduler(scheduler.clone()).await;
        }
        // CLOACI-T-0634: register the fleet executor on this runner's dispatcher
        // so tasks routed to "fleet" reach the agent fleet. Must run before the
        // runner starts claiming work; the global runner's registration does not
        // cover tenant schemas.
        if let Some(registrar) = &self.fleet_registrar {
            registrar(&runner);
        }
        let runner = Arc::new(runner);

        let held = {
            let mut cache = self.cache.lock().await;
            cache.insert(tenant_id.to_string(), runner.clone());
            cache.len()
        };
        if held > self.warn_above.get() {
            warn!(
                tenant_runners = held,
                limit = self.warn_above.get(),
                "more tenant runners than --tenant-runner-cache-size; none is stopped. \
                 Each tenant uses up to 2 database connections on each replica: check \
                 Postgres max_connections, then raise the limit"
            );
        }

        Ok(runner)
    }

    /// Explicitly evict a tenant's runner from the cache, awaiting its
    /// graceful shutdown. Used by tenant-deletion teardown (CLOACI-T-0581).
    /// Returns `Ok(true)` if a runner was evicted, `Ok(false)` if none
    /// was cached.
    pub async fn evict(&self, tenant_id: &str) -> Result<bool, cloacina::WorkflowExecutionError> {
        let runner = {
            let mut cache = self.cache.lock().await;
            cache.remove(tenant_id)
        };
        match runner {
            Some(r) => {
                info!(
                    tenant_id = %tenant_id,
                    "evicting tenant runner (explicit)"
                );
                r.shutdown().await?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// CLOACI-T-0581: bounded-drain eviction. Returns an `EvictOutcome`
    /// distinguishing clean drain, timeout, shutdown error, and missing
    /// entry. On timeout the runner is already removed from the cache;
    /// its `shutdown()` future continues unawaited in the background.
    pub async fn evict_with_timeout(
        &self,
        tenant_id: &str,
        drain_timeout: std::time::Duration,
    ) -> EvictOutcome {
        let runner = {
            let mut cache = self.cache.lock().await;
            cache.remove(tenant_id)
        };
        match runner {
            None => EvictOutcome::Missing,
            Some(r) => {
                let tenant_owned = tenant_id.to_string();
                match tokio::time::timeout(drain_timeout, r.shutdown()).await {
                    Ok(Ok(())) => {
                        info!(
                            tenant_id = %tenant_owned,
                            "tenant runner drained within timeout"
                        );
                        EvictOutcome::Drained
                    }
                    Ok(Err(e)) => {
                        warn!(
                            tenant_id = %tenant_owned,
                            %e,
                            "tenant runner shutdown returned error"
                        );
                        EvictOutcome::ShutdownError(e.to_string())
                    }
                    Err(_) => {
                        warn!(
                            tenant_id = %tenant_owned,
                            drain_timeout_s = drain_timeout.as_secs(),
                            "tenant runner shutdown exceeded drain timeout; proceeding (CLOACI-T-0581)"
                        );
                        EvictOutcome::Timeout
                    }
                }
            }
        }
    }

    /// Shut down every cached runner. Called during server graceful
    /// shutdown. Returns a map of `tenant_id -> shutdown_result` so the
    /// caller can log failures without halting on the first one.
    pub async fn shutdown_all(&self) -> HashMap<String, Result<(), String>> {
        let runners: Vec<(String, Arc<DefaultRunner>)> = {
            let mut cache = self.cache.lock().await;
            cache.drain().collect()
        };
        let mut results = HashMap::with_capacity(runners.len());
        for (tenant_id, runner) in runners {
            let r = runner.shutdown().await.map_err(|e| e.to_string());
            results.insert(tenant_id, r);
        }
        results
    }

    /// The tenants that have a runner here.
    pub async fn tenant_ids(&self) -> Vec<String> {
        self.cache.lock().await.keys().cloned().collect()
    }

    /// `true` if `tenant_id` has a runner here.
    pub async fn contains(&self, tenant_id: &str) -> bool {
        self.cache.lock().await.contains_key(tenant_id)
    }

    /// Current number of cached runners. Test/observability helper.
    pub async fn len(&self) -> usize {
        self.cache.lock().await.len()
    }

    /// `true` if the cache holds no runners. Convenience for tests.
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

/// Make sure `tenant_id` has a live runner on this replica (CLOACINA-T-1031).
/// The `public` tenant is served by the global runner and needs none.
pub async fn ensure_runner(state: &crate::AppState, tenant_id: &str) -> Result<(), String> {
    if tenant_id == "public" {
        return Ok(());
    }
    let database = state
        .tenant_databases
        .resolve(tenant_id, &state.database)
        .await
        .map_err(|e| format!("tenant database: {e}"))?;
    state
        .tenant_runners
        .get_or_create(tenant_id, database)
        .await
        .map(|_| ())
        .map_err(|e| format!("tenant runner: {e}"))
}

/// [`ensure_runner`] for each of `tenants`. A tenant that fails is logged and
/// the others go on. Returns how many runners were made.
pub async fn ensure_runners(state: &crate::AppState, tenants: &[String]) -> usize {
    let mut made = 0;
    for tenant in tenants {
        if tenant == "public" || state.tenant_runners.contains(tenant).await {
            continue;
        }
        match ensure_runner(state, tenant).await {
            Ok(()) => made += 1,
            Err(e) => warn!(
                tenant_id = %tenant,
                error = %e,
                "tenant runner not started; its schedules do not fire until the next pass"
            ),
        }
    }
    made
}

/// One pass (CLOACINA-T-1031): a runner for each tenant schema that has
/// none, and no runner for a schema that is gone (a tenant deleted through
/// another replica).
pub async fn run_pass(state: &crate::AppState) {
    // Read the held set before the list: a tenant made after the list was
    // read is then not taken for a deleted one.
    let held_before = state.tenant_runners.tenant_ids().await;
    let tenants = match cloacina::database::DatabaseAdmin::new(state.database.clone())
        .list_tenant_schemas()
        .await
    {
        Ok(tenants) => tenants,
        Err(e) => {
            warn!(error = %e, "tenant runners: could not list tenant schemas; retrying next pass");
            return;
        }
    };
    let (made, stopped) = apply_tenant_list(state, &held_before, &tenants).await;
    if made > 0 || stopped > 0 {
        info!(
            made,
            stopped,
            tenants = tenants.len(),
            "tenant runners brought in line with the tenant schemas"
        );
    } else {
        debug!(tenants = tenants.len(), "tenant runners: nothing to do");
    }
}

/// Bring the runners in line with `tenants`, the tenant schemas that exist.
/// A tenant of `held_before` (the tenants that had a runner before the list
/// was read) that is not in `tenants` loses its runner and its cached
/// database. Each tenant in `tenants` with no runner gets one. Returns the
/// runners made and the runners stopped.
pub async fn apply_tenant_list(
    state: &crate::AppState,
    held_before: &[String],
    tenants: &[String],
) -> (usize, usize) {
    let mut stopped = 0;
    for gone in held_before.iter().filter(|held| !tenants.contains(held)) {
        let outcome = state
            .tenant_runners
            .evict_with_timeout(gone, state.tenant_deletion_drain_timeout)
            .await;
        state.tenant_databases.evict(gone).await;
        if outcome.was_present() {
            stopped += 1;
            info!(tenant_id = %gone, "tenant schema is gone; its runner is stopped");
        }
    }
    (ensure_runners(state, tenants).await, stopped)
}

/// Start the loop that keeps a runner for every tenant (CLOACINA-T-1031).
/// The first pass runs at once, so after a restart each tenant's schedules
/// fire again with no call for that tenant. Every replica runs it: each
/// replica holds its own runner for each tenant.
pub fn spawn(state: crate::AppState, mut shutdown: tokio::sync::watch::Receiver<bool>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(RUNNER_PASS_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = ticker.tick() => run_pass(&state).await,
                res = shutdown.changed() => {
                    if res.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(n: usize) -> NonZeroUsize {
        NonZeroUsize::new(n).expect("non-zero cap")
    }

    #[tokio::test]
    async fn empty_cache_is_empty() {
        let cache = TenantRunnerCache::new(cap(8), DefaultRunnerConfig::default());
        assert!(cache.is_empty().await);
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test]
    async fn evict_missing_tenant_returns_false() {
        let cache = TenantRunnerCache::new(cap(8), DefaultRunnerConfig::default());
        let evicted = cache.evict("never-cached").await.expect("evict ok");
        assert!(!evicted);
    }

    #[tokio::test]
    async fn shared_runtime_is_stable_arc() {
        let cache = TenantRunnerCache::new(cap(8), DefaultRunnerConfig::default());
        let a = cache.shared_runtime();
        let b = cache.shared_runtime();
        // Same allocation — confirms `Arc::clone`, not a fresh `Runtime::new()`.
        assert!(Arc::ptr_eq(&a, &b));
    }
}
