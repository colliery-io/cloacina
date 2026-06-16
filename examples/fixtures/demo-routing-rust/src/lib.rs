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

//! Demo routing computation-graph fixture (CLOACI-I-0124 / WS-8).
//!
//! Ported from the `10-routing` tutorial: a `decision` node returns a Rust
//! enum and the graph's `=>` routing syntax dispatches each variant to a
//! different downstream branch — `Trade -> signal_handler` or
//! `NoAction -> audit_logger`. Gives the UI's Graphs view real branching
//! structure (one node fanning out to two terminal paths). The standalone
//! tutorial's accumulator-runtime / `main()` wiring is dropped.

use cloacina_macros::reactor;
use cloacina_workflow_plugin as cloacina_plugin;
use serde::{Deserialize, Serialize};

cloacina_plugin::package!();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookData {
    pub best_bid: f64,
    pub best_ask: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingData {
    pub mid_price: f64,
}

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

#[reactor(
    name = "market_maker_reactor",
    accumulators = [orderbook, pricing],
    criteria = when_any(orderbook, pricing),
)]
pub struct MarketMakerReactor;

#[cloacina_macros::computation_graph(
    trigger = reactor("market_maker_reactor"),
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

    /// Decision engine (the routing node): evaluates market data and returns
    /// the enum the macro matches on to choose the downstream branch.
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

    /// Trade branch terminal: executes the trade.
    pub async fn signal_handler(signal: &TradeSignal) -> TradeConfirmation {
        TradeConfirmation {
            executed: true,
            message: format!(
                "{} @ {:.2} (confidence: {:.4})",
                signal.direction, signal.price, signal.confidence
            ),
        }
    }

    /// No-action branch terminal: records why no action was taken.
    pub async fn audit_logger(reason: &NoActionReason) -> AuditRecord {
        AuditRecord {
            logged: true,
            reason: reason.reason.clone(),
        }
    }
}
