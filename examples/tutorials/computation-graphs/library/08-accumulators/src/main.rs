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

//! Tutorial 08: Accumulators
//!
//! This tutorial introduces accumulators — the data sources that feed computation
//! graphs. You'll implement a passthrough accumulator, create the runtime with
//! explicit channel plumbing, spawn it as a tokio task, and push events through
//! a reactor into a compiled graph.
//!
//! Concepts covered:
//! - `Accumulator` trait: `process()`, Event/Output associated types
//! - `AccumulatorContext`, `BoundarySender`, `AccumulatorRuntimeConfig`
//! - `accumulator_runtime()` — spawning the 3-task merge channel model
//! - `shutdown_signal()` for graceful shutdown
//! - Channel plumbing: socket, boundary, manual, shutdown
//! - `Reactor` with `WhenAny` + `Latest`
//! - Pushing serialized events and watching the graph fire

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use cloacina::computation_graph::accumulator::{
    accumulator_runtime, shutdown_signal, AccumulatorContext, AccumulatorRuntimeConfig,
    BoundarySender,
};
use cloacina::computation_graph::reactor::{
    CompiledGraphFn, InputStrategy, ReactionCriteria, Reactor,
};
use cloacina::computation_graph::types::{serialize, GraphResult, InputCache, SourceName};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Step 1: Define boundary types (same as Tutorial 07)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingUpdate {
    pub mid_price: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingSignal {
    pub price: f64,
    pub change_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalOutput {
    pub message: String,
}

// ---------------------------------------------------------------------------
// Step 2: Define the computation graph (reuses Tutorial 07 pattern)
// ---------------------------------------------------------------------------

#[cloacina_macros::reactor(
    name = "pricing_graph_reactor",
    accumulators = [pricing],
    criteria = when_any(pricing),
)]
pub struct PricingGraphReactor;

#[cloacina_macros::computation_graph(
    trigger = reactor(PricingGraphReactor),
    graph = {
        ingest(pricing) -> analyze,
        analyze -> format_signal,
    }
)]
pub mod pricing_graph {
    use super::*;

    pub async fn ingest(pricing: Option<&PricingSignal>) -> PricingSignal {
        pricing.expect("pricing data should be present").clone()
    }

    pub async fn analyze(input: &PricingSignal) -> PricingSignal {
        // Flag large moves
        let change_pct = if input.price > 100.0 {
            ((input.price - 100.0) / 100.0) * 100.0
        } else {
            0.0
        };
        PricingSignal {
            price: input.price,
            change_pct,
        }
    }

    pub async fn format_signal(input: &PricingSignal) -> SignalOutput {
        SignalOutput {
            message: format!(
                "Price: {:.2}, Change: {:.2}%",
                input.price, input.change_pct
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Step 3: Implement a passthrough accumulator
//
// The Accumulator trait has two associated types:
//   - Event: what the accumulator receives (from external sources)
//   - Output: what it emits (to the reactor via BoundarySender)
//
// `process()` transforms an event into an output. For a passthrough
// accumulator, we just convert the event type.
// ---------------------------------------------------------------------------

struct PricingAccumulator;

#[async_trait::async_trait]
impl cloacina::computation_graph::Accumulator for PricingAccumulator {
    type Output = PricingSignal;

    fn process(&mut self, event: Vec<u8>) -> Option<PricingSignal> {
        // Deserialize the incoming bytes — we know the sender uses types::serialize (bincode)
        let update: PricingUpdate = cloacina::computation_graph::types::deserialize(&event).ok()?;
        // Passthrough: convert PricingUpdate → PricingSignal
        Some(PricingSignal {
            price: update.mid_price,
            change_pct: 0.0, // raw — analysis happens in the graph
        })
    }
}

// ---------------------------------------------------------------------------
// Step 4: Wire everything together in main()
//
// This is the explicit channel plumbing that teaches how the pieces connect:
//   1. Create channels: socket (external→accumulator), boundary (accumulator→reactor)
//   2. Create shutdown signal
//   3. Spawn accumulator runtime
//   4. Create and spawn reactor
//   5. Push events via socket channel
//   6. Observe graph firing
//   7. Shut down gracefully
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    println!("=== Tutorial 08: Accumulators ===\n");

    // --- Channel plumbing ---

    // Boundary channel: accumulator sends serialized data to reactor
    let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(10);

    // Socket channel: external code pushes raw events to accumulator
    let (socket_tx, socket_rx) = tokio::sync::mpsc::channel(10);

    // Manual command channel for reactor (unused here, but required)
    let (_manual_tx, manual_rx) = tokio::sync::mpsc::channel(10);

    // Shutdown signal — shared by all components
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    // --- Spawn the accumulator ---

    let boundary_sender = BoundarySender::new(boundary_tx, SourceName::new("pricing"));
    let acc_ctx = AccumulatorContext {
        output: boundary_sender,
        name: "pricing".to_string(),
        shutdown: shutdown_rx.clone(),
        checkpoint: None,
        health: None,
    };

    println!("Spawning accumulator runtime...");
    let _acc_handle = tokio::spawn(accumulator_runtime(
        PricingAccumulator,
        acc_ctx,
        socket_rx,
        AccumulatorRuntimeConfig::default(),
    ));

    // --- Create the reactor ---

    // Track how many times the graph fires
    let fire_count = Arc::new(AtomicU32::new(0));
    let fire_count_inner = fire_count.clone();

    let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
        let fc = fire_count_inner.clone();
        Box::pin(async move {
            fc.fetch_add(1, Ordering::SeqCst);
            pricing_graph_compiled(&cache).await
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

    println!("Spawning reactor...\n");
    let _reactor_handle = tokio::spawn(reactor.run());

    // --- Push events ---

    let events = vec![
        PricingUpdate {
            mid_price: 99.50,
            timestamp: 1,
        },
        PricingUpdate {
            mid_price: 101.25,
            timestamp: 2,
        },
        PricingUpdate {
            mid_price: 103.75,
            timestamp: 3,
        },
    ];

    for (i, event) in events.iter().enumerate() {
        println!("Pushing event {}: {:?}", i + 1, event);
        let bytes = serialize(event).expect("serialization should succeed");
        socket_tx
            .send(bytes)
            .await
            .expect("socket send should succeed");

        // Give the pipeline time to process
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        println!(
            "  Graph has fired {} time(s)\n",
            fire_count.load(Ordering::SeqCst)
        );
    }

    // --- Shutdown ---

    println!("Shutting down...");
    shutdown_tx.send(true).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    println!(
        "\nTotal graph executions: {}",
        fire_count.load(Ordering::SeqCst)
    );
    println!("\n=== Tutorial 08 Complete ===");
}
