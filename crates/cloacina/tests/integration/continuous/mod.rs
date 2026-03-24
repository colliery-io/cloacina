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

//! Integration tests for the continuous scheduling pipeline.
//!
//! These tests exercise the full reactive loop:
//! detector output → accumulator → task fires → ledger records completion

pub mod accumulator_persistence;
pub mod recovery_e2e;
pub mod runner_lifecycle;
pub mod soak;

use chrono::Utc;
use cloacina::continuous::accumulator::{SignalAccumulator, WatermarkMode, WindowedAccumulator};
use cloacina::continuous::boundary::{BoundaryKind, ComputationBoundary};
use cloacina::continuous::datasource::{
    ConnectionDescriptor, DataConnection, DataConnectionError, DataSource, DataSourceMetadata,
};
use cloacina::continuous::detector::{DetectorOutput, DETECTOR_OUTPUT_KEY};
use cloacina::continuous::graph::{assemble_graph, ContinuousTaskRegistration};
use cloacina::continuous::ledger::{ExecutionLedger, LedgerEvent};
use cloacina::continuous::ledger_trigger::{LedgerMatchMode, LedgerTrigger};
use cloacina::continuous::scheduler::{ContinuousScheduler, ContinuousSchedulerConfig};
use cloacina::continuous::trigger_policy::Immediate;
use cloacina::continuous::watermark::BoundaryLedger;
use cloacina::trigger::Trigger;
use cloacina_workflow::Task;

/// A simple continuous task for integration tests that passes through context.
struct PassthroughTask {
    id: String,
}

impl PassthroughTask {
    fn new(id: &str) -> Self {
        Self { id: id.into() }
    }
}

#[async_trait::async_trait]
impl Task for PassthroughTask {
    async fn execute(
        &self,
        mut context: cloacina_workflow::Context<serde_json::Value>,
    ) -> Result<cloacina_workflow::Context<serde_json::Value>, cloacina_workflow::TaskError> {
        let _ = context.insert("executed_by", serde_json::json!(self.id));
        Ok(context)
    }
    fn id(&self) -> &str {
        &self.id
    }
    fn dependencies(&self) -> &[cloacina_workflow::TaskNamespace] {
        &[]
    }
}
use cloacina_workflow::Context;
use parking_lot::RwLock;
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

struct MockConn;
impl DataConnection for MockConn {
    fn connect(&self) -> Result<Box<dyn Any>, DataConnectionError> {
        Ok(Box::new("mock_handle".to_string()))
    }
    fn descriptor(&self) -> ConnectionDescriptor {
        ConnectionDescriptor {
            system_type: "mock".into(),
            location: "test".into(),
        }
    }
    fn system_metadata(&self) -> serde_json::Value {
        serde_json::json!({})
    }
}

fn make_source(name: &str) -> DataSource {
    DataSource {
        name: name.into(),
        connection: Box::new(MockConn),
        detector_workflow: format!("detect_{}", name),
        lineage: DataSourceMetadata::default(),
    }
}

fn make_boundary(start: i64, end: i64) -> ComputationBoundary {
    ComputationBoundary {
        kind: BoundaryKind::OffsetRange { start, end },
        metadata: Some(serde_json::json!({"row_count": end - start})),
        emitted_at: Utc::now(),
    }
}

fn make_detector_completion(task_name: &str, boundaries: Vec<ComputationBoundary>) -> LedgerEvent {
    let mut ctx = Context::new();
    let output = DetectorOutput::Change { boundaries };
    ctx.insert(DETECTOR_OUTPUT_KEY, serde_json::to_value(&output).unwrap())
        .unwrap();
    LedgerEvent::TaskCompleted {
        task: task_name.into(),
        at: Utc::now(),
        context: ctx,
    }
}

