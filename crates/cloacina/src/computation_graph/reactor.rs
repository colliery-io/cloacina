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

//! Reactor — the long-lived process that wires accumulators to a compiled
//! computation graph.
//!
//! Three concerns:
//! 1. **Receiver**: accepts boundaries from accumulators, updates cache
//! 2. **Strategy**: evaluates reaction criteria to decide when to fire
//! 3. **Executor**: calls the compiled graph function
//!
//! See CLOACI-S-0005 for the full specification.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, watch, RwLock};

use super::types::{GraphResult, InputCache, SourceName};

// =============================================================================
// Reactor Health
// =============================================================================

/// Health state of a reactor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum ReactorHealth {
    /// Loading cache from DAL, spawning accumulators.
    Starting,
    /// Some accumulators healthy, waiting for all.
    Warming {
        healthy: Vec<String>,
        waiting: Vec<String>,
    },
    /// All accumulators healthy, evaluating criteria.
    Live,
    /// Was live, an accumulator disconnected. Running with stale data.
    Degraded { disconnected: Vec<String> },
}

impl std::fmt::Display for ReactorHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Starting => write!(f, "starting"),
            Self::Warming { .. } => write!(f, "warming"),
            Self::Live => write!(f, "live"),
            Self::Degraded { .. } => write!(f, "degraded"),
        }
    }
}

/// Create a reactor health reporting channel.
pub fn reactor_health_channel() -> (watch::Sender<ReactorHealth>, watch::Receiver<ReactorHealth>) {
    watch::channel(ReactorHealth::Starting)
}

/// Reaction criteria — when to fire the graph.
#[derive(Debug, Clone)]
pub enum ReactionCriteria {
    /// Fire if any dirty flag is set.
    WhenAny,
    /// Fire if all dirty flags are set.
    WhenAll,
}

/// Input strategy — how the reactor handles data between executions.
#[derive(Debug, Clone)]
pub enum InputStrategy {
    /// One slot per source, overwritten on each update. Always fires with freshest.
    Latest,
    /// Boundaries preserved in order, one execution per boundary.
    Sequential,
}

/// Dirty flags — one boolean per source.
#[derive(Debug, Clone)]
pub struct DirtyFlags {
    pub(crate) flags: HashMap<SourceName, bool>,
}

impl DirtyFlags {
    pub fn new() -> Self {
        Self {
            flags: HashMap::new(),
        }
    }

    /// Create dirty flags pre-seeded with expected source names (all initially false).
    ///
    /// Required for `WhenAll` — ensures `all_set()` returns false until
    /// every expected source has emitted, not just the sources seen so far.
    pub fn with_sources(sources: &[SourceName]) -> Self {
        let mut flags = HashMap::new();
        for source in sources {
            flags.insert(source.clone(), false);
        }
        Self { flags }
    }

    pub fn set(&mut self, source: SourceName, dirty: bool) {
        self.flags.insert(source, dirty);
    }

    pub fn any_set(&self) -> bool {
        self.flags.values().any(|&v| v)
    }

    pub fn all_set(&self) -> bool {
        !self.flags.is_empty() && self.flags.values().all(|&v| v)
    }

    pub fn clear_all(&mut self) {
        for v in self.flags.values_mut() {
            *v = false;
        }
    }
}

impl Default for DirtyFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// Signals sent from receiver to executor.
#[derive(Debug)]
pub enum StrategySignal {
    /// A boundary was received — check reaction criteria.
    BoundaryReceived,
    /// Force-fire regardless of criteria.
    ForceFire,
}

/// Manual commands accepted by the reactor.
#[derive(Debug)]
pub enum ManualCommand {
    /// Fire with current cache state.
    ForceFire,
    /// Fire with injected state (replaces cache).
    FireWith(InputCache),
}

/// Commands sent by WebSocket operators to a reactor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum ReactorCommand {
    ForceFire,
    FireWith { cache: HashMap<String, Vec<u8>> },
    GetState,
    Pause,
    Resume,
}

/// Responses sent back to WebSocket operators.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReactorResponse {
    Fired,
    State { cache: HashMap<String, String> },
    Paused,
    Resumed,
    Error { message: String },
}

