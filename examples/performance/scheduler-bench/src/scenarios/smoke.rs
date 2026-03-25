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

//! Smoke test — submits a single workflow and reports metrics.
//! Proves the harness infrastructure works end-to-end.

use crate::metrics::{BenchmarkResult, MetricCollector};
use cloacina::executor::PipelineExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError};
use serde_json::json;
use std::time::{Duration, Instant};

#[task(id = "smoke_task", dependencies = [])]
async fn smoke_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("smoke", json!("ok"))?;
    Ok(())
}

pub async fn run(
    _db: &cloacina::Database,
    _duration: Duration,
) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let config = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(false)
        .enable_registry_reconciler(false)
        .db_pool_size(4)
        .build();

    let runner = DefaultRunner::with_config(
        "sqlite://bench-smoke.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL",
        config,
    )
    .await?;

    let _wf = workflow! {
        name: "smoke_bench",
        tasks: [smoke_task]
    };

    let mut collector = MetricCollector::new();

    for _ in 0..10 {
        let start = Instant::now();
        let result = runner.execute("smoke_bench", Context::new()).await;
        let latency = start.elapsed();

        match result {
            Ok(_) => collector.record_success(latency),
            Err(_) => collector.record_failure(),
        }
    }

    runner.shutdown().await?;

    // Clean up temp db
    let _ = std::fs::remove_file("bench-smoke.db");

    Ok(vec![collector.finalize("smoke")])
}
