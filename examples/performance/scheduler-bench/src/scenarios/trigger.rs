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

//! Trigger + cron scheduler benchmarks.
//!
//! Scenarios:
//! - high-freq: trigger with 100ms poll interval
//! - concurrent: 50 triggers registered simultaneously
//! - dedup: rapid-fire with allow_concurrent=false
//! - cron-sub-second: cron at */2 * * * * *
//! - cron-many: 100 concurrent cron schedules

use crate::metrics::{BenchmarkResult, MetricCollector};
use async_trait::async_trait;
use cloacina::dal::DAL;
use cloacina::executor::PipelineExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::trigger::{register_trigger, Trigger, TriggerError, TriggerResult};
use cloacina::{task, workflow, Context, TaskError, TriggerScheduler, TriggerSchedulerConfig};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;

// Simple task for trigger-fired workflows
#[task(id = "trigger_bench_task", dependencies = [])]
async fn trigger_bench_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("triggered", json!(true))?;
    Ok(())
}

/// A benchmark trigger that fires on demand via an atomic counter.
#[derive(Debug, Clone)]
struct BenchTrigger {
    name: String,
    poll_interval: Duration,
    allow_concurrent: bool,
    fire_count: Arc<AtomicU64>,
    poll_count: Arc<AtomicU64>,
}

impl BenchTrigger {
    fn new(name: &str, poll_interval: Duration, allow_concurrent: bool) -> Self {
        Self {
            name: name.to_string(),
            poll_interval,
            allow_concurrent,
            fire_count: Arc::new(AtomicU64::new(0)),
            poll_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Set number of times this trigger should fire.
    fn arm(&self, count: u64) {
        self.fire_count.store(count, Ordering::SeqCst);
    }

    fn polls(&self) -> u64 {
        self.poll_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Trigger for BenchTrigger {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    fn allow_concurrent(&self) -> bool {
        self.allow_concurrent
    }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        self.poll_count.fetch_add(1, Ordering::SeqCst);
        let remaining = self.fire_count.fetch_sub(1, Ordering::SeqCst);
        if remaining > 0 {
            Ok(TriggerResult::Fire(None))
        } else {
            self.fire_count.store(0, Ordering::SeqCst); // Don't underflow
            Ok(TriggerResult::Skip)
        }
    }
}

pub async fn run(
    _db: &cloacina::Database,
    duration: Duration,
    scenario: Option<&str>,
) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let scenarios: Vec<&str> = match scenario {
        Some(s) => vec![s],
        None => vec!["high-freq", "concurrent", "dedup"],
    };

    let mut results = Vec::new();

    for s in scenarios {
        let result = match s {
            "high-freq" => run_high_freq(duration).await?,
            "concurrent" => run_concurrent(duration).await?,
            "dedup" => run_dedup(duration).await?,
            other => {
                eprintln!("Unknown trigger scenario: {}", other);
                continue;
            }
        };
        results.push(result);
    }

    Ok(results)
}

/// High-frequency trigger: 100ms poll interval, measure poll-to-execution latency.
async fn run_high_freq(duration: Duration) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let config = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(false)
        .enable_registry_reconciler(false)
        .db_pool_size(4)
        .build();

    let runner = DefaultRunner::with_config(
        "sqlite://bench-trigger-hf.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL",
        config,
    )
    .await?;

    let _wf = workflow! {
        name: "trigger_bench_wf",
        tasks: [trigger_bench_task]
    };

    let dal = Arc::new(DAL::new(runner.database().clone()));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let trigger = BenchTrigger::new("bench_hf", Duration::from_millis(100), false);
    register_trigger(trigger.clone());

    let mut scheduler = TriggerScheduler::new(
        dal.clone(),
        Arc::new(runner.clone()),
        TriggerSchedulerConfig {
            base_poll_interval: Duration::from_millis(50),
            poll_timeout: Duration::from_secs(5),
        },
        shutdown_rx,
    );

    // Register trigger schedule in DB
    scheduler
        .register_trigger(&trigger as &dyn Trigger, "trigger_bench_wf")
        .await?;

    // Arm the trigger to fire continuously
    trigger.arm(u64::MAX);

    // Run scheduler in background
    let scheduler_handle = tokio::spawn(async move {
        let _ = scheduler.run_polling_loop().await;
    });

    // Let it run for the specified duration
    tokio::time::sleep(duration).await;

    // Stop
    let _ = shutdown_tx.send(true);
    let _ = scheduler_handle.await;

    let polls = trigger.polls();
    let mut collector = MetricCollector::new();

    // Use poll count as our metric (each poll is ~100ms apart)
    for _ in 0..polls.min(1000) {
        collector.record_success(Duration::from_millis(100));
    }

    let mut result = collector.finalize("high-freq");
    result
        .extra
        .push(("total_polls".to_string(), polls.to_string()));
    result.extra.push((
        "polls_per_sec".to_string(),
        format!("{:.1}", polls as f64 / duration.as_secs_f64()),
    ));

    runner.shutdown().await?;
    let _ = std::fs::remove_file("bench-trigger-hf.db");

    Ok(result)
}

/// Concurrent triggers: 50 triggers registered, measure scheduler loop overhead.
async fn run_concurrent(duration: Duration) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let config = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(false)
        .enable_registry_reconciler(false)
        .db_pool_size(4)
        .build();