/// Full reactive loop: detector emits boundaries → accumulator receives → task fires.
#[tokio::test]
async fn test_full_reactive_loop() {
    let graph = assemble_graph(
        vec![make_source("raw_events")],
        vec![ContinuousTaskRegistration {
            id: "aggregate_hourly".into(),
            sources: vec!["raw_events".into()],
            referenced: vec![],
        }],
    )
    .unwrap();

    let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));

    // Simulate detector workflow completing with boundaries
    {
        let mut l = ledger.write();
        l.append(make_detector_completion(
            "detect_raw_events",
            vec![make_boundary(0, 100), make_boundary(100, 200)],
        ));
    }

    let mut scheduler = ContinuousScheduler::new(
        graph,
        ledger.clone(),
        ContinuousSchedulerConfig {
            poll_interval: Duration::from_millis(10),
            max_fired_tasks: 10_000,
            task_timeout: None,
        },
    );
    scheduler.register_task(std::sync::Arc::new(PassthroughTask::new(
        "aggregate_hourly",
    )));

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    // Let the scheduler process
    tokio::time::sleep(Duration::from_millis(100)).await;
    tx.send(true).unwrap();

    let fired = handle.await.unwrap();
    assert_eq!(fired.len(), 1, "expected exactly one task to fire");
    assert_eq!(fired[0].task_id, "aggregate_hourly");
    assert!(fired[0].executed, "task should have been executed");
    assert!(fired[0].error.is_none(), "task should not have errored");

    // Verify TaskCompleted was written to ledger with task output
    let l = ledger.read();
    let all_events = l.events_since(0);
    let completed: Vec<_> = all_events
        .iter()
        .filter(|e| {
            if let LedgerEvent::TaskCompleted { task, .. } = e {
                task == "aggregate_hourly"
            } else {
                false
            }
        })
        .collect();
    assert!(
        !completed.is_empty(),
        "ledger should contain TaskCompleted for aggregate_hourly"
    );
}

/// Multiple detector outputs accumulate before firing.
#[tokio::test]
async fn test_multiple_detector_outputs_accumulate() {
    let graph = assemble_graph(
        vec![make_source("clicks")],
        vec![ContinuousTaskRegistration {
            id: "click_agg".into(),
            sources: vec!["clicks".into()],
            referenced: vec![],
        }],
    )
    .unwrap();

    let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));

    // Two separate detector completions
    {
        let mut l = ledger.write();
        l.append(make_detector_completion(
            "detect_clicks",
            vec![make_boundary(0, 50)],
        ));
        l.append(make_detector_completion(
            "detect_clicks",
            vec![make_boundary(50, 100)],
        ));
    }

    let scheduler = ContinuousScheduler::new(
        graph,
        ledger.clone(),
        ContinuousSchedulerConfig {
            poll_interval: Duration::from_millis(10),
            max_fired_tasks: 10_000,
            task_timeout: None,
        },
    );

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    tokio::time::sleep(Duration::from_millis(100)).await;
    tx.send(true).unwrap();

    let fired = handle.await.unwrap();
    // With Immediate policy, the first poll fires (draining all), then possibly a second
    // The key assertion: at least one fire happened
    assert!(!fired.is_empty(), "expected at least one task to fire");
    assert_eq!(fired[0].task_id, "click_agg");
}

/// Multi-source task: boundaries arrive on two sources.
#[tokio::test]
async fn test_multi_source_task() {
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

    // Only one source fires — JoinMode::Any should still fire the task
    {
        let mut l = ledger.write();
        l.append(make_detector_completion(
            "detect_clicks",
            vec![make_boundary(0, 100)],
        ));
    }

    let scheduler = ContinuousScheduler::new(
        graph,
        ledger.clone(),
        ContinuousSchedulerConfig {
            poll_interval: Duration::from_millis(10),
            max_fired_tasks: 10_000,
            task_timeout: None,
        },
    );

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    tokio::time::sleep(Duration::from_millis(100)).await;
    tx.send(true).unwrap();

    let fired = handle.await.unwrap();
    assert!(
        !fired.is_empty(),
        "JoinMode::Any should fire with only one source ready"
    );
}

