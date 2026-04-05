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

//! Reactive Scheduler — spawns, supervises, and shuts down accumulator/reactor
//! tasks from computation graph declarations.
//!
//! The reactive counterpart to the Unified Scheduler. Receives declarations
//! from the reconciler, wires channels, spawns tokio tasks, registers endpoints,
//! and restarts tasks on panic.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, watch, RwLock};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use super::accumulator::{
    accumulator_runtime, shutdown_signal, Accumulator, AccumulatorContext,
    AccumulatorRuntimeConfig, BoundarySender,
};
use super::reactor::{CompiledGraphFn, InputStrategy, ReactionCriteria, Reactor, ReactorHandle};
use super::registry::EndpointRegistry;
use super::types::SourceName;

/// Declaration of a computation graph to be loaded by the Reactive Scheduler.
#[derive(Clone)]
pub struct ComputationGraphDeclaration {
    /// Unique name for this computation graph.
    pub name: String,
    /// Accumulator declarations.
    pub accumulators: Vec<AccumulatorDeclaration>,
    /// Reactor declaration.
    pub reactor: ReactorDeclaration,
}

/// Declaration for a single accumulator.
#[derive(Clone)]
pub struct AccumulatorDeclaration {
    /// Accumulator name (used as WebSocket endpoint name).
    pub name: String,
    /// Factory that creates the accumulator instance.
    pub factory: Arc<dyn AccumulatorFactory>,
}

/// Factory trait for creating accumulator instances.
///
/// We can't clone trait objects, so we use a factory that produces them.
pub trait AccumulatorFactory: Send + Sync {
    /// Create a new accumulator instance and its runtime components.
    ///
    /// Returns:
    /// - socket_tx: sender for the accumulator's socket channel
    /// - boundary_tx: sender for boundary output to reactor
    /// - join_handle: spawned task handle
    fn spawn(
        &self,
        name: String,
        boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>,
        shutdown_rx: watch::Receiver<bool>,
    ) -> (mpsc::Sender<Vec<u8>>, JoinHandle<()>);
}

/// Declaration for the reactor.
#[derive(Clone)]
pub struct ReactorDeclaration {
    /// Reaction criteria (when_any / when_all).
    pub criteria: ReactionCriteria,
    /// Input strategy (latest / sequential).
    pub strategy: InputStrategy,
    /// The compiled graph function.
    pub graph_fn: CompiledGraphFn,
}

/// Status of a managed computation graph.
#[derive(Debug, Clone)]
pub struct GraphStatus {
    pub name: String,
    pub accumulators: Vec<String>,
    pub reactor_paused: bool,
    pub running: bool,
}

/// State for a running computation graph.
struct RunningGraph {
    /// Shutdown signal sender.
    shutdown_tx: watch::Sender<bool>,
    /// Accumulator task handles.
    accumulator_handles: Vec<(String, JoinHandle<()>)>,
    /// Reactor task handle.
    reactor_handle: JoinHandle<()>,
    /// Reactor handle for pause/resume queries.
    reactor_shared: ReactorHandle,
    /// Declaration (for restarts).
    declaration: ComputationGraphDeclaration,
}

/// The Reactive Scheduler.
pub struct ReactiveScheduler {
    /// Endpoint registry for WebSocket routing.
    registry: EndpointRegistry,
    /// Running computation graphs.
    graphs: Arc<RwLock<HashMap<String, RunningGraph>>>,
}

