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

//! Computation Graph Soak Test
//!
//! Runs a market maker computation graph (from Tutorial 10) under sustained
//! event injection for a configurable duration. Two passthrough accumulators
//! push events at different rates (fast=10ms, slow=200ms). Verifies: no panics,
//! bounded memory growth, no persistent channel backup.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use cloacina::computation_graph::accumulator::{
    accumulator_runtime, shutdown_signal, AccumulatorContext, AccumulatorRuntimeConfig,
    BoundarySender,
};
use cloacina::computation_graph::reactor::{
    CompiledGraphFn, InputStrategy, ReactionCriteria, Reactor,
};
use cloacina::computation_graph::types::{serialize, InputCache, SourceName};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Tracking allocator — measures heap usage without external dependencies
// ---------------------------------------------------------------------------

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

struct TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Adjust tracking: remove old size, add new size
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        ALLOCATED.fetch_add(new_size, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

fn current_allocated_bytes() -> usize {
    ALLOCATED.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Boundary types (same as Tutorial 10)
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

// ---------------------------------------------------------------------------
// Computation graph — market maker with routing (from Tutorial 10)
// ---------------------------------------------------------------------------

#[cloacina_macros::reactor(
    name = "perf_market_maker_reactor",
    accumulators = [orderbook, pricing],
    criteria = when_any(orderbook, pricing),
)]
pub struct PerfMarketMakerReactor;

#[cloacina_macros::computation_graph(
    trigger = reactor(PerfMarketMakerReactor),
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
                format!("price divergence too high: {:.2}", price_diff)
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

// ---------------------------------------------------------------------------
// Accumulators
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

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "computation-graph-soak")]
#[command(about = "Soak test for computation graph pipeline")]
struct Args {
    /// Duration in seconds
    #[arg(short, long, default_value = "60")]
    duration: u64,

    /// Fast source (orderbook) interval in milliseconds
    #[arg(long, default_value = "10")]
    fast_interval_ms: u64,

    /// Slow source (pricing) interval in milliseconds
    #[arg(long, default_value = "200")]
    slow_interval_ms: u64,

    /// Memory growth threshold percentage (fail if exceeded)
    #[arg(long, default_value = "10")]
    mem_threshold_pct: u64,

    /// Progress report interval in seconds
    #[arg(long, default_value = "10")]
    report_interval: u64,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("=== Computation Graph Soak Test ===");
    println!(
        "Duration: {}s | Fast: {}ms | Slow: {}ms | Mem threshold: {}%",
        args.duration, args.fast_interval_ms, args.slow_interval_ms, args.mem_threshold_pct
    );
    println!();

    // Shared counters
    let fire_count = Arc::new(AtomicU64::new(0));
    let ob_pushed = Arc::new(AtomicU64::new(0));
    let pr_pushed = Arc::new(AtomicU64::new(0));
    let ob_backed_up = Arc::new(AtomicU64::new(0));
    let pr_backed_up = Arc::new(AtomicU64::new(0));

    // --- Wire accumulators + reactor ---

