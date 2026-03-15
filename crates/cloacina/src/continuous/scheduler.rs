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

//! Continuous scheduler — the core reactive loop orchestrator.
//!
//! Observes detector completions via the `ExecutionLedger`, routes boundaries
//! to per-edge accumulators, checks task readiness, and submits work.
//!
//! See CLOACI-S-0008 for the full specification.

use super::detector::DetectorOutput;
use super::graph::{DataSourceGraph, JoinMode, LateArrivalPolicy};
use super::ledger::{ExecutionLedger, LedgerEvent};
use super::watermark::BoundaryLedger;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::watch;
use tracing::{debug, info};

/// Configuration for the continuous scheduler.
#[derive(Debug, Clone)]
pub struct ContinuousSchedulerConfig {
    /// How often to poll the execution ledger for new events.
    pub poll_interval: Duration,
}

impl Default for ContinuousSchedulerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(100),
        }
    }
}

/// The continuous reactive scheduler.
///
/// Runs as a tokio task, polling the execution ledger for detector completions
/// and routing boundaries through accumulators to fire continuous tasks.
pub struct ContinuousScheduler {
    /// The reactive graph.
    graph: DataSourceGraph,
    /// Shared execution ledger.
    ledger: Arc<RwLock<ExecutionLedger>>,
    /// Source watermark tracking.
    boundary_ledger: Arc<RwLock<BoundaryLedger>>,
    /// Reverse lookup: detector_workflow name → data source name.
    detector_to_source: HashMap<String, String>,
    /// Exit edges: task name → workflow names to fire on completion.
    exit_edges: HashMap<String, Vec<String>>,
    /// Configuration.
    config: ContinuousSchedulerConfig,
}

impl ContinuousScheduler {
    /// Create a new continuous scheduler.
    pub fn new(
        graph: DataSourceGraph,
        ledger: Arc<RwLock<ExecutionLedger>>,
        config: ContinuousSchedulerConfig,
    ) -> Self {
        // Build reverse lookup: detector_workflow → data source name
        let detector_to_source: HashMap<String, String> = graph
            .data_sources
            .iter()
            .map(|(name, ds)| (ds.detector_workflow.clone(), name.clone()))
            .collect();

        Self {
            graph,
            ledger,
            boundary_ledger: Arc::new(RwLock::new(BoundaryLedger::new())),
            detector_to_source,
            exit_edges: HashMap::new(),
            config,
        }
    }

    /// Get a reference to the boundary ledger (for WindowedAccumulator integration).
    pub fn boundary_ledger(&self) -> &Arc<RwLock<BoundaryLedger>> {
        &self.boundary_ledger
    }

    /// Register an exit edge: when `task_id` completes, fire `workflow_name`.
    pub fn add_exit_edge(&mut self, task_id: String, workflow_name: String) {
        self.exit_edges
            .entry(task_id)
            .or_default()
            .push(workflow_name);
    }

