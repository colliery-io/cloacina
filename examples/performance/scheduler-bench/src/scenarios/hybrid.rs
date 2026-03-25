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

//! Hybrid benchmarks — all schedulers running simultaneously.
//!
//! Scenarios:
//! - mixed-load: direct execution + trigger + continuous all active

use crate::metrics::{BenchmarkResult, MetricCollector};
use cloacina::executor::PipelineExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[task(id = "hybrid_task", dependencies = [])]
async fn hybrid_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("hybrid", json!(true))?;
    Ok(())
}

pub async fn run(
    _db: &cloacina::Database,
    duration: Duration,
    scenario: Option<&str>,
) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let scenarios: Vec<&str> = match scenario {
        Some(s) => vec![s],
        None => vec!["mixed-load"],
    };

    let mut results = Vec::new();

    for s in scenarios {
        let result = match s {
            "mixed-load" => run_mixed_load(duration).await?,
            other => {
                eprintln!("Unknown hybrid scenario: {}", other);
                continue;
            }
        };
        results.push(result);
    }

    Ok(results)
}

/// Mixed load: cron scheduling + direct execution simultaneously.
///
/// Starts a full DefaultRunner with cron enabled, then hammers
/// direct workflow submissions while cron fires in the background.
async fn run_mixed_load(duration: Duration) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let config = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .cron_poll_interval(Duration::from_secs(2))
        .enable_registry_reconciler(false)
        .db_pool_size(8)
        .build();

    let runner = DefaultRunner::with_config(
        "sqlite://bench-hybrid.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL",
        config,
    )
    .await?;

    let _wf = workflow! {
        name: "hybrid_bench_wf",
        tasks: [hybrid_task]
    };

    let direct_count = Arc::new(AtomicU64::new(0));
    let mut collector = MetricCollector::new();
    let start = Instant::now();

    // Submit direct workflows concurrently while cron runs in background
    let batch_size = 5;
    while start.elapsed() < duration {
        let mut handles = Vec::new();

        for _ in 0..batch_size {
            let r = runner.clone();
            let dc = direct_count.clone();
            handles.push(tokio::spawn(async move {
                let op_start = Instant::now();
                let result = r.execute("hybrid_bench_wf", Context::new()).await;
                if result.is_ok() {
                    dc.fetch_add(1, Ordering::SeqCst);
                }
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

    let mut result = collector.finalize("mixed-load");
    result.extra.push((
        "direct_executions".to_string(),
        direct_count.load(Ordering::SeqCst).to_string(),
    ));
    result
        .extra
        .push(("cron_enabled".to_string(), "true (2s interval)".to_string()));
    result
        .extra
        .push(("batch_size".to_string(), batch_size.to_string()));

    runner.shutdown().await?;
    let _ = std::fs::remove_file("bench-hybrid.db");

    Ok(result)
}