/// Handle to a running reactor — exposes shared state for WebSocket queries.
///
/// Returned by `Reactor::handle()` before calling `run()`.
#[derive(Clone)]
pub struct ReactorHandle {
    /// Shared cache — readable by WebSocket handlers for GetState.
    pub cache: Arc<RwLock<InputCache>>,
    /// Pause flag — when true, executor skips graph execution.
    pub paused: Arc<AtomicBool>,
}

impl ReactorHandle {
    /// Read the current cache as a JSON-friendly map.
    pub async fn get_state(&self) -> HashMap<String, String> {
        let cache = self.cache.read().await;
        cache.entries_as_json()
    }

    /// Check if the reactor is paused.
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    /// Pause the reactor (stop executing, continue accepting boundaries).
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    /// Resume the reactor.
    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }
}

/// Type alias for the compiled graph function.
/// Re-exported from `cloacina-computation-graph`.
pub use cloacina_computation_graph::CompiledGraphFn;

/// The Reactor.
pub struct Reactor {
    /// The compiled graph function to call.
    graph: CompiledGraphFn,
    /// Reaction criteria.
    criteria: ReactionCriteria,
    /// Input strategy.
    input_strategy: InputStrategy,
    /// Channel receiving boundaries from accumulators.
    accumulator_rx: mpsc::Receiver<(SourceName, Vec<u8>)>,
    /// Channel for manual operations.
    manual_rx: mpsc::Receiver<ManualCommand>,
    /// Shutdown signal.
    shutdown: watch::Receiver<bool>,
    /// Shared cache (also accessible via ReactorHandle).
    cache: Arc<RwLock<InputCache>>,
    /// Pause flag (also accessible via ReactorHandle).
    paused: Arc<AtomicBool>,
    /// Expected source names (used to seed DirtyFlags for WhenAll).
    expected_sources: Vec<SourceName>,
    /// Graph name (for DAL persistence keying).
    graph_name: String,
    /// DAL handle for cache persistence. None in embedded mode.
    dal: Option<crate::dal::unified::DAL>,
    /// Health state reporter. None when health tracking not needed.
    health: Option<watch::Sender<ReactorHealth>>,
    /// Accumulator health receivers for startup gating and degraded mode detection.
    accumulator_health_rxs: Vec<(
        String,
        watch::Receiver<super::accumulator::AccumulatorHealth>,
    )>,
    /// Flush senders for batch accumulators — signalled after graph execution.
    batch_flush_senders: Vec<mpsc::Sender<()>>,
}

