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

//! Continuous scheduler soak test — sustained boundary injection at high rate.
//!
//! Configurable via environment variables:
//!
//! ```bash
//! # Quick smoke (defaults)
//! cargo test -p cloacina -- continuous::soak --nocapture
//!
//! # Long soak: 100k boundaries, 10 per batch, 5 minute duration cap
//! SOAK_BOUNDARIES=100000 SOAK_BATCH_SIZE=10 SOAK_DURATION_SECS=300 \
//!   cargo test -p cloacina -- continuous::soak::test_continuous_soak_sustained_load --nocapture
//!
//! # High throughput: 50k boundaries, no yield (max injection rate)
//! SOAK_BOUNDARIES=50000 SOAK_YIELD_EVERY=0 SOAK_SETTLE_MS=2000 \
//!   cargo test -p cloacina -- continuous::soak::test_continuous_soak_sustained_load --nocapture
//!
//! # Multi-source stress: 10k boundaries across 2 sources
//! SOAK_BOUNDARIES=10000 \
//!   cargo test -p cloacina -- continuous::soak::test_continuous_soak_multi_source --nocapture
//! ```
//!
//! Environment variables:
//!   SOAK_BOUNDARIES      — Total boundaries to inject (default: 1000)
//!   SOAK_BATCH_SIZE      — Boundaries per detector completion (default: 1)
//!   SOAK_POLL_MS         — Scheduler poll interval in ms (default: 5)
//!   SOAK_YIELD_EVERY     — Yield to scheduler every N injections, 0=never (default: 50)
//!   SOAK_YIELD_MS        — Sleep duration on yield in ms (default: 1)
//!   SOAK_SETTLE_MS       — Wait after injection for scheduler to finish (default: 500)
//!   SOAK_DURATION_SECS   — Max test duration in seconds, 0=unlimited (default: 0)
//!   SOAK_SOURCES         — Number of data sources for multi-source test (default: 2)
//!   SOAK_INJECTORS       — Number of concurrent injector tasks (default: 1)

use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Read an env var as u64 with a default.
fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Soak test configuration from environment.
struct SoakConfig {
    boundaries: u64,
    batch_size: u64,
    poll_ms: u64,
    yield_every: u64,
    yield_ms: u64,
    settle_ms: u64,
    duration_secs: u64,
    sources: u64,
    injectors: u64,
}

impl SoakConfig {
    fn from_env() -> Self {
        Self {
            boundaries: env_u64("SOAK_BOUNDARIES", 1000),
            batch_size: env_u64("SOAK_BATCH_SIZE", 1),
            poll_ms: env_u64("SOAK_POLL_MS", 5),
            yield_every: env_u64("SOAK_YIELD_EVERY", 50),
            yield_ms: env_u64("SOAK_YIELD_MS", 1),
            settle_ms: env_u64("SOAK_SETTLE_MS", 500),
            duration_secs: env_u64("SOAK_DURATION_SECS", 0),
            sources: env_u64("SOAK_SOURCES", 2),
            injectors: env_u64("SOAK_INJECTORS", 1),
        }
    }

    fn print(&self, test_name: &str) {
        println!("\n=== {} Config ===", test_name);
        println!("  SOAK_BOUNDARIES:    {}", self.boundaries);
        println!("  SOAK_BATCH_SIZE:    {}", self.batch_size);
        println!("  SOAK_POLL_MS:       {}", self.poll_ms);
        println!("  SOAK_YIELD_EVERY:   {}", self.yield_every);
        println!("  SOAK_YIELD_MS:      {}", self.yield_ms);
        println!("  SOAK_SETTLE_MS:     {}", self.settle_ms);
        println!("  SOAK_DURATION_SECS: {}", self.duration_secs);
        println!("  SOAK_SOURCES:       {}", self.sources);
        println!("  SOAK_INJECTORS:     {}", self.injectors);
    }
}

/// A task that counts executions atomically.
struct CountingTask {
    id: String,
    count: Arc<AtomicU64>,
}

impl CountingTask {
    fn new(id: &str, count: Arc<AtomicU64>) -> Self {
        Self {
            id: id.into(),
            count,
        }
    }
}

#[async_trait::async_trait]
impl Task for CountingTask {
    async fn execute(
        &self,
        context: cloacina_workflow::Context<serde_json::Value>,
    ) -> Result<cloacina_workflow::Context<serde_json::Value>, cloacina_workflow::TaskError> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(context)
    }
    fn id(&self) -> &str {
        &self.id
    }
    fn dependencies(&self) -> &[cloacina_workflow::TaskNamespace] {
        &[]
    }
}