/// Ledger records accumulator drains.
#[tokio::test]
async fn test_ledger_records_drains() {
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

    {
        let mut l = ledger.write();
        l.append(make_detector_completion(
            "detect_events",
            vec![make_boundary(0, 50)],
        ));
    }

    let scheduler = ContinuousScheduler::new(
        graph,
        ledger.clone(),
        ContinuousSchedulerConfig {
            poll_interval: Duration::from_millis(10),
            max_fired_tasks: 10_000,
            task_timeout: None,
        },
    );

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    tokio::time::sleep(Duration::from_millis(100)).await;
    tx.send(true).unwrap();
    handle.await.unwrap();

    // Check ledger has AccumulatorDrained events (beyond the initial TaskCompleted)
    let l = ledger.read();
    let all_events = l.events_since(0);
    let drain_events: Vec<_> = all_events
        .iter()
        .filter(|e| matches!(e, LedgerEvent::AccumulatorDrained { .. }))
        .collect();
    assert!(
        !drain_events.is_empty(),
        "ledger should contain AccumulatorDrained events"
    );
}

// === I-0024 Integration Tests ===

/// WindowedAccumulator with WaitForWatermark blocks until watermark covers boundary.
#[tokio::test]
async fn test_windowed_accumulator_waits_for_watermark() {
    let bl = std::sync::Arc::new(RwLock::new(BoundaryLedger::new()));

    let mut acc = WindowedAccumulator::new(
        Box::new(Immediate),
        WatermarkMode::WaitForWatermark,
        bl.clone(),
        "events".into(),
    );

    // Receive boundary [0, 100)
    acc.receive(make_boundary(0, 100));
    assert!(!acc.is_ready(), "should block without watermark");

    // Advance watermark to cover [0, 200)
    {
        let mut ledger = bl.write();
        ledger.advance("events", make_boundary(0, 200)).unwrap();
    }

    assert!(acc.is_ready(), "should fire after watermark covers");

    let ctx = acc.drain();
    assert!(ctx.get("__boundary").is_some());
    assert_eq!(ctx.get("__signals_coalesced"), Some(&serde_json::json!(1)));
}

/// LedgerTrigger completes the reactive feedback loop.
#[tokio::test]
async fn test_ledger_trigger_feedback_loop() {
    let ledger = std::sync::Arc::new(RwLock::new(ExecutionLedger::new()));

    // Set up trigger watching for "aggregate_hourly" completion
    let trigger = LedgerTrigger::new(
        "detect_hourly_stats".into(),
        vec!["aggregate_hourly".into()],
        LedgerMatchMode::Any,
        ledger.clone(),
    );

    // No events yet — should skip
    let result = trigger.poll().await.unwrap();
    assert!(!result.should_fire());

    // Simulate aggregate_hourly completing
    {
        let mut l = ledger.write();
        l.append(LedgerEvent::TaskCompleted {
            task: "aggregate_hourly".into(),
            at: Utc::now(),
            context: cloacina_workflow::Context::new(),
        });
    }

    // Now trigger should fire
    let result = trigger.poll().await.unwrap();
    assert!(
        result.should_fire(),
        "LedgerTrigger should fire after task completion"
    );

    // Second poll — cursor advanced, should skip
    let result = trigger.poll().await.unwrap();
    assert!(!result.should_fire(), "should not re-fire on same event");
}

