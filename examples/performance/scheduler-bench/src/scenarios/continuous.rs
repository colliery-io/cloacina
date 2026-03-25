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

//! Continuous scheduler benchmarks.
//!
//! Scenarios:
//! - steady: constant boundary injection, Immediate policy
//! - burst: inject many boundaries at once, measure coalescing
//! - multi-source: multiple data sources feeding one scheduler

use crate::metrics::{BenchmarkResult, MetricCollector};
use cloacina::continuous::boundary::{BoundaryKind, ComputationBoundary};
use cloacina::continuous::datasource::{
    ConnectionDescriptor, DataConnection, DataConnectionError, DataSource, DataSourceMetadata,
};
use cloacina::continuous::detector::{DetectorOutput, DETECTOR_OUTPUT_KEY};
use cloacina::continuous::graph::{assemble_graph, ContinuousTaskRegistration};
use cloacina::continuous::ledger::ExecutionLedger;
use cloacina::continuous::ledger::LedgerEvent;
use cloacina::continuous::scheduler::{ContinuousScheduler, ContinuousSchedulerConfig};
use cloacina_workflow::{Context, Task, TaskError, TaskNamespace};
use parking_lot::RwLock;
use std::any::Any;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;

// --- Test helpers (same pattern as integration/continuous tests) ---

struct BenchConn;
impl DataConnection for BenchConn {
    fn connect(&self) -> Result<Box<dyn Any>, DataConnectionError> {
        Ok(Box::new("bench_handle".to_string()))
    }
    fn descriptor(&self) -> ConnectionDescriptor {
        ConnectionDescriptor {
            system_type: "bench".into(),
            location: "perf".into(),
        }
    }
    fn system_metadata(&self) -> serde_json::Value {
        serde_json::json!({})
    }
}

fn make_source(name: &str) -> DataSource {
    DataSource {
        name: name.into(),
        connection: Box::new(BenchConn),
        detector_workflow: format!("detect_{}", name),
        lineage: DataSourceMetadata::default(),
    }
}

struct CountingTask {
    id: String,
    count: Arc<AtomicU64>,
}

impl CountingTask {
    fn new(id: &str, count: Arc<AtomicU64>) -> Self {
        Self {
            id: id.to_string(),
            count,
        }
    }
}

#[async_trait::async_trait]
impl Task for CountingTask {
    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(context)
    }
    fn id(&self) -> &str {
        &self.id
    }
    fn dependencies(&self) -> &[TaskNamespace] {
        &[]
    }
}

fn make_boundary(start: i64, end: i64) -> ComputationBoundary {
    ComputationBoundary {
        kind: BoundaryKind::OffsetRange { start, end },
        metadata: None,
        emitted_at: chrono::Utc::now(),
    }
}

fn make_detector_completion(task_name: &str, boundaries: Vec<ComputationBoundary>) -> LedgerEvent {
    let mut ctx = Context::new();
    let output = DetectorOutput::Change { boundaries };
    ctx.insert(DETECTOR_OUTPUT_KEY, serde_json::to_value(&output).unwrap())
        .unwrap();
    LedgerEvent::TaskCompleted {
        task: task_name.into(),
        at: chrono::Utc::now(),
        context: ctx,
    }
}

// --- Scenarios ---

pub async fn run(
    _db: &cloacina::Database,
    duration: Duration,
    scenario: Option<&str>,
) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let scenarios: Vec<&str> = match scenario {
        Some(s) => vec![s],
        None => vec!["steady", "burst", "multi-source"],
    };

    let mut results = Vec::new();

    for s in scenarios {
        let result = match s {
            "steady" => run_steady(duration).await?,
            "burst" => run_burst().await?,
            "multi-source" => run_multi_source(duration).await?,
            other => {
                eprintln!("Unknown continuous scenario: {}", other);
                continue;
            }
        };
        results.push(result);
    }

    Ok(results)
}

/// Steady state: inject boundaries at constant rate, Immediate policy.
async fn run_steady(duration: Duration) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let exec_count = Arc::new(AtomicU64::new(0));

    let graph = assemble_graph(
        vec![make_source("perf_source")],
        vec![ContinuousTaskRegistration {
            id: "process".into(),
            sources: vec!["perf_source".into()],
            referenced: vec![],
        }],
    )?;

    let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
    let mut scheduler = ContinuousScheduler::new(
        graph,
        ledger.clone(),
        ContinuousSchedulerConfig {
            poll_interval: Duration::from_millis(5),
            max_fired_tasks: 100_000,
            task_timeout: None,
        },
    );
    scheduler.register_task(Arc::new(CountingTask::new("process", exec_count.clone())));

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    let mut collector = MetricCollector::new();
    let start = Instant::now();
    let mut offset = 0i64;
    let batch_size = 50;

    while start.elapsed() < duration {
        let inject_start = Instant::now();
        let boundaries: Vec<ComputationBoundary> = (0..batch_size)
            .map(|_| {
                let b = make_boundary(offset, offset + 100);
                offset += 100;
                b
            })
            .collect();
        {
            let mut l = ledger.write();
            l.append(make_detector_completion("detect_perf_source", boundaries));
        }
        collector.record_success(inject_start.elapsed());
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    let _ = tx.send(true);
    let fired = handle.await?;

    let execs = exec_count.load(Ordering::SeqCst);
    let boundaries = offset / 100;

    let mut result = collector.finalize("steady");
    result
        .extra
        .push(("boundaries_injected".to_string(), boundaries.to_string()));
    result
        .extra
        .push(("tasks_fired".to_string(), fired.len().to_string()));
    result
        .extra
        .push(("tasks_executed".to_string(), execs.to_string()));
    result.extra.push((
        "injection_rate".to_string(),
        format!("{:.0}/s", boundaries as f64 / result.duration.as_secs_f64()),
    ));

    Ok(result)
}

