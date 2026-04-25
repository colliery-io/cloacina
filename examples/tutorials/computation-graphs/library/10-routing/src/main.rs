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

//! Tutorial 10: Routing and Enum Dispatch
//!
//! This tutorial introduces routing — the `=>` syntax that sends data down
//! conditional paths based on a decision function's return value. This is the
//! full market maker scenario: a decision engine takes order book + pricing data
//! and routes to either a signal handler (trade) or an audit logger (no action).
//!
//! Concepts covered:
//! - `=>` routing syntax in topology declaration
//! - Rust enum as routing type with variant data
//! - Multiple downstream paths from one decision node
//! - Terminal nodes on each branch
//! - How input values determine which path executes

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
pub struct OrderBookData {
    pub best_bid: f64,
    pub best_ask: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingData {
    pub mid_price: f64,
}

// ---------------------------------------------------------------------------
// Routing types — the decision engine returns one of these variants
// ---------------------------------------------------------------------------

/// Data carried when the decision is to trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignal {
    pub direction: String,
    pub price: f64,
    pub confidence: f64,
}

/// Data carried when the decision is no action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoActionReason {
    pub reason: String,
}

/// Terminal output from the signal handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeConfirmation {
    pub executed: bool,
    pub message: String,
}

/// Terminal output from the audit logger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub logged: bool,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// The computation graph with routing
//
// Topology uses `=>` for enum dispatch:
//   decision(orderbook, pricing) => {
//       Trade -> signal_handler,     // when decision returns Trade variant
//       NoAction -> audit_logger,    // when decision returns NoAction variant
//   }
//
// The decision function returns a Rust enum. The macro matches on the variant
// and routes the inner data to the corresponding downstream node.
// ---------------------------------------------------------------------------

#[cloacina_macros::reactor(
    name = "market_maker_reactor",
    accumulators = [orderbook, pricing],
    criteria = when_any(orderbook, pricing),
)]
pub struct MarketMakerReactor;

#[cloacina_macros::computation_graph(
    trigger = reactor(MarketMakerReactor),
    graph = {
        decision(orderbook, pricing) => {
            Trade -> signal_handler,
            NoAction -> audit_logger,
        },
    }
)]
pub mod market_maker {
    use super::*;

    /// The routing enum. Each variant carries data for its downstream node.
    #[derive(Debug, Clone)]
    pub enum DecisionOutcome {
        Trade(TradeSignal),
        NoAction(NoActionReason),
    }

    /// Decision engine: evaluates market data and decides whether to trade.
    ///
    /// This is the routing node — its return type is the enum above.
    pub async fn decision(
        orderbook: Option<&OrderBookData>,
        pricing: Option<&PricingData>,
    ) -> DecisionOutcome {
        let (bid, ask) = match orderbook {
            Some(ob) => (ob.best_bid, ob.best_ask),
            None => {
                return DecisionOutcome::NoAction(NoActionReason {
                    reason: "no order book data".to_string(),
                });
            }
        };

        let mid = (bid + ask) / 2.0;
        let spread = ask - bid;
        let pricing_mid = pricing.map(|p| p.mid_price).unwrap_or(mid);

        // Trade if spread is tight and pricing confirms
        let price_diff = (mid - pricing_mid).abs();
        if spread < 0.20 && price_diff < 0.50 {
            DecisionOutcome::Trade(TradeSignal {
                direction: if pricing_mid > mid {
                    "BUY".to_string()
                } else {
                    "SELL".to_string()
                },
                price: mid,
                confidence: 1.0 - (price_diff / mid),
            })
        } else {
            let reason = if spread >= 0.20 {
                format!("spread too wide: {:.2}", spread)
            } else {
                format!("price divergence too high: {:.2}", price_diff)
            };
            DecisionOutcome::NoAction(NoActionReason { reason })
        }
    }

    /// Signal handler: executes the trade (terminal node on Trade path).
    pub async fn signal_handler(signal: &TradeSignal) -> TradeConfirmation {
        TradeConfirmation {
            executed: true,
            message: format!(
                "{} @ {:.2} (confidence: {:.4})",
                signal.direction, signal.price, signal.confidence
            ),
        }
    }

