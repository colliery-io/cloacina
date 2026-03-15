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

//! Continuous Scheduling Example
//!
//! Demonstrates the reactive scheduling pipeline:
//! 1. A simulated detector emits boundaries for a data source
//! 2. Boundaries are buffered in an accumulator
//! 3. When the trigger policy fires, the task executes with coalesced boundaries
//! 4. The execution is recorded in the ledger

use chrono::Utc;
use cloacina::continuous::boundary::{BoundaryKind, ComputationBoundary};
use cloacina::continuous::datasource::{
    ConnectionDescriptor, DataConnection, DataConnectionError, DataSource, DataSourceMetadata,
};
use cloacina::continuous::detector::{DetectorOutput, DETECTOR_OUTPUT_KEY};
use cloacina::continuous::graph::{assemble_graph, ContinuousTaskRegistration};
use cloacina::continuous::ledger::{ExecutionLedger, LedgerEvent};
use cloacina::continuous::scheduler::{ContinuousScheduler, ContinuousSchedulerConfig};
use cloacina_workflow::{Context, Task, TaskError, TaskNamespace};
use std::any::Any;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::watch;

/// The actual continuous task that processes aggregated data.
struct AggregateHourlyTask;

#[async_trait::async_trait]
impl Task for AggregateHourlyTask {
    async fn execute(
        &self,
        mut context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        // Read the boundary the scheduler injected
        let boundary = context.get("__boundary").cloned();
        let signals = context.get("__signals_coalesced").cloned();

        println!("   [TASK EXECUTING] aggregate_hourly");
        if let Some(b) = &boundary {
            if let Some(kind) = b.get("kind") {
                println!(
                    "   Processing offsets [{}, {})",
                    kind.get("start").unwrap_or(&serde_json::json!("?")),
                    kind.get("end").unwrap_or(&serde_json::json!("?"))
                );
            }
        }
        if let Some(s) = &signals {
            println!("   Coalesced from {} detector signals", s);
        }

        // Simulate doing real work
        let _ = context.insert("aggregation_complete", serde_json::json!(true));
        let _ = context.insert("rows_aggregated", serde_json::json!(250));

        println!("   [TASK COMPLETE] aggregate_hourly — 250 rows aggregated");
        Ok(context)
    }

    fn id(&self) -> &str {
        "aggregate_hourly"
    }

    fn dependencies(&self) -> &[TaskNamespace] {
        &[]
    }
}

/// Simulated database connection for the example.
struct SimulatedDbConnection {
    table: String,
}

impl DataConnection for SimulatedDbConnection {
    fn connect(&self) -> Result<Box<dyn Any>, DataConnectionError> {
        Ok(Box::new(format!("connection_to_{}", self.table)))
    }

    fn descriptor(&self) -> ConnectionDescriptor {
        ConnectionDescriptor {
            system_type: "simulated_db".into(),
            location: format!("localhost/{}", self.table),
        }
    }

    fn system_metadata(&self) -> serde_json::Value {
        serde_json::json!({ "table": self.table })
    }
}