    /// Run the continuous scheduling loop.
    ///
    /// This method runs until the shutdown signal is received. It polls the
    /// execution ledger for new events, routes boundaries to accumulators,
    /// checks task readiness, and records completions.
    ///
    /// Returns a `SchedulerHandle` that can be used to submit task completions
    /// and track fired tasks.
    pub async fn run(&self, mut shutdown: watch::Receiver<bool>) -> Vec<FiredTask> {
        let mut cursor: usize = 0;
        let mut fired_tasks: Vec<FiredTask> = Vec::new();

        info!(
            "ContinuousScheduler starting with {} tasks, {} edges",
            self.graph.tasks.len(),
            self.graph.edges.len()
        );

        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("ContinuousScheduler received shutdown signal");
                        break;
                    }
                }
                _ = tokio::time::sleep(self.config.poll_interval) => {
                    // Step 1: Read new ledger events
                    let new_events = {
                        let ledger = self.ledger.read().unwrap();
                        let events = ledger.events_since(cursor);
                        let collected: Vec<_> = events.iter().map(|e| match e {
                            LedgerEvent::TaskCompleted { task, context, .. } => {
                                Some((task.clone(), DetectorOutput::from_context(context)))
                            }
                            _ => None,
                        }).collect();
                        cursor = ledger.len();
                        collected
                    };

                    // Step 2: Process detector completions
                    for event in &new_events {
                        if let Some((task_name, detector_output)) = event {
                            if let Some(output) = detector_output {
                                self.process_detector_output(task_name, output);
                            }
                        }
                    }

                    // Step 3: Check task readiness and fire
                    let ready_tasks = self.check_readiness();
                    for (task_id, drained_contexts) in ready_tasks {
                        debug!("Firing continuous task: {}", task_id);
                        fired_tasks.push(FiredTask {
                            task_id: task_id.clone(),
                            fired_at: Utc::now(),
                            boundary_context: drained_contexts,
                        });

                        // Write completion to ledger (in real impl, this happens
                        // after actual execution via callback)
                        let mut ledger = self.ledger.write().unwrap();
                        ledger.append(LedgerEvent::AccumulatorDrained {
                            task: task_id.clone(),
                            boundary: super::boundary::ComputationBoundary {
                                kind: super::boundary::BoundaryKind::Cursor {
                                    value: format!("fired_{}", fired_tasks.len()),
                                },
                                metadata: None,
                                emitted_at: Utc::now(),
                            },
                        });
                    }
                }
            }
        }

        fired_tasks
    }

    /// Process a detector output: route watermarks and boundaries.
    fn process_detector_output(&self, detector_task: &str, output: &DetectorOutput) {
        // Resolve which data source this detector belongs to
        let source_name = self.detector_to_source.get(detector_task).cloned();

        // Handle watermark advances first
        if let Some(watermark) = output.watermark() {
            if let Some(ref src) = source_name {
                let mut bl = self.boundary_ledger.write().unwrap();
                match bl.advance(src, watermark.clone()) {
                    Ok(()) => {
                        debug!("Watermark advanced for source '{}'", src);
                    }
                    Err(e) => {
                        debug!("Watermark advance rejected for '{}': {}", src, e);
                    }
                }
            }
        }

        // Route change boundaries to accumulators
        let boundaries = output.boundaries();
        if boundaries.is_empty() {
            return;
        }

        // If we know the source, route only to that source's edges
        // Otherwise, broadcast to all edges (fallback for unmatched detectors)
        let target_sources: Vec<String> = if let Some(src) = source_name {
            vec![src]
        } else {
            self.graph.data_sources.keys().cloned().collect()
        };

        for src in &target_sources {
            let edges = self.graph.edges_for_source(src);
            for edge in edges {
                let mut acc = edge.accumulator.lock().unwrap();
                for boundary in boundaries {
                    // Check consumer watermark for late arrival
                    let is_late = if let Some(consumer_wm) = acc.consumer_watermark() {
                        let bl = self.boundary_ledger.read().unwrap();
                        bl.covers(src, boundary)
                    } else {
                        false
                    };

                    if is_late {
                        match &edge.late_arrival_policy {
                            LateArrivalPolicy::Discard => {
                                debug!("Late boundary discarded: {} -> {}", src, edge.task);
                            }
                            LateArrivalPolicy::AccumulateForward => {
                                acc.receive(boundary.clone());
                                debug!("Late boundary forwarded: {} -> {}", src, edge.task);
                            }
                            LateArrivalPolicy::Retrigger => {
                                acc.receive(boundary.clone());
                                debug!("Late boundary retriggered: {} -> {}", src, edge.task);
                            }
                            LateArrivalPolicy::RouteToSideChannel { task_name } => {
                                debug!(
                                    "Late boundary routed to side channel '{}': {} -> {}",
                                    task_name, src, edge.task
                                );
                                // Side channel storage would go here in production
                            }
                        }
                    } else {
                        acc.receive(boundary.clone());
                        debug!("Routed boundary to accumulator: {} -> {}", src, edge.task);
                    }
                }
            }
        }
    }

    /// Check all tasks for readiness based on their JoinMode.
    fn check_readiness(&self) -> Vec<(String, Vec<cloacina_workflow::Context<serde_json::Value>>)> {
        let mut ready = Vec::new();

        for (task_id, config) in &self.graph.tasks {
            let is_ready = match config.join_mode {
                JoinMode::Any => {
                    // Fire when any accumulator is ready
                    config.triggered_edges.iter().any(|&idx| {
                        if let Some(edge) = self.graph.edges.get(idx) {
                            let acc = edge.accumulator.lock().unwrap();
                            acc.is_ready()
                        } else {
                            false
                        }
                    })
                }
                JoinMode::All => {
                    // Fire when all accumulators are ready
                    !config.triggered_edges.is_empty()
                        && config.triggered_edges.iter().all(|&idx| {
                            if let Some(edge) = self.graph.edges.get(idx) {
                                let acc = edge.accumulator.lock().unwrap();
                                acc.is_ready()
                            } else {
                                false
                            }
                        })
                }
            };

            if is_ready {
                // Drain all ready accumulators
                let mut contexts = Vec::new();
                for &idx in &config.triggered_edges {
                    if let Some(edge) = self.graph.edges.get(idx) {
                        let mut acc = edge.accumulator.lock().unwrap();
                        if acc.is_ready() {
                            contexts.push(acc.drain());
                        }
                    }
                }
                ready.push((task_id.clone(), contexts));
            }
        }

        ready
    }
}

/// A task that was fired by the scheduler.
#[derive(Debug)]
pub struct FiredTask {
    /// The task ID that was fired.
    pub task_id: String,
    /// When the task was fired.
    pub fired_at: chrono::DateTime<Utc>,
    /// The drained boundary contexts.
    pub boundary_context: Vec<cloacina_workflow::Context<serde_json::Value>>,
}