    /// Audit logger: records why no action was taken (terminal on NoAction path).
    pub async fn audit_logger(reason: &NoActionReason) -> AuditRecord {
        AuditRecord {
            logged: true,
            reason: reason.reason.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Accumulators + reactor wiring (same pattern as Tutorial 09)
// ---------------------------------------------------------------------------

struct OrderBookAccumulator;

#[async_trait::async_trait]
impl cloacina::computation_graph::Accumulator for OrderBookAccumulator {
    type Output = OrderBookData;
    fn process(&mut self, event: Vec<u8>) -> Option<OrderBookData> {
        cloacina::computation_graph::types::deserialize(&event).ok()
    }
}

struct PricingAccumulator;

#[async_trait::async_trait]
impl cloacina::computation_graph::Accumulator for PricingAccumulator {
    type Output = PricingData;
    fn process(&mut self, event: Vec<u8>) -> Option<PricingData> {
        cloacina::computation_graph::types::deserialize(&event).ok()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    println!("=== Tutorial 10: Routing and Enum Dispatch ===\n");

    let (boundary_tx, boundary_rx) = tokio::sync::mpsc::channel(32);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    // Accumulator 1: Order Book
    let (ob_tx, ob_rx) = tokio::sync::mpsc::channel(10);
    let _ob = tokio::spawn(accumulator_runtime(
        OrderBookAccumulator,
        AccumulatorContext {
            output: BoundarySender::new(boundary_tx.clone(), SourceName::new("orderbook")),
            name: "orderbook".to_string(),
            shutdown: shutdown_rx.clone(),
            checkpoint: None,
            health: None,
        },
        ob_rx,
        AccumulatorRuntimeConfig::default(),
    ));

    // Accumulator 2: Pricing
    let (pr_tx, pr_rx) = tokio::sync::mpsc::channel(10);
    let _pr = tokio::spawn(accumulator_runtime(
        PricingAccumulator,
        AccumulatorContext {
            output: BoundarySender::new(boundary_tx, SourceName::new("pricing")),
            name: "pricing".to_string(),
            shutdown: shutdown_rx.clone(),
            checkpoint: None,
            health: None,
        },
        pr_rx,
        AccumulatorRuntimeConfig::default(),
    ));

    // Reactor
    let (_manual_tx, manual_rx) = tokio::sync::mpsc::channel(10);
    let fire_count = Arc::new(AtomicU32::new(0));
    let fc = fire_count.clone();

    let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
        let fc = fc.clone();
        Box::pin(async move {
            fc.fetch_add(1, Ordering::SeqCst);
            let result = market_maker_compiled(&cache).await;
            if let GraphResult::Completed { outputs } = &result {
                for output in outputs {
                    if let Some(confirm) = output.downcast_ref::<TradeConfirmation>() {
                        println!("    [TRADE] {}", confirm.message);
                    }
                    if let Some(audit) = output.downcast_ref::<AuditRecord>() {
                        println!("    [NO ACTION] {}", audit.reason);
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
    let _reactor = tokio::spawn(reactor.run());

    println!("Market maker running: decision engine with Trade/NoAction routing\n");

    // --- Scenario 1: No order book → NoAction ---
    println!("1. Push pricing only (no order book yet):");
    pr_tx
        .send(serialize(&PricingData { mid_price: 100.05 }).unwrap())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // --- Scenario 2: Tight spread + confirmed pricing → Trade ---
    println!("\n2. Push tight order book (spread=0.10) + pricing confirms:");
    ob_tx
        .send(
            serialize(&OrderBookData {
                best_bid: 100.00,
                best_ask: 100.10,
            })
            .unwrap(),
        )
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // --- Scenario 3: Wide spread → NoAction ---
    println!("\n3. Push wide order book (spread=1.00):");
    ob_tx
        .send(
            serialize(&OrderBookData {
                best_bid: 99.50,
                best_ask: 100.50,
            })
            .unwrap(),
        )
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // --- Scenario 4: Tight spread, divergent pricing → NoAction ---
    println!("\n4. Push tight order book but divergent pricing:");
    ob_tx
        .send(
            serialize(&OrderBookData {
                best_bid: 100.00,
                best_ask: 100.10,
            })
            .unwrap(),
        )
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    pr_tx
        .send(serialize(&PricingData { mid_price: 105.00 }).unwrap())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // --- Scenario 5: Everything aligned → Trade ---
    println!("\n5. Push aligned data (tight spread + confirmed):");
    ob_tx
        .send(
            serialize(&OrderBookData {
                best_bid: 102.00,
                best_ask: 102.08,
            })
            .unwrap(),
        )
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    pr_tx
        .send(serialize(&PricingData { mid_price: 102.05 }).unwrap())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // Shutdown
    println!("\nShutting down...");
    shutdown_tx.send(true).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    println!("Total fires: {}", fire_count.load(Ordering::SeqCst));
    println!("\n=== Tutorial 10 Complete ===");
}
