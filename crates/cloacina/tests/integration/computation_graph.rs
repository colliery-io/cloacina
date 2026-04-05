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

//! End-to-end tests for the `#[computation_graph]` macro.
//!
//! These tests verify that the macro correctly parses topology, validates the
//! graph, and generates a callable async function that routes data correctly.

use cloacina::computation_graph::types::{serialize, GraphResult, InputCache, SourceName};
use serde::{Deserialize, Serialize};

// --- Test boundary types ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlphaData {
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessedData {
    pub result: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputConfirmation {
    pub published: bool,
    pub value: f64,
}

// =============================================================================
// Test 1: Linear chain (A -> B -> C)
// =============================================================================

#[cloacina_macros::computation_graph(
    react = when_any(alpha),
    graph = {
        entry(alpha) -> process,
        process -> output,
    }
)]
pub mod linear_chain {
    use super::*;

    pub async fn entry(alpha: Option<&AlphaData>) -> ProcessedData {
        let a = alpha.unwrap();
        ProcessedData {
            result: a.value * 2.0,
        }
    }

    pub async fn process(input: &ProcessedData) -> ProcessedData {
        ProcessedData {
            result: input.result + 10.0,
        }
    }

    pub async fn output(input: &ProcessedData) -> OutputConfirmation {
        OutputConfirmation {
            published: true,
            value: input.result,
        }
    }
}

#[tokio::test]
async fn test_linear_chain() {
    let mut cache = InputCache::new();
    cache.update(
        SourceName::new("alpha"),
        serialize(&AlphaData { value: 5.0 }).unwrap(),
    );

    let result: GraphResult = linear_chain_compiled(&cache).await;
    assert!(result.is_completed());
}

// =============================================================================
// Test 2: Enum routing (A => { X -> B, Y -> C })
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BetaData {
    pub estimate: f64,
}

#[cloacina_macros::computation_graph(
    react = when_any(alpha, beta),
    graph = {
        decision(alpha, beta) => {
            Signal -> signal_handler,
            NoAction -> audit_logger,
        },
    }
)]
pub mod routing_graph {
    use super::*;

