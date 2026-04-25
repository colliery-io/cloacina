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

#[cloacina_macros::reactor(
    name = "linear_chain_reactor",
    accumulators = [alpha],
    criteria = when_any(alpha),
)]
pub struct LinearChainReactor;

#[cloacina_macros::computation_graph(
    trigger = reactor(LinearChainReactor),
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

#[cloacina_macros::reactor(
    name = "routing_decision_reactor",
    accumulators = [alpha, beta],
    criteria = when_any(alpha, beta),
)]
pub struct RoutingDecisionReactor;

#[cloacina_macros::computation_graph(
    trigger = reactor(RoutingDecisionReactor),
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
    type Output = AlphaData;

    fn process(&mut self, event: Vec<u8>) -> Option<AlphaData> {
        serde_json::from_slice(&event).ok()
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
        checkpoint: None,
        health: None,
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
    socket_tx
        .send(serde_json::to_vec(&event).unwrap())
        .await
        .unwrap();

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
        .send(serde_json::to_vec(&AlphaData { value: 99.0 }).unwrap())
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
// Test 5: ComputationGraphScheduler — load graph, push via registry, verify fire
// =============================================================================

use cloacina::computation_graph::registry::EndpointRegistry;
use cloacina::computation_graph::scheduler::{
    AccumulatorDeclaration, AccumulatorFactory, AccumulatorSpawnConfig,
    ComputationGraphDeclaration, ComputationGraphScheduler, ReactorDeclaration,
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
        config: AccumulatorSpawnConfig,
    ) -> (tokio_mpsc::Sender<Vec<u8>>, JoinHandle<()>) {
        let (socket_tx, socket_rx) = tokio_mpsc::channel(64);

        struct Passthrough;

        #[async_trait::async_trait]
        impl cloacina::computation_graph::Accumulator for Passthrough {
            type Output = AlphaData;
            fn process(&mut self, event: Vec<u8>) -> Option<AlphaData> {
                serde_json::from_slice(&event).ok()
            }
        }

        let sender = BoundarySender::new(boundary_tx, SourceName::new(&name));
        let ctx = AccumulatorContext {
            output: sender,
            name: name.clone(),
            shutdown: shutdown_rx,
            checkpoint: None,
            health: config.health_tx,
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
async fn test_computation_graph_scheduler_end_to_end() {
    let registry = EndpointRegistry::new();
    let scheduler = ComputationGraphScheduler::new(registry.clone());

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
        tenant_id: None,
    };

    scheduler.load_graph(decl).await.unwrap();

    // Push event via registry (simulates WebSocket push)
    let event = AlphaData { value: 5.0 };
    registry
        .send_to_accumulator("alpha", serde_json::to_vec(&event).unwrap())
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
    assert!(!graphs[0].paused);

    // Pause the reactor via handle
    let handle = registry.get_reactor_handle("scheduler_test").await.unwrap();
    handle.pause();
    assert!(handle.is_paused());

    // Push again — reactor is paused, should NOT fire
    registry
        .send_to_accumulator(
            "alpha",
            serde_json::to_vec(&AlphaData { value: 10.0 }).unwrap(),
        )
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
        checkpoint: None,
        health: None,
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
    type Output = AlphaData;

    fn process_batch(&mut self, events: Vec<Vec<u8>>) -> Option<AlphaData> {
        let parsed: Vec<AlphaData> = events
            .iter()
            .filter_map(|raw| serde_json::from_slice(raw).ok())
            .collect();
        let sum: f64 = parsed.iter().map(|e| e.value).sum();
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
        checkpoint: None,
        health: None,
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
            .send(serde_json::to_vec(&AlphaData { value: v }).unwrap())
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

// =============================================================================
// Test 8: WhenAll reaction criteria — waits until all sources emit
// =============================================================================

#[cloacina_macros::reactor(
    name = "when_all_graph_reactor",
    accumulators = [alpha, beta],
    criteria = when_all(alpha, beta),
)]
pub struct WhenAllGraphReactor;

#[cloacina_macros::computation_graph(
    trigger = reactor(WhenAllGraphReactor),
    graph = {
        combine(alpha, beta) -> output,
    }
)]
pub mod when_all_graph {
    use super::*;

    pub async fn combine(alpha: Option<&AlphaData>, beta: Option<&BetaData>) -> ProcessedData {
        let a = alpha.map(|a| a.value).unwrap_or(0.0);
        let b = beta.map(|b| b.estimate).unwrap_or(0.0);
        ProcessedData { result: a + b }
    }

    pub async fn output(input: &ProcessedData) -> OutputConfirmation {
        OutputConfirmation {
            published: true,
            value: input.result,
        }
    }
}

#[tokio::test]
async fn test_when_all_waits_for_both_sources() {
    let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(10);
    let (alpha_tx, alpha_rx) = tokio::sync::mpsc::channel(10);
    let (beta_tx, beta_rx) = tokio::sync::mpsc::channel(10);
    let (_manual_tx, manual_rx) = tokio::sync::mpsc::channel(10);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    // Spawn alpha accumulator
    let alpha_sender = BoundarySender::new(boundary_tx.clone(), SourceName::new("alpha"));
    let _alpha_handle = tokio::spawn(accumulator_runtime(
        TestPassthroughAccumulator,
        AccumulatorContext {
            output: alpha_sender,
            name: "alpha".to_string(),
            shutdown: shutdown_rx.clone(),
            checkpoint: None,
            health: None,
        },
        alpha_rx,
        AccumulatorRuntimeConfig::default(),
    ));

    // Spawn beta accumulator (reuse passthrough pattern with BetaData)
    struct BetaPassthrough;
    #[async_trait::async_trait]
    impl cloacina::computation_graph::Accumulator for BetaPassthrough {
        type Output = BetaData;
        fn process(&mut self, event: Vec<u8>) -> Option<BetaData> {
            serde_json::from_slice(&event).ok()
        }
    }

    let beta_sender = BoundarySender::new(boundary_tx, SourceName::new("beta"));
    let _beta_handle = tokio::spawn(accumulator_runtime(
        BetaPassthrough,
        AccumulatorContext {
            output: beta_sender,
            name: "beta".to_string(),
            shutdown: shutdown_rx.clone(),
            checkpoint: None,
            health: None,
        },
        beta_rx,
        AccumulatorRuntimeConfig::default(),
    ));

    // Reactor with WhenAll + expected sources
    let fire_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fc = fire_count.clone();

    let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
        let fc = fc.clone();
        Box::pin(async move {
            fc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            when_all_graph_compiled(&cache).await
        })
    });

    let reactor = Reactor::new(
        graph_fn,
        ReactionCriteria::WhenAll,
        InputStrategy::Latest,
        boundary_rx,
        manual_rx,
        shutdown_rx,
    )
    .with_expected_sources(vec![SourceName::new("alpha"), SourceName::new("beta")]);

    let _reactor_handle = tokio::spawn(reactor.run());

    // Push alpha only — should NOT fire (WhenAll requires both)
    alpha_tx
        .send(serde_json::to_vec(&AlphaData { value: 10.0 }).unwrap())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert_eq!(
        fire_count.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "should NOT fire with only alpha"
    );

    // Push beta — now both dirty → should fire
    beta_tx
        .send(serde_json::to_vec(&BetaData { estimate: 5.0 }).unwrap())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert_eq!(
        fire_count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "should fire once when both sources have emitted"
    );

    // Push alpha again — only alpha dirty → should NOT fire
    alpha_tx
        .send(serde_json::to_vec(&AlphaData { value: 20.0 }).unwrap())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert_eq!(
        fire_count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "should NOT fire with only alpha dirty after clear"
    );

    // Push beta again — both dirty again → fires
    beta_tx
        .send(serde_json::to_vec(&BetaData { estimate: 15.0 }).unwrap())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert_eq!(
        fire_count.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "should fire again when both sources emit again"
    );

    shutdown_tx.send(true).unwrap();
}