#[tokio::main]
async fn main() {
    println!("=== Continuous Scheduling Example ===\n");

    // Step 1: Define data sources
    println!("1. Defining data sources...");
    let raw_events_source = DataSource {
        name: "raw_events".into(),
        connection: Box::new(SimulatedDbConnection {
            table: "raw_events".into(),
        }),
        detector_workflow: "detect_raw_events".into(),
        lineage: DataSourceMetadata {
            description: Some("Raw event stream from application".into()),
            owner: Some("data-platform".into()),
            tags: vec!["events".into(), "raw".into()],
        },
    };

    let config_source = DataSource {
        name: "config_table".into(),
        connection: Box::new(SimulatedDbConnection {
            table: "config".into(),
        }),
        detector_workflow: "detect_config_changes".into(),
        lineage: DataSourceMetadata::default(),
    };

    println!(
        "   - {} ({})",
        raw_events_source.name,
        raw_events_source.connection.descriptor()
    );
    println!(
        "   - {} ({})",
        config_source.name,
        config_source.connection.descriptor()
    );

    // Step 2: Assemble the graph
    println!("\n2. Assembling reactive graph...");
    let graph = assemble_graph(
        vec![raw_events_source, config_source],
        vec![ContinuousTaskRegistration {
            id: "aggregate_hourly".into(),
            sources: vec!["raw_events".into()],
            referenced: vec!["config_table".into()],
        }],
    )
    .expect("graph assembly failed");

    println!(
        "   Graph: {} data sources, {} tasks, {} edges",
        graph.data_sources.len(),
        graph.tasks.len(),
        graph.edges.len()
    );

    // Step 3: Create ledger and scheduler
    println!("\n3. Starting continuous scheduler...");
    let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));

    // Simulate detector workflow completions arriving in the ledger
    // (In production, these come from cron-triggered detector workflows)
    {
        let mut l = ledger.write().unwrap();

        // First batch: offsets 0-100
        let mut ctx1 = Context::new();
        let output1 = DetectorOutput::Change {
            boundaries: vec![ComputationBoundary {
                kind: BoundaryKind::OffsetRange { start: 0, end: 100 },
                metadata: Some(serde_json::json!({"row_count": 100})),
                emitted_at: Utc::now(),
            }],
        };
        ctx1.insert(DETECTOR_OUTPUT_KEY, serde_json::to_value(&output1).unwrap())
            .unwrap();
        l.append(LedgerEvent::TaskCompleted {
            task: "detect_raw_events".into(),
            at: Utc::now(),
            context: ctx1,
        });
        println!("   Simulated detector: raw_events changed (offsets 0-100)");

        // Second batch: offsets 100-250
        let mut ctx2 = Context::new();
        let output2 = DetectorOutput::Change {
            boundaries: vec![ComputationBoundary {
                kind: BoundaryKind::OffsetRange {
                    start: 100,
                    end: 250,
                },
                metadata: Some(serde_json::json!({"row_count": 150})),
                emitted_at: Utc::now(),
            }],
        };
        ctx2.insert(DETECTOR_OUTPUT_KEY, serde_json::to_value(&output2).unwrap())
            .unwrap();
        l.append(LedgerEvent::TaskCompleted {
            task: "detect_raw_events".into(),
            at: Utc::now(),
            context: ctx2,
        });
        println!("   Simulated detector: raw_events changed (offsets 100-250)");
    }

    // Step 4: Run the scheduler with REAL task execution
    let mut scheduler = ContinuousScheduler::new(
        graph,
        ledger.clone(),
        ContinuousSchedulerConfig {
            poll_interval: Duration::from_millis(10),
        },
    );

    // Register the actual task — this is what makes it execute
    scheduler.register_task(Arc::new(AggregateHourlyTask));

    let (tx, rx) = watch::channel(false);
    let handle = tokio::spawn(async move { scheduler.run(rx).await });

    // Let it process
    tokio::time::sleep(Duration::from_millis(200)).await;
    tx.send(true).unwrap();

    let fired = handle.await.unwrap();

    // Step 5: Report results
    println!("\n4. Results:");
    println!("   Tasks fired: {}", fired.len());
    for task in &fired {
        println!(
            "   - {} | executed: {} | error: {:?}",
            task.task_id, task.executed, task.error
        );
    }

    // Check ledger
    let l = ledger.read().unwrap();
    println!("\n5. Execution Ledger ({} events):", l.len());
    for (i, event) in l.events_since(0).iter().enumerate() {
        match event {
            LedgerEvent::TaskCompleted { task, .. } => {
                println!("   [{}] TaskCompleted: {}", i, task);
            }
            LedgerEvent::AccumulatorDrained { task, .. } => {
                println!("   [{}] AccumulatorDrained: {}", i, task);
            }
            LedgerEvent::BoundaryEmitted { source, .. } => {
                println!("   [{}] BoundaryEmitted: {}", i, source);
            }
            LedgerEvent::TaskFailed { task, error, .. } => {
                println!("   [{}] TaskFailed: {} ({})", i, task, error);
            }
        }
    }

    println!("\n=== Done ===");
}