    #[derive(Debug, Clone)]
    pub enum DecisionOutcome {
        Signal(SignalData),
        NoAction(NoActionReason),
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct SignalData {
        pub output: f64,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct NoActionReason {
        pub reason: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct AuditRecord {
        pub logged: bool,
    }

    pub async fn decision(alpha: Option<&AlphaData>, beta: Option<&BetaData>) -> DecisionOutcome {
        let a = alpha.map(|a| a.value).unwrap_or(0.0);
        let b = beta.map(|b| b.estimate).unwrap_or(0.0);
        if a + b > 10.0 {
            DecisionOutcome::Signal(SignalData { output: a + b })
        } else {
            DecisionOutcome::NoAction(NoActionReason {
                reason: "below threshold".to_string(),
            })
        }
    }

    pub async fn signal_handler(signal: &SignalData) -> OutputConfirmation {
        OutputConfirmation {
            published: true,
            value: signal.output,
        }
    }

    pub async fn audit_logger(reason: &NoActionReason) -> AuditRecord {
        AuditRecord {
            logged: !reason.reason.is_empty(),
        }
    }
}

#[tokio::test]
async fn test_routing_signal_path() {
    let mut cache = InputCache::new();
    cache.update(
        SourceName::new("alpha"),
        serialize(&AlphaData { value: 8.0 }).unwrap(),
    );
    cache.update(
        SourceName::new("beta"),
        serialize(&BetaData { estimate: 5.0 }).unwrap(),
    );

    let result: GraphResult = routing_graph_compiled(&cache).await;
    assert!(result.is_completed());
}

#[tokio::test]
async fn test_routing_no_action_path() {
    let mut cache = InputCache::new();
    cache.update(
        SourceName::new("alpha"),
        serialize(&AlphaData { value: 2.0 }).unwrap(),
    );
    cache.update(
        SourceName::new("beta"),
        serialize(&BetaData { estimate: 1.0 }).unwrap(),
    );

    let result: GraphResult = routing_graph_compiled(&cache).await;
    assert!(result.is_completed());
}

// =============================================================================
// Test 4: End-to-end — Accumulator → Reactor → Compiled Graph
// =============================================================================

use cloacina::computation_graph::accumulator::{
    accumulator_runtime, shutdown_signal, AccumulatorContext, AccumulatorRuntimeConfig,
    BoundarySender,
};
use cloacina::computation_graph::reactor::{
    CompiledGraphFn, InputStrategy, ReactionCriteria, Reactor,
};
use std::sync::Arc;

struct TestPassthroughAccumulator;

#[async_trait::async_trait]
impl cloacina::computation_graph::Accumulator for TestPassthroughAccumulator {
    type Event = AlphaData;
    type Output = AlphaData;

    fn process(&mut self, event: AlphaData) -> Option<AlphaData> {
        Some(event)
    }
}

#[tokio::test]
async fn test_end_to_end_accumulator_reactor_graph() {
    let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(10);
    let (socket_tx, socket_rx) = tokio::sync::mpsc::channel(10);
    let (_manual_tx, manual_rx) = tokio::sync::mpsc::channel(10);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    let acc_sender = BoundarySender::new(boundary_tx, SourceName::new("alpha"));
    let acc_ctx = AccumulatorContext {
        output: acc_sender,
        name: "alpha".to_string(),
        shutdown: shutdown_rx.clone(),
    };

    let acc_handle = tokio::spawn(accumulator_runtime(
        TestPassthroughAccumulator,
        acc_ctx,
        socket_rx,
        AccumulatorRuntimeConfig::default(),
    ));

    let fire_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fire_count_inner = fire_count.clone();

    let last_output: Arc<tokio::sync::Mutex<Option<OutputConfirmation>>> =
        Arc::new(tokio::sync::Mutex::new(None));
    let last_output_inner = last_output.clone();

    let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
        let fc = fire_count_inner.clone();
        let lo = last_output_inner.clone();
        Box::pin(async move {
            fc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let result = linear_chain_compiled(&cache).await;
            let captured = if let GraphResult::Completed { outputs } = &result {
                outputs
                    .iter()
                    .find_map(|o| o.downcast_ref::<OutputConfirmation>().cloned())
            } else {
                None
            };
            if let Some(c) = captured {
                *lo.lock().await = Some(c);
            }
            result
        })
    });

    let reactor = Reactor::new(
        graph_fn,
        ReactionCriteria::WhenAny,
        InputStrategy::Latest,
        boundary_rx,
        manual_rx,
        shutdown_rx,
    );

    let reactor_handle = tokio::spawn(reactor.run());

    // Push event into accumulator socket → accumulator processes → boundary to reactor → graph fires
    let event = AlphaData { value: 7.0 };
    socket_tx.send(serialize(&event).unwrap()).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    assert_eq!(
        fire_count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "graph should have fired once"
    );

    // Verify actual output: entry(7.0) → 7.0*2=14.0, process(14.0) → 14.0+10=24.0, output → {published: true, value: 24.0}
    {
        let output = last_output.lock().await;
        let confirm = output
            .as_ref()
            .expect("should have captured terminal output");
        assert!(confirm.published);
        assert_eq!(confirm.value, 24.0);
    }

    // Push again — fires again with different value
    socket_tx
        .send(serialize(&AlphaData { value: 99.0 }).unwrap())
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    assert_eq!(
        fire_count.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "graph should have fired twice"
    );

    // Verify second output: entry(99.0) → 99.0*2=198.0, process(198.0) → 198.0+10=208.0
    {
        let output = last_output.lock().await;
        let confirm = output.as_ref().unwrap();
        assert!(confirm.published);
        assert_eq!(confirm.value, 208.0);
    }

    shutdown_tx.send(true).unwrap();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), acc_handle).await;
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), reactor_handle).await;
}

// =============================================================================
// Test 5: ReactiveScheduler — load graph, push via registry, verify fire
// =============================================================================

use cloacina::computation_graph::registry::EndpointRegistry;
use cloacina::computation_graph::scheduler::{
    AccumulatorDeclaration, AccumulatorFactory, ComputationGraphDeclaration, ReactiveScheduler,
    ReactorDeclaration,
};
use tokio::sync::mpsc as tokio_mpsc;
use tokio::task::JoinHandle;