// =============================================================================
// Test 9: Sequential input strategy — every boundary fires separately
// =============================================================================

#[tokio::test]
async fn test_sequential_input_strategy() {
    let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(32);
    let (socket_tx, socket_rx) = tokio::sync::mpsc::channel(32);
    let (_manual_tx, manual_rx) = tokio::sync::mpsc::channel(10);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    let acc_sender = BoundarySender::new(boundary_tx, SourceName::new("alpha"));
    let acc_ctx = AccumulatorContext {
        output: acc_sender,
        name: "alpha".to_string(),
        shutdown: shutdown_rx.clone(),
        checkpoint: None,
        health: None,
    };

    let _acc_handle = tokio::spawn(accumulator_runtime(
        TestPassthroughAccumulator,
        acc_ctx,
        socket_rx,
        AccumulatorRuntimeConfig::default(),
    ));

    // Track each execution's output value
    let output_values: Arc<tokio::sync::Mutex<Vec<f64>>> =
        Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let ov = output_values.clone();

    let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
        let ov = ov.clone();
        Box::pin(async move {
            let result = linear_chain_compiled(&cache).await;
            let captured = if let GraphResult::Completed { ref outputs } = result {
                outputs
                    .iter()
                    .find_map(|o| o.downcast_ref::<OutputConfirmation>().map(|c| c.value))
            } else {
                None
            };
            if let Some(val) = captured {
                ov.lock().await.push(val);
            }
            result
        })
    });

    let reactor = Reactor::new(
        graph_fn,
        ReactionCriteria::WhenAny,
        InputStrategy::Sequential,
        boundary_rx,
        manual_rx,
        shutdown_rx,
    );
    let _reactor_handle = tokio::spawn(reactor.run());

    // Push 5 events rapidly — with Sequential, each should fire separately
    for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
        socket_tx
            .send(serde_json::to_vec(&AlphaData { value: v }).unwrap())
            .await
            .unwrap();
    }

    // Wait for all 5 to process
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let results = output_values.lock().await;
    assert_eq!(
        results.len(),
        5,
        "Sequential should fire 5 times for 5 boundaries, got {}",
        results.len()
    );

    // Verify order preserved: entry doubles, process adds 10
    // 1.0 → 2.0 → 12.0, 2.0 → 4.0 → 14.0, etc.
    assert_eq!(results[0], 12.0);
    assert_eq!(results[1], 14.0);
    assert_eq!(results[2], 16.0);
    assert_eq!(results[3], 18.0);
    assert_eq!(results[4], 20.0);

    shutdown_tx.send(true).unwrap();
}

// =============================================================================
// Resilience Tests (T-0414)
// These tests use in-memory SQLite for DAL, so they require the sqlite feature.
// =============================================================================
#[cfg(feature = "sqlite")]
mod resilience_tests {
    use super::*;

    /// Helper: create an in-memory SQLite DAL for testing.
    /// Uses shared-cache in-memory DB so the pool can have multiple connections
    /// to the same database without creating temp files on disk.
    async fn test_dal() -> cloacina::dal::unified::DAL {
        let url = format!(
            "file:resilience_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = cloacina::database::Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        cloacina::dal::unified::DAL::new(db)
    }

    #[tokio::test]
    async fn test_boundary_sender_sequence_numbers() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);
        let sender = cloacina::computation_graph::accumulator::BoundarySender::new(
            tx,
            SourceName::new("test"),
        );

        assert_eq!(sender.sequence_number(), 0);

        sender.send(&AlphaData { value: 1.0 }).await.unwrap();
        assert_eq!(sender.sequence_number(), 1);

        sender.send(&AlphaData { value: 2.0 }).await.unwrap();
        assert_eq!(sender.sequence_number(), 2);