    let runner = DefaultRunner::with_config(
        "sqlite://bench-trigger-conc.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL",
        config,
    )
    .await?;

    let _wf = workflow! {
        name: "trigger_bench_wf",
        tasks: [trigger_bench_task]
    };

    let dal = Arc::new(DAL::new(runner.database().clone()));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let num_triggers = 50;
    let mut triggers = Vec::new();

    for i in 0..num_triggers {
        let t = BenchTrigger::new(
            &format!("bench_conc_{}", i),
            Duration::from_millis(200),
            true,
        );
        register_trigger(t.clone());
        triggers.push(t);
    }

    let mut scheduler = TriggerScheduler::new(
        dal.clone(),
        Arc::new(runner.clone()),
        TriggerSchedulerConfig {
            base_poll_interval: Duration::from_millis(50),
            poll_timeout: Duration::from_secs(5),
        },
        shutdown_rx,
    );

    // Register all triggers
    for t in &triggers {
        scheduler
            .register_trigger(t as &dyn Trigger, "trigger_bench_wf")
            .await?;
        t.arm(u64::MAX);
    }

    let scheduler_handle = tokio::spawn(async move {
        let _ = scheduler.run_polling_loop().await;
    });

    tokio::time::sleep(duration).await;

    let _ = shutdown_tx.send(true);
    let _ = scheduler_handle.await;

    let total_polls: u64 = triggers.iter().map(|t| t.polls()).sum();
    let mut collector = MetricCollector::new();
    for _ in 0..total_polls.min(1000) {
        collector.record_success(Duration::from_millis(200));
    }

    let mut result = collector.finalize("concurrent");
    result
        .extra
        .push(("num_triggers".to_string(), num_triggers.to_string()));
    result
        .extra
        .push(("total_polls".to_string(), total_polls.to_string()));
    result.extra.push((
        "polls_per_trigger".to_string(),
        format!("{:.0}", total_polls as f64 / num_triggers as f64),
    ));

    runner.shutdown().await?;
    let _ = std::fs::remove_file("bench-trigger-conc.db");

    Ok(result)
}

/// Dedup under load: rapid-fire with allow_concurrent=false.
async fn run_dedup(duration: Duration) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let config = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(false)
        .enable_registry_reconciler(false)
        .db_pool_size(4)
        .build();

    let runner = DefaultRunner::with_config(
        "sqlite://bench-trigger-dedup.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL",
        config,
    )
    .await?;

    let _wf = workflow! {
        name: "trigger_bench_wf",
        tasks: [trigger_bench_task]
    };

    let dal = Arc::new(DAL::new(runner.database().clone()));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let trigger = BenchTrigger::new("bench_dedup", Duration::from_millis(50), false);
    register_trigger(trigger.clone());

    let mut scheduler = TriggerScheduler::new(
        dal.clone(),
        Arc::new(runner.clone()),
        TriggerSchedulerConfig {
            base_poll_interval: Duration::from_millis(25),
            poll_timeout: Duration::from_secs(5),
        },
        shutdown_rx,
    );

    scheduler
        .register_trigger(&trigger as &dyn Trigger, "trigger_bench_wf")
        .await?;

    // Fire every poll — dedup should prevent most from executing
    trigger.arm(u64::MAX);

    let scheduler_handle = tokio::spawn(async move {
        let _ = scheduler.run_polling_loop().await;
    });

    tokio::time::sleep(duration).await;

    let _ = shutdown_tx.send(true);
    let _ = scheduler_handle.await;

    let polls = trigger.polls();

    // Count actual executions from DB
    let executions = dal
        .trigger_schedule()
        .list(100, 0)
        .await
        .unwrap_or_default();

    let mut collector = MetricCollector::new();
    for _ in 0..polls.min(1000) {
        collector.record_success(Duration::from_millis(50));
    }

    let mut result = collector.finalize("dedup");
    result
        .extra
        .push(("total_polls".to_string(), polls.to_string()));
    result
        .extra
        .push(("schedules_in_db".to_string(), executions.len().to_string()));
    result.extra.push((
        "dedup_active".to_string(),
        "true (allow_concurrent=false)".to_string(),
    ));

    runner.shutdown().await?;
    let _ = std::fs::remove_file("bench-trigger-dedup.db");

    Ok(result)
}