struct TestAccumulatorFactory;

impl AccumulatorFactory for TestAccumulatorFactory {
    fn spawn(
        &self,
        name: String,
        boundary_tx: tokio_mpsc::Sender<(SourceName, Vec<u8>)>,
        shutdown_rx: tokio::sync::watch::Receiver<bool>,
    ) -> (tokio_mpsc::Sender<Vec<u8>>, JoinHandle<()>) {
        let (socket_tx, socket_rx) = tokio_mpsc::channel(64);

        struct Passthrough;

        #[async_trait::async_trait]
        impl cloacina::computation_graph::Accumulator for Passthrough {
            type Event = AlphaData;
            type Output = AlphaData;
            fn process(&mut self, event: AlphaData) -> Option<AlphaData> {
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
async fn test_reactive_scheduler_end_to_end() {
    let registry = EndpointRegistry::new();
    let scheduler = ReactiveScheduler::new(registry.clone());

    let fire_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fire_count_inner = fire_count.clone();

    let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
        let fc = fire_count_inner.clone();
        Box::pin(async move {
            fc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            linear_chain_compiled(&cache).await
        })
    });

    let decl = ComputationGraphDeclaration {
        name: "scheduler_test".to_string(),
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

    // Push event via registry (simulates WebSocket push)
    let event = AlphaData { value: 5.0 };
    registry
        .send_to_accumulator("alpha", serialize(&event).unwrap())
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert_eq!(
        fire_count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "graph should have fired via scheduler"
    );

    // Verify reactor is listed
    let graphs = scheduler.list_graphs().await;
    assert_eq!(graphs.len(), 1);
    assert_eq!(graphs[0].name, "scheduler_test");
    assert!(!graphs[0].reactor_paused);

    // Pause the reactor via handle
    let handle = registry.get_reactor_handle("scheduler_test").await.unwrap();
    handle.pause();
    assert!(handle.is_paused());

    // Push again — reactor is paused, should NOT fire
    registry
        .send_to_accumulator("alpha", serialize(&AlphaData { value: 10.0 }).unwrap())
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert_eq!(
        fire_count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "graph should NOT have fired while paused"
    );

    // Resume and force-fire
    handle.resume();
    use cloacina::computation_graph::reactor::ManualCommand;
    registry
        .send_to_reactor("scheduler_test", ManualCommand::ForceFire)
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert_eq!(
        fire_count.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "graph should have fired after resume + force-fire"
    );

    // Unload
    scheduler.unload_graph("scheduler_test").await.unwrap();

    // Registry should be empty
    assert!(registry.list_reactors().await.is_empty());
    assert_eq!(registry.accumulator_count("alpha").await, 0);
}

// =============================================================================
// Test 6: Polling accumulator → reactor → compiled graph
// =============================================================================

use cloacina::computation_graph::accumulator::polling_accumulator_runtime;
use cloacina::computation_graph::PollingAccumulator;

struct TestPoller {
    value: f64,
}

#[async_trait::async_trait]
impl PollingAccumulator for TestPoller {
    type Output = AlphaData;

    async fn poll(&mut self) -> Option<AlphaData> {
        self.value += 1.0;
        if self.value <= 3.0 {
            Some(AlphaData { value: self.value })
        } else {
            None // stop producing after 3 polls
        }
    }

    fn interval(&self) -> std::time::Duration {
        std::time::Duration::from_millis(50)
    }
}

#[tokio::test]
async fn test_polling_accumulator_to_reactor() {
    let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(10);
    let (_socket_tx, socket_rx) = tokio::sync::mpsc::channel(10);
    let (_manual_tx, manual_rx) = tokio::sync::mpsc::channel(10);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    let sender = BoundarySender::new(boundary_tx, SourceName::new("alpha"));
    let ctx = AccumulatorContext {
        output: sender,
        name: "alpha".to_string(),
        shutdown: shutdown_rx.clone(),
    };

    let _poll_handle = tokio::spawn(polling_accumulator_runtime(
        TestPoller { value: 0.0 },
        ctx,
        socket_rx,
    ));

    let fire_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fc = fire_count.clone();

    let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
        let fc = fc.clone();
        Box::pin(async move {
            fc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            linear_chain_compiled(&cache).await
        })
    });

    let reactor = Reactor::new(
        graph_fn,
        ReactionCriteria::WhenAny,
        InputStrategy::Latest,
        boundary_rx,
        manual_rx,
        shutdown_rx,
    );
    let _reactor_handle = tokio::spawn(reactor.run());

    // Wait for 3 polls (50ms each + first tick skip = ~200ms)
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    assert!(
        fire_count.load(std::sync::atomic::Ordering::SeqCst) >= 3,
        "polling accumulator should have fired graph at least 3 times, got {}",
        fire_count.load(std::sync::atomic::Ordering::SeqCst)
    );

    shutdown_tx.send(true).unwrap();
}

// =============================================================================
// Test 7: Batch accumulator → reactor → compiled graph
// =============================================================================

use cloacina::computation_graph::accumulator::{
    batch_accumulator_runtime, flush_signal, BatchAccumulatorConfig,
};
use cloacina::computation_graph::BatchAccumulator;

struct TestBatcher;

#[async_trait::async_trait]
impl BatchAccumulator for TestBatcher {
    type Event = AlphaData;
    type Output = AlphaData;

