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

//! Tutorial 09: Full Reactive Pipeline
//!
//! This tutorial wires the full reactive pipeline: two accumulators feeding one
//! reactor. One is a passthrough accumulator (pricing updates via socket), the
//! other simulates a stream accumulator (order book via socket). The reactor
//! fires on `when_any` — meaning any new data from either source triggers the
//! graph.
//!
//! Concepts covered:
//! - Multiple accumulators feeding one reactor
//! - `ReactionCriteria::WhenAny` — fire when any source has new data
//! - `InputStrategy::Latest` — cache overwrites intermediate values
//! - Multiple boundary senders sharing one boundary channel
//! - Pushing to different sources and watching the reactor fire each time

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
// Boundary types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookUpdate {
    pub best_bid: f64,
    pub best_ask: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingUpdate {
    pub mid_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketView {
    pub spread: f64,
    pub mid_price: f64,
    pub pricing_mid: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingSignal {
    pub action: String,
    pub confidence: f64,
}

// ---------------------------------------------------------------------------
// Computation graph: takes both orderbook and pricing inputs
// ---------------------------------------------------------------------------

#[cloacina_macros::reactor(
    name = "market_pipeline_reactor",
    accumulators = [orderbook, pricing],
    criteria = when_any(orderbook, pricing),
)]
pub struct MarketPipelineReactor;

#[cloacina_macros::computation_graph(
    trigger = reactor(MarketPipelineReactor),
    graph = {
        combine(orderbook, pricing) -> evaluate,
        evaluate -> signal,
    }
)]
pub mod market_pipeline {
    use super::*;

    /// Entry node: combines data from both sources.
    /// Inputs are `Option<&T>` — may be None if that source hasn't emitted yet.
    pub async fn combine(
        orderbook: Option<&OrderBookUpdate>,
        pricing: Option<&PricingUpdate>,
    ) -> MarketView {
        let (spread, mid) = match orderbook {
            Some(ob) => (ob.best_ask - ob.best_bid, (ob.best_ask + ob.best_bid) / 2.0),
            None => (0.0, 0.0),
        };
        let pricing_mid = pricing.map(|p| p.mid_price).unwrap_or(0.0);

        MarketView {
            spread,
            mid_price: mid,
            pricing_mid,
        }
    }

    /// Evaluate the combined market view.
    pub async fn evaluate(view: &MarketView) -> TradingSignal {
        let confidence = if view.spread > 0.0 && view.pricing_mid > 0.0 {
            let diff = (view.mid_price - view.pricing_mid).abs();
            1.0 - (diff / view.mid_price).min(1.0)
        } else {
            0.0
        };

        let action = if confidence > 0.8 {
            "TRADE".to_string()
        } else if confidence > 0.5 {
            "MONITOR".to_string()
        } else {
            "WAIT".to_string()
        };

        TradingSignal { action, confidence }
    }

    /// Terminal node: formats the signal.
    pub async fn signal(input: &TradingSignal) -> TradingSignal {
        input.clone()
    }
}

// ---------------------------------------------------------------------------
// Accumulators
// ---------------------------------------------------------------------------

struct OrderBookAccumulator;

#[async_trait::async_trait]
impl cloacina::computation_graph::Accumulator for OrderBookAccumulator {
    type Output = OrderBookUpdate;

    fn process(&mut self, event: Vec<u8>) -> Option<OrderBookUpdate> {
        cloacina::computation_graph::types::deserialize(&event).ok()
    }
}

struct PricingAccumulator;

#[async_trait::async_trait]
impl cloacina::computation_graph::Accumulator for PricingAccumulator {
    type Output = PricingUpdate;

    fn process(&mut self, event: Vec<u8>) -> Option<PricingUpdate> {
        cloacina::computation_graph::types::deserialize(&event).ok()
    }
}

