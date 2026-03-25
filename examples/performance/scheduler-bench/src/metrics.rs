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

//! Shared metric collection for scheduler benchmarks.

use std::time::{Duration, Instant};

/// Collected benchmark results for a single scenario.
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub scenario: String,
    pub duration: Duration,
    pub total_operations: u64,
    pub successful: u64,
    pub failed: u64,
    pub latencies: LatencyStats,
    pub throughput: f64,
    pub extra: Vec<(String, String)>,
}

/// Latency statistics computed from collected samples.
#[derive(Debug, Clone, Default)]
pub struct LatencyStats {
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub min: Duration,
    pub max: Duration,
    pub mean: Duration,
}

/// Collects latency samples and computes statistics.
pub struct MetricCollector {
    samples: Vec<Duration>,
    start: Instant,
    successes: u64,
    failures: u64,
}

impl MetricCollector {
    pub fn new() -> Self {
        Self {
            samples: Vec::with_capacity(10_000),
            start: Instant::now(),
            successes: 0,
            failures: 0,
        }
    }

    /// Record a successful operation with its latency.
    pub fn record_success(&mut self, latency: Duration) {
        self.samples.push(latency);
        self.successes += 1;
    }

    /// Record a failed operation.
    pub fn record_failure(&mut self) {
        self.failures += 1;
    }

    /// Finalize and compute stats.
    pub fn finalize(mut self, scenario: &str) -> BenchmarkResult {
        let elapsed = self.start.elapsed();
        let total = self.successes + self.failures;
        let throughput = if elapsed.as_secs_f64() > 0.0 {
            self.successes as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        let latencies = if self.samples.is_empty() {
            LatencyStats::default()
        } else {
            self.samples.sort();
            let len = self.samples.len();
            let sum: Duration = self.samples.iter().sum();

            LatencyStats {
                p50: self.samples[len * 50 / 100],
                p95: self.samples[len * 95 / 100],
                p99: self.samples[std::cmp::min(len * 99 / 100, len - 1)],
                min: self.samples[0],
                max: self.samples[len - 1],
                mean: sum / len as u32,
            }
        };

        BenchmarkResult {
            scenario: scenario.to_string(),
            duration: elapsed,
            total_operations: total,
            successful: self.successes,
            failed: self.failures,
            latencies,
            throughput,
            extra: Vec::new(),
        }
    }
}
