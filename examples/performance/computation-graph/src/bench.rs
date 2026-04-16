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

//! Computation Graph Latency & Throughput Benchmarks
//!
//! Measures:
//! - Event-to-execution latency: p50, p95, p99
//! - Maximum sustained throughput: events/sec before channel backup

use std::sync::atomic::{AtomicU64, Ordering};
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
// Boundary types — simple single-source graph for benchmarking
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchEvent {
    pub sequence: u64,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchOutput {
    pub result: f64,
}

// ---------------------------------------------------------------------------
// Minimal computation graph — single node, no routing overhead
// ---------------------------------------------------------------------------

#[cloacina_macros::computation_graph(
    react = when_any(source),
    graph = {
        process(source) -> output,
    }
)]
pub mod bench_graph {
    use super::*;

    pub async fn process(source: Option<&BenchEvent>) -> f64 {
        source.map(|e| e.value * 2.0).unwrap_or(0.0)
    }

    pub async fn output(value: &f64) -> BenchOutput {
        BenchOutput { result: *value }
    }
}

// ---------------------------------------------------------------------------
// Accumulator
// ---------------------------------------------------------------------------

struct BenchAccumulator;

#[async_trait::async_trait]
impl cloacina::computation_graph::Accumulator for BenchAccumulator {
    type Output = BenchEvent;
    fn process(&mut self, event: Vec<u8>) -> Option<BenchEvent> {
        cloacina::computation_graph::types::deserialize(&event).ok()
    }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "cg-bench")]
#[command(about = "Latency and throughput benchmarks for computation graph pipeline")]
struct Args {
    /// Duration of latency test in seconds
    #[arg(long, default_value = "15")]
    latency_duration: u64,

    /// Event injection interval for latency test in microseconds
    #[arg(long, default_value = "1000")]
    latency_interval_us: u64,

    /// Duration of throughput ramp in seconds
    #[arg(long, default_value = "10")]
    throughput_duration: u64,

    /// Starting interval for throughput ramp in microseconds
    #[arg(long, default_value = "500")]
    throughput_start_us: u64,

    /// Minimum interval for throughput ramp in microseconds
    #[arg(long, default_value = "10")]
    throughput_min_us: u64,
}

