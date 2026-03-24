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

use super::boundary::validate_boundary;
use super::detector::DetectorOutput;
use super::detector_state_store::DetectorStateStore;
use super::graph::{DataSourceGraph, JoinMode, LateArrivalPolicy};
use super::ledger::{ExecutionLedger, LedgerEvent};
use super::watermark::BoundaryLedger;
use chrono::Utc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

/// Configuration for the continuous scheduler.
#[derive(Debug, Clone)]
pub struct ContinuousSchedulerConfig {
    /// How often to poll the execution ledger for new events.
    pub poll_interval: Duration,
    /// Maximum number of fired task records to retain. Oldest records are
    /// discarded when this limit is reached. Set to 0 for unlimited (not
    /// recommended for long-running schedulers).
    pub max_fired_tasks: usize,
    /// Maximum time a task execution is allowed to run before being cancelled.
    /// `None` means no timeout (not recommended for production).
    pub task_timeout: Option<Duration>,
}

impl Default for ContinuousSchedulerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(100),
            max_fired_tasks: 10_000,
            task_timeout: Some(Duration::from_secs(300)), // 5 minutes
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
    /// Registered continuous task implementations.
    task_registry: HashMap<String, Arc<dyn cloacina_workflow::Task>>,
    /// Exit edges: task name → workflow names to fire on completion.
    exit_edges: HashMap<String, Vec<String>>,
    /// Configuration.
    config: ContinuousSchedulerConfig,
    /// Optional DAL for accumulator state persistence.
    dal: Option<Arc<crate::dal::DAL>>,
    /// In-memory detector state store with committed/latest tracking.
    detector_state_store: DetectorStateStore,
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
            task_registry: HashMap::new(),
            exit_edges: HashMap::new(),
            config,
            dal: None,
            detector_state_store: DetectorStateStore::new(),
        }
    }

    /// Register a continuous task implementation.
    ///
    /// The task ID must match a task in the graph. When the scheduler
    /// determines this task should fire, it calls `task.execute(context)`.
    pub fn register_task(&mut self, task: Arc<dyn cloacina_workflow::Task>) -> &mut Self {
        let id = task.id().to_string();
        self.task_registry.insert(id, task);
        self
    }

    /// Enable accumulator state persistence via DAL.
    ///
    /// When set, consumer watermarks are persisted on drain and
    /// restored on startup via `restore_from_persisted_state()`.
    pub fn with_dal(mut self, dal: Arc<crate::dal::DAL>) -> Self {
        self.dal = Some(dal);
        self
    }

    /// Get a reference to the detector state store.
    pub fn detector_state_store(&self) -> &DetectorStateStore {
        &self.detector_state_store
    }

    /// Restore accumulator consumer watermarks from persisted state.
    ///
    /// Call this after construction and before `run()`. Loads persisted
    /// state from the DB, matches edge IDs to current graph edges, and
    /// logs warnings for orphaned state.
    pub async fn restore_from_persisted_state(&self) {
        let dal = match &self.dal {
            Some(dal) => dal,
            None => return,
        };

        let acc_dal = crate::dal::unified::AccumulatorStateDAL::new(dal);
        match acc_dal.load_all().await {
            Ok(states) => {
                let current_edge_ids: std::collections::HashSet<String> = self
                    .graph
                    .edges
                    .iter()
                    .map(|e| format!("{}:{}", e.source, e.task))
                    .collect();

                for state in &states {
                    if current_edge_ids.contains(&state.edge_id) {
                        // Restore consumer watermark from persisted state
                        if let Some(ref wm_json) = state.consumer_watermark {
                            match serde_json::from_str::<super::boundary::ComputationBoundary>(
                                wm_json,
                            ) {
                                Ok(watermark) => {
                                    // Find the matching edge and set its watermark
                                    for edge in &self.graph.edges {
                                        let edge_id = format!("{}:{}", edge.source, edge.task);
                                        if edge_id == state.edge_id {
                                            let mut acc = edge.accumulator.lock();
                                            acc.set_consumer_watermark(watermark.clone());
                                            info!(
                                                "Restored consumer watermark for edge '{}', last drain: {:?}",
                                                state.edge_id, state.last_drain_at
                                            );
                                            break;
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to deserialize persisted watermark for edge '{}': {}",
                                        state.edge_id, e
                                    );
                                }
                            }
                        } else {
                            info!(
                                "Found persisted state for edge '{}' (no watermark), last drain: {:?}",
                                state.edge_id, state.last_drain_at
                            );
                        }
                    } else {
                        tracing::warn!(
                            "Orphaned accumulator state: edge '{}' not in current graph",
                            state.edge_id
                        );
                    }
                }
                info!(
                    "Loaded {} persisted accumulator states ({} orphaned)",
                    states.len(),
                    states
                        .iter()
                        .filter(|s| !current_edge_ids.contains(&s.edge_id))
                        .count()
                );
            }
            Err(e) => {
                tracing::warn!("Failed to load persisted accumulator state: {}", e);
            }
        }
    }

    /// Get a reference to the boundary ledger (for WindowedAccumulator integration).
    pub fn boundary_ledger(&self) -> &Arc<RwLock<BoundaryLedger>> {
        &self.boundary_ledger
    }

    /// Get per-edge accumulator metrics for observability.
    pub fn graph_metrics(&self) -> Vec<super::accumulator::EdgeMetrics> {
        self.graph
            .edges
            .iter()
            .map(|edge| {
                let acc = edge.accumulator.lock();
                super::accumulator::EdgeMetrics {
                    source: edge.source.clone(),
                    task: edge.task.clone(),
                    accumulator: acc.metrics(),
                }
            })
            .collect()
    }

    /// Register an exit edge: when `task_id` completes, fire `workflow_name`.
    pub fn add_exit_edge(&mut self, task_id: String, workflow_name: String) {
        self.exit_edges
            .entry(task_id)
            .or_default()
            .push(workflow_name);
    }

    /// Restore pending boundaries from WAL into accumulators on startup.
    /// Must be called BEFORE restore_from_persisted_state.
    pub async fn restore_pending_boundaries(&self) {
        let dal = match &self.dal {
            Some(dal) => dal,
            None => return,
        };

        let pb_dal = crate::dal::unified::PendingBoundaryDAL::new(dal);

        for edge in &self.graph.edges {
            let edge_id = format!("{}:{}", edge.source, edge.task);
            let cursor = match pb_dal.load_cursor(edge_id.clone()).await {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to load drain cursor for '{}': {}", edge_id, e);
                    0
                }
            };

            match pb_dal.load_after_cursor(edge.source.clone(), cursor).await {
                Ok(rows) => {
                    if !rows.is_empty() {
                        let mut acc = edge.accumulator.lock();
                        for row in &rows {
                            if let Ok(boundary) = serde_json::from_str::<
                                super::boundary::ComputationBoundary,
                            >(&row.boundary_json)
                            {
                                acc.receive(boundary);
                            } else {
                                warn!(
                                    "Failed to deserialize pending boundary id={} for edge '{}'",
                                    row.id, edge_id
                                );
                            }
                        }
                        info!(
                            "Restored {} pending boundaries for edge '{}' (cursor was {})",
                            rows.len(),
                            edge_id,
                            cursor
                        );
                    }
                }
                Err(e) => {
                    warn!("Failed to load pending boundaries for '{}': {}", edge_id, e);
                }
            }
        }
    }

    /// Restore detector states from DB into the detector state store.
    pub async fn restore_detector_state(&self) {
        let dal = match &self.dal {
            Some(dal) => dal,
            None => return,
        };

        let ds_dal = crate::dal::unified::DetectorStateDAL::new(dal);
        match ds_dal.load_all().await {
            Ok(states) => {
                for state in &states {
                    if let Some(ref committed_json) = state.committed_state {
                        match serde_json::from_str::<serde_json::Value>(committed_json) {
                            Ok(value) => {
                                self.detector_state_store
                                    .load_committed(&state.source_name, value);
                                info!("Restored detector state for source '{}'", state.source_name);
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to deserialize detector state for '{}': {}",
                                    state.source_name, e
                                );
                            }
                        }
                    }
                }
                info!("Loaded {} detector states", states.len());
            }
            Err(e) => {
                warn!("Failed to load detector states: {}", e);
            }
        }
    }

    /// Initialize edge drain cursors for all edges in the graph.
    pub async fn init_drain_cursors(&self) {
        let dal = match &self.dal {
            Some(dal) => dal,
            None => return,
        };

        let pb_dal = crate::dal::unified::PendingBoundaryDAL::new(dal);
        for edge in &self.graph.edges {
            let edge_id = format!("{}:{}", edge.source, edge.task);
            if let Err(e) = pb_dal
                .init_cursor(edge_id.clone(), edge.source.clone())
                .await
            {
                warn!("Failed to init drain cursor for '{}': {}", edge_id, e);
            }
        }
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
        let mut drain_counter: usize = 0;

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
                        let ledger = self.ledger.read();
                        let events = ledger.events_since(cursor);
                        let collected: Vec<_> = events.iter().map(|e| match e {
                            LedgerEvent::TaskCompleted { task, context, .. } => {
                                Some((
                                    task.clone(),
                                    DetectorOutput::from_context(context),
                                    context.get("__last_known_state").cloned(),
                                ))
                            }
                            _ => None,
                        }).collect();
                        cursor = ledger.len();
                        collected
                    };

                    // Step 2: Process detector completions
                    for event in &new_events {
                        if let Some((task_name, detector_output, last_known_state)) = event {
                            // Persist detector state if present
                            if let Some(state) = last_known_state {
                                if let Some(source_name) = self.detector_to_source.get(task_name.as_str()) {
                                    self.detector_state_store.update_latest(source_name, state.clone());
                                }
                            }
                            if let Some(output) = detector_output {
                                self.process_detector_output(task_name, output);
                            } else {
                                warn!(
                                    "Task '{}' completed but detector output could not be deserialized — boundaries lost",
                                    task_name
                                );
                            }
                        }
                    }

                    // Step 2.5: Persist boundaries to WAL
                    if let Some(ref dal) = self.dal {
                        let pb_dal = crate::dal::unified::PendingBoundaryDAL::new(dal);
                        for event in &new_events {
                            if let Some((task_name, detector_output, _)) = event {
                                if let Some(output) = detector_output {
                                    if let Some(source_name) = self.detector_to_source.get(task_name.as_str()) {
                                        for boundary in output.boundaries() {
                                            let boundary_json = serde_json::to_string(boundary).unwrap_or_default();
                                            if let Err(e) = pb_dal.append(source_name.clone(), boundary_json).await {
                                                error!("Failed to persist boundary for source '{}': {}", source_name, e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Step 3: Check task readiness and fire
                    let ready_tasks = self.check_readiness();

                    // Collect persistence work (all sync, scoped locks)
                    let mut persist_batch: Vec<crate::dal::unified::models::NewAccumulatorState> = Vec::new();

                    // Collect tasks to execute (sync — gather context and task refs)
                    struct TaskExecution {
                        task_id: String,
                        task: Arc<dyn cloacina_workflow::Task>,
                        context: cloacina_workflow::Context<serde_json::Value>,
                    }
                    let mut executions: Vec<TaskExecution> = Vec::new();

                    for (task_id, drained_contexts) in &ready_tasks {
                        debug!("Continuous task ready: {}", task_id);

                        // Write AccumulatorDrained to ledger (scoped lock)
                        {
                            let mut ledger = self.ledger.write();
                            ledger.append(LedgerEvent::AccumulatorDrained {
                                task: task_id.clone(),
                                boundary: super::boundary::ComputationBoundary {
                                    kind: super::boundary::BoundaryKind::Cursor {
                                        value: format!("drain_{}", drain_counter + 1),
                                    },
                                    metadata: None,
                                    emitted_at: Utc::now(),
                                },
                            });
                        }

                        // Collect persistence state (scoped locks)
                        if self.dal.is_some() {
                            if let Some(config) = self.graph.tasks.get(task_id) {
                                for &idx in &config.triggered_edges {
                                    if let Some(edge) = self.graph.edges.get(idx) {
                                        let acc = edge.accumulator.lock();
                                        let edge_id = format!("{}:{}", edge.source, edge.task);
                                        let watermark_json = acc
                                            .consumer_watermark()
                                            .and_then(|wm| serde_json::to_string(wm).ok());
                                        persist_batch.push(crate::dal::unified::models::NewAccumulatorState {
                                            edge_id,
                                            consumer_watermark: watermark_json,
                                            drain_metadata: "{}".to_string(),
                                        });
                                    }
                                }
                            }
                        }

                        // Build merged context from drained accumulators
                        let mut merged_context = cloacina_workflow::Context::new();
                        for drain_ctx in drained_contexts {
                            for (key, value) in drain_ctx.data().iter() {
                                let _ = merged_context.insert(key, value.clone());
                            }
                        }

                        // Look up the task implementation
                        if let Some(task) = self.task_registry.get(task_id) {
                            executions.push(TaskExecution {
                                task_id: task_id.clone(),
                                task: task.clone(),
                                context: merged_context,
                            });
                        } else {
                            // No registered task — record as fired but not executed
                            debug!(
                                "No task implementation registered for '{}', recording as fired",
                                task_id
                            );
                            drain_counter += 1;
                            fired_tasks.push(FiredTask {
                                task_id: task_id.clone(),
                                fired_at: Utc::now(),
                                boundary_context: vec![],
                                executed: false,
                                error: Some("no task implementation registered".into()),
                            });
                        }
                    }

                    // Execute tasks (async — no locks held)
                    for exec in executions {
                        info!("Executing continuous task: {}", exec.task_id);

                        // Apply timeout if configured
                        let result = if let Some(timeout_duration) = self.config.task_timeout {
                            match tokio::time::timeout(
                                timeout_duration,
                                exec.task.execute(exec.context.clone_data()),
                            )
                            .await
                            {
                                Ok(task_result) => task_result,
                                Err(_elapsed) => {
                                    tracing::error!(
                                        "Continuous task '{}' timed out after {:?}",
                                        exec.task_id,
                                        timeout_duration
                                    );

                                    // Record timeout in ledger
                                    {
                                        let mut ledger = self.ledger.write();
                                        ledger.append(LedgerEvent::TaskFailed {
                                            task: exec.task_id.clone(),
                                            at: Utc::now(),
                                            error: format!(
                                                "task timed out after {:?}",
                                                timeout_duration
                                            ),
                                        });
                                    }

                                    drain_counter += 1;
                                    fired_tasks.push(FiredTask {
                                        task_id: exec.task_id,
                                        fired_at: Utc::now(),
                                        boundary_context: vec![exec.context],
                                        executed: true,
                                        error: Some(format!(
                                            "task timed out after {:?}",
                                            timeout_duration
                                        )),
                                    });
                                    continue;
                                }
                            }
                        } else {
                            exec.task.execute(exec.context.clone_data()).await
                        };

                        match result {
                            Ok(output_context) => {
                                info!("Continuous task '{}' completed successfully", exec.task_id);

                                // Write TaskCompleted to ledger
                                {
                                    let mut ledger = self.ledger.write();
                                    ledger.append(LedgerEvent::TaskCompleted {
                                        task: exec.task_id.clone(),
                                        at: Utc::now(),
                                        context: output_context,
                                    });
                                }

                                drain_counter += 1;
                                fired_tasks.push(FiredTask {
                                    task_id: exec.task_id,
                                    fired_at: Utc::now(),
                                    boundary_context: vec![exec.context],
                                    executed: true,
                                    error: None,
                                });
                            }
                            Err(e) => {
                                info!("Continuous task '{}' failed: {}", exec.task_id, e);

                                // Write TaskFailed to ledger
                                {
                                    let mut ledger = self.ledger.write();
                                    ledger.append(LedgerEvent::TaskFailed {
                                        task: exec.task_id.clone(),
                                        at: Utc::now(),
                                        error: e.to_string(),
                                    });
                                }

                                drain_counter += 1;
                                fired_tasks.push(FiredTask {
                                    task_id: exec.task_id,
                                    fired_at: Utc::now(),
                                    boundary_context: vec![exec.context],
                                    executed: true,
                                    error: Some(e.to_string()),
                                });
                            }
                        }
                    }

                    // Persist accumulator state batch (async, no locks held)
                    if let Some(ref dal) = self.dal {
                        let acc_dal = crate::dal::unified::AccumulatorStateDAL::new(dal);
                        for state in persist_batch {
                            let eid = state.edge_id.clone();
                            if let Err(e) = acc_dal.save(state).await {
                                error!("Failed to persist accumulator state for '{}': {}", eid, e);
                            }
                        }

                        // Advance drain cursors and check commit gate
                        let pb_dal = crate::dal::unified::PendingBoundaryDAL::new(dal);
                        let ds_dal = crate::dal::unified::DetectorStateDAL::new(dal);

                        for (task_id, _) in &ready_tasks {
                            if let Some(config) = self.graph.tasks.get(task_id) {
                                for &idx in &config.triggered_edges {
                                    if let Some(edge) = self.graph.edges.get(idx) {
                                        let edge_id = format!("{}:{}", edge.source, edge.task);
                                        let source = &edge.source;

                                        // Advance cursor to the latest boundary for this source
                                        if let Ok(Some(max_id)) = pb_dal.max_id_for_source(source.clone()).await {
                                            let _ = pb_dal.advance_cursor(edge_id.clone(), max_id).await;
                                        }

                                        // Record edge drain in detector state store
                                        self.detector_state_store.record_edge_drain(source, &edge_id);

                                        // Check if all edges for this source have drained
                                        if let Ok(min_cursor) = pb_dal.min_cursor_for_source(source.clone()).await {
                                            if let Ok(Some(max_boundary)) = pb_dal.max_id_for_source(source.clone()).await {
                                                if min_cursor >= max_boundary {
                                                    // All consumers caught up — commit detector state
                                                    if let Some(committed) = self.detector_state_store.commit(source) {
                                                        let state_json = serde_json::to_string(&committed).unwrap_or_default();
                                                        let _ = ds_dal.save(crate::dal::unified::models::NewDetectorState {
                                                            source_name: source.clone(),
                                                            committed_state: Some(state_json),
                                                        }).await;
                                                    }
                                                    // Cleanup consumed boundaries
                                                    let _ = pb_dal.cleanup(source.clone(), min_cursor).await;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Trim fired_tasks to prevent unbounded growth
                    if self.config.max_fired_tasks > 0 && fired_tasks.len() > self.config.max_fired_tasks {
                        let excess = fired_tasks.len() - self.config.max_fired_tasks;
                        fired_tasks.drain(..excess);
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
                let mut bl = self.boundary_ledger.write();
                match bl.advance(src, watermark.clone()) {
                    Ok(()) => {
                        debug!("Watermark advanced for source '{}'", src);
                    }
                    Err(e) => {
                        warn!("Watermark advance rejected for '{}': {}", src, e);
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
                let mut acc = edge.accumulator.lock();
                for boundary in boundaries {
                    // Validate Custom boundaries before routing
                    if let Err(e) = validate_boundary(boundary) {
                        warn!(
                            "Rejected invalid Custom boundary for {} -> {}: {}",
                            src, edge.task, e
                        );
                        continue;
                    }

                    // Check consumer watermark for late arrival
                    let is_late = if let Some(_consumer_wm) = acc.consumer_watermark() {
                        let bl = self.boundary_ledger.read();
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
                                let r = acc.receive(boundary.clone());
                                if r == super::accumulator::ReceiveResult::AcceptedWithDrop {
                                    warn!(
                                        "Backpressure on late-arrival forward: {} -> {}",
                                        src, edge.task
                                    );
                                }
                                debug!("Late boundary forwarded: {} -> {}", src, edge.task);
                            }
                            LateArrivalPolicy::Retrigger => {
                                let r = acc.receive(boundary.clone());
                                if r == super::accumulator::ReceiveResult::AcceptedWithDrop {
                                    warn!(
                                        "Backpressure on late-arrival retrigger: {} -> {}",
                                        src, edge.task
                                    );
                                }
                                debug!("Late boundary retriggered: {} -> {}", src, edge.task);
                            }
                        }
                    } else {
                        let result = acc.receive(boundary.clone());
                        if result == super::accumulator::ReceiveResult::AcceptedWithDrop {
                            debug!("Backpressure on edge {} -> {}", src, edge.task);
                        }
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
            match config.join_mode {
                JoinMode::Any => {
                    // Fire when any accumulator is ready — try_drain under single lock
                    let mut contexts = Vec::new();
                    for &idx in &config.triggered_edges {
                        if let Some(edge) = self.graph.edges.get(idx) {
                            let mut acc = edge.accumulator.lock();
                            if let Some(ctx) = acc.try_drain() {
                                contexts.push(ctx);
                            }
                        }
                    }
                    if !contexts.is_empty() {
                        ready.push((task_id.clone(), contexts));
                    }
                }
                JoinMode::All => {
                    // Fire when all accumulators are ready.
                    // First check all are ready (single lock each), then drain all.
                    if config.triggered_edges.is_empty() {
                        continue;
                    }
                    let expected_count = config.triggered_edges.len();
                    let all_ready = config.triggered_edges.iter().all(|&idx| {
                        if let Some(edge) = self.graph.edges.get(idx) {
                            let acc = edge.accumulator.lock();
                            acc.is_ready()
                        } else {
                            false
                        }
                    });
                    if all_ready {
                        let mut contexts = Vec::new();
                        for &idx in &config.triggered_edges {
                            if let Some(edge) = self.graph.edges.get(idx) {
                                let mut acc = edge.accumulator.lock();
                                if let Some(ctx) = acc.try_drain() {
                                    contexts.push(ctx);
                                }
                            }
                        }
                        // Fire if we got any contexts. In the unlikely TOCTOU race
                        // where some edges lost readiness between check and drain,
                        // we still fire with partial data rather than silently dropping
                        // already-drained contexts. Log a warning if partial.
                        if contexts.len() == expected_count {
                            ready.push((task_id.clone(), contexts));
                        } else if !contexts.is_empty() {
                            warn!(
                                "JoinMode::All for '{}': only {}/{} edges drained (readiness race), firing with partial data",
                                task_id, contexts.len(), expected_count
                            );
                            ready.push((task_id.clone(), contexts));
                        }
                    }
                }
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
    /// Whether the task was actually executed (vs just recorded as ready).
    pub executed: bool,
    /// Error message if the task failed or couldn't be executed.
    pub error: Option<String>,
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
                max_fired_tasks: 10_000,
                task_timeout: None,
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
                max_fired_tasks: 10_000,
                task_timeout: None,
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
                max_fired_tasks: 10_000,
                task_timeout: None,
            },
        );

        // Process a WatermarkAdvance
        let output = DetectorOutput::WatermarkAdvance {
            boundary: make_boundary(0, 500),
        };
        scheduler.process_detector_output("detect_events", &output);

        // Check boundary ledger was updated
        let bl = scheduler.boundary_ledger().read();
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
                max_fired_tasks: 10_000,
                task_timeout: None,
            },
        );

        // Process a Both output
        let output = DetectorOutput::Both {
            boundaries: vec![make_boundary(0, 100)],
            watermark: make_boundary(0, 500),
        };
        scheduler.process_detector_output("detect_events", &output);

        // Watermark should be updated
        let bl = scheduler.boundary_ledger().read();
        assert!(bl.watermark("events").is_some());

        // Accumulator should have received boundaries — check readiness
        let ready = scheduler.check_readiness();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].0, "agg");
    }

    // --- Late Arrival Policy Tests ---

    /// Helper: create a scheduler, drain once to set consumer watermark,
    /// then return the scheduler ready for late boundary testing.
    fn setup_scheduler_with_watermark(
        policy: super::super::graph::LateArrivalPolicy,
    ) -> (ContinuousScheduler, Arc<RwLock<ExecutionLedger>>) {
        let mut graph = assemble_graph(
            vec![make_source("events")],
            vec![ContinuousTaskRegistration {
                id: "agg".into(),
                sources: vec!["events".into()],
                referenced: vec![],
            }],
        )
        .unwrap();

        // Set the late arrival policy on the edge
        graph.edges[0].late_arrival_policy = policy;

        // Send a boundary and drain to establish consumer watermark at [0, 100)
        {
            let mut acc = graph.edges[0].accumulator.lock();
            acc.receive(make_boundary(0, 100));
            let _ = acc.drain(); // consumer watermark now at [0, 100)
        }

        // Advance boundary ledger so the scheduler's late arrival check
        // can see that [0, 100) is covered
        let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
        let scheduler = ContinuousScheduler::new(
            graph,
            ledger.clone(),
            ContinuousSchedulerConfig {
                poll_interval: Duration::from_millis(10),
                max_fired_tasks: 10_000,
                task_timeout: None,
            },
        );

        // Advance boundary ledger to cover [0, 200) so late check triggers
        {
            let mut bl = scheduler.boundary_ledger().write();
            bl.advance("events", make_boundary(0, 200)).unwrap();
        }

        (scheduler, ledger)
    }

    #[tokio::test]
    async fn test_late_arrival_discard_drops_boundary() {
        let (scheduler, _) = setup_scheduler_with_watermark(LateArrivalPolicy::Discard);

        // Send a "late" boundary [0, 50) — behind consumer watermark [0, 100)
        let output = DetectorOutput::Change {
            boundaries: vec![make_boundary(0, 50)],
        };
        scheduler.process_detector_output("detect_events", &output);

        // Accumulator should NOT have received the boundary (discarded)
        let ready = scheduler.check_readiness();
        assert!(
            ready.is_empty(),
            "Discard policy: late boundary should be dropped, task should not fire"
        );
    }

    #[tokio::test]
    async fn test_late_arrival_accumulate_forward() {
        let (scheduler, _) = setup_scheduler_with_watermark(LateArrivalPolicy::AccumulateForward);

        // Send a "late" boundary [0, 50)
        let output = DetectorOutput::Change {
            boundaries: vec![make_boundary(0, 50)],
        };
        scheduler.process_detector_output("detect_events", &output);

        // AccumulateForward: boundary IS forwarded to accumulator
        let ready = scheduler.check_readiness();
        assert_eq!(
            ready.len(),
            1,
            "AccumulateForward: late boundary should be forwarded, task should fire"
        );
    }

    #[tokio::test]
    async fn test_late_arrival_retrigger() {
        let (scheduler, _) = setup_scheduler_with_watermark(LateArrivalPolicy::Retrigger);

        // Send a "late" boundary [0, 50)
        let output = DetectorOutput::Change {
            boundaries: vec![make_boundary(0, 50)],
        };
        scheduler.process_detector_output("detect_events", &output);

        // Retrigger: boundary IS forwarded for re-processing
        let ready = scheduler.check_readiness();
        assert_eq!(
            ready.len(),
            1,
            "Retrigger: late boundary should be forwarded for re-execution"
        );
    }

    // Note: test_late_arrival_route_to_side_channel was removed along with
    // the RouteToSideChannel variant — it was a no-op that silently dropped data.

    #[tokio::test]
    async fn test_non_late_boundary_passes_through_regardless_of_policy() {
        let (scheduler, _) = setup_scheduler_with_watermark(LateArrivalPolicy::Discard);

        // Send a boundary AHEAD of watermark: [200, 300)
        // Consumer watermark is at [0, 100), boundary ledger covers [0, 200)
        // [200, 300) is NOT covered by boundary ledger → not late → passes through
        let output = DetectorOutput::Change {
            boundaries: vec![make_boundary(200, 300)],
        };
        scheduler.process_detector_output("detect_events", &output);

        let ready = scheduler.check_readiness();
        assert_eq!(
            ready.len(),
            1,
            "Non-late boundary should pass through even with Discard policy"
        );
    }

    // --- Real Execution Tests ---

    /// A test task that writes to context proving it ran.
    struct RealTask {
        id: String,
    }

    #[async_trait::async_trait]
    impl cloacina_workflow::Task for RealTask {
        async fn execute(
            &self,
            mut context: cloacina_workflow::Context<serde_json::Value>,
        ) -> Result<cloacina_workflow::Context<serde_json::Value>, cloacina_workflow::TaskError>
        {
            let _ = context.insert("task_executed", serde_json::json!(true));
            let _ = context.insert("task_id", serde_json::json!(self.id));
            Ok(context)
        }
        fn id(&self) -> &str {
            &self.id
        }
        fn dependencies(&self) -> &[cloacina_workflow::TaskNamespace] {
            &[]
        }
    }

    #[tokio::test]
    async fn test_scheduler_actually_executes_registered_task() {
        let graph = assemble_graph(
            vec![make_source("events")],
            vec![ContinuousTaskRegistration {
                id: "real_task".into(),
                sources: vec!["events".into()],
                referenced: vec![],
            }],
        )
        .unwrap();

        let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));

        // Pre-load detector completion
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
            let mut l = ledger.write();
            l.append(LedgerEvent::TaskCompleted {
                task: "detect_events".into(),
                at: Utc::now(),
                context: ctx,
            });
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

        // Register the actual task implementation
        scheduler.register_task(Arc::new(RealTask {
            id: "real_task".into(),
        }));

        let (tx, rx) = watch::channel(false);
        let handle = tokio::spawn(async move { scheduler.run(rx).await });

        tokio::time::sleep(Duration::from_millis(100)).await;
        tx.send(true).unwrap();

        let fired = handle.await.unwrap();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].task_id, "real_task");
        assert!(fired[0].executed, "task should have been executed");
        assert!(fired[0].error.is_none(), "task should not have errored");

        // Verify TaskCompleted was written to ledger by the scheduler
        let l = ledger.read();
        let all_events = l.events_since(0);
        let completed: Vec<_> = all_events
            .iter()
            .filter(|e| {
                if let LedgerEvent::TaskCompleted { task, .. } = e {
                    task == "real_task"
                } else {
                    false
                }
            })
            .collect();
        assert!(
            !completed.is_empty(),
            "scheduler should write TaskCompleted to ledger after execution"
        );
    }

    #[tokio::test]
    async fn test_scheduler_handles_task_failure() {
        struct FailingTask;

        #[async_trait::async_trait]
        impl cloacina_workflow::Task for FailingTask {
            async fn execute(
                &self,
                _context: cloacina_workflow::Context<serde_json::Value>,
            ) -> Result<cloacina_workflow::Context<serde_json::Value>, cloacina_workflow::TaskError>
            {
                Err(cloacina_workflow::TaskError::ExecutionFailed {
                    message: "intentional failure".into(),
                    task_id: "failing_task".into(),
                    timestamp: Utc::now(),
                })
            }
            fn id(&self) -> &str {
                "failing_task"
            }
            fn dependencies(&self) -> &[cloacina_workflow::TaskNamespace] {
                &[]
            }
        }

        let graph = assemble_graph(
            vec![make_source("events")],
            vec![ContinuousTaskRegistration {
                id: "failing_task".into(),
                sources: vec!["events".into()],
                referenced: vec![],
            }],
        )
        .unwrap();

        let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
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
            let mut l = ledger.write();
            l.append(LedgerEvent::TaskCompleted {
                task: "detect_events".into(),
                at: Utc::now(),
                context: ctx,
            });
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
        scheduler.register_task(Arc::new(FailingTask));

        let (tx, rx) = watch::channel(false);
        let handle = tokio::spawn(async move { scheduler.run(rx).await });

        tokio::time::sleep(Duration::from_millis(100)).await;
        tx.send(true).unwrap();

        let fired = handle.await.unwrap();
        assert_eq!(fired.len(), 1);
        assert!(fired[0].executed);
        assert!(fired[0].error.is_some());
        assert!(fired[0]
            .error
            .as_ref()
            .unwrap()
            .contains("intentional failure"));

        // Verify TaskFailed was written to ledger
        let l = ledger.read();
        let all_events = l.events_since(0);
        let failed: Vec<_> = all_events
            .iter()
            .filter(|e| matches!(e, LedgerEvent::TaskFailed { .. }))
            .collect();
        assert!(
            !failed.is_empty(),
            "scheduler should write TaskFailed to ledger"
        );
    }

    #[tokio::test]
    async fn test_unregistered_task_records_not_executed() {
        let graph = assemble_graph(
            vec![make_source("events")],
            vec![ContinuousTaskRegistration {
                id: "orphan_task".into(),
                sources: vec!["events".into()],
                referenced: vec![],
            }],
        )
        .unwrap();

        let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
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
            let mut l = ledger.write();
            l.append(LedgerEvent::TaskCompleted {
                task: "detect_events".into(),
                at: Utc::now(),
                context: ctx,
            });
        }

        // Do NOT register any task implementation
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
        assert_eq!(fired.len(), 1);
        assert!(
            !fired[0].executed,
            "unregistered task should not be executed"
        );
        assert!(fired[0].error.is_some());
    }

    /// Test crash recovery: persist boundaries to WAL, "crash" (drop scheduler),
    /// create new scheduler with same DAL, restore, verify boundaries recovered.
    #[tokio::test]
    async fn test_crash_recovery_restores_pending_boundaries() {
        // Create a real in-memory database
        let db = crate::database::Database::try_new_with_schema(
            &format!(
                "file:cr_test_{}?mode=memory&cache=shared",
                uuid::Uuid::new_v4()
            ),
            "",
            1,
            None,
        )
        .expect("db");
        db.run_migrations().await.expect("migrations");
        let dal = Arc::new(crate::dal::DAL::new(db.clone()));

        // Session 1: simulate boundaries persisted to WAL before crash
        {
            let pb_dal = crate::dal::unified::PendingBoundaryDAL::new(&dal);

            // Write boundaries to WAL as if the scheduler persisted them
            let b1 = serde_json::to_string(&make_boundary(0, 100)).unwrap();
            let b2 = serde_json::to_string(&make_boundary(100, 200)).unwrap();
            pb_dal
                .append("events".to_string(), b1)
                .await
                .expect("persist b1");
            pb_dal
                .append("events".to_string(), b2)
                .await
                .expect("persist b2");

            // "Crash" — no graceful shutdown, boundaries stuck in WAL
        }

        // Session 2: new scheduler restores from WAL
        {
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
                    max_fired_tasks: 10_000,
                    task_timeout: None,
                },
            )
            .with_dal(dal.clone());

            // Restore from persistence (same sequence as startup)
            scheduler.init_drain_cursors().await;
            scheduler.restore_pending_boundaries().await;

            // Verify boundaries were restored — task should be ready
            let ready = scheduler.check_readiness();
            assert!(
                !ready.is_empty(),
                "After crash recovery, task should be ready from restored WAL boundaries"
            );
            assert_eq!(ready[0].0, "agg");
        }
    }
}
