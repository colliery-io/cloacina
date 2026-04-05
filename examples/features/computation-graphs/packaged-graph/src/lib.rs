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

//! Packaged computation graph example — market maker.
//!
//! This demonstrates a computation graph compiled as a cdylib plugin
//! that can be loaded by the reconciler and executed via FFI.

use serde::{Deserialize, Serialize};

// --- Boundary types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookData {
    pub best_bid: f64,
    pub best_ask: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingData {
    pub mid_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignal {
    pub direction: String,
    pub price: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoActionReason {
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeConfirmation {
    pub executed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub logged: bool,
    pub reason: String,
}

// --- Computation graph ---

#[cloacina_macros::computation_graph(
    react = when_any(orderbook, pricing),
    graph = {
        decision(orderbook, pricing) => {
            Trade -> signal_handler,
            NoAction -> audit_logger,
        },
    }
)]
pub mod market_maker {
    use super::*;

    #[derive(Debug, Clone)]
    pub enum DecisionOutcome {
        Trade(TradeSignal),
        NoAction(NoActionReason),
    }

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
                format!("price divergence: {:.2}", price_diff)
            };
            DecisionOutcome::NoAction(NoActionReason { reason })
        }
    }

    pub async fn signal_handler(signal: &TradeSignal) -> TradeConfirmation {
        TradeConfirmation {
            executed: true,
            message: format!(
                "{} @ {:.2} (confidence: {:.4})",
                signal.direction, signal.price, signal.confidence
            ),
        }
    }

    pub async fn audit_logger(reason: &NoActionReason) -> AuditRecord {
        AuditRecord {
            logged: true,
            reason: reason.reason.clone(),
        }
    }
}