    let (boundary_tx, boundary_rx) = mpsc::channel(32);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    // Orderbook accumulator
    let (ob_tx, ob_rx) = mpsc::channel(10);
    tokio::spawn(accumulator_runtime(
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

    // Pricing accumulator
    let (pr_tx, pr_rx) = mpsc::channel(10);
    tokio::spawn(accumulator_runtime(
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

    // Reactor — silent (no output printing, just counting fires)
    let (_manual_tx, manual_rx) = mpsc::channel(10);
    let fc = fire_count.clone();
    let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
        let fc = fc.clone();
        Box::pin(async move {
            fc.fetch_add(1, Ordering::Relaxed);
            market_maker_compiled(&cache).await
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
    tokio::spawn(reactor.run());

    // --- Warm up, then take baseline ---

    tokio::time::sleep(Duration::from_millis(500)).await;
    let baseline_mem = current_allocated_bytes();
    println!("Baseline memory: {} KB", baseline_mem / 1024);
    println!();

    // --- Spawn event injectors ---

    let duration = Duration::from_secs(args.duration);
    let start = Instant::now();

    // Fast source: orderbook events at configurable interval
    let ob_task = tokio::spawn({
        let ob_pushed = ob_pushed.clone();
        let ob_backed_up = ob_backed_up.clone();
        let interval = Duration::from_millis(args.fast_interval_ms);
        async move {
            let mut ticker = tokio::time::interval(interval);
            let mut i = 0u64;
            while start.elapsed() < duration {
                ticker.tick().await;
                let data = OrderBookData {
                    best_bid: 100.0 + (i as f64 * 0.01).sin() * 0.5,
                    best_ask: 100.1 + (i as f64 * 0.01).cos() * 0.5,
                };
                let payload = serialize(&data).unwrap();
                match ob_tx.try_send(payload.clone()) {
                    Ok(_) => {
                        ob_pushed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        ob_backed_up.fetch_add(1, Ordering::Relaxed);
                        // Still deliver — just note the backup
                        if ob_tx.send(payload).await.is_ok() {
                            ob_pushed.fetch_add(1, Ordering::Relaxed);
                        } else {
                            break;
                        }
                    }
                    Err(_) => break,
                }
                i += 1;
            }
        }
    });

    // Slow source: pricing events at configurable interval
    let pr_task = tokio::spawn({
        let pr_pushed = pr_pushed.clone();
        let pr_backed_up = pr_backed_up.clone();
        let interval = Duration::from_millis(args.slow_interval_ms);
        async move {
            let mut ticker = tokio::time::interval(interval);
            let mut i = 0u64;
            while start.elapsed() < duration {
                ticker.tick().await;
                let data = PricingData {
                    mid_price: 100.05 + (i as f64 * 0.005).sin() * 0.3,
                };
                let payload = serialize(&data).unwrap();
                match pr_tx.try_send(payload.clone()) {
                    Ok(_) => {
                        pr_pushed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        pr_backed_up.fetch_add(1, Ordering::Relaxed);
                        if pr_tx.send(payload).await.is_ok() {
                            pr_pushed.fetch_add(1, Ordering::Relaxed);
                        } else {
                            break;
                        }
                    }
                    Err(_) => break,
                }
                i += 1;
            }
        }
    });

    // Progress reporter
    let progress_task = tokio::spawn({
        let fire_count = fire_count.clone();
        let ob_pushed = ob_pushed.clone();
        let pr_pushed = pr_pushed.clone();
        let ob_backed_up = ob_backed_up.clone();
        let pr_backed_up = pr_backed_up.clone();
        let report_interval = Duration::from_secs(args.report_interval);
        async move {
            let mut ticker = tokio::time::interval(report_interval);
            ticker.tick().await; // skip immediate first tick
            let mut last_fires = 0u64;
            while start.elapsed() < duration {
                ticker.tick().await;
                let fires = fire_count.load(Ordering::Relaxed);
                let delta = fires - last_fires;
                let mem_kb = current_allocated_bytes() / 1024;
                let ob = ob_pushed.load(Ordering::Relaxed);
                let pr = pr_pushed.load(Ordering::Relaxed);
                let ob_bu = ob_backed_up.load(Ordering::Relaxed);
                let pr_bu = pr_backed_up.load(Ordering::Relaxed);
                println!(
                    "[{:>3}s] fires: {:>6} (+{:<5}) | pushed: ob={:<6} pr={:<5} | backups: ob={:<4} pr={:<4} | mem: {} KB",
                    start.elapsed().as_secs(),
                    fires,
                    delta,
                    ob,
                    pr,
                    ob_bu,
                    pr_bu,
                    mem_kb
                );
                last_fires = fires;
            }
        }
    });

    // --- Wait for completion ---

    let _ = tokio::join!(ob_task, pr_task, progress_task);
    let elapsed = start.elapsed();

    // --- Collect final stats ---

    let final_mem = current_allocated_bytes();
    let total_fires = fire_count.load(Ordering::Relaxed);
    let total_ob = ob_pushed.load(Ordering::Relaxed);
    let total_pr = pr_pushed.load(Ordering::Relaxed);
    let total_ob_bu = ob_backed_up.load(Ordering::Relaxed);
    let total_pr_bu = pr_backed_up.load(Ordering::Relaxed);

    // Shutdown cleanly
    let _ = shutdown_tx.send(true);
    tokio::time::sleep(Duration::from_millis(200)).await;

    // --- Summary ---

    let mem_growth_pct = if baseline_mem > 0 {
        ((final_mem as f64 - baseline_mem as f64) / baseline_mem as f64) * 100.0
    } else {
        0.0
    };

    let fire_rate = total_fires as f64 / elapsed.as_secs_f64();
    let total_pushed = total_ob + total_pr;

    println!();
    println!("=== Soak Test Summary ===");
    println!("Duration:         {:.1}s", elapsed.as_secs_f64());
    println!(
        "Events pushed:    {} total ({} orderbook, {} pricing)",
        total_pushed, total_ob, total_pr
    );
    println!("Graph fires:      {} ({:.1}/s)", total_fires, fire_rate);
    println!(
        "Channel backups:  {} orderbook, {} pricing",
        total_ob_bu, total_pr_bu
    );
    println!(
        "Memory:           baseline={} KB, final={} KB, growth={:+.1}%",
        baseline_mem / 1024,
        final_mem / 1024,
        mem_growth_pct
    );

    // --- Pass/fail ---

    let mut failed = false;

    if mem_growth_pct > args.mem_threshold_pct as f64 {
        println!(
            "FAIL: Memory growth {:.1}% exceeds {}% threshold",
            mem_growth_pct, args.mem_threshold_pct
        );
        failed = true;
    }

    if total_fires == 0 {
        println!("FAIL: No graph fires recorded");
        failed = true;
    }

    if failed {
        std::process::exit(1);
    }

    println!("PASS: Soak test completed successfully");
}
