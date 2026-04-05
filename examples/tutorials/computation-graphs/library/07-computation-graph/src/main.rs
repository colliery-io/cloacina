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

//! Tutorial 07: Your First Computation Graph
//!
//! This tutorial introduces Cloacina's computation graph system. You'll define
//! a simple pricing pipeline using the `#[computation_graph]` macro, then call
//! the compiled function directly with a hand-built InputCache.
//!
//! Concepts covered:
//! - `#[computation_graph]` attribute macro
//! - Node functions (async functions in the graph module)
//! - Topology declaration: `graph = { entry(input) -> next, next -> output }`
//! - `InputCache`, `serialize()`, `SourceName`
//! - Calling `{module}_compiled(&cache)` and inspecting `GraphResult`

use cloacina::computation_graph::types::{serialize, GraphResult, InputCache, SourceName};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Step 1: Define your boundary types
//
// These are the data types that flow between nodes. They must be
// Serialize + Deserialize because the cache stores serialized bytes.
// ---------------------------------------------------------------------------

/// Raw order book snapshot — our input data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub best_bid: f64,
    pub best_ask: f64,
    pub timestamp: u64,
}

/// Computed spread signal — intermediate result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadSignal {
    pub spread: f64,
    pub mid_price: f64,
}

/// Final formatted output — terminal node result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattedOutput {
    pub message: String,
    pub mid_price: f64,
    pub spread_bps: f64,
}

// ---------------------------------------------------------------------------
// Step 2: Define the computation graph
//
// The `#[computation_graph]` macro declares the topology and reaction criteria.
// Inside the module, each function is a "node" in the graph.
//
// Topology: ingest(orderbook) -> compute_spread -> format_output
//   - `ingest` is an entry node: it reads from the cache (Option<&T>)
//   - `compute_spread` receives the output of `ingest`
//   - `format_output` is the terminal node: its output goes into GraphResult
// ---------------------------------------------------------------------------

#[cloacina_macros::computation_graph(
    react = when_any(orderbook),
    graph = {
        ingest(orderbook) -> compute_spread,
        compute_spread -> format_output,
    }
)]
pub mod pricing_pipeline {
    use super::*;

    /// Entry node: reads the order book from the cache and extracts key fields.
    pub async fn ingest(orderbook: Option<&OrderBookSnapshot>) -> SpreadSignal {
        let book = orderbook.expect("orderbook should be present");
        let spread = book.best_ask - book.best_bid;
        let mid_price = (book.best_ask + book.best_bid) / 2.0;
        SpreadSignal { spread, mid_price }
    }

    /// Processing node: computes spread in basis points.
    pub async fn compute_spread(input: &SpreadSignal) -> SpreadSignal {
        // Convert spread to basis points relative to mid price
        let spread_bps = (input.spread / input.mid_price) * 10_000.0;
        SpreadSignal {
            spread: spread_bps,
            mid_price: input.mid_price,
        }
    }

    /// Terminal node: formats the result for display.
    pub async fn format_output(input: &SpreadSignal) -> FormattedOutput {
        FormattedOutput {
            message: format!(
                "Mid: {:.2}, Spread: {:.1} bps",
                input.mid_price, input.spread
            ),
            mid_price: input.mid_price,
            spread_bps: input.spread,
        }
    }
}

// ---------------------------------------------------------------------------
// Step 3: Run the compiled graph
//
// The macro generates `pricing_pipeline_compiled(&cache) -> GraphResult`.
// We build an InputCache by hand, call the function, and inspect the result.
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    println!("=== Tutorial 07: Your First Computation Graph ===\n");

    // Build an InputCache with our order book data
    let mut cache = InputCache::new();
    let orderbook = OrderBookSnapshot {
        best_bid: 100.50,
        best_ask: 100.55,
        timestamp: 1234567890,
    };

    println!("Input: {:?}\n", orderbook);

    // Serialize and insert into the cache under the source name "orderbook"
    // (must match the name in `react = when_any(orderbook)`)
    cache.update(
        SourceName::new("orderbook"),
        serialize(&orderbook).expect("serialization should succeed"),
    );

    // Call the compiled graph function
    let result: GraphResult = pricing_pipeline_compiled(&cache).await;

    // Inspect the result
    match result {
        GraphResult::Completed { outputs } => {
            println!("Graph completed with {} terminal output(s)", outputs.len());

            // Downcast the terminal output to our expected type
            for output in &outputs {
                if let Some(formatted) = output.downcast_ref::<FormattedOutput>() {
                    println!("  Output: {}", formatted.message);
                    println!("  Mid price: {:.2}", formatted.mid_price);
                    println!("  Spread: {:.1} bps", formatted.spread_bps);
                }
            }
        }
        GraphResult::Error(e) => {
            eprintln!("Graph execution failed: {}", e);
        }
    }

    println!("\n=== Tutorial 07 Complete ===");
}
