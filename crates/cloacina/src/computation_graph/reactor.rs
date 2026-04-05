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

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, watch, RwLock};

use super::types::{GraphResult, InputCache, SourceName};

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
    flags: HashMap<SourceName, bool>,
}

impl DirtyFlags {
    pub fn new() -> Self {
        Self {
            flags: HashMap::new(),
        }
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
pub type CompiledGraphFn =
    Arc<dyn Fn(InputCache) -> Pin<Box<dyn Future<Output = GraphResult> + Send>> + Send + Sync>;

/// The Reactor.
pub struct Reactor {
    /// The compiled graph function to call.
    graph: CompiledGraphFn,
    /// Reaction criteria.
    criteria: ReactionCriteria,
    /// Input strategy.
    _input_strategy: InputStrategy,
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
            _input_strategy: input_strategy,
            accumulator_rx,
            manual_rx,
            shutdown,
            cache: Arc::new(RwLock::new(InputCache::new())),
            paused: Arc::new(AtomicBool::new(false)),
        }
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
    pub async fn run(self) {
        let cache = self.cache.clone();
        let dirty = Arc::new(RwLock::new(DirtyFlags::new()));
        let paused = self.paused.clone();

        let (strategy_tx, mut strategy_rx) = mpsc::channel::<StrategySignal>(64);

        // Spawn receiver task
        let cache_recv = cache.clone();
        let dirty_recv = dirty.clone();
        let mut shutdown_recv = self.shutdown.clone();
        let mut accumulator_rx = self.accumulator_rx;
        let mut manual_rx = self.manual_rx;
        let strategy_tx_recv = strategy_tx.clone();

        let receiver_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some((source, bytes)) = accumulator_rx.recv() => {
                        cache_recv.write().await.update(source.clone(), bytes);
                        dirty_recv.write().await.set(source, true);
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
        let mut shutdown_exec = self.shutdown.clone();
        let graph = self.graph.clone();
        let criteria = self.criteria.clone();

        loop {
            tokio::select! {
                Some(signal) = strategy_rx.recv() => {
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
                        // Snapshot cache and clear dirty flags
                        let snapshot = cache_exec.read().await.snapshot();
                        dirty_exec.write().await.clear_all();

                        // Execute the compiled graph
                        let result = (graph)(snapshot).await;

                        match &result {
                            GraphResult::Completed { .. } => {
                                tracing::debug!("graph execution completed");
                            }
                            GraphResult::Error(e) => {
                                tracing::error!("graph execution failed: {}", e);
                            }
                        }
                    }
                }
                _ = shutdown_exec.changed() => {
                    tracing::debug!("reactor executor shutting down");
                    break;
                }
            }
        }

        // Wait for receiver to finish
        let _ = receiver_handle.await;
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
