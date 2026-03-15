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

use chrono::Utc;
use cloacina::continuous::boundary::{BoundaryKind, ComputationBoundary};
use cloacina::continuous::datasource::{
    ConnectionDescriptor, DataConnection, DataConnectionError, DataSource, DataSourceMetadata,
};
use cloacina::continuous::detector::{DetectorOutput, DETECTOR_OUTPUT_KEY};
use cloacina::continuous::graph::{assemble_graph, ContinuousTaskRegistration};
use cloacina::continuous::ledger::{ExecutionLedger, LedgerEvent};
use cloacina::continuous::scheduler::{ContinuousScheduler, ContinuousSchedulerConfig};
use cloacina_workflow::Context;
use std::any::Any;
use std::sync::{Arc, RwLock};
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
        let mut l = ledger.write().unwrap();
        l.append(make_detector_completion(
            "detect_raw_events",
            vec![make_boundary(0, 100), make_boundary(100, 200)],
        ));
    }

    let scheduler = ContinuousScheduler::new(
        graph,
        ledger.clone(),
        ContinuousSchedulerConfig {
            poll_interval: Duration::from_millis(10),
        },
    );

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    // Let the scheduler process
    tokio::time::sleep(Duration::from_millis(100)).await;
    tx.send(true).unwrap();

    let fired = handle.await.unwrap();
    assert_eq!(fired.len(), 1, "expected exactly one task to fire");
    assert_eq!(fired[0].task_id, "aggregate_hourly");

    // Verify boundary context was produced
    assert!(!fired[0].boundary_context.is_empty());
    let ctx = &fired[0].boundary_context[0];
    assert!(
        ctx.get("__boundary").is_some(),
        "missing __boundary in context"
    );
    assert_eq!(ctx.get("__signals_coalesced"), Some(&serde_json::json!(2)));
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
        let mut l = ledger.write().unwrap();
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
        let mut l = ledger.write().unwrap();
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
        let mut l = ledger.write().unwrap();
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
        },
    );

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    tokio::time::sleep(Duration::from_millis(100)).await;
    tx.send(true).unwrap();
    handle.await.unwrap();

    // Check ledger has AccumulatorDrained events (beyond the initial TaskCompleted)
    let l = ledger.read().unwrap();
    let drain_events: Vec<_> = l
        .events_since(0)
        .iter()
        .filter(|e| matches!(e, LedgerEvent::AccumulatorDrained { .. }))
        .collect();
    assert!(
        !drain_events.is_empty(),
        "ledger should contain AccumulatorDrained events"
    );
}