// ---------------------------------------------------------------------------
// Wire it all together
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    println!("=== Tutorial 09: Full Reactive Pipeline ===\n");

    // Shared boundary channel — both accumulators send to the same reactor
    let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(32);

    // Shutdown signal
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    // --- Accumulator 1: Order Book ---
    let (ob_socket_tx, ob_socket_rx) = tokio::sync::mpsc::channel(10);
    let ob_sender = BoundarySender::new(boundary_tx.clone(), SourceName::new("orderbook"));
    let ob_ctx = AccumulatorContext {
        output: ob_sender,
        name: "orderbook".to_string(),
        shutdown: shutdown_rx.clone(),
        checkpoint: None,
        health: None,
    };
    let _ob_handle = tokio::spawn(accumulator_runtime(
        OrderBookAccumulator,
        ob_ctx,
        ob_socket_rx,
        AccumulatorRuntimeConfig::default(),
    ));

    // --- Accumulator 2: Pricing ---
    let (pr_socket_tx, pr_socket_rx) = tokio::sync::mpsc::channel(10);
    let pr_sender = BoundarySender::new(boundary_tx, SourceName::new("pricing"));
    let pr_ctx = AccumulatorContext {
        output: pr_sender,
        name: "pricing".to_string(),
        shutdown: shutdown_rx.clone(),
        checkpoint: None,
        health: None,
    };
    let _pr_handle = tokio::spawn(accumulator_runtime(
        PricingAccumulator,
        pr_ctx,
        pr_socket_rx,
        AccumulatorRuntimeConfig::default(),
    ));

    // --- Reactor ---
    let (_manual_tx, manual_rx) = tokio::sync::mpsc::channel(10);

    let fire_count = Arc::new(AtomicU32::new(0));
    let fire_count_inner = fire_count.clone();

    let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
        let fc = fire_count_inner.clone();
        Box::pin(async move {
            fc.fetch_add(1, Ordering::SeqCst);
            let result = market_pipeline_compiled(&cache).await;
            // Print the terminal output
            if let GraphResult::Completed { outputs } = &result {
                for output in outputs {
                    if let Some(signal) = output.downcast_ref::<TradingSignal>() {
                        println!(
                            "    -> Signal: {} (confidence: {:.2})",
                            signal.action, signal.confidence
                        );
                    }
                }
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

    println!("Pipeline running: 2 accumulators → 1 reactor → compiled graph\n");

    // --- Push events from different sources ---

    // Push order book first — pricing is still None
    println!("1. Push order book (pricing not yet available):");
    ob_socket_tx
        .send(
            serialize(&OrderBookUpdate {
                best_bid: 100.0,
                best_ask: 100.10,
            })
            .unwrap(),
        )
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    println!("   Fires: {}\n", fire_count.load(Ordering::SeqCst));

    // Push pricing — now both sources have data
    println!("2. Push pricing (both sources now available):");
    pr_socket_tx
        .send(serialize(&PricingUpdate { mid_price: 100.05 }).unwrap())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    println!("   Fires: {}\n", fire_count.load(Ordering::SeqCst));

    // Push updated order book — reactor fires again with latest from both
    println!("3. Push updated order book (wider spread):");
    ob_socket_tx
        .send(
            serialize(&OrderBookUpdate {
                best_bid: 99.50,
                best_ask: 100.50,
            })
            .unwrap(),
        )
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    println!("   Fires: {}\n", fire_count.load(Ordering::SeqCst));

    // Push another pricing update
    println!("4. Push pricing update (divergent from order book):");
    pr_socket_tx
        .send(serialize(&PricingUpdate { mid_price: 105.00 }).unwrap())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    println!("   Fires: {}\n", fire_count.load(Ordering::SeqCst));

    // --- Shutdown ---
    println!("Shutting down...");
    shutdown_tx.send(true).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    println!("Total fires: {}", fire_count.load(Ordering::SeqCst));
    println!("\n=== Tutorial 09 Complete ===");
}