        // Drain the channel
        let _ = rx.recv().await;
        let _ = rx.recv().await;
    }

    #[tokio::test]
    async fn test_boundary_sender_with_sequence_recovery() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);
        let sender = cloacina::computation_graph::accumulator::BoundarySender::with_sequence(
            tx,
            SourceName::new("test"),
            42,
        );

        assert_eq!(sender.sequence_number(), 42);

        sender.send(&AlphaData { value: 1.0 }).await.unwrap();
        assert_eq!(sender.sequence_number(), 43);

        let _ = rx.recv().await;
    }

    #[tokio::test]
    async fn test_accumulator_health_channel() {
        use cloacina::computation_graph::accumulator::{health_channel, AccumulatorHealth};

        let (tx, rx) = health_channel();

        // Initial state is Starting
        assert_eq!(*rx.borrow(), AccumulatorHealth::Starting);

        // Transition to Live
        tx.send(AccumulatorHealth::Live).unwrap();
        assert_eq!(*rx.borrow(), AccumulatorHealth::Live);

        // Transition to Disconnected
        tx.send(AccumulatorHealth::Disconnected).unwrap();
        assert_eq!(*rx.borrow(), AccumulatorHealth::Disconnected);

        // Back to Live
        tx.send(AccumulatorHealth::Live).unwrap();
        assert_eq!(*rx.borrow(), AccumulatorHealth::Live);
    }

    #[tokio::test]
    async fn test_checkpoint_dal_round_trip() {
        let dal = test_dal().await;

        // Save a checkpoint
        dal.checkpoint()
            .save_checkpoint("test_graph", "alpha", b"hello world".to_vec())
            .await
            .unwrap();

        // Load it back
        let loaded = dal
            .checkpoint()
            .load_checkpoint("test_graph", "alpha")
            .await
            .unwrap();
        assert_eq!(loaded, Some(b"hello world".to_vec()));

        // Non-existent returns None
        let missing = dal
            .checkpoint()
            .load_checkpoint("test_graph", "nonexistent")
            .await
            .unwrap();
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn test_checkpoint_dal_upsert() {
        let dal = test_dal().await;

        // Save initial
        dal.checkpoint()
            .save_checkpoint("g", "a", b"v1".to_vec())
            .await
            .unwrap();

        // Upsert with new data
        dal.checkpoint()
            .save_checkpoint("g", "a", b"v2".to_vec())
            .await
            .unwrap();

        // Should get v2
        let loaded = dal.checkpoint().load_checkpoint("g", "a").await.unwrap();
        assert_eq!(loaded, Some(b"v2".to_vec()));
    }

    #[tokio::test]
    async fn test_boundary_dal_with_sequence() {
        let dal = test_dal().await;

        // Save boundary with sequence 5
        dal.checkpoint()
            .save_boundary("g", "alpha", b"data1".to_vec(), 5)
            .await
            .unwrap();

        let loaded = dal.checkpoint().load_boundary("g", "alpha").await.unwrap();
        assert_eq!(loaded, Some((b"data1".to_vec(), 5)));

        // Upsert with higher sequence
        dal.checkpoint()
            .save_boundary("g", "alpha", b"data2".to_vec(), 10)
            .await
            .unwrap();

        let loaded = dal.checkpoint().load_boundary("g", "alpha").await.unwrap();
        assert_eq!(loaded, Some((b"data2".to_vec(), 10)));
    }

    #[tokio::test]
    async fn test_reactor_state_dal_round_trip() {
        let dal = test_dal().await;

        // Save reactor state
        dal.checkpoint()
            .save_reactor_state("test_graph", b"cache".to_vec(), b"flags".to_vec(), None)
            .await
            .unwrap();

        let loaded = dal
            .checkpoint()
            .load_reactor_state("test_graph")
            .await
            .unwrap();
        assert!(loaded.is_some());
        let (cache, flags, queue) = loaded.unwrap();
        assert_eq!(cache, b"cache");
        assert_eq!(flags, b"flags");
        assert_eq!(queue, None);
    }

    #[tokio::test]
    async fn test_reactor_state_dal_with_sequential_queue() {
        let dal = test_dal().await;

        dal.checkpoint()
            .save_reactor_state(
                "g",
                b"cache".to_vec(),
                b"flags".to_vec(),
                Some(b"queue_data".to_vec()),
            )
            .await
            .unwrap();

        let loaded = dal.checkpoint().load_reactor_state("g").await.unwrap();
        let (_, _, queue) = loaded.unwrap();
        assert_eq!(queue, Some(b"queue_data".to_vec()));
    }

    #[tokio::test]
    async fn test_state_buffer_dal_round_trip() {
        let dal = test_dal().await;

        dal.checkpoint()
            .save_state_buffer("g", "prev_outputs", b"[1,2,3]".to_vec(), 10)
            .await
            .unwrap();

        let loaded = dal
            .checkpoint()
            .load_state_buffer("g", "prev_outputs")
            .await
            .unwrap();
        assert_eq!(loaded, Some((b"[1,2,3]".to_vec(), 10)));
    }

    #[tokio::test]
    async fn test_delete_graph_state() {
        let dal = test_dal().await;

        // Populate all tables for a graph
        dal.checkpoint()
            .save_checkpoint("g", "a", b"cp".to_vec())
            .await
            .unwrap();
        dal.checkpoint()
            .save_boundary("g", "a", b"bd".to_vec(), 1)
            .await
            .unwrap();
        dal.checkpoint()
            .save_reactor_state("g", b"c".to_vec(), b"d".to_vec(), None)
            .await
            .unwrap();
        dal.checkpoint()
            .save_state_buffer("g", "s", b"buf".to_vec(), 5)
            .await
            .unwrap();

        // Delete all state for the graph
        dal.checkpoint().delete_graph_state("g").await.unwrap();

        // All should be None now
        assert_eq!(
            dal.checkpoint().load_checkpoint("g", "a").await.unwrap(),
            None
        );
        assert_eq!(
            dal.checkpoint().load_boundary("g", "a").await.unwrap(),
            None
        );
        assert_eq!(
            dal.checkpoint().load_reactor_state("g").await.unwrap(),
            None
        );
        assert_eq!(
            dal.checkpoint().load_state_buffer("g", "s").await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn test_checkpoint_handle_typed_round_trip() {
        let dal = test_dal().await;

        let handle = cloacina::computation_graph::accumulator::CheckpointHandle::new(
            dal,
            "test_graph".to_string(),
            "alpha".to_string(),
        );

        // Save a typed value
        let value = AlphaData { value: 42.0 };
        handle.save(&value).await.unwrap();

        // Load it back with correct type
        let loaded: Option<AlphaData> = handle.load().await.unwrap();
        assert_eq!(loaded, Some(AlphaData { value: 42.0 }));
    }

    #[tokio::test]
    async fn test_checkpoint_handle_load_empty() {
        let dal = test_dal().await;

        let handle = cloacina::computation_graph::accumulator::CheckpointHandle::new(
            dal,
            "test_graph".to_string(),
            "nonexistent".to_string(),
        );

        let loaded: Option<AlphaData> = handle.load().await.unwrap();
        assert_eq!(loaded, None);
    }

    // =============================================================================
    // T-0421: End-to-end resilience validation tests
    // =============================================================================

    use cloacina::computation_graph::accumulator::{
        health_channel, AccumulatorHealth, CheckpointHandle,
    };
    use cloacina::computation_graph::reactor::{reactor_health_channel, ReactorHealth};

    /// Test: Reactor cache persists to DAL and survives restart.
    ///
    /// Pushes events through an accumulator → reactor with DAL wired → verifies
    /// the reactor persists its cache → creates a new reactor with the same DAL →
    /// verifies the cache is restored and the graph can fire with restored state.
    #[tokio::test]
    async fn test_reactor_cache_recovery_across_restart() {
        let dal = test_dal().await;

        // --- First run: push events, reactor persists cache ---
        let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(10);
        let (socket_tx, socket_rx) = tokio::sync::mpsc::channel(10);
        let (_manual_tx, manual_rx) = tokio::sync::mpsc::channel(10);
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let acc_sender = BoundarySender::new(boundary_tx, SourceName::new("alpha"));
        let acc_ctx = AccumulatorContext {
            output: acc_sender,
            name: "alpha".to_string(),
            shutdown: shutdown_rx.clone(),
            checkpoint: Some(CheckpointHandle::new(
                dal.clone(),
                "recovery_test".to_string(),
                "alpha".to_string(),
            )),
            health: None,
        };

        let _acc_handle = tokio::spawn(accumulator_runtime(
            TestPassthroughAccumulator,
            acc_ctx,
            socket_rx,
            AccumulatorRuntimeConfig::default(),
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
            graph_fn.clone(),
            ReactionCriteria::WhenAny,
            InputStrategy::Latest,
            boundary_rx,
            manual_rx,
            shutdown_rx,
        )
        .with_graph_name("recovery_test".to_string())
        .with_dal(dal.clone());

        let _reactor_handle = tokio::spawn(reactor.run());

        // Push event → accumulator → reactor → graph fires → cache persisted
        socket_tx
            .send(serde_json::to_vec(&AlphaData { value: 7.0 }).unwrap())
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert_eq!(
            fire_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "graph should have fired once"
        );

        // Shut down cleanly (triggers final persist)
        shutdown_tx.send(true).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Verify cache was persisted in DAL
        let persisted = dal
            .checkpoint()
            .load_reactor_state("recovery_test")
            .await
            .unwrap();
        assert!(
            persisted.is_some(),
            "reactor state should be persisted in DAL"
        );

        // --- Second run: new reactor, same DAL → cache should be restored ---
        let (_boundary_tx2, boundary_rx2) = tokio::sync::mpsc::channel(10);
        let (_manual_tx2, manual_rx2) = tokio::sync::mpsc::channel(10);
        let (shutdown_tx2, shutdown_rx2) = shutdown_signal();

        let fire_count2 = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let fc2 = fire_count2.clone();
        let captured_cache: Arc<tokio::sync::Mutex<Option<InputCache>>> =
            Arc::new(tokio::sync::Mutex::new(None));
        let cc = captured_cache.clone();

        let graph_fn2: CompiledGraphFn = Arc::new(move |cache: InputCache| {
            let fc2 = fc2.clone();
            let cc = cc.clone();
            Box::pin(async move {
                fc2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                *cc.lock().await = Some(cache.clone());
                linear_chain_compiled(&cache).await
            })
        });

        let reactor2 = Reactor::new(
            graph_fn2,
            ReactionCriteria::WhenAny,
            InputStrategy::Latest,
            boundary_rx2,
            manual_rx2,
            shutdown_rx2,
        )
        .with_graph_name("recovery_test".to_string())
        .with_dal(dal.clone());

        let _reactor_handle2 = tokio::spawn(reactor2.run());

        // Give it a moment to load cache from DAL
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Force-fire to verify the restored cache has data
        _manual_tx2
            .send(cloacina::computation_graph::reactor::ManualCommand::ForceFire)
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        assert_eq!(
            fire_count2.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "graph should fire with restored cache"
        );

        // Verify the cache had the alpha source from the first run
        let snapshot = captured_cache.lock().await;
        assert!(
            snapshot.as_ref().unwrap().has("alpha"),
            "restored cache should contain 'alpha' from first run"
        );

        shutdown_tx2.send(true).unwrap();
    }

    /// Test: Health state machine transitions — Starting → Warming → Live.
    ///
    /// Creates a reactor with accumulator health channels, verifies it starts in
    /// Starting, transitions to Warming while waiting for accumulators, then to
    /// Live when all accumulators report healthy.
    #[tokio::test]
    async fn test_reactor_health_warming_to_live() {
        let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(10);
        let (_manual_tx, manual_rx) = tokio::sync::mpsc::channel(10);
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let graph_fn: CompiledGraphFn = Arc::new(|_cache: InputCache| {
            Box::pin(async { cloacina::computation_graph::types::GraphResult::completed(vec![]) })
        });

        let (reactor_health_tx, reactor_health_rx) = reactor_health_channel();

        // Create accumulator health channels (simulating 2 accumulators)
        let (alpha_health_tx, alpha_health_rx) = health_channel();
        let (beta_health_tx, beta_health_rx) = health_channel();

        let reactor = Reactor::new(
            graph_fn,
            ReactionCriteria::WhenAny,
            InputStrategy::Latest,
            boundary_rx,
            manual_rx,
            shutdown_rx,
        )
        .with_health(reactor_health_tx)
        .with_accumulator_health(vec![
            ("alpha".to_string(), alpha_health_rx),
            ("beta".to_string(), beta_health_rx),
        ]);

        // Reactor starts — should report Starting then Warming
        let _reactor_handle = tokio::spawn(reactor.run());

        // Give it a moment to enter the gating loop
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Should be Warming (both accumulators are still in Starting state)
        let health = reactor_health_rx.borrow().clone();
        assert!(
            matches!(health, ReactorHealth::Warming { .. }),
            "reactor should be Warming, got {:?}",
            health
        );

        // Alpha goes Live
        alpha_health_tx.send(AccumulatorHealth::Live).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Should still be Warming (beta not yet healthy)
        let health = reactor_health_rx.borrow().clone();
        assert!(
            matches!(health, ReactorHealth::Warming { .. }),
            "reactor should still be Warming with only alpha live, got {:?}",
            health
        );

        // Beta goes SocketOnly (passthrough — healthy by definition)
        beta_health_tx.send(AccumulatorHealth::SocketOnly).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Now both healthy ��� should be Live
        let health = reactor_health_rx.borrow().clone();
        assert_eq!(
            health,
            ReactorHealth::Live,
            "reactor should be Live when all accumulators healthy"
        );

        shutdown_tx.send(true).unwrap();
    }

    /// Test: Boundary sequence continuity across restart.
    ///
    /// Pushes events through an accumulator with DAL → verifies boundary+sequence
    /// persisted → creates new BoundarySender with restored sequence → verifies
    /// continuity (no gap).
    #[tokio::test]
    async fn test_boundary_sequence_continuity_across_restart() {
        let dal = test_dal().await;

        // --- First run: push 5 events, persist boundaries with sequence ---
        let (boundary_tx, mut boundary_rx) = tokio::sync::mpsc::channel(32);
        let (socket_tx, socket_rx) = tokio::sync::mpsc::channel(32);
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let checkpoint =
            CheckpointHandle::new(dal.clone(), "seq_test".to_string(), "alpha".to_string());

        let sender = BoundarySender::new(boundary_tx, SourceName::new("alpha"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "alpha".to_string(),
            shutdown: shutdown_rx.clone(),
            checkpoint: Some(checkpoint),
            health: None,
        };

        let _acc_handle = tokio::spawn(accumulator_runtime(
            TestPassthroughAccumulator,
            ctx,
            socket_rx,
            AccumulatorRuntimeConfig::default(),
        ));

        // Push 5 events
        for i in 1..=5 {
            socket_tx
                .send(serde_json::to_vec(&AlphaData { value: i as f64 }).unwrap())
                .await
                .unwrap();
        }

        // Drain receiver to let processing complete
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let mut received = 0;
        while boundary_rx.try_recv().is_ok() {
            received += 1;
        }
        assert_eq!(received, 5, "should receive all 5 boundaries");

        // Shut down
        shutdown_tx.send(true).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Check persisted boundary has sequence number
        let persisted = dal
            .checkpoint()
            .load_boundary("seq_test", "alpha")
            .await
            .unwrap();
        assert!(persisted.is_some(), "boundary should be persisted");
        let (_, seq) = persisted.unwrap();
        assert_eq!(seq, 5, "persisted sequence should be 5 after 5 events");

        // --- Second run: create new sender starting from persisted sequence ---
        let (boundary_tx2, mut boundary_rx2) = tokio::sync::mpsc::channel(32);
        let sender2 =
            BoundarySender::with_sequence(boundary_tx2, SourceName::new("alpha"), seq as u64);

        assert_eq!(
            sender2.sequence_number(),
            5,
            "restored sender should start at 5"
        );

        // Send one more event
        sender2.send(&AlphaData { value: 99.0 }).await.unwrap();
        assert_eq!(
            sender2.sequence_number(),
            6,
            "after one more send, sequence should be 6"
        );

        let _ = boundary_rx2.recv().await;
    }

    /// Test: State accumulator persists VecDeque to DAL and restores on restart.
    ///
    /// Writes values to a state accumulator → verifies persisted in DAL → restarts
    /// runtime with same DAL → verifies VecDeque loaded and initial boundary emitted.
    #[tokio::test]
    async fn test_state_accumulator_survives_restart() {
        use cloacina::computation_graph::accumulator::{
            state_accumulator_runtime, StateAccumulator,
        };

        let dal = test_dal().await;

        // --- First run: push 3 values into state accumulator ---
        let (boundary_tx, mut boundary_rx) = tokio::sync::mpsc::channel(32);
        let (socket_tx, socket_rx) = tokio::sync::mpsc::channel(32);
        let (shutdown_tx, shutdown_rx) = shutdown_signal();

        let checkpoint = CheckpointHandle::new(
            dal.clone(),
            "state_test".to_string(),
            "prev_outputs".to_string(),
        );

        let sender = BoundarySender::new(boundary_tx, SourceName::new("prev_outputs"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "prev_outputs".to_string(),
            shutdown: shutdown_rx,
            checkpoint: Some(checkpoint),
            health: None,
        };

        let acc = StateAccumulator::<AlphaData>::new(10); // capacity 10

        let _handle = tokio::spawn(state_accumulator_runtime(acc, ctx, socket_rx));

        // Push 3 values
        for v in [1.0, 2.0, 3.0] {
            socket_tx
                .send(serde_json::to_vec(&AlphaData { value: v }).unwrap())
                .await
                .unwrap();
        }

        // Wait for processing + persistence
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // Drain boundaries (each write emits the full list)
        let mut last_boundary: Option<Vec<AlphaData>> = None;
        while let Ok((_, bytes)) = boundary_rx.try_recv() {
            if let Ok(list) =
                cloacina::computation_graph::types::deserialize::<Vec<AlphaData>>(&bytes)
            {
                last_boundary = Some(list);
            }
        }

        // Last boundary should be the full list [1.0, 2.0, 3.0]
        let list = last_boundary.expect("should have received boundary");
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].value, 1.0);
        assert_eq!(list[1].value, 2.0);
        assert_eq!(list[2].value, 3.0);

        // Shut down
        shutdown_tx.send(true).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Verify persisted in DAL
        let persisted = dal
            .checkpoint()
            .load_state_buffer("state_test", "prev_outputs")
            .await
            .unwrap();
        assert!(persisted.is_some(), "state buffer should be persisted");

        // --- Second run: new state accumulator, same DAL → should restore ---
        let (boundary_tx2, mut boundary_rx2) = tokio::sync::mpsc::channel(32);
        let (_socket_tx2, socket_rx2) = tokio::sync::mpsc::channel(32);
        let (shutdown_tx2, shutdown_rx2) = shutdown_signal();

        let checkpoint2 = CheckpointHandle::new(
            dal.clone(),
            "state_test".to_string(),
            "prev_outputs".to_string(),
        );

        let sender2 = BoundarySender::new(boundary_tx2, SourceName::new("prev_outputs"));
        let ctx2 = AccumulatorContext {
            output: sender2,
            name: "prev_outputs".to_string(),
            shutdown: shutdown_rx2,
            checkpoint: Some(checkpoint2),
            health: None,
        };

        let acc2 = StateAccumulator::<AlphaData>::new(10);
        let _handle2 = tokio::spawn(state_accumulator_runtime(acc2, ctx2, socket_rx2));

        // On startup, state_accumulator_runtime loads from DAL and emits the list
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // Should receive the restored list as initial boundary
        let (_, bytes) = boundary_rx2
            .try_recv()
            .expect("should receive initial boundary from restored state");
        let restored: Vec<AlphaData> =
            cloacina::computation_graph::types::deserialize(&bytes).unwrap();

        assert_eq!(restored.len(), 3, "restored list should have 3 items");
        assert_eq!(restored[0].value, 1.0);
        assert_eq!(restored[1].value, 2.0);
        assert_eq!(restored[2].value, 3.0);

        shutdown_tx2.send(true).unwrap();
    }

    /// Test: Batch buffer survives crash via checkpoint.
    ///
    /// Buffers events in a batch accumulator → drops the runtime without flushing
    /// (simulating crash) → restarts with same DAL → verifies buffered events
    /// are restored from checkpoint.
    #[tokio::test]
    async fn test_batch_buffer_crash_recovery() {
        let dal = test_dal().await;

        // --- First run: buffer events, then "crash" (drop without flush) ---
        let (boundary_tx, _boundary_rx) = tokio::sync::mpsc::channel(32);
        let (socket_tx, socket_rx) = tokio::sync::mpsc::channel(32);
        let (_shutdown_tx, shutdown_rx) = shutdown_signal(); // never send shutdown = simulate crash

        let checkpoint = CheckpointHandle::new(
            dal.clone(),
            "batch_crash_test".to_string(),
            "batcher".to_string(),
        );

        let sender = BoundarySender::new(boundary_tx, SourceName::new("batcher"));
        let ctx = AccumulatorContext {
            output: sender,
            name: "batcher".to_string(),
            shutdown: shutdown_rx,
            checkpoint: Some(checkpoint),
            health: None,
        };

        struct SumBatcher;
        #[async_trait::async_trait]
        impl cloacina::computation_graph::BatchAccumulator for SumBatcher {
            type Output = AlphaData;
            fn process_batch(&mut self, events: Vec<Vec<u8>>) -> Option<AlphaData> {
                let parsed: Vec<AlphaData> = events
                    .iter()
                    .filter_map(|raw| serde_json::from_slice(raw).ok())
                    .collect();
                let sum: f64 = parsed.iter().map(|e| e.value).sum();
                Some(AlphaData { value: sum })
            }
        }

        let (_flush_tx, flush_rx) = cloacina::computation_graph::flush_signal();
        let config = cloacina::computation_graph::BatchAccumulatorConfig::default();

        let handle = tokio::spawn(cloacina::computation_graph::batch_accumulator_runtime(
            SumBatcher, ctx, socket_rx, flush_rx, config,
        ));

        // Push 4 events — they buffer without flushing
        for v in [10.0, 20.0, 30.0, 40.0] {
            socket_tx
                .send(serde_json::to_vec(&AlphaData { value: v }).unwrap())
                .await
                .unwrap();
        }

        // Wait for events to be buffered and checkpointed
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // "Crash" — abort the task without sending shutdown (no flush)
        handle.abort();
        let _ = handle.await; // wait for abort to complete

        // Verify the buffer was checkpointed in DAL
        let cp = dal
            .checkpoint()
            .load_checkpoint("batch_crash_test", "batcher")
            .await
            .unwrap();
        assert!(
            cp.is_some(),
            "batch buffer should be checkpointed even without flush"
        );

        // --- Second run: new batch accumulator, same DAL → buffer should restore ---
        let (boundary_tx2, mut boundary_rx2) = tokio::sync::mpsc::channel(32);
        let (_socket_tx2, socket_rx2) = tokio::sync::mpsc::channel(32);
        let (shutdown_tx2, shutdown_rx2) = shutdown_signal();

        let checkpoint2 = CheckpointHandle::new(
            dal.clone(),
            "batch_crash_test".to_string(),
            "batcher".to_string(),
        );

        let sender2 = BoundarySender::new(boundary_tx2, SourceName::new("batcher"));
        let ctx2 = AccumulatorContext {
            output: sender2,
            name: "batcher".to_string(),
            shutdown: shutdown_rx2,
            checkpoint: Some(checkpoint2),
            health: None,
        };

        let (flush_tx2, flush_rx2) = cloacina::computation_graph::flush_signal();
        let config2 = cloacina::computation_graph::BatchAccumulatorConfig::default();

        let _handle2 = tokio::spawn(cloacina::computation_graph::batch_accumulator_runtime(
            SumBatcher, ctx2, socket_rx2, flush_rx2, config2,
        ));

        // Wait for restore
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Flush the restored buffer
        flush_tx2.send(()).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Should get a boundary with the sum of the 4 restored events
        let (_, bytes) = boundary_rx2
            .try_recv()
            .expect("should receive boundary from restored+flushed buffer");
        let result: AlphaData = cloacina::computation_graph::types::deserialize(&bytes).unwrap();
        assert_eq!(
            result.value, 100.0,
            "restored batch should sum to 100 (10+20+30+40)"
        );

        shutdown_tx2.send(true).unwrap();
    }

    /// Test: Supervisor restarts crashed accumulator individually.
    ///
    /// Uses the ComputationGraphScheduler with an accumulator factory that produces
    /// an accumulator which panics after N events. Verifies the supervisor
    /// detects the crash and respawns the accumulator.
    #[tokio::test]
    async fn test_supervisor_individual_accumulator_restart() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let registry = EndpointRegistry::new();
        let scheduler = Arc::new(ComputationGraphScheduler::new(registry.clone()));

        let fire_count = Arc::new(AtomicU32::new(0));
        let fc = fire_count.clone();

        let graph_fn: CompiledGraphFn = Arc::new(move |_cache: InputCache| {
            let fc = fc.clone();
            Box::pin(async move {
                fc.fetch_add(1, Ordering::SeqCst);
                cloacina::computation_graph::types::GraphResult::completed(vec![])
            })
        });

        /// Factory that produces accumulators that panic after 2 events on first spawn,
        /// then work normally on subsequent spawns.
        struct PanicAfterTwoFactory {
            spawn_count: std::sync::atomic::AtomicU32,
        }

        impl AccumulatorFactory for PanicAfterTwoFactory {
            fn spawn(
                &self,
                name: String,
                boundary_tx: tokio_mpsc::Sender<(SourceName, Vec<u8>)>,
                shutdown_rx: tokio::sync::watch::Receiver<bool>,
                config: AccumulatorSpawnConfig,
            ) -> (tokio_mpsc::Sender<Vec<u8>>, JoinHandle<()>) {
                let (socket_tx, socket_rx) = tokio_mpsc::channel(64);
                let spawn_num = self
                    .spawn_count
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                struct MaybePanicAccumulator {
                    count: u32,
                    should_panic: bool,
                }

                #[async_trait::async_trait]
                impl cloacina::computation_graph::Accumulator for MaybePanicAccumulator {
                    type Output = AlphaData;
                    fn process(&mut self, event: Vec<u8>) -> Option<AlphaData> {
                        self.count += 1;
                        if self.should_panic && self.count >= 2 {
                            panic!("intentional test panic after 2 events");
                        }
                        serde_json::from_slice(&event).ok()
                    }
                }

                let sender = BoundarySender::new(boundary_tx, SourceName::new(&name));
                let ctx = AccumulatorContext {
                    output: sender,
                    name: name.clone(),
                    shutdown: shutdown_rx,
                    checkpoint: None,
                    health: config.health_tx,
                };

                let handle = tokio::spawn(accumulator_runtime(
                    MaybePanicAccumulator {
                        count: 0,
                        should_panic: spawn_num == 0, // only panic on first spawn
                    },
                    ctx,
                    socket_rx,
                    AccumulatorRuntimeConfig::default(),
                ));

                (socket_tx, handle)
            }
        }

        let decl = ComputationGraphDeclaration {
            name: "restart_test".to_string(),
            accumulators: vec![AccumulatorDeclaration {
                name: "alpha".to_string(),
                factory: Arc::new(PanicAfterTwoFactory {
                    spawn_count: AtomicU32::new(0),
                }),
            }],
            reactor: ReactorDeclaration {
                criteria: ReactionCriteria::WhenAny,
                strategy: InputStrategy::Latest,
                graph_fn,
            },
            tenant_id: None,
        };

        scheduler.load_graph(decl).await.unwrap();

        // Helper: poll a predicate until it's true, or panic on timeout. Used in
        // place of fixed `sleep` waits so this test stays deterministic under
        // CPU contention from parallel test execution (the original cause of
        // CLOACI-T-0530's intermittent failures in `angreal cloacina integration`).
        async fn poll_until<F: FnMut() -> bool>(
            mut pred: F,
            timeout: std::time::Duration,
            label: &str,
        ) {
            let deadline = std::time::Instant::now() + timeout;
            while std::time::Instant::now() < deadline {
                if pred() {
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            panic!("timed out waiting for: {}", label);
        }

        // Push event 1 and wait for the reactor to fire it. The reactor has a
        // ~100ms warming gate before going Live; under load this can be longer,
        // so poll instead of using a fixed sleep.
        let event = AlphaData { value: 1.0 };
        registry
            .send_to_accumulator("alpha", serde_json::to_vec(&event).unwrap())
            .await
            .unwrap();

        poll_until(
            || fire_count.load(Ordering::SeqCst) >= 1,
            std::time::Duration::from_secs(5),
            "first event to fire reactor",
        )
        .await;
        let fires_before = fire_count.load(Ordering::SeqCst);

        // Push event 2 — accumulator will panic on the 2nd process()
        registry
            .send_to_accumulator(
                "alpha",
                serde_json::to_vec(&AlphaData { value: 2.0 }).unwrap(),
            )
            .await
            .unwrap();

        // Trigger supervisor check — poll because the panic propagation through
        // the spawned task is async and `is_finished()` may not be true the
        // instant after `send_to_accumulator` returns.
        let mut restarted = 0;
        let restart_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < restart_deadline {
            restarted = scheduler.check_and_restart_failed().await;
            if restarted >= 1 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert_eq!(
            restarted, 1,
            "supervisor should detect and restart 1 accumulator"
        );

        // Push event 3 — the respawned accumulator (spawn_num=1) won't panic.
        // Poll for the next fire instead of relying on a fixed sleep.
        registry
            .send_to_accumulator(
                "alpha",
                serde_json::to_vec(&AlphaData { value: 3.0 }).unwrap(),
            )
            .await
            .unwrap();

        poll_until(
            || fire_count.load(Ordering::SeqCst) > fires_before,
            std::time::Duration::from_secs(5),
            "graph to fire after accumulator restart",
        )
        .await;

        scheduler.shutdown_all().await;
    }
} // mod resilience_tests

// =============================================================================
// Split-form and trigger-less macros (CLOACI-T-0538)
// =============================================================================
//
// These exercise the `#[reactor]` macro and the new `trigger = reactor(T)` /
// no-trigger forms of `#[computation_graph]`. End-to-end scheduler wiring
// for the split form is covered elsewhere (M5); these tests verify that the
// macros expand, type-bind at compile time, and land in the expected
// Runtime registries via inventory.

#[cloacina_macros::reactor(
    name = "cloaci_t_0538_reactor_split",
    accumulators = [alpha],
    criteria = when_any(alpha),
)]
pub struct CloaciT0538SplitReactor;

#[cloacina_macros::computation_graph(
    trigger = reactor(CloaciT0538SplitReactor),
    graph = {
        entry(alpha) -> output,
    }
)]
pub mod cloaci_t_0538_split_graph {
    use super::*;

    pub async fn entry(alpha: Option<&AlphaData>) -> ProcessedData {
        ProcessedData {
            result: alpha.map(|a| a.value).unwrap_or(0.0) * 3.0,
        }
    }

    pub async fn output(input: &ProcessedData) -> OutputConfirmation {
        OutputConfirmation {
            published: true,
            value: input.result,
        }
    }
}

#[cloacina_macros::computation_graph(graph = {
    entry(alpha) -> output,
})]
pub mod cloaci_t_0538_triggerless_graph {
    use super::*;