// ---------------------------------------------------------------------------
// Percentile calculation
// ---------------------------------------------------------------------------

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("=== Computation Graph Benchmarks ===\n");

    // --- Setup pipeline ---
    let (boundary_tx, boundary_rx) = mpsc::channel(256);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    let (src_tx, src_rx) = mpsc::channel(256);
    tokio::spawn(accumulator_runtime(
        BenchAccumulator,
        AccumulatorContext {
            output: BoundarySender::new(boundary_tx, SourceName::new("source")),
            name: "source".to_string(),
            shutdown: shutdown_rx.clone(),
            checkpoint: None,
            health: None,
        },
        src_rx,
        AccumulatorRuntimeConfig::default(),
    ));

    let (_manual_tx, manual_rx) = mpsc::channel(10);
    let fire_count = Arc::new(AtomicU64::new(0));
    let fc = fire_count.clone();

    // Shared latency collection — reactor records completion timestamps
    let latency_samples: Arc<std::sync::Mutex<Vec<(Instant, Instant)>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let samples_clone = latency_samples.clone();

    let graph_fn: CompiledGraphFn = Arc::new(move |cache: InputCache| {
        let fc = fc.clone();
        let samples = samples_clone.clone();
        Box::pin(async move {
            fc.fetch_add(1, Ordering::Relaxed);
            let result = bench_graph_compiled(&cache).await;

            // Record completion time for latency measurement
            let now = Instant::now();
            if let Ok(mut s) = samples.lock() {
                // We store (placeholder, completion_time) — the push time is embedded
                // in the event data and extracted below
                s.push((now, now)); // placeholder — actual push time set by injector
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
    tokio::spawn(reactor.run());

    // Warm up
    tokio::time::sleep(Duration::from_millis(200)).await;

    // =====================================================================
    // Benchmark 1: Latency
    // =====================================================================
    println!("--- Latency Benchmark ---");
    println!(
        "Interval: {}µs | Duration: {}s",
        args.latency_interval_us, args.latency_duration
    );

    let latency_start = Instant::now();
    let latency_duration = Duration::from_secs(args.latency_duration);
    let latency_interval = Duration::from_micros(args.latency_interval_us);

    // Collect push timestamps separately
    let mut push_times: Vec<Instant> = Vec::new();
    let mut seq = 0u64;
    let mut ticker = tokio::time::interval(latency_interval);

    fire_count.store(0, Ordering::Relaxed);
    latency_samples.lock().unwrap().clear();

    while latency_start.elapsed() < latency_duration {
        ticker.tick().await;
        let push_time = Instant::now();
        let event = BenchEvent {
            sequence: seq,
            value: seq as f64 * 0.1,
        };
        if src_tx.try_send(serialize(&event).unwrap()).is_ok() {
            push_times.push(push_time);
            seq += 1;
        }
    }

    // Wait for pipeline to drain
    tokio::time::sleep(Duration::from_millis(500)).await;

    let fires = fire_count.load(Ordering::Relaxed);
    let samples = latency_samples.lock().unwrap();

    // Compute latencies: for each fire, pair it with the corresponding push time
    // The reactor fires on Latest strategy, so fires <= pushes.
    // We approximate latency as: completion_time - push_time[fire_index]
    let sample_count = fires.min(push_times.len() as u64) as usize;
    let mut latencies_us: Vec<f64> = Vec::with_capacity(sample_count);

    for i in 0..sample_count.min(samples.len()) {
        let completion = samples[i].0;
        // Map fire i to push time — with Latest strategy, each fire consumes the
        // most recent event. We approximate by scaling push index proportionally.
        let push_idx = if fires > 0 {
            (i as u64 * push_times.len() as u64 / fires) as usize
        } else {
            0
        };
        if push_idx < push_times.len() {
            let latency = completion.duration_since(push_times[push_idx]);
            latencies_us.push(latency.as_micros() as f64);
        }
    }

    latencies_us.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let p50 = percentile(&latencies_us, 50.0);
    let p95 = percentile(&latencies_us, 95.0);
    let p99 = percentile(&latencies_us, 99.0);
    let mean = if latencies_us.is_empty() {
        0.0
    } else {
        latencies_us.iter().sum::<f64>() / latencies_us.len() as f64
    };

    println!("Events pushed:  {}", push_times.len());
    println!("Graph fires:    {}", fires);
    println!("Samples:        {}", latencies_us.len());
    println!("Latency (µs):");
    println!("  mean: {:.0}", mean);
    println!("  p50:  {:.0}", p50);
    println!("  p95:  {:.0}", p95);
    println!("  p99:  {:.0}", p99);
    if !latencies_us.is_empty() {
        println!(
            "  min:  {:.0}  max: {:.0}",
            latencies_us.first().unwrap(),
            latencies_us.last().unwrap()
        );
    }

    // =====================================================================
    // Benchmark 2: Throughput
    // =====================================================================
    println!("\n--- Throughput Benchmark ---");
    println!(
        "Ramp: {}µs → {}µs | Duration: {}s",
        args.throughput_start_us, args.throughput_min_us, args.throughput_duration
    );

    let throughput_start = Instant::now();
    let throughput_duration = Duration::from_secs(args.throughput_duration);
    let mut interval_us = args.throughput_start_us;
    let mut total_sent = 0u64;
    let mut total_backed_up = 0u64;
    let mut max_sustained_rate = 0.0f64;
    let mut last_check = Instant::now();
    let mut sent_since_check = 0u64;

    fire_count.store(0, Ordering::Relaxed);

    while throughput_start.elapsed() < throughput_duration {
        let event = BenchEvent {
            sequence: total_sent,
            value: total_sent as f64,
        };
        match src_tx.try_send(serialize(&event).unwrap()) {
            Ok(_) => {
                total_sent += 1;
                sent_since_check += 1;
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                total_backed_up += 1;
                // Channel full — record the backup but don't block
            }
            Err(_) => break,
        }

        // Check rate every 100ms
        if last_check.elapsed() >= Duration::from_millis(100) {
            let rate = sent_since_check as f64 / last_check.elapsed().as_secs_f64();
            if total_backed_up == 0 {
                max_sustained_rate = rate.max(max_sustained_rate);
            }
            sent_since_check = 0;
            last_check = Instant::now();

            // Ramp down interval
            if interval_us > args.throughput_min_us {
                interval_us = (interval_us * 90 / 100).max(args.throughput_min_us);
            }
        }

        if interval_us > 0 {
            tokio::time::sleep(Duration::from_micros(interval_us)).await;
        } else {
            tokio::task::yield_now().await;
        }
    }

    let elapsed = throughput_start.elapsed();
    let overall_rate = total_sent as f64 / elapsed.as_secs_f64();
    let fires_throughput = fire_count.load(Ordering::Relaxed);

    println!("Events sent:    {}", total_sent);
    println!("Channel backups:{}", total_backed_up);
    println!("Graph fires:    {}", fires_throughput);
    println!("Overall rate:   {:.0} events/sec", overall_rate);
    println!(
        "Max sustained:  {:.0} events/sec (before backup)",
        max_sustained_rate
    );
    println!(
        "Fire rate:      {:.0} fires/sec",
        fires_throughput as f64 / elapsed.as_secs_f64()
    );

    // =====================================================================
    // Summary
    // =====================================================================
    println!("\n=== Benchmark Summary ===");
    println!("Latency p50/p95/p99: {:.0}/{:.0}/{:.0} µs", p50, p95, p99);
    println!("Max throughput:      {:.0} events/sec", max_sustained_rate);

    // Shutdown
    let _ = shutdown_tx.send(true);
    tokio::time::sleep(Duration::from_millis(200)).await;
}