/// LedgerTrigger All mode: waits for both upstream tasks.
#[tokio::test]
async fn test_ledger_trigger_all_mode_multi_dependency() {
    let ledger = std::sync::Arc::new(RwLock::new(ExecutionLedger::new()));

    let trigger = LedgerTrigger::new(
        "detect_joined_data".into(),
        vec!["task_a".into(), "task_b".into()],
        LedgerMatchMode::All,
        ledger.clone(),
    );

    // Only task_a completes
    {
        let mut l = ledger.write();
        l.append(LedgerEvent::TaskCompleted {
            task: "task_a".into(),
            at: Utc::now(),
            context: cloacina_workflow::Context::new(),
        });
    }
    let result = trigger.poll().await.unwrap();
    assert!(!result.should_fire(), "All mode should wait for both tasks");

    // task_b completes
    {
        let mut l = ledger.write();
        l.append(LedgerEvent::TaskCompleted {
            task: "task_b".into(),
            at: Utc::now(),
            context: cloacina_workflow::Context::new(),
        });
    }
    let result = trigger.poll().await.unwrap();
    assert!(
        result.should_fire(),
        "All mode should fire after both tasks complete"
    );
}

/// Full scheduler loop with watermark advance via Both output.
#[tokio::test]
async fn test_scheduler_watermark_advance_via_both() {
    let graph = assemble_graph(
        vec![make_source("events")],
        vec![ContinuousTaskRegistration {
            id: "agg".into(),
            sources: vec!["events".into()],
            referenced: vec![],
        }],
    )
    .unwrap();

    let ledger = std::sync::Arc::new(RwLock::new(ExecutionLedger::new()));

    // Detector emits Both: boundaries + watermark
    {
        let mut ctx = cloacina_workflow::Context::new();
        let output = DetectorOutput::Both {
            boundaries: vec![make_boundary(0, 100)],
            watermark: make_boundary(0, 500),
        };
        ctx.insert(DETECTOR_OUTPUT_KEY, serde_json::to_value(&output).unwrap())
            .unwrap();
        let mut l = ledger.write();
        l.append(LedgerEvent::TaskCompleted {
            task: "detect_events".into(),
            at: Utc::now(),
            context: ctx,
        });
    }

    let scheduler = ContinuousScheduler::new(
        graph,
        ledger.clone(),
        ContinuousSchedulerConfig {
            poll_interval: Duration::from_millis(10),
            max_fired_tasks: 10_000,
            task_timeout: None,
        },
    );

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    tokio::time::sleep(Duration::from_millis(100)).await;
    tx.send(true).unwrap();

    let fired = handle.await.unwrap();
    assert!(!fired.is_empty(), "task should fire from Both output");
}