impl std::fmt::Debug for ContinuousScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContinuousScheduler")
            .field("graph", &self.graph)
            .field("detector_to_source", &self.detector_to_source)
            .field("exit_edges", &self.exit_edges)
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuous::boundary::{BoundaryKind, ComputationBoundary};
    use crate::continuous::datasource::{
        ConnectionDescriptor, DataConnection, DataConnectionError, DataSource, DataSourceMetadata,
    };
    use crate::continuous::graph::{assemble_graph, ContinuousTaskRegistration};
    use std::any::Any;

    struct MockConn;
    impl DataConnection for MockConn {
        fn connect(&self) -> Result<Box<dyn Any>, DataConnectionError> {
            Ok(Box::new("mock"))
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
            metadata: None,
            emitted_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_scheduler_processes_detector_output() {
        let graph = assemble_graph(
            vec![make_source("events")],
            vec![ContinuousTaskRegistration {
                id: "agg".into(),
                sources: vec!["events".into()],
                referenced: vec![],
            }],
        )
        .unwrap();

        let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
        let scheduler = ContinuousScheduler::new(
            graph,
            ledger.clone(),
            ContinuousSchedulerConfig {
                poll_interval: Duration::from_millis(10),
            },
        );

        // Simulate: detector emits boundaries directly to accumulators
        let output = DetectorOutput::Change {
            boundaries: vec![make_boundary(0, 100)],
        };
        scheduler.process_detector_output("detect_events", &output);

        // Check readiness
        let ready = scheduler.check_readiness();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].0, "agg");
    }

    #[tokio::test]
    async fn test_scheduler_run_loop_with_shutdown() {
        let graph = assemble_graph(
            vec![make_source("events")],
            vec![ContinuousTaskRegistration {
                id: "agg".into(),
                sources: vec!["events".into()],
                referenced: vec![],
            }],
        )
        .unwrap();

        let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));

        // Pre-populate ledger with a detector completion containing DetectorOutput
        {
            let mut ctx = cloacina_workflow::Context::new();
            let output = DetectorOutput::Change {
                boundaries: vec![make_boundary(0, 100)],
            };
            ctx.insert(
                crate::continuous::detector::DETECTOR_OUTPUT_KEY,
                serde_json::to_value(&output).unwrap(),
            )
            .unwrap();

            let mut l = ledger.write().unwrap();
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
            },
        );

        let (tx, rx) = watch::channel(false);

        // Run scheduler for a short time then shut down
        let handle = tokio::spawn(async move { scheduler.run(rx).await });

        tokio::time::sleep(Duration::from_millis(100)).await;
        tx.send(true).unwrap();

        let fired = handle.await.unwrap();
        assert!(!fired.is_empty(), "expected at least one task to fire");
        assert_eq!(fired[0].task_id, "agg");
    }

    #[tokio::test]
    async fn test_scheduler_empty_graph_runs_cleanly() {
        let graph = assemble_graph(vec![], vec![]).unwrap();
        let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
        let scheduler = ContinuousScheduler::new(
            graph,
            ledger,
            ContinuousSchedulerConfig {
                poll_interval: Duration::from_millis(10),
            },
        );

        let (tx, rx) = watch::channel(false);
        let handle = tokio::spawn(async move { scheduler.run(rx).await });

        tokio::time::sleep(Duration::from_millis(50)).await;
        tx.send(true).unwrap();

        let fired = handle.await.unwrap();
        assert!(fired.is_empty());
    }

    #[tokio::test]
    async fn test_watermark_advance_updates_boundary_ledger() {
        let graph = assemble_graph(
            vec![make_source("events")],
            vec![ContinuousTaskRegistration {
                id: "agg".into(),
                sources: vec!["events".into()],
                referenced: vec![],
            }],
        )
        .unwrap();

        let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
        let scheduler = ContinuousScheduler::new(
            graph,
            ledger.clone(),
            ContinuousSchedulerConfig {
                poll_interval: Duration::from_millis(10),
            },
        );

        // Process a WatermarkAdvance
        let output = DetectorOutput::WatermarkAdvance {
            boundary: make_boundary(0, 500),
        };
        scheduler.process_detector_output("detect_events", &output);

        // Check boundary ledger was updated
        let bl = scheduler.boundary_ledger().read().unwrap();
        assert!(bl.watermark("events").is_some());
    }

    #[tokio::test]
    async fn test_both_output_routes_watermark_and_boundaries() {
        let graph = assemble_graph(
            vec![make_source("events")],
            vec![ContinuousTaskRegistration {
                id: "agg".into(),
                sources: vec!["events".into()],
                referenced: vec![],
            }],
        )
        .unwrap();

        let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
        let scheduler = ContinuousScheduler::new(
            graph,
            ledger.clone(),
            ContinuousSchedulerConfig {
                poll_interval: Duration::from_millis(10),
            },
        );

        // Process a Both output
        let output = DetectorOutput::Both {
            boundaries: vec![make_boundary(0, 100)],
            watermark: make_boundary(0, 500),
        };
        scheduler.process_detector_output("detect_events", &output);

        // Watermark should be updated
        let bl = scheduler.boundary_ledger().read().unwrap();
        assert!(bl.watermark("events").is_some());

        // Accumulator should have received boundaries — check readiness
        let ready = scheduler.check_readiness();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].0, "agg");
    }
}
