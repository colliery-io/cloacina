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

//! Demo full-pipeline computation-graph fixture (CLOACI-I-0124 / WS-8).
//!
//! Ported from the `09-full-pipeline` tutorial: two accumulators (`orderbook`,
//! `pricing`) feed one reactor that fires on `when_any`, then a three-node
//! pipeline `combine → evaluate → signal`. Gives the UI's Graphs view real
//! multi-source fan-in structure to render. The standalone tutorial's
//! accumulator-runtime / `main()` wiring is dropped — in a packaged graph the
//! accumulators are declared by the macro and sourced via sockets.

use cloacina_macros::reactor;
use cloacina_workflow_plugin as cloacina_plugin;
use serde::{Deserialize, Serialize};

cloacina_plugin::package!();

// CLOACI-T-0768: deriving schemars::JsonSchema opts these accumulator boundary
// types into the typed inject/fire interface (rich slot schemas instead of {}).
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OrderBookUpdate {
    pub best_bid: f64,
    pub best_ask: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
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

/// Two accumulators feeding one reactor; `when_any` fires the graph as soon as
/// either source has new data.
#[reactor(
    name = "market_pipeline_reactor",
    accumulators = [orderbook, pricing],
    criteria = when_any(orderbook, pricing),
)]
pub struct MarketPipelineReactor;

#[cloacina_macros::computation_graph(
    trigger = reactor("market_pipeline_reactor"),
    graph = {
        combine(orderbook, pricing) -> evaluate,
        evaluate -> signal,
    }
)]
pub mod market_pipeline {
    use super::*;

    /// Entry node: combines both sources. Inputs are `Option<&T>` — either may
    /// be `None` if that source hasn't emitted yet.
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

    /// Evaluate the combined market view into a trading signal.
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

    /// Terminal node.
    pub async fn signal(input: &TradingSignal) -> TradingSignal {
        input.clone()
    }
}