fn print_results(
    test_name: &str,
    config: &SoakConfig,
    fired: &[cloacina::continuous::scheduler::FiredTask],
    exec_count: u64,
    ledger_len: usize,
    elapsed: std::time::Duration,
) {
    let fired_count = fired.len() as u64;
    let fired_errors = fired.iter().filter(|f| f.error.is_some()).count();
    let total_boundaries = config.boundaries;

    println!("\n=== {} Results ===", test_name);
    println!("  Boundaries injected:  {}", total_boundaries);
    println!("  Tasks fired:          {}", fired_count);
    println!("  Tasks executed:       {}", exec_count);
    println!("  Task errors:          {}", fired_errors);
    println!("  Ledger events:        {}", ledger_len);
    println!("  Wall time:            {:.2?}", elapsed);
    println!(
        "  Injection rate:       {:.0} boundaries/sec",
        total_boundaries as f64 / elapsed.as_secs_f64()
    );
    if fired_count > 0 {
        println!(
            "  Fire rate:            {:.1} tasks/sec",
            fired_count as f64 / elapsed.as_secs_f64()
        );
    }
    let passed = fired_count > 0 && fired_errors == 0 && exec_count == fired_count;
    println!("\n  {}", if passed { "PASS" } else { "FAIL" });
    println!("==============================");
}

/// Sustained load: inject N boundaries, verify all are processed.
#[tokio::test]
async fn test_continuous_soak_sustained_load() {
    let config = SoakConfig::from_env();
    config.print("Sustained Load");

    let task_exec_count = Arc::new(AtomicU64::new(0));

    let graph = assemble_graph(
        vec![make_source("events")],
        vec![ContinuousTaskRegistration {
            id: "process".into(),
            sources: vec!["events".into()],
            referenced: vec![],
        }],
    )
    .unwrap();

    let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
    let mut scheduler = ContinuousScheduler::new(
        graph,
        ledger.clone(),
        ContinuousSchedulerConfig {
            poll_interval: Duration::from_millis(config.poll_ms),
            max_fired_tasks: (config.boundaries * 2) as usize,
            task_timeout: None,
        },
    );
    scheduler.register_task(Arc::new(CountingTask::new(
        "process",
        task_exec_count.clone(),
    )));

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    let start = Instant::now();

    // Inject boundaries at sustained rate using N concurrent injectors
    let num_injectors = config.injectors.max(1);
    let batches_total = config.boundaries / config.batch_size.max(1);
    let batches_per_injector = batches_total / num_injectors;
    let batch_size = config.batch_size;
    let yield_every = config.yield_every;
    let yield_ms = config.yield_ms;
    let duration_limit = if config.duration_secs > 0 {
        Some(Duration::from_secs(config.duration_secs))
    } else {
        None
    };

    let mut inject_handles = Vec::new();
    for injector_id in 0..num_injectors {
        let ledger_clone = ledger.clone();
        let start_clone = start;
        let handle = tokio::spawn(async move {
            let base_offset = (injector_id * batches_per_injector * batch_size) as i64 * 100;
            let mut boundary_offset = base_offset;

            for i in 0..batches_per_injector {
                if let Some(limit) = duration_limit {
                    if start_clone.elapsed() > limit {
                        break;
                    }
                }

                let boundaries: Vec<ComputationBoundary> = (0..batch_size)
                    .map(|_| {
                        let b = make_boundary(boundary_offset, boundary_offset + 100);
                        boundary_offset += 100;
                        b
                    })
                    .collect();

                {
                    let mut l = ledger_clone.write();
                    l.append(make_detector_completion("detect_events", boundaries));
                }

                if yield_every > 0 && i % yield_every == 0 {
                    tokio::time::sleep(Duration::from_millis(yield_ms)).await;
                }
            }
        });
        inject_handles.push(handle);
    }

    for h in inject_handles {
        h.await.unwrap();
    }

    // Settle — let scheduler process remaining
    tokio::time::sleep(Duration::from_millis(config.settle_ms)).await;

    tx.send(true).unwrap();
    let fired = handle.await.unwrap();
    let elapsed = start.elapsed();

    let exec_count = task_exec_count.load(Ordering::SeqCst);
    let ledger_len = ledger.read().len();

    print_results(
        "Sustained Load",
        &config,
        &fired,
        exec_count,
        ledger_len,
        elapsed,
    );

    // Assertions
    assert!(fired.len() > 0, "At least one task should have fired");
    assert_eq!(
        exec_count,
        fired.len() as u64,
        "All fired tasks should have executed"
    );
    assert!(
        fired.iter().all(|f| f.error.is_none()),
        "No task errors expected"
    );
    let expected_injections = config.boundaries / config.batch_size.max(1);
    assert!(
        ledger_len > expected_injections as usize,
        "Ledger should have more events than just injections ({} > {})",
        ledger_len,
        expected_injections
    );
}