    fn process_batch(&mut self, events: Vec<AlphaData>) -> Option<AlphaData> {
        let sum: f64 = events.iter().map(|e| e.value).sum();
        Some(AlphaData { value: sum })
    }
}

#[tokio::test]
async fn test_batch_accumulator_to_reactor() {
    let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(10);
    let (socket_tx, socket_rx) = tokio::sync::mpsc::channel(10);
    let (flush_tx, flush_rx) = flush_signal();
    let (_manual_tx, manual_rx) = tokio::sync::mpsc::channel(10);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    let sender = BoundarySender::new(boundary_tx, SourceName::new("alpha"));
    let ctx = AccumulatorContext {
        output: sender,
        name: "alpha".to_string(),
        shutdown: shutdown_rx.clone(),
    };

    let config = BatchAccumulatorConfig::default(); // flush via signal, not timer

    let _batch_handle = tokio::spawn(batch_accumulator_runtime(
        TestBatcher,
        ctx,
        socket_rx,
        flush_rx,
        config,
    ));

    let fire_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fc = fire_count.clone();

    let last_output: Arc<tokio::sync::Mutex<Option<OutputConfirmation>>> =
        Arc::new(tokio::sync::Mutex::new(None));
    let lo = last_output.clone();

    let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
        let fc = fc.clone();
        let lo = lo.clone();
        Box::pin(async move {
            fc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let result = linear_chain_compiled(&cache).await;
            let captured = if let GraphResult::Completed { outputs } = &result {
                outputs
                    .iter()
                    .find_map(|o| o.downcast_ref::<OutputConfirmation>().cloned())
            } else {
                None
            };
            if let Some(c) = captured {
                *lo.lock().await = Some(c);
            }
            result
        })
    });

    let reactor = Reactor::new(
        graph_fn,
        ReactionCriteria::WhenAny,
        InputStrategy::Latest,
        boundary_rx,
        manual_rx,
        shutdown_rx,
    );
    let _reactor_handle = tokio::spawn(reactor.run());

    // Push 5 events quickly
    for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
        socket_tx
            .send(serialize(&AlphaData { value: v }).unwrap())
            .await
            .unwrap();
    }

    // Let events buffer, then send flush signal
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    flush_tx.send(()).await.unwrap();

    // Wait for flush + processing
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Batch sums to 15.0 → entry doubles to 30.0 → process adds 10 → 40.0
    assert_eq!(
        fire_count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "batch should produce exactly one boundary → one fire"
    );

    let output = last_output.lock().await;
    let confirm = output.as_ref().expect("should have terminal output");
    assert!(confirm.published);
    assert_eq!(confirm.value, 40.0); // (15.0 * 2) + 10 = 40.0

    shutdown_tx.send(true).unwrap();
}