impl Reactor {
    pub fn new(
        graph: CompiledGraphFn,
        criteria: ReactionCriteria,
        input_strategy: InputStrategy,
        accumulator_rx: mpsc::Receiver<(SourceName, Vec<u8>)>,
        manual_rx: mpsc::Receiver<ManualCommand>,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            graph,
            criteria,
            input_strategy: input_strategy,
            accumulator_rx,
            manual_rx,
            shutdown,
            cache: Arc::new(RwLock::new(InputCache::new())),
            paused: Arc::new(AtomicBool::new(false)),
            expected_sources: Vec::new(),
            graph_name: String::new(),
            dal: None,
            health: None,
            accumulator_health_rxs: Vec::new(),
            batch_flush_senders: Vec::new(),
        }
    }

    /// Add batch flush senders — reactor will signal these after each graph execution.
    pub fn with_batch_flush_senders(mut self, senders: Vec<mpsc::Sender<()>>) -> Self {
        self.batch_flush_senders = senders;
        self
    }

    /// Set the graph name (used as key for DAL persistence).
    pub fn with_graph_name(mut self, name: String) -> Self {
        self.graph_name = name;
        self
    }

    /// Set the DAL handle for cache persistence.
    pub fn with_dal(mut self, dal: crate::dal::unified::DAL) -> Self {
        self.dal = Some(dal);
        self
    }

    /// Set the health reporter channel.
    pub fn with_health(mut self, health: watch::Sender<ReactorHealth>) -> Self {
        self.health = Some(health);
        self
    }

    /// Set the expected source names for WhenAll criteria.
    ///
    /// Seeds DirtyFlags so `all_set()` correctly requires all sources to emit
    /// before firing, not just the sources seen so far.
    pub fn with_expected_sources(mut self, sources: Vec<SourceName>) -> Self {
        self.expected_sources = sources;
        self
    }

    /// Set accumulator health receivers for startup gating and degraded mode.
    pub fn with_accumulator_health(
        mut self,
        rxs: Vec<(
            String,
            watch::Receiver<super::accumulator::AccumulatorHealth>,
        )>,
    ) -> Self {
        self.accumulator_health_rxs = rxs;
        self
    }

    /// Get a handle to this reactor's shared state.
    ///
    /// Call before `run()` to get a handle that WebSocket handlers can use
    /// for GetState, Pause, and Resume operations.
    pub fn handle(&self) -> ReactorHandle {
        ReactorHandle {
            cache: self.cache.clone(),
            paused: self.paused.clone(),
        }
    }

    /// Run the reactor. Spawns receiver + executor tasks.
    pub async fn run(mut self) {
        // Report starting health
        if let Some(ref health) = self.health {
            let _ = health.send(ReactorHealth::Starting);
        }

        // Load cache from DAL if available (instant recovery)
        let cache = self.cache.clone();
        if let Some(ref dal) = self.dal {
            if !self.graph_name.is_empty() {
                match dal.checkpoint().load_reactor_state(&self.graph_name).await {
                    Ok(Some((cache_data, _dirty_data, _seq_queue))) => {
                        // Restore cache — deserialize the entries map
                        if let Ok(entries) =
                            serde_json::from_slice::<HashMap<SourceName, Vec<u8>>>(&cache_data)
                        {
                            let mut c = cache.write().await;
                            for (source, bytes) in entries {
                                c.update(source, bytes);
                            }
                            tracing::info!(graph = %self.graph_name, "reactor cache restored from DAL");
                        }
                    }
                    Ok(None) => {
                        tracing::debug!(graph = %self.graph_name, "no persisted reactor state found");
                    }
                    Err(e) => {
                        tracing::warn!(graph = %self.graph_name, "failed to load reactor state: {}", e);
                    }
                }
            }
        }

        let dirty = if self.expected_sources.is_empty() {
            Arc::new(RwLock::new(DirtyFlags::new()))
        } else {
            Arc::new(RwLock::new(DirtyFlags::with_sources(
                &self.expected_sources,
            )))
        };

        // Startup gating — wait for all accumulators to become healthy before going Live
        if !self.accumulator_health_rxs.is_empty() {
            use super::accumulator::AccumulatorHealth;

            let all_names: Vec<String> = self
                .accumulator_health_rxs
                .iter()
                .map(|(n, _)| n.clone())
                .collect();
            let mut healthy_set: std::collections::HashSet<String> =
                std::collections::HashSet::new();

            // Report Warming
            if let Some(ref health) = self.health {
                let _ = health.send(ReactorHealth::Warming {
                    healthy: vec![],
                    waiting: all_names.clone(),
                });
            }

            let mut shutdown_gate = self.shutdown.clone();

            // Wait until all accumulators are healthy or shutdown fires
            'gating: loop {
                // Check current state of all receivers
                for (name, rx) in &self.accumulator_health_rxs {
                    let h = rx.borrow().clone();
                    match h {
                        AccumulatorHealth::Live | AccumulatorHealth::SocketOnly => {
                            healthy_set.insert(name.clone());
                        }
                        _ => {}
                    }
                }

                if healthy_set.len() >= all_names.len() {
                    break 'gating;
                }

                // Wait for any health change or shutdown
                tokio::select! {
                    _ = shutdown_gate.changed() => {
                        tracing::debug!(graph = %self.graph_name, "shutdown during startup gating");
                        return;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                        // Poll again — watch receivers update in place
                    }
                }

                // Update Warming status
                if let Some(ref health) = self.health {
                    let waiting: Vec<String> = all_names
                        .iter()
                        .filter(|n| !healthy_set.contains(*n))
                        .cloned()
                        .collect();
                    let _ = health.send(ReactorHealth::Warming {
                        healthy: healthy_set.iter().cloned().collect(),
                        waiting,
                    });
                }
            }
        }

        // All accumulators healthy — go Live
        if let Some(ref health) = self.health {
            let _ = health.send(ReactorHealth::Live);
        }

        // Spawn degraded-mode monitor — watches accumulator health and toggles
        // between Live and Degraded based on accumulator disconnections.
        let _degraded_monitor = if let Some(ref health) = self.health {
            let health_tx = health.clone();
            let acc_rxs = std::mem::take(&mut self.accumulator_health_rxs);
            let mut shutdown_mon = self.shutdown.clone();
            let graph_name = self.graph_name.clone();
            Some(tokio::spawn(async move {
                use super::accumulator::AccumulatorHealth;
                if acc_rxs.is_empty() {
                    // No accumulators to monitor — just wait for shutdown
                    let _ = shutdown_mon.changed().await;
                    return;
                }
                loop {
                    tokio::select! {
                        _ = shutdown_mon.changed() => break,
                        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                            let disconnected: Vec<String> = acc_rxs
                                .iter()
                                .filter(|(_, rx)| {
                                    matches!(*rx.borrow(), AccumulatorHealth::Disconnected)
                                })
                                .map(|(name, _)| name.clone())
                                .collect();

                            if disconnected.is_empty() {
                                // All healthy — ensure we're Live (not Degraded)
                                if matches!(&*health_tx.borrow(), ReactorHealth::Degraded { .. }) {
                                    tracing::info!(graph = %graph_name, "all accumulators recovered — back to Live");
                                    let _ = health_tx.send(ReactorHealth::Live);
                                }
                            } else {
                                // Some disconnected — enter/stay Degraded
                                if !matches!(&*health_tx.borrow(), ReactorHealth::Degraded { .. }) {
                                    tracing::warn!(graph = %graph_name, ?disconnected, "accumulator(s) disconnected — entering Degraded mode");
                                }
                                let _ = health_tx.send(ReactorHealth::Degraded { disconnected });
                            }
                        }
                    }
                }
            }))
        } else {
            None
        };

        let paused = self.paused.clone();
        let input_strategy = self.input_strategy.clone();

        // Sequential queue — only used when InputStrategy::Sequential
        let seq_queue: Arc<RwLock<VecDeque<(SourceName, Vec<u8>)>>> =
            Arc::new(RwLock::new(VecDeque::new()));

        let (strategy_tx, mut strategy_rx) = mpsc::channel::<StrategySignal>(64);

        // Spawn receiver task
        let cache_recv = cache.clone();
        let dirty_recv = dirty.clone();
        let seq_queue_recv = seq_queue.clone();
        let input_strategy_recv = input_strategy.clone();
        let mut shutdown_recv = self.shutdown.clone();
        let mut accumulator_rx = self.accumulator_rx;
        let mut manual_rx = self.manual_rx;
        let strategy_tx_recv = strategy_tx.clone();

        let receiver_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some((source, bytes)) = accumulator_rx.recv() => {
                        match input_strategy_recv {
                            InputStrategy::Latest => {
                                cache_recv.write().await.update(source.clone(), bytes);
                                dirty_recv.write().await.set(source, true);
                            }
                            InputStrategy::Sequential => {
                                // Queue the boundary — don't update cache yet
                                seq_queue_recv.write().await.push_back((source, bytes));
                            }
                        }
                        let _ = strategy_tx_recv.send(StrategySignal::BoundaryReceived).await;
                    }
                    Some(cmd) = manual_rx.recv() => {
                        match cmd {
                            ManualCommand::ForceFire => {
                                let _ = strategy_tx_recv.send(StrategySignal::ForceFire).await;
                            }
                            ManualCommand::FireWith(new_cache) => {
                                cache_recv.write().await.replace_all(new_cache);
                                let _ = strategy_tx_recv.send(StrategySignal::ForceFire).await;
                            }
                        }
                    }
                    _ = shutdown_recv.changed() => {
                        tracing::debug!("reactor receiver shutting down");
                        break;
                    }
                }
            }
        });

        // Executor runs on current task
        let cache_exec = cache.clone();
        let dirty_exec = dirty.clone();
        let seq_queue_exec = seq_queue.clone();
        let mut shutdown_exec = self.shutdown.clone();
        let graph = self.graph.clone();
        let criteria = self.criteria.clone();
        let dal_exec = self.dal.clone();
        let graph_name_exec = self.graph_name.clone();
        let batch_flush = self.batch_flush_senders.clone();

        loop {
            tokio::select! {
                Some(signal) = strategy_rx.recv() => {
                    match input_strategy {
                        InputStrategy::Latest => {
                            let should_run = match signal {
                                StrategySignal::BoundaryReceived => {
                                    let d = dirty_exec.read().await;
                                    match &criteria {
                                        ReactionCriteria::WhenAny => d.any_set(),
                                        ReactionCriteria::WhenAll => d.all_set(),
                                    }
                                }
                                StrategySignal::ForceFire => true,
                            };

                            if should_run && !paused.load(Ordering::SeqCst) {
                                let snapshot = cache_exec.read().await.snapshot();
                                dirty_exec.write().await.clear_all();
                                let result = (graph)(snapshot).await;
                                match &result {
                                    GraphResult::Completed { .. } => {
                                        tracing::debug!("graph execution completed");
                                        // Persist cache after successful execution
                                        persist_reactor_state(
                                            &dal_exec, &graph_name_exec, &cache_exec, &dirty_exec, None,
                                        ).await;
                                        // Signal batch accumulators to flush
                                        for sender in &batch_flush {
                                            let _ = sender.try_send(());
                                        }
                                    }
                                    GraphResult::Error(e) => {
                                        tracing::error!("graph execution failed: {}", e);
                                    }
                                }
                            }
                        }
                        InputStrategy::Sequential => {
                            if paused.load(Ordering::SeqCst) {
                                continue;
                            }
                            // Persist queue BEFORE draining so crash mid-drain doesn't lose items
                            persist_reactor_state(
                                &dal_exec, &graph_name_exec, &cache_exec, &dirty_exec,
                                Some(&seq_queue_exec),
                            ).await;
                            // Drain the queue — one execution per queued boundary
                            loop {
                                let item = seq_queue_exec.write().await.pop_front();
                                match item {
                                    Some((source, bytes)) => {
                                        cache_exec.write().await.update(source, bytes);
                                        let snapshot = cache_exec.read().await.snapshot();
                                        let result = (graph)(snapshot).await;
                                        match &result {
                                            GraphResult::Completed { .. } => {
                                                tracing::debug!("graph execution completed (sequential)");
                                                persist_reactor_state(
                                                    &dal_exec, &graph_name_exec, &cache_exec,
                                                    &dirty_exec, Some(&seq_queue_exec),
                                                ).await;
                                                // Signal batch accumulators to flush
                                                for sender in &batch_flush {
                                                    let _ = sender.try_send(());
                                                }
                                            }
                                            GraphResult::Error(e) => {
                                                tracing::error!("graph execution failed: {}", e);
                                            }
                                        }
                                    }
                                    None => break,
                                }
                            }
                        }
                    }
                }
                _ = shutdown_exec.changed() => {
                    tracing::debug!("reactor executor shutting down");
                    // Final persist on orderly shutdown
                    persist_reactor_state(
                        &dal_exec, &graph_name_exec, &cache_exec, &dirty_exec,
                        Some(&seq_queue_exec),
                    ).await;
                    break;
                }
            }
        }

        // Wait for receiver to finish
        let _ = receiver_handle.await;
    }
}

