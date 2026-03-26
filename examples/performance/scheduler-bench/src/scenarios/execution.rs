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

//! Task execution engine benchmarks.
//!
//! Scenarios:
//! - simple: linear 5-task pipeline
//! - fan-out: 1 root -> N parallel tasks -> 1 join
//! - concurrent: N workflows submitted simultaneously

use crate::metrics::{BenchmarkResult, MetricCollector};
use cloacina::executor::PipelineExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError};
use serde_json::json;
use std::time::{Duration, Instant};

#[task(id = "bench_step_1", dependencies = [])]
async fn bench_step_1(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("step", json!(1))?;
    Ok(())
}

#[task(id = "bench_step_2", dependencies = ["bench_step_1"])]
async fn bench_step_2(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("step", json!(2))?;
    Ok(())
}

#[task(id = "bench_step_3", dependencies = ["bench_step_2"])]
async fn bench_step_3(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("step", json!(3))?;
    Ok(())
}

#[task(id = "bench_step_4", dependencies = ["bench_step_3"])]
async fn bench_step_4(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("step", json!(4))?;
    Ok(())
}

#[task(id = "bench_step_5", dependencies = ["bench_step_4"])]
async fn bench_step_5(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("step", json!(5))?;
    Ok(())
}

// Fan-out tasks (no deps — root fires them all in parallel)
#[task(id = "fan_root", dependencies = [])]
async fn fan_root(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("root", json!(true))?;
    Ok(())
}

#[task(id = "fan_worker", dependencies = ["fan_root"])]
async fn fan_worker(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("worker", json!(true))?;
    Ok(())
}

pub async fn run(
    _db: &cloacina::Database,
    duration: Duration,
    scenario: Option<&str>,
) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let scenarios: Vec<&str> = match scenario {
        Some(s) => vec![s],
        None => vec!["simple", "concurrent"],
    };

    let mut results = Vec::new();

    for s in scenarios {
        let result = match s {
            "simple" => run_simple(duration).await?,
            "concurrent" => run_concurrent(duration).await?,
            other => {
                eprintln!("Unknown execution scenario: {}", other);
                continue;
            }
        };
        results.push(result);
    }

    Ok(results)
}

/// Simple linear 5-task pipeline, measure per-pipeline latency.
async fn run_simple(duration: Duration) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let config = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(false)
        .enable_registry_reconciler(false)
        .scheduler_poll_interval(Duration::from_millis(1))
        .db_pool_size(4)
        .build();

    let runner = DefaultRunner::with_config(
        "sqlite://bench-exec-simple.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL",
        config,
    )
    .await?;

    let _wf = workflow! {
        name: "bench_pipeline",
        tasks: [bench_step_1, bench_step_2, bench_step_3, bench_step_4, bench_step_5]
    };

    let mut collector = MetricCollector::new();
    let start = Instant::now();

    while start.elapsed() < duration {
        let op_start = Instant::now();
        match runner.execute("bench_pipeline", Context::new()).await {
            Ok(_) => collector.record_success(op_start.elapsed()),
            Err(_) => collector.record_failure(),
        }
    }

    runner.shutdown().await?;
    let _ = std::fs::remove_file("bench-exec-simple.db");

    Ok(collector.finalize("simple"))
}

/// Concurrent submissions: N workflows at once, measure throughput under contention.
async fn run_concurrent(duration: Duration) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let config = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(false)
        .enable_registry_reconciler(false)
        .db_pool_size(8)
        .build();

    let runner = DefaultRunner::with_config(
        "sqlite://bench-exec-conc.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL",
        config,
    )
    .await?;

    let _wf = workflow! {
        name: "bench_pipeline",
        tasks: [bench_step_1, bench_step_2, bench_step_3, bench_step_4, bench_step_5]
    };

    let mut collector = MetricCollector::new();
    let start = Instant::now();
    let batch_size = 10;

    while start.elapsed() < duration {
        let mut handles = Vec::new();

        for _ in 0..batch_size {
            let r = runner.clone();
            handles.push(tokio::spawn(async move {
                let op_start = Instant::now();
                let result = r.execute("bench_pipeline", Context::new()).await;
                (op_start.elapsed(), result.is_ok())
            }));
        }

        for handle in handles {
            if let Ok((latency, success)) = handle.await {
                if success {
                    collector.record_success(latency);
                } else {
                    collector.record_failure();
                }
            }
        }
    }

    let mut result = collector.finalize("concurrent");
    result
        .extra
        .push(("batch_size".to_string(), batch_size.to_string()));

    runner.shutdown().await?;
    let _ = std::fs::remove_file("bench-exec-conc.db");

    Ok(result)
}
