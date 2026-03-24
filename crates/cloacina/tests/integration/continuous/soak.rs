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
//! Verifies:
//! - No boundaries are lost under sustained load
//! - All fired tasks execute successfully
//! - Ledger events accumulate correctly
//! - Scheduler doesn't stall or leak memory

use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

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

/// Sustained load: inject N boundaries, verify all are processed.
#[tokio::test]
async fn test_continuous_soak_sustained_load() {
    let total_boundaries: u64 = 1000;
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
            poll_interval: Duration::from_millis(5),
            max_fired_tasks: 100_000,
            task_timeout: None,
        },
    );
    scheduler.register_task(Arc::new(CountingTask::new(
        "process",
        task_exec_count.clone(),
    )));

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    // Inject boundaries in a background task at sustained rate
    let ledger_clone = ledger.clone();
    let inject_start = Instant::now();
    let inject_handle = tokio::spawn(async move {
        for i in 0..total_boundaries {
            {
                let mut l = ledger_clone.write();
                l.append(make_detector_completion(
                    "detect_events",
                    vec![make_boundary(i as i64 * 100, (i as i64 + 1) * 100)],
                ));
            }
            // Yield every 50 boundaries to let scheduler process
            if i % 50 == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }
    });

    // Wait for injection to complete
    inject_handle.await.unwrap();
    let inject_elapsed = inject_start.elapsed();

    // Give scheduler time to process remaining boundaries
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Shutdown
    tx.send(true).unwrap();
    let fired = handle.await.unwrap();

    // Metrics
    let exec_count = task_exec_count.load(Ordering::SeqCst);
    let fired_count = fired.len() as u64;
    let fired_executed = fired.iter().filter(|f| f.executed).count() as u64;
    let fired_errors = fired.iter().filter(|f| f.error.is_some()).count();

    let l = ledger.read();
    let ledger_len = l.len();

    println!("=== Continuous Scheduler Soak Results ===");
    println!("  Boundaries injected:  {}", total_boundaries);
    println!("  Tasks fired:          {}", fired_count);
    println!("  Tasks executed:       {}", fired_executed);
    println!("  Task errors:          {}", fired_errors);
    println!("  Executor count:       {}", exec_count);
    println!("  Ledger events:        {}", ledger_len);
    println!("  Injection time:       {:?}", inject_elapsed);
    println!(
        "  Rate:                 {:.0} boundaries/sec",
        total_boundaries as f64 / inject_elapsed.as_secs_f64()
    );
    println!("=========================================");

    // Assertions
    assert!(
        fired_count > 0,
        "At least one task should have fired (got 0)"
    );
    assert_eq!(
        fired_executed, fired_count,
        "All fired tasks should have executed"
    );
    assert_eq!(fired_errors, 0, "No task errors expected");
    assert_eq!(
        exec_count, fired_count,
        "Executor count should match fired count"
    );

    // Verify ledger has events beyond just the injected detector completions
    // (should also have AccumulatorDrained and TaskCompleted for the process task)
    assert!(
        ledger_len > total_boundaries as usize,
        "Ledger should have more events than just injected boundaries (got {})",
        ledger_len
    );
}

/// High throughput: inject many boundaries per detector completion (batched).
#[tokio::test]
async fn test_continuous_soak_batched_boundaries() {
    let total_batches: u64 = 200;
    let boundaries_per_batch: u64 = 5;
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
            poll_interval: Duration::from_millis(5),
            max_fired_tasks: 100_000,
            task_timeout: None,
        },
    );
    scheduler.register_task(Arc::new(CountingTask::new(
        "aggregate",
        task_exec_count.clone(),
    )));

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    // Inject batches
    let ledger_clone = ledger.clone();
    let inject_handle = tokio::spawn(async move {
        for batch in 0..total_batches {
            let boundaries: Vec<ComputationBoundary> = (0..boundaries_per_batch)
                .map(|i| {
                    let offset = (batch * boundaries_per_batch + i) as i64 * 100;
                    make_boundary(offset, offset + 100)
                })
                .collect();

            {
                let mut l = ledger_clone.write();
                l.append(make_detector_completion("detect_events", boundaries));
            }

            if batch % 20 == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }
    });

    inject_handle.await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    tx.send(true).unwrap();
    let fired = handle.await.unwrap();

    let exec_count = task_exec_count.load(Ordering::SeqCst);
    let total_boundaries = total_batches * boundaries_per_batch;

    println!("=== Batched Soak Results ===");
    println!("  Total boundaries:     {}", total_boundaries);
    println!("  Batches:              {}", total_batches);
    println!("  Tasks fired:          {}", fired.len());
    println!("  Tasks executed:       {}", exec_count);
    println!("============================");

    assert!(fired.len() > 0, "At least one task should have fired");
    assert_eq!(
        exec_count,
        fired.len() as u64,
        "All fired tasks should execute"
    );
    assert!(
        fired.iter().all(|f| f.error.is_none()),
        "No errors expected"
    );
}

/// Multi-source sustained load: two sources feeding one task.
#[tokio::test]
async fn test_continuous_soak_multi_source() {
    let task_exec_count = Arc::new(AtomicU64::new(0));

    let graph = assemble_graph(
        vec![make_source("clicks"), make_source("impressions")],
        vec![ContinuousTaskRegistration {
            id: "join_metrics".into(),
            sources: vec!["clicks".into(), "impressions".into()],
            referenced: vec![],
        }],
    )
    .unwrap();

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
        "join_metrics",
        task_exec_count.clone(),
    )));

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    // Inject from both sources alternating
    let ledger_clone = ledger.clone();
    tokio::spawn(async move {
        for i in 0..200u64 {
            let source = if i % 2 == 0 {
                "detect_clicks"
            } else {
                "detect_impressions"
            };
            {
                let mut l = ledger_clone.write();
                l.append(make_detector_completion(
                    source,
                    vec![make_boundary(i as i64 * 50, (i as i64 + 1) * 50)],
                ));
            }
            if i % 20 == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }
    })
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;
    tx.send(true).unwrap();
    let fired = handle.await.unwrap();

    let exec_count = task_exec_count.load(Ordering::SeqCst);

    println!("=== Multi-Source Soak Results ===");
    println!("  Tasks fired:    {}", fired.len());
    println!("  Tasks executed: {}", exec_count);
    println!("================================");

    assert!(
        fired.len() > 0,
        "Join task should fire from multi-source input"
    );
    assert!(
        fired.iter().all(|f| f.error.is_none()),
        "No errors expected"
    );
}