    pub async fn entry(alpha: Option<&AlphaData>) -> ProcessedData {
        ProcessedData {
            result: alpha.map(|a| a.value).unwrap_or(0.0) + 1.0,
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
async fn test_cloaci_t_0538_reactor_trait_constants() {
    use cloacina::{ComputationReactionMode, Reactor};
    assert_eq!(
        <CloaciT0538SplitReactor as Reactor>::NAME,
        "cloaci_t_0538_reactor_split"
    );
    assert_eq!(
        <CloaciT0538SplitReactor as Reactor>::ACCUMULATORS,
        &["alpha"]
    );
    assert_eq!(
        <CloaciT0538SplitReactor as Reactor>::REACTION_MODE,
        ComputationReactionMode::WhenAny
    );
}

#[tokio::test]
async fn test_cloaci_t_0540_graph_handle_consts() {
    // T-0540 M1: every #[computation_graph] emits a __CGHandle_<mod> unit
    // struct that implements `Graph` with NAME + IS_TRIGGERLESS consts.
    use cloacina::Graph;
    assert_eq!(
        <__CGHandle_cloaci_t_0538_split_graph as Graph>::NAME,
        "cloaci_t_0538_split_graph"
    );
    assert!(!<__CGHandle_cloaci_t_0538_split_graph as Graph>::IS_TRIGGERLESS);
    assert_eq!(
        <__CGHandle_cloaci_t_0538_triggerless_graph as Graph>::NAME,
        "cloaci_t_0538_triggerless_graph"
    );
    assert!(<__CGHandle_cloaci_t_0538_triggerless_graph as Graph>::IS_TRIGGERLESS);
}

#[tokio::test]
async fn test_cloaci_t_0538_split_form_compiled_fn_runs() {
    // Split-form graph: compiled fn is present and runs against an
    // InputCache identically to a bundled-form graph with the same
    // topology. This proves the macro expansion produced valid Rust
    // (including the compile-time subset-check const block referencing
    // `<CloaciT0538SplitReactor as Reactor>::ACCUMULATORS`).
    let mut cache = InputCache::new();
    cache.update(
        SourceName::new("alpha"),
        serialize(&AlphaData { value: 2.0 }).unwrap(),
    );
    let result = cloaci_t_0538_split_graph_compiled(&cache).await;
    assert!(result.is_completed(), "split-form graph should complete");
}

#[tokio::test]
async fn test_cloaci_t_0538_triggerless_form_compiled_fn_runs() {
    let mut cache = InputCache::new();
    cache.update(
        SourceName::new("alpha"),
        serialize(&AlphaData { value: 4.0 }).unwrap(),
    );
    let result = cloaci_t_0538_triggerless_graph_compiled(&cache).await;
    assert!(result.is_completed(), "trigger-less graph should complete");
}

#[tokio::test]
async fn test_cloaci_t_0538_split_form_scheduler_end_to_end() {
    // Exercise the new `ComputationGraphScheduler::load_graph_split` path
    // against a reactor registration (what `#[reactor]` emits) and a graph
    // compiled function (what the split-form `#[computation_graph]` emits).
    // Push an event through the accumulator registry and assert the graph
    // fires.
    use cloacina::computation_graph::registry::EndpointRegistry;
    use cloacina::computation_graph::scheduler::{
        AccumulatorDeclaration, ComputationGraphScheduler,
    };
    use cloacina::{ComputationReactionMode, ReactorRegistration};
    use cloacina_computation_graph::CompiledGraphFn;
    use std::sync::atomic::{AtomicU32, Ordering};

    let registry = EndpointRegistry::new();
    let scheduler = ComputationGraphScheduler::new(registry.clone());

    let fire_count = Arc::new(AtomicU32::new(0));
    let fire_count_inner = fire_count.clone();
    let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
        let fc = fire_count_inner.clone();
        Box::pin(async move {
            fc.fetch_add(1, Ordering::SeqCst);
            cloaci_t_0538_split_graph_compiled(&cache).await
        })
    });

    let reactor_reg = ReactorRegistration {
        name: "cloaci_t_0538_reactor_split".to_string(),
        accumulator_names: vec!["alpha".to_string()],
        reaction_mode: ComputationReactionMode::WhenAny,
    };

    scheduler
        .load_graph_split(
            "cloaci_t_0538_split_scheduler".to_string(),
            graph_fn,
            &reactor_reg,
            vec![AccumulatorDeclaration {
                name: "alpha".to_string(),
                factory: Arc::new(TestAccumulatorFactory),
            }],
            None,
        )
        .await
        .expect("load_graph_split should succeed");

    registry
        .send_to_accumulator(
            "alpha",
            serde_json::to_vec(&AlphaData { value: 7.0 }).unwrap(),
        )
        .await
        .unwrap();

    // Poll for the fire rather than a fixed sleep.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline && fire_count.load(Ordering::SeqCst) == 0 {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_eq!(
        fire_count.load(Ordering::SeqCst),
        1,
        "split-form graph should have fired via the reactor"
    );