/// Multi-cycle reactive loop: source → task_a → derived source → task_b.
///
/// This tests the core "make"-like behavior: task_a completes, which triggers
/// a derived detector via LedgerTrigger, which produces boundaries for task_b.
///
/// Since the scheduler doesn't actually execute tasks (that's the Dispatcher's job),
/// we simulate the multi-cycle by:
/// 1. Pre-load detector completion for source "raw_events" → fires task_a ("aggregate")
/// 2. After scheduler processes cycle 1, inject a TaskCompleted for "aggregate"
///    with DetectorOutput for derived source "hourly_stats" → fires task_b ("dashboard")
#[tokio::test]
async fn test_multi_cycle_reactive_loop() {
    // Two-level graph:
    //   raw_events → aggregate (task_a)
    //   hourly_stats → dashboard (task_b)
    let graph = assemble_graph(
        vec![make_source("raw_events"), make_source("hourly_stats")],
        vec![
            ContinuousTaskRegistration {
                id: "aggregate".into(),
                sources: vec!["raw_events".into()],
                referenced: vec![],
            },
            ContinuousTaskRegistration {
                id: "dashboard".into(),
                sources: vec!["hourly_stats".into()],
                referenced: vec![],
            },
        ],
    )
    .unwrap();

    let ledger = std::sync::Arc::new(RwLock::new(ExecutionLedger::new()));

    // Cycle 1: raw_events detector completed with boundaries
    {
        let mut l = ledger.write();
        l.append(make_detector_completion(
            "detect_raw_events",
            vec![make_boundary(0, 100)],
        ));
    }

    let scheduler = ContinuousScheduler::new(
        graph,
        ledger.clone(),
        ContinuousSchedulerConfig {
            poll_interval: Duration::from_millis(10),
            max_fired_tasks: 10_000,
            task_timeout: None,
        },
    );

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    // Let cycle 1 process (aggregate fires)
    tokio::time::sleep(Duration::from_millis(80)).await;

    // Simulate: aggregate task completed, which would trigger
    // the derived detector for hourly_stats.
    // In production, LedgerTrigger would fire detect_hourly_stats.
    // Here we simulate that detector completing:
    {
        let mut l = ledger.write();

        // First: aggregate completed (this is what LedgerTrigger would see)
        l.append(LedgerEvent::TaskCompleted {
            task: "aggregate".into(),
            at: Utc::now(),
            context: cloacina_workflow::Context::new(),
        });

        // Then: the derived detector for hourly_stats found changes
        l.append(make_detector_completion(
            "detect_hourly_stats",
            vec![make_boundary(0, 50)],
        ));
    }

    // Let cycle 2 process (dashboard fires)
    tokio::time::sleep(Duration::from_millis(80)).await;
    tx.send(true).unwrap();

    let fired = handle.await.unwrap();

    // Both tasks should have fired
    let fired_ids: Vec<&str> = fired.iter().map(|f| f.task_id.as_str()).collect();
    assert!(
        fired_ids.contains(&"aggregate"),
        "aggregate should have fired in cycle 1, got: {:?}",
        fired_ids
    );
    assert!(
        fired_ids.contains(&"dashboard"),
        "dashboard should have fired in cycle 2, got: {:?}",
        fired_ids
    );

    // Verify ledger has events from both cycles
    let l = ledger.read();
    let all_events = l.events_since(0);

    let drain_events: Vec<&str> = all_events
        .iter()
        .filter_map(|e| {
            if let LedgerEvent::AccumulatorDrained { task, .. } = e {
                Some(task.as_str())
            } else {
                None
            }
        })
        .collect();

    assert!(
        drain_events.contains(&"aggregate"),
        "ledger should record aggregate drain"
    );
    assert!(
        drain_events.contains(&"dashboard"),
        "ledger should record dashboard drain"
    );
}

/// LedgerTrigger integration: verify it correctly bridges task completion
/// to detector firing in the reactive loop.
#[tokio::test]
async fn test_ledger_trigger_bridges_cycles() {
    let ledger = std::sync::Arc::new(RwLock::new(ExecutionLedger::new()));

    // LedgerTrigger watches for "aggregate" completion
    let trigger = LedgerTrigger::new(
        "detect_hourly_stats".into(),
        vec!["aggregate".into()],
        LedgerMatchMode::Any,
        ledger.clone(),
    );

    // Step 1: No completions yet
    let result = trigger.poll().await.unwrap();
    assert!(!result.should_fire(), "should not fire without events");

    // Step 2: aggregate completes
    {
        let mut l = ledger.write();
        l.append(LedgerEvent::TaskCompleted {
            task: "aggregate".into(),
            at: Utc::now(),
            context: cloacina_workflow::Context::new(),
        });
    }

    // Step 3: Trigger fires — this would kick off detect_hourly_stats
    let result = trigger.poll().await.unwrap();
    assert!(
        result.should_fire(),
        "should fire after aggregate completes"
    );

    // Step 4: Cursor advanced — doesn't re-fire on same event
    let result = trigger.poll().await.unwrap();
    assert!(!result.should_fire(), "should not re-fire");

    // Step 5: aggregate completes again (next cycle)
    {
        let mut l = ledger.write();
        l.append(LedgerEvent::TaskCompleted {
            task: "aggregate".into(),
            at: Utc::now(),
            context: cloacina_workflow::Context::new(),
        });
    }

    // Step 6: Fires again for the new completion
    let result = trigger.poll().await.unwrap();
    assert!(
        result.should_fire(),
        "should fire again for new aggregate completion"
    );
}