/// Persist reactor state to DAL (best-effort, logs on failure).
async fn persist_reactor_state(
    dal: &Option<crate::dal::unified::DAL>,
    graph_name: &str,
    cache: &Arc<RwLock<InputCache>>,
    dirty: &Arc<RwLock<DirtyFlags>>,
    seq_queue: Option<&Arc<RwLock<VecDeque<(SourceName, Vec<u8>)>>>>,
) {
    let dal = match dal {
        Some(d) if !graph_name.is_empty() => d,
        _ => return,
    };

    let cache_snapshot = cache.read().await;
    let dirty_snapshot = dirty.read().await;

    // Serialize cache entries as JSON map
    let cache_bytes = match serde_json::to_vec(&cache_snapshot.entries_raw()) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(graph = %graph_name, "cache serialization failed: {}", e);
            return;
        }
    };

    let dirty_bytes = match serde_json::to_vec(&dirty_snapshot.flags) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(graph = %graph_name, "dirty flags serialization failed: {}", e);
            return;
        }
    };

    let seq_bytes = if let Some(q) = seq_queue {
        let queue = q.read().await;
        if queue.is_empty() {
            None
        } else {
            match serde_json::to_vec(&*queue) {
                Ok(b) => Some(b),
                Err(e) => {
                    tracing::warn!(graph = %graph_name, "sequential queue serialization failed: {}", e);
                    None
                }
            }
        }
    } else {
        None
    };

    if let Err(e) = dal
        .checkpoint()
        .save_reactor_state(graph_name, cache_bytes, dirty_bytes, seq_bytes)
        .await
    {
        tracing::warn!(graph = %graph_name, "reactor state persistence failed: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_flags_when_any() {
        let mut flags = DirtyFlags::new();
        assert!(!flags.any_set());

        flags.set(SourceName::new("alpha"), true);
        assert!(flags.any_set());

        flags.set(SourceName::new("beta"), false);
        assert!(flags.any_set()); // alpha still dirty
    }

    #[test]
    fn test_dirty_flags_when_all() {
        let mut flags = DirtyFlags::new();
        flags.set(SourceName::new("alpha"), true);
        flags.set(SourceName::new("beta"), false);
        assert!(!flags.all_set());

        flags.set(SourceName::new("beta"), true);
        assert!(flags.all_set());
    }

    #[test]
    fn test_dirty_flags_clear_all() {
        let mut flags = DirtyFlags::new();
        flags.set(SourceName::new("alpha"), true);
        flags.set(SourceName::new("beta"), true);
        assert!(flags.all_set());

        flags.clear_all();
        assert!(!flags.any_set());
    }

    #[test]
    fn test_dirty_flags_empty_all_set() {
        let flags = DirtyFlags::new();
        // Empty flags: all_set should be false (no sources registered)
        assert!(!flags.all_set());
    }

    #[tokio::test]
    async fn test_reactor_fires_on_boundary() {
        let (acc_tx, acc_rx) = mpsc::channel(10);
        let (_manual_tx, manual_rx) = mpsc::channel(10);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Track how many times the graph fires
        let fire_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let fire_count_inner = fire_count.clone();

        let graph: CompiledGraphFn = Arc::new(move |_cache: InputCache| {
            let fc = fire_count_inner.clone();
            Box::pin(async move {
                fc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                GraphResult::completed(vec![])
            })
        });

        let reactor = Reactor::new(
            graph,
            ReactionCriteria::WhenAny,
            InputStrategy::Latest,
            acc_rx,
            manual_rx,
            shutdown_rx,
        );

        let handle = tokio::spawn(reactor.run());

        // Push a boundary
        let bytes = super::super::types::serialize(&42u32).unwrap();
        acc_tx
            .send((SourceName::new("alpha"), bytes))
            .await
            .unwrap();

        // Give it a moment to process
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert_eq!(fire_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        shutdown_tx.send(true).unwrap();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
    }

    #[tokio::test]
    async fn test_reactor_manual_force_fire() {
        let (_acc_tx, acc_rx) = mpsc::channel(10);
        let (manual_tx, manual_rx) = mpsc::channel(10);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let fire_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let fire_count_inner = fire_count.clone();

        let graph: CompiledGraphFn = Arc::new(move |_cache: InputCache| {
            let fc = fire_count_inner.clone();
            Box::pin(async move {
                fc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                GraphResult::completed(vec![])
            })
        });

        let reactor = Reactor::new(
            graph,
            ReactionCriteria::WhenAny,
            InputStrategy::Latest,
            acc_rx,
            manual_rx,
            shutdown_rx,
        );

        let handle = tokio::spawn(reactor.run());

        // Force fire without any boundaries
        manual_tx.send(ManualCommand::ForceFire).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert_eq!(fire_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        shutdown_tx.send(true).unwrap();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
    }

    #[tokio::test]
    async fn test_reactor_cache_snapshot_isolation() {
        let (acc_tx, acc_rx) = mpsc::channel(10);
        let (_manual_tx, manual_rx) = mpsc::channel(10);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let captured_cache = Arc::new(tokio::sync::Mutex::new(None));
        let captured_inner = captured_cache.clone();

        let graph: CompiledGraphFn = Arc::new(move |cache: InputCache| {
            let ci = captured_inner.clone();
            Box::pin(async move {
                // Store the snapshot we received
                *ci.lock().await = Some(cache);
                GraphResult::completed(vec![])
            })
        });

        let reactor = Reactor::new(
            graph,
            ReactionCriteria::WhenAny,
            InputStrategy::Latest,
            acc_rx,
            manual_rx,
            shutdown_rx,
        );

        let handle = tokio::spawn(reactor.run());

        // Push boundary with value 1
        acc_tx
            .send((
                SourceName::new("alpha"),
                super::super::types::serialize(&1u32).unwrap(),
            ))
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Verify the graph received the correct value
        let snapshot = captured_cache.lock().await;
        assert!(snapshot.is_some());
        let cache = snapshot.as_ref().unwrap();
        let val: u32 = cache.get("alpha").unwrap().unwrap();
        assert_eq!(val, 1);

        shutdown_tx.send(true).unwrap();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
    }
}