/// Burst: inject 10k boundaries at once, measure coalescing.
async fn run_burst() -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let exec_count = Arc::new(AtomicU64::new(0));

    let graph = assemble_graph(
        vec![make_source("burst_source")],
        vec![ContinuousTaskRegistration {
            id: "process".into(),
            sources: vec!["burst_source".into()],
            referenced: vec![],
        }],
    )?;

    let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
    let mut scheduler = ContinuousScheduler::new(
        graph,
        ledger.clone(),
        ContinuousSchedulerConfig {
            poll_interval: Duration::from_millis(5),
            max_fired_tasks: 100_000,
            task_timeout: None,
        },
    );
    scheduler.register_task(Arc::new(CountingTask::new("process", exec_count.clone())));

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    let num_boundaries = 10_000i64;
    let mut collector = MetricCollector::new();

    // Inject all at once
    let inject_start = Instant::now();
    {
        let mut l = ledger.write();
        let boundaries: Vec<ComputationBoundary> = (0..num_boundaries)
            .map(|i| make_boundary(i * 10, (i + 1) * 10))
            .collect();
        l.append(make_detector_completion("detect_burst_source", boundaries));
    }
    let inject_time = inject_start.elapsed();
    collector.record_success(inject_time);

    // Wait for processing
    tokio::time::sleep(Duration::from_secs(2)).await;

    let _ = tx.send(true);
    let fired = handle.await?;
    let execs = exec_count.load(Ordering::SeqCst);

    let mut result = collector.finalize("burst");
    result.extra.push((
        "boundaries_injected".to_string(),
        num_boundaries.to_string(),
    ));
    result
        .extra
        .push(("tasks_fired".to_string(), fired.len().to_string()));
    result
        .extra
        .push(("tasks_executed".to_string(), execs.to_string()));
    result.extra.push((
        "coalescing_ratio".to_string(),
        format!("{:.1}", num_boundaries as f64 / fired.len().max(1) as f64),
    ));
    result.extra.push((
        "inject_time_ms".to_string(),
        format!("{:.1}", inject_time.as_secs_f64() * 1000.0),
    ));

    Ok(result)
}

/// Multi-source: 3 sources feeding one scheduler, measure per-source throughput.
async fn run_multi_source(
    duration: Duration,
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let exec_count = Arc::new(AtomicU64::new(0));

    let sources = vec!["source_a", "source_b", "source_c"];
    let data_sources: Vec<DataSource> = sources.iter().map(|n| make_source(n)).collect();

    let graph = assemble_graph(
        data_sources,
        vec![ContinuousTaskRegistration {
            id: "aggregator".into(),
            sources: sources.iter().map(|s| s.to_string()).collect(),
            referenced: vec![],
        }],
    )?;

    let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
    let mut scheduler = ContinuousScheduler::new(
        graph,
        ledger.clone(),
        ContinuousSchedulerConfig {
            poll_interval: Duration::from_millis(5),
            max_fired_tasks: 100_000,
            task_timeout: None,
        },
    );
    scheduler.register_task(Arc::new(CountingTask::new(
        "aggregator",
        exec_count.clone(),
    )));

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    let mut collector = MetricCollector::new();
    let start = Instant::now();
    let mut offsets = vec![0i64; sources.len()];

    while start.elapsed() < duration {
        let op_start = Instant::now();
        for (i, name) in sources.iter().enumerate() {
            let boundary = make_boundary(offsets[i], offsets[i] + 50);
            offsets[i] += 50;
            let mut l = ledger.write();
            l.append(make_detector_completion(
                &format!("detect_{}", name),
                vec![boundary],
            ));
        }
        collector.record_success(op_start.elapsed());
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    let _ = tx.send(true);
    let fired = handle.await?;
    let execs = exec_count.load(Ordering::SeqCst);
    let total_boundaries: i64 = offsets.iter().map(|o| o / 50).sum();

    let mut result = collector.finalize("multi-source");
    result
        .extra
        .push(("num_sources".to_string(), sources.len().to_string()));
    result
        .extra
        .push(("total_boundaries".to_string(), total_boundaries.to_string()));
    result
        .extra
        .push(("tasks_fired".to_string(), fired.len().to_string()));
    result
        .extra
        .push(("tasks_executed".to_string(), execs.to_string()));

    Ok(result)
}
