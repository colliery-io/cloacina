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

//! Output formatting for benchmark results.

use crate::metrics::BenchmarkResult;
use crate::OutputFormat;

pub fn print_results(category: &str, results: &[BenchmarkResult], format: &OutputFormat) {
    match format {
        OutputFormat::Table => print_table(category, results),
        OutputFormat::Json => print_json(category, results),
    }
}

fn print_table(category: &str, results: &[BenchmarkResult]) {
    println!("=== {} ===", category);
    println!(
        "{:<25} {:>8} {:>8} {:>8} {:>10} {:>10} {:>10} {:>10}",
        "Scenario", "Ops", "OK", "Fail", "p50", "p95", "p99", "ops/s"
    );
    println!("{}", "-".repeat(100));

    for r in results {
        println!(
            "{:<25} {:>8} {:>8} {:>8} {:>10} {:>10} {:>10} {:>10.1}",
            r.scenario,
            r.total_operations,
            r.successful,
            r.failed,
            format_duration(r.latencies.p50),
            format_duration(r.latencies.p95),
            format_duration(r.latencies.p99),
            r.throughput,
        );

        for (key, value) in &r.extra {
            println!("  {} = {}", key, value);
        }
    }
    println!();
}

fn print_json(category: &str, results: &[BenchmarkResult]) {
    let json_results: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            let mut obj = serde_json::json!({
                "category": category,
                "scenario": r.scenario,
                "duration_ms": r.duration.as_millis(),
                "total_operations": r.total_operations,
                "successful": r.successful,
                "failed": r.failed,
                "throughput_ops_per_sec": r.throughput,
                "latency": {
                    "p50_us": r.latencies.p50.as_micros(),
                    "p95_us": r.latencies.p95.as_micros(),
                    "p99_us": r.latencies.p99.as_micros(),
                    "min_us": r.latencies.min.as_micros(),
                    "max_us": r.latencies.max.as_micros(),
                    "mean_us": r.latencies.mean.as_micros(),
                }
            });
            if !r.extra.is_empty() {
                let extra: serde_json::Map<String, serde_json::Value> = r
                    .extra
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                    .collect();
                obj["extra"] = serde_json::Value::Object(extra);
            }
            obj
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json_results).unwrap());
}

fn format_duration(d: std::time::Duration) -> String {
    let us = d.as_micros();
    if us < 1000 {
        format!("{}us", us)
    } else if us < 1_000_000 {
        format!("{:.1}ms", us as f64 / 1000.0)
    } else {
        format!("{:.2}s", us as f64 / 1_000_000.0)
    }
}