/// High throughput batched: multiple boundaries per detector completion.
#[tokio::test]
async fn test_continuous_soak_batched_boundaries() {
    let mut config = SoakConfig::from_env();
    if config.batch_size <= 1 {
        config.batch_size = 5; // Force batching for this test
    }
    config.print("Batched Boundaries");

    let task_exec_count = Arc::new(AtomicU64::new(0));

    let graph = assemble_graph(
        vec![make_source("events")],
        vec![ContinuousTaskRegistration {
            id: "aggregate".into(),
            sources: vec!["events".into()],
            referenced: vec![],
        }],
    )
    .unwrap();

    let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
    let mut scheduler = ContinuousScheduler::new(
        graph,
        ledger.clone(),
        ContinuousSchedulerConfig {
            poll_interval: Duration::from_millis(config.poll_ms),
            max_fired_tasks: (config.boundaries * 2) as usize,
            task_timeout: None,
        },
    );
    scheduler.register_task(Arc::new(CountingTask::new(
        "aggregate",
        task_exec_count.clone(),
    )));

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    let start = Instant::now();

    let ledger_clone = ledger.clone();
    let batches = config.boundaries / config.batch_size.max(1);
    let batch_size = config.batch_size;
    let yield_every = config.yield_every;
    let yield_ms = config.yield_ms;

    tokio::spawn(async move {
        let mut offset: i64 = 0;
        for i in 0..batches {
            let boundaries: Vec<ComputationBoundary> = (0..batch_size)
                .map(|_| {
                    let b = make_boundary(offset, offset + 100);
                    offset += 100;
                    b
                })
                .collect();

            {
                let mut l = ledger_clone.write();
                l.append(make_detector_completion("detect_events", boundaries));
            }

            if yield_every > 0 && i % yield_every == 0 {
                tokio::time::sleep(Duration::from_millis(yield_ms)).await;
            }
        }
    })
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(config.settle_ms)).await;
    tx.send(true).unwrap();
    let fired = handle.await.unwrap();
    let elapsed = start.elapsed();

    let exec_count = task_exec_count.load(Ordering::SeqCst);
    let ledger_len = ledger.read().len();

    print_results(
        "Batched Boundaries",
        &config,
        &fired,
        exec_count,
        ledger_len,
        elapsed,
    );

    assert!(fired.len() > 0, "At least one task should have fired");
    assert!(
        fired.iter().all(|f| f.error.is_none()),
        "No errors expected"
    );
}

/// Multi-source sustained load: N sources feeding one task.
#[tokio::test]
async fn test_continuous_soak_multi_source() {
    let config = SoakConfig::from_env();
    config.print("Multi-Source");

    let task_exec_count = Arc::new(AtomicU64::new(0));

    // Create N sources
    let sources: Vec<DataSource> = (0..config.sources)
        .map(|i| make_source(&format!("source_{}", i)))
        .collect();
    let source_names: Vec<String> = sources.iter().map(|s| s.name.clone()).collect();

    let graph = assemble_graph(
        sources,
        vec![ContinuousTaskRegistration {
            id: "join_all".into(),
            sources: source_names.clone(),
            referenced: vec![],
        }],
    )
    .unwrap();

    let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
    let mut scheduler = ContinuousScheduler::new(
        graph,
        ledger.clone(),
        ContinuousSchedulerConfig {
            poll_interval: Duration::from_millis(config.poll_ms),
            max_fired_tasks: (config.boundaries * 2) as usize,
            task_timeout: None,
        },
    );
    scheduler.register_task(Arc::new(CountingTask::new(
        "join_all",
        task_exec_count.clone(),
    )));

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    let start = Instant::now();

    let ledger_clone = ledger.clone();
    let num_sources = config.sources;
    let boundaries = config.boundaries;
    let yield_every = config.yield_every;
    let yield_ms = config.yield_ms;

    tokio::spawn(async move {
        for i in 0..boundaries {
            let source_idx = (i % num_sources) as usize;
            let source_name = format!("detect_source_{}", source_idx);

            {
                let mut l = ledger_clone.write();
                l.append(make_detector_completion(
                    &source_name,
                    vec![make_boundary(i as i64 * 50, (i as i64 + 1) * 50)],
                ));
            }

            if yield_every > 0 && i % yield_every == 0 {
                tokio::time::sleep(Duration::from_millis(yield_ms)).await;
            }
        }
    })
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(config.settle_ms)).await;
    tx.send(true).unwrap();
    let fired = handle.await.unwrap();
    let elapsed = start.elapsed();

    let exec_count = task_exec_count.load(Ordering::SeqCst);
    let ledger_len = ledger.read().len();

    print_results(
        "Multi-Source",
        &config,
        &fired,
        exec_count,
        ledger_len,
        elapsed,
    );

    assert!(
        fired.len() > 0,
        "Join task should fire from multi-source input"
    );
    assert!(
        fired.iter().all(|f| f.error.is_none()),
        "No errors expected"
    );
}