    scheduler
        .unload_graph("cloaci_t_0538_split_scheduler")
        .await
        .expect("unload should succeed");
}

#[tokio::test]
async fn test_cloaci_t_0538_triggerless_scheduler_invocation() {
    use cloacina::computation_graph::registry::EndpointRegistry;
    use cloacina::computation_graph::scheduler::ComputationGraphScheduler;
    use cloacina_computation_graph::CompiledGraphFn;

    let registry = EndpointRegistry::new();
    let scheduler = ComputationGraphScheduler::new(registry);

    let graph_fn: CompiledGraphFn = Arc::new(|cache: InputCache| {
        Box::pin(async move { cloaci_t_0538_triggerless_graph_compiled(&cache).await })
    });

    scheduler
        .register_triggerless_graph("cloaci_t_0538_triggerless".to_string(), graph_fn.clone())
        .await
        .expect("register should succeed");

    assert!(scheduler
        .triggerless_graph_names()
        .await
        .iter()
        .any(|n| n == "cloaci_t_0538_triggerless"));

    // Duplicate registration is rejected.
    assert!(scheduler
        .register_triggerless_graph("cloaci_t_0538_triggerless".to_string(), graph_fn.clone(),)
        .await
        .is_err());

    // Direct invocation runs the compiled function.
    let mut cache = InputCache::new();
    cache.update(
        SourceName::new("alpha"),
        serialize(&AlphaData { value: 11.0 }).unwrap(),
    );
    let result = scheduler
        .invoke_triggerless_graph("cloaci_t_0538_triggerless", cache)
        .await
        .expect("invocation should return Some(result)");
    assert!(result.is_completed());

    // Unregister and confirm it's gone.
    assert!(
        scheduler
            .unregister_triggerless_graph("cloaci_t_0538_triggerless")
            .await
    );
    assert!(scheduler.triggerless_graph_names().await.is_empty());
    assert!(scheduler
        .invoke_triggerless_graph("cloaci_t_0538_triggerless", InputCache::new())
        .await
        .is_none());
}

