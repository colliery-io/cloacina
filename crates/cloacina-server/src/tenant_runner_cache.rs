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

//! Per-tenant `DefaultRunner` cache with LRU eviction. CLOACI-T-0580.
//!
//! Each cached entry is a fully-constructed runner with its own scheduler
//! loop, executor pool, and heartbeat — bound to the tenant's `Database`
//! from `TenantDatabaseCache` so workflow execution writes land in the
//! correct tenant schema.
//!
//! `Runtime` inventory (`TaskRegistry`, `WorkflowRegistry`,
//! `TriggerRegistry`, `ReactorRegistry`, `GraphRegistry`) is shared by
//! `Arc` across all per-tenant runners — they're inventory-seeded at
//! process start (post-T-0506) and have no per-tenant state.
//!
//! Eviction is graceful: when an entry is dropped from the LRU, its
//! `shutdown()` is awaited so background tasks join cleanly before the
//! next cache lookup.

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use lru::LruCache;
use tokio::sync::Mutex;
use tracing::{info, warn};

use cloacina::computation_graph::scheduler::ComputationGraphScheduler;
use cloacina::Database;
use cloacina::Runtime;
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

/// LRU-bounded cache of per-tenant `DefaultRunner` instances.
pub struct TenantRunnerCache {
    cache: Mutex<LruCache<String, Arc<DefaultRunner>>>,
    /// Shared `Runtime` for every per-tenant runner — inventory-seeded
    /// once at process start, never mutated.
    shared_runtime: Arc<Runtime>,
    /// Base runner config; cloned for each new tenant runner.
    base_config: DefaultRunnerConfig,
    /// CLOACI-T-0581 follow-up: optional shared `ComputationGraphScheduler`
    /// installed on every per-tenant runner via `set_graph_scheduler` so
    /// the tenant's reconciler can route packaged CGs into it. The
    /// scheduler itself stores `tenant_id` per graph (T-0579), so
    /// cross-tenant filtering at the health-endpoint layer still works
    /// even with a shared scheduler.
    graph_scheduler: Option<Arc<ComputationGraphScheduler>>,
}

impl TenantRunnerCache {
    /// Build a new cache with the given LRU cap. The shared runtime is
    /// constructed from inventory at this call site; callers shouldn't
    /// construct it separately.
    pub fn new(capacity: NonZeroUsize, base_config: DefaultRunnerConfig) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(capacity)),
            shared_runtime: Arc::new(Runtime::new()),
            base_config,
            graph_scheduler: None,
        }
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
    /// `DefaultRunner::with_database`, sharing the cache's `Runtime`.
    /// On cache fill past capacity: the LRU evicts the least-recently-used
    /// runner; its `shutdown()` is awaited before the new entry is
    /// installed.
    pub async fn get_or_create(
        &self,
        tenant_id: &str,
        tenant_database: Database,
    ) -> Result<Arc<DefaultRunner>, cloacina::WorkflowExecutionError> {
        // Fast path: cached?
        {
            let mut cache = self.cache.lock().await;
            if let Some(runner) = cache.get(tenant_id) {
                return Ok(runner.clone());
            }
        }

        // Slow path: build a new runner with the shared Runtime.
        info!(
            tenant_id = %tenant_id,
            "constructing per-tenant DefaultRunner (CLOACI-T-0580)"
        );
        let runner = DefaultRunner::with_database(
            tenant_database,
            self.base_config.clone(),
            Some(self.shared_runtime.clone()),
        )
        .await?;
        // CLOACI-T-0581 follow-up: install the shared graph scheduler if
        // present so the tenant's reconciler can route packaged CGs.
        if let Some(scheduler) = &self.graph_scheduler {
            runner.set_graph_scheduler(scheduler.clone()).await;
        }
        let runner = Arc::new(runner);

        // Install — evicting the LRU entry if we're at cap, with a
        // graceful shutdown of the evicted runner.
        let evicted = {
            let mut cache = self.cache.lock().await;
            // Double-check: another task may have raced to insert.
            if let Some(existing) = cache.get(tenant_id) {
                return Ok(existing.clone());
            }
            // `lru::LruCache::push` returns the evicted entry (if any).
            cache.push(tenant_id.to_string(), runner.clone())
        };
        if let Some((evicted_id, evicted_runner)) = evicted {
            if evicted_id == tenant_id {
                // Edge case: cap=1 and the entry we just pushed kicked
                // out the same slot — shouldn't happen given the
                // double-check above, but guard anyway.
            } else {
                tokio::spawn(async move {
                    info!(
                        tenant_id = %evicted_id,
                        "evicting LRU tenant runner; shutting down gracefully"
                    );
                    if let Err(e) = evicted_runner.shutdown().await {
                        warn!(
                            tenant_id = %evicted_id,
                            %e,
                            "evicted runner shutdown returned error"
                        );
                    }
                });
            }
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
            cache.pop(tenant_id)
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
            cache.pop(tenant_id)
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
            let mut drained = Vec::with_capacity(cache.len());
            while let Some((k, v)) = cache.pop_lru() {
                drained.push((k, v));
            }
            drained
        };
        let mut results = HashMap::with_capacity(runners.len());
        for (tenant_id, runner) in runners {
            let r = runner.shutdown().await.map_err(|e| e.to_string());
            results.insert(tenant_id, r);
        }
        results
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