impl ReactiveScheduler {
    pub fn new(registry: EndpointRegistry) -> Self {
        Self {
            registry,
            graphs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load and start a computation graph.
    pub async fn load_graph(&self, decl: ComputationGraphDeclaration) -> Result<(), String> {
        let name = decl.name.clone();

        // Check if already loaded
        {
            let graphs = self.graphs.read().await;
            if graphs.contains_key(&name) {
                return Err(format!("graph '{}' already loaded", name));
            }
        }

        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        // Create boundary channel (all accumulators → reactor)
        let (boundary_tx, boundary_rx) = mpsc::channel(256);

        // Spawn accumulators
        let mut accumulator_handles = Vec::new();
        for acc_decl in &decl.accumulators {
            let (socket_tx, handle) = acc_decl.factory.spawn(
                acc_decl.name.clone(),
                boundary_tx.clone(),
                shutdown_rx.clone(),
            );

            // Register in endpoint registry
            self.registry
                .register_accumulator(acc_decl.name.clone(), socket_tx)
                .await;

            accumulator_handles.push((acc_decl.name.clone(), handle));
        }

        // Create manual command channel
        let (manual_tx, manual_rx) = mpsc::channel(64);

        // Create and spawn reactor
        let reactor = Reactor::new(
            decl.reactor.graph_fn.clone(),
            decl.reactor.criteria.clone(),
            decl.reactor.strategy.clone(),
            boundary_rx,
            manual_rx,
            shutdown_rx,
        );

        let reactor_shared = reactor.handle();

        // Register reactor in endpoint registry
        self.registry
            .register_reactor(name.clone(), manual_tx, reactor_shared.clone())
            .await;

        let reactor_handle = tokio::spawn(reactor.run());

        info!(graph = %name, "computation graph loaded and running");

        let running = RunningGraph {
            shutdown_tx,
            accumulator_handles,
            reactor_handle,
            reactor_shared,
            declaration: decl,
        };

        self.graphs.write().await.insert(name, running);
        Ok(())
    }

    /// Unload and shut down a computation graph.
    pub async fn unload_graph(&self, name: &str) -> Result<(), String> {
        let running = {
            let mut graphs = self.graphs.write().await;
            graphs
                .remove(name)
                .ok_or_else(|| format!("graph '{}' not loaded", name))?
        };

        // Send shutdown signal
        let _ = running.shutdown_tx.send(true);

        // Wait for reactor
        let _ =
            tokio::time::timeout(std::time::Duration::from_secs(5), running.reactor_handle).await;

        // Wait for accumulators
        for (acc_name, handle) in running.accumulator_handles {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle).await;
            self.registry.deregister_accumulator(&acc_name).await;
        }

        // Deregister reactor
        self.registry.deregister_reactor(name).await;

        info!(graph = %name, "computation graph unloaded");
        Ok(())
    }

    /// List all loaded computation graphs with status.
    pub async fn list_graphs(&self) -> Vec<GraphStatus> {
        let graphs = self.graphs.read().await;
        graphs
            .iter()
            .map(|(name, running)| GraphStatus {
                name: name.clone(),
                accumulators: running
                    .accumulator_handles
                    .iter()
                    .map(|(n, _)| n.clone())
                    .collect(),
                reactor_paused: running.reactor_shared.is_paused(),
                running: !running.reactor_handle.is_finished(),
            })
            .collect()
    }

    /// Graceful shutdown of all graphs.
    pub async fn shutdown_all(&self) {
        let names: Vec<String> = {
            let graphs = self.graphs.read().await;
            graphs.keys().cloned().collect()
        };

        for name in names {
            if let Err(e) = self.unload_graph(&name).await {
                warn!(graph = %name, error = %e, "failed to unload graph during shutdown");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::computation_graph::types::{serialize, GraphResult, InputCache};
    use serde::{Deserialize, Serialize};
    use std::sync::atomic::{AtomicU32, Ordering};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestEvent {
        value: f64,
    }

    /// A simple passthrough accumulator for testing.
    struct TestAccumulatorFactory;

    impl AccumulatorFactory for TestAccumulatorFactory {
        fn spawn(
            &self,
            name: String,
            boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>,
            shutdown_rx: watch::Receiver<bool>,
        ) -> (mpsc::Sender<Vec<u8>>, JoinHandle<()>) {
            let (socket_tx, socket_rx) = mpsc::channel(64);

            struct Passthrough;

            #[async_trait::async_trait]
            impl Accumulator for Passthrough {
                type Event = TestEvent;
                type Output = TestEvent;
                fn process(&mut self, event: TestEvent) -> Option<TestEvent> {
                    Some(event)
                }
            }

            let sender = BoundarySender::new(boundary_tx, SourceName::new(&name));
            let ctx = AccumulatorContext {
                output: sender,
                name: name.clone(),
                shutdown: shutdown_rx,
            };

            let handle = tokio::spawn(accumulator_runtime(
                Passthrough,
                ctx,
                socket_rx,
                AccumulatorRuntimeConfig::default(),
            ));

            (socket_tx, handle)
        }
    }

    #[tokio::test]
    async fn test_load_graph_push_event_fires() {
        let registry = EndpointRegistry::new();
        let scheduler = ReactiveScheduler::new(registry.clone());

        let fire_count = Arc::new(AtomicU32::new(0));
        let fire_count_inner = fire_count.clone();

        let graph_fn: CompiledGraphFn = Arc::new(move |_cache: InputCache| {
            let fc = fire_count_inner.clone();
            Box::pin(async move {
                fc.fetch_add(1, Ordering::SeqCst);
                GraphResult::completed(vec![])
            })
        });

        let decl = ComputationGraphDeclaration {
            name: "test_graph".to_string(),
            accumulators: vec![AccumulatorDeclaration {
                name: "alpha".to_string(),
                factory: Arc::new(TestAccumulatorFactory),
            }],
            reactor: ReactorDeclaration {
                criteria: ReactionCriteria::WhenAny,
                strategy: InputStrategy::Latest,
                graph_fn,
            },
        };

        scheduler.load_graph(decl).await.unwrap();

        // Push event via registry (simulating WebSocket push)
        let event = TestEvent { value: 42.0 };
        let bytes = serialize(&event).unwrap();
        registry.send_to_accumulator("alpha", bytes).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        assert_eq!(fire_count.load(Ordering::SeqCst), 1, "graph should fire");

        // List graphs
        let graphs = scheduler.list_graphs().await;
        assert_eq!(graphs.len(), 1);
        assert_eq!(graphs[0].name, "test_graph");
        assert!(!graphs[0].reactor_paused);

        scheduler.shutdown_all().await;
    }

    #[tokio::test]
    async fn test_unload_graph_deregisters() {
        let registry = EndpointRegistry::new();
        let scheduler = ReactiveScheduler::new(registry.clone());

        let graph_fn: CompiledGraphFn =
            Arc::new(|_cache: InputCache| Box::pin(async { GraphResult::completed(vec![]) }));

        let decl = ComputationGraphDeclaration {
            name: "test_graph".to_string(),
            accumulators: vec![AccumulatorDeclaration {
                name: "alpha".to_string(),
                factory: Arc::new(TestAccumulatorFactory),
            }],
            reactor: ReactorDeclaration {
                criteria: ReactionCriteria::WhenAny,
                strategy: InputStrategy::Latest,
                graph_fn,
            },
        };

        scheduler.load_graph(decl).await.unwrap();

        // Verify registered
        assert_eq!(registry.accumulator_count("alpha").await, 1);
        assert!(registry
            .list_reactors()
            .await
            .contains(&"test_graph".to_string()));

        // Unload
        scheduler.unload_graph("test_graph").await.unwrap();

        // Verify deregistered
        assert_eq!(registry.accumulator_count("alpha").await, 0);
        assert!(registry.list_reactors().await.is_empty());
    }

    #[tokio::test]
    async fn test_duplicate_load_rejected() {
        let registry = EndpointRegistry::new();
        let scheduler = ReactiveScheduler::new(registry.clone());

        let graph_fn: CompiledGraphFn =
            Arc::new(|_cache: InputCache| Box::pin(async { GraphResult::completed(vec![]) }));

        let decl = ComputationGraphDeclaration {
            name: "dup".to_string(),
            accumulators: vec![],
            reactor: ReactorDeclaration {
                criteria: ReactionCriteria::WhenAny,
                strategy: InputStrategy::Latest,
                graph_fn,
            },
        };

        scheduler.load_graph(decl.clone()).await.unwrap();
        let err = scheduler.load_graph(decl).await.unwrap_err();
        assert!(err.contains("already loaded"));

        scheduler.shutdown_all().await;
    }
}