#[tokio::test]
async fn test_cloaci_t_0538_split_missing_accumulator_fails() {
    use cloacina::computation_graph::registry::EndpointRegistry;
    use cloacina::computation_graph::scheduler::ComputationGraphScheduler;
    use cloacina::{ComputationReactionMode, ReactorRegistration};
    use cloacina_computation_graph::CompiledGraphFn;

    let registry = EndpointRegistry::new();
    let scheduler = ComputationGraphScheduler::new(registry);

    let graph_fn: CompiledGraphFn =
        Arc::new(|_cache: InputCache| Box::pin(async { unreachable!() }));

    // Reactor declares `alpha` but we supply no accumulators.
    let reactor_reg = ReactorRegistration {
        name: "cloaci_t_0538_broken_reactor".to_string(),
        accumulator_names: vec!["alpha".to_string()],
        reaction_mode: ComputationReactionMode::WhenAny,
    };

    let result = scheduler
        .load_graph_split(
            "cloaci_t_0538_broken".to_string(),
            graph_fn,
            &reactor_reg,
            vec![],
            None,
        )
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("accumulator 'alpha'"));
}

#[tokio::test]
async fn test_cloaci_t_0538_runtime_reactor_registry_shape() {
    // The `#[computation_graph]` and `#[reactor]` macros gate their
    // `inventory::submit!` entries with `#[cfg(not(test))]` for test
    // isolation, so we can't observe the emitted registrations here by
    // seeding from inventory. Instead, simulate what the macros would emit
    // and verify the Runtime registry surface.
    let rt = cloacina::Runtime::empty();

    rt.register_reactor("cloaci_t_0538_reactor_split".to_string(), || {
        cloacina::ReactorRegistration {
            name: "cloaci_t_0538_reactor_split".to_string(),
            accumulator_names: vec!["alpha".to_string()],
            reaction_mode: cloacina::ComputationReactionMode::WhenAny,
        }
    });

    assert_eq!(
        rt.reactor_names(),
        vec!["cloaci_t_0538_reactor_split".to_string()]
    );

    let reactor = rt.get_reactor("cloaci_t_0538_reactor_split").unwrap();
    assert_eq!(reactor.accumulator_names, vec!["alpha".to_string()]);
    assert_eq!(
        reactor.reaction_mode,
        cloacina::ComputationReactionMode::WhenAny
    );
}
