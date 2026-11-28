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

//! Scheduler loop and workflow execution processing.
//!
//! This module contains the main scheduling loop that continuously processes
//! active workflow executions and manages task readiness.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::time;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::Runtime;
use crate::dal::DAL;
use crate::database::BackendType;
use crate::database::universal_types::UniversalUuid;
use crate::dispatcher::{Admission, DispatchError, Dispatcher, TaskReadyEvent};
use crate::error::ValidationError;
use crate::models::task_execution::TaskExecution;
use crate::models::workflow_execution::WorkflowExecutionRecord;

use super::state_manager::StateManager;

/// Maximum backoff interval during sustained errors (30 seconds).
const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// Max ready tasks dispatched concurrently per tick (CLOACI-T-0745).
/// `dispatcher.dispatch()` blocks until the task COMPLETES (the fleet executor
/// waits on the result channel), so dispatching serially ran tasks one-at-a-time
/// and the executor's slots were never used concurrently. We dispatch up to this
/// many at once so the fleet actually saturates; tasks beyond the executor's
/// capacity get `NoCapacity` (expected backpressure) and stay Ready for a later
/// tick. Set comfortably above typical fleet aggregate capacity.
const MAX_CONCURRENT_DISPATCH: usize = 64;

/// Dispatch a single Ready task and log the outcome (CLOACI-T-0745). Shared by
/// the postgres (spawned, concurrent) and sqlite (serial) dispatch paths.
/// NoCapacity is expected backpressure (the task stays Ready, retried later);
/// other errors are surfaced as warnings.
async fn dispatch_one(dispatcher: &Arc<dyn Dispatcher>, task: TaskExecution, turn: Admission) {
    let event = TaskReadyEvent::new(
        task.id,
        task.workflow_execution_id,
        task.task_name.clone(),
        task.attempt,
    )
    .with_admission(turn.clone());
    let outcome = dispatcher.dispatch(event).await;
    // The dispatch is over: whatever the executor did, the next one may go.
    turn.release();
    match outcome {
        Ok(()) => {}
        Err(DispatchError::NoCapacity(_)) => {
            debug!(
                task_id = %task.id,
                task_name = %task.task_name,
                "Dispatch deferred: executor at capacity"
            );
        }
        Err(e) => {
            warn!(
                task_id = %task.id,
                task_name = %task.task_name,
                error = %e,
                "Failed to dispatch ready task"
            );
        }
    }
}

/// The longest a dispatch waits for the one before it in the pass to decide
/// about its slot (CLOACINA-T-1045). After it the dispatch goes on: one slow
/// dispatch does not hold the rest of the pass.
const ADMISSION_WAIT: Duration = Duration::from_secs(5);

/// How often the waiting-task gauges are read from the database
/// (CLOACINA-T-1024). The loop ticks every `poll_interval` (100 ms by
/// default); a count on each tick would be ten extra queries a second.
const WAITING_GAUGE_INTERVAL: Duration = Duration::from_secs(5);

/// Number of consecutive errors before logging a circuit-open warning.
const CIRCUIT_OPEN_THRESHOLD: u32 = 5;

/// Scheduler loop operations.
pub struct SchedulerLoop<'a> {
    dal: &'a DAL,
    runtime: Arc<Runtime>,
    instance_id: Uuid,
    poll_interval: Duration,
    /// Optional dispatcher for push-based task execution
    dispatcher: Option<Arc<dyn Dispatcher>>,
    /// Shutdown signal — when the sender drops or sends, the loop exits cleanly.
    shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
    /// Consecutive error count for circuit breaker / backoff.
    consecutive_errors: u32,
    /// When the waiting-task gauges were last read. `None` before the first
    /// read. A mutex because `process_active_executions` takes `&self`.
    waiting_gauge_read_at: std::sync::Mutex<Option<std::time::Instant>>,
    /// The pools the waiting-task gauges were set for (CLOACINA-T-1027). A
    /// pool whose last waiting task left gets a zero once more, or its series
    /// would keep the last value.
    waiting_gauge_pools: std::sync::Mutex<std::collections::BTreeSet<String>>,
    /// The ageing interval of waiting work (CLOACINA-T-1045).
    priority_aging_interval: Duration,
    /// The last turn of the ordered pass before this one, and when that pass
    /// began (CLOACINA-T-1045). While its dispatches still take their turns,
    /// a new pass dispatches nothing: the tasks it would offer are the ones
    /// that already wait for their turn.
    ordered_pass:
        std::sync::Mutex<Option<(tokio::sync::oneshot::Receiver<()>, std::time::Instant)>>,
}

impl<'a> SchedulerLoop<'a> {
    /// Creates a new SchedulerLoop with an optional dispatcher.
    pub fn with_dispatcher(
        dal: &'a DAL,
        runtime: Arc<Runtime>,
        instance_id: Uuid,
        poll_interval: Duration,
        dispatcher: Option<Arc<dyn Dispatcher>>,
    ) -> Self {
        Self {
            dal,
            runtime,
            instance_id,
            poll_interval,
            dispatcher,
            shutdown_rx: None,
            consecutive_errors: 0,
            waiting_gauge_read_at: std::sync::Mutex::new(None),
            waiting_gauge_pools: std::sync::Mutex::new(std::collections::BTreeSet::new()),
            priority_aging_interval: cloacina_api_types::DEFAULT_PRIORITY_AGING_INTERVAL,
            ordered_pass: std::sync::Mutex::new(None),
        }
    }

    /// True while the dispatches of the ordered pass before this one still
    /// take their turns (CLOACINA-T-1045). A pass that began more than twice
    /// [`ADMISSION_WAIT`] ago does not count: a stuck dispatch does not stop
    /// the loop.
    fn ordered_pass_in_flight(&self) -> bool {
        let mut last = self.ordered_pass.lock().unwrap_or_else(|e| e.into_inner());
        let in_flight = match last.as_mut() {
            Some((released, began)) => {
                began.elapsed() < ADMISSION_WAIT * 2
                    && matches!(
                        released.try_recv(),
                        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
                    )
            }
            None => false,
        };
        if !in_flight {
            *last = None;
        }
        in_flight
    }

    /// Set the ageing interval of waiting work (CLOACINA-T-1045). The
    /// default is two minutes. Zero switches ageing off.
    pub fn with_priority_aging_interval(mut self, interval: Duration) -> Self {
        self.priority_aging_interval = interval;
        self
    }

    /// Set the shutdown receiver for graceful termination.
    pub fn with_shutdown(mut self, shutdown_rx: tokio::sync::watch::Receiver<bool>) -> Self {
        self.shutdown_rx = Some(shutdown_rx);
        self
    }

    /// Runs the main scheduling loop that continuously processes active workflow executions.
    ///
    /// This loop:
    /// 1. Checks for active workflow executions
    /// 2. Updates task readiness based on dependencies and trigger rules
    /// 3. Marks completed workflow executions
    /// 4. Repeats at the configured poll interval
    pub async fn run(&mut self) -> Result<(), ValidationError> {
        info!(
            "Starting task scheduler loop (instance: {}, poll_interval: {:?})",
            self.instance_id, self.poll_interval
        );
        let mut interval = time::interval(self.poll_interval);

        loop {
            if let Some(ref mut shutdown_rx) = self.shutdown_rx {
                tokio::select! {
                    _ = interval.tick() => {}
                    _ = shutdown_rx.changed() => {
                        info!("SchedulerLoop shutting down");
                        break;
                    }
                }
            } else {
                interval.tick().await;
            }

            match self.process_active_executions().await {
                Ok(_) => {
                    if self.consecutive_errors > 0 {
                        info!(
                            "Scheduler loop recovered after {} consecutive errors",
                            self.consecutive_errors
                        );
                        self.consecutive_errors = 0;
                    }
                    debug!("Scheduling loop completed successfully");
                }
                Err(e) => {
                    self.consecutive_errors += 1;

                    if self.consecutive_errors == CIRCUIT_OPEN_THRESHOLD {
                        warn!(
                            "Scheduler loop circuit open: {} consecutive errors — backing off (latest: {})",
                            self.consecutive_errors, e
                        );
                    } else if self.consecutive_errors.is_multiple_of(10) {
                        // Rate-limited logging: every 10th error after circuit opens
                        warn!(
                            "Scheduler loop still failing: {} consecutive errors (latest: {})",
                            self.consecutive_errors, e
                        );
                    } else if self.consecutive_errors < CIRCUIT_OPEN_THRESHOLD {
                        error!("Scheduling loop error: {}", e);
                    }

                    // Exponential backoff: poll_interval * 2^min(errors, 8) capped at MAX_BACKOFF
                    let backoff_exp = self.consecutive_errors.min(8);
                    let backoff = self
                        .poll_interval
                        .saturating_mul(1u32 << backoff_exp)
                        .min(MAX_BACKOFF);
                    if backoff > self.poll_interval {
                        time::sleep(backoff - self.poll_interval).await;
                    }
                }
            }
        }

        Ok(())
    }

    /// Set `cloacina_waiting_tasks` and
    /// `cloacina_oldest_waiting_task_age_seconds` from the database, at most
    /// once per [`WAITING_GAUGE_INTERVAL`] (CLOACINA-T-1024). Best-effort: on
    /// a query error the gauges keep their last value.
    async fn refresh_waiting_task_gauges(&self) {
        {
            let mut read_at = self
                .waiting_gauge_read_at
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if read_at.is_some_and(|at| at.elapsed() < WAITING_GAUGE_INTERVAL) {
                return;
            }
            *read_at = Some(std::time::Instant::now());
        }
        // CLOACINA-T-1027: one series per pool. The pool `default` always has
        // one, so a tenant with no pools reads as before with `pool="default"`.
        match self.dal.task_execution().waiting_task_stats_by_pool().await {
            Ok(stats) => {
                let tenant = self.dal.tenant_label().to_string();
                let now = chrono::Utc::now();
                let mut pools = self
                    .waiting_gauge_pools
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                pools.insert(cloacina_api_types::DEFAULT_AGENT_POOL.to_string());
                pools.extend(stats.iter().map(|p| p.pool.clone()));
                for pool in pools.iter() {
                    let (count, age) = stats
                        .iter()
                        .find(|p| &p.pool == pool)
                        .map(|p| (p.waiting.count as f64, p.waiting.oldest_age_seconds(now)))
                        .unwrap_or((0.0, 0.0));
                    metrics::gauge!(
                        "cloacina_waiting_tasks",
                        "tenant" => tenant.clone(),
                        "pool" => pool.clone()
                    )
                    .set(count);
                    metrics::gauge!(
                        "cloacina_oldest_waiting_task_age_seconds",
                        "tenant" => tenant.clone(),
                        "pool" => pool.clone()
                    )
                    .set(age);
                }
            }
            Err(e) => {
                warn!("failed to read waiting tasks for gauges: {}", e);
            }
        }
    }

    /// Processes all active workflow executions to update task readiness.
    pub async fn process_active_executions(&self) -> Result<(), ValidationError> {
        let active_executions = self
            .dal
            .workflow_execution()
            .get_active_executions()
            .await?;

        // SQL-derived gauges — re-seeded every tick so they cannot drift on
        // crash, claim loss, or any path that skips
        // finalize_workflow_execution (workflows) /
        // `ThreadTaskExecutor::execute_task` (tasks). See T-0534 + T-0589.
        // The `tenant` label keeps the runners of one process apart: a server
        // runs one per tenant schema (CLOACINA-T-1036).
        let tenant = self.dal.tenant_label().to_string();
        metrics::gauge!("cloacina_active_workflows", "tenant" => tenant.clone())
            .set(active_executions.len() as f64);
        match self.dal.task_execution().count_running_tasks().await {
            Ok(n) => {
                metrics::gauge!("cloacina_active_tasks", "tenant" => tenant).set(n as f64);
            }
            Err(e) => {
                // Best-effort — leave the previous tick's value rather than
                // zeroing the gauge on a transient DB hiccup.
                warn!("failed to count running tasks for gauge re-seed: {}", e);
            }
        }
        self.refresh_waiting_task_gauges().await;

        if active_executions.is_empty() {
            // Even with no active workflow executions, dispatch any Ready tasks (e.g., retries)
            if self.dispatcher.is_some() {
                self.dispatch_ready_tasks().await?;
            }
            return Ok(());
        }

        // Batch process all active workflow executions (evaluates pending tasks, marks them Ready)
        self.process_executions_batch(active_executions).await?;

        // Dispatch all Ready tasks (including newly marked and retry tasks)
        if self.dispatcher.is_some() {
            self.dispatch_ready_tasks().await?;
        }

        Ok(())
    }

    /// Processes multiple workflow executions in batch for better performance.
    ///
    /// This method optimizes workflow execution processing by:
    /// 1. Loading all pending tasks across all workflow executions in one query
    /// 2. Grouping tasks by workflow execution for processing
    /// 3. Batch checking workflow execution completion
    async fn process_executions_batch(
        &self,
        active_executions: Vec<WorkflowExecutionRecord>,
    ) -> Result<(), ValidationError> {
        let execution_ids: Vec<UniversalUuid> = active_executions.iter().map(|e| e.id).collect();

        // Batch load all pending tasks across all active workflow executions
        let all_pending_tasks = self
            .dal
            .task_execution()
            .get_pending_tasks_batch(execution_ids.clone())
            .await?;

        // Group tasks by workflow execution ID for processing
        let mut tasks_by_execution: HashMap<UniversalUuid, Vec<TaskExecution>> = HashMap::new();
        for task in all_pending_tasks {
            tasks_by_execution
                .entry(task.workflow_execution_id)
                .or_default()
                .push(task);
        }

        // CLOACI-T-0745: one query for the full task-status map per execution.
        // Dependency gating + status-based trigger conditions resolve from this
        // in memory, replacing the previous per-task `get_by_id` + per-task
        // status queries that made each tick O(executions × pending_tasks).
        let status_by_execution = self
            .dal
            .task_execution()
            .get_all_task_statuses_for_executions(execution_ids)
            .await?;
        let empty_statuses: HashMap<String, String> = HashMap::new();

        let state_manager = StateManager::new(self.dal, self.runtime.clone());

        // Process each workflow execution's tasks
        for execution in &active_executions {
            let statuses = status_by_execution
                .get(&execution.id)
                .unwrap_or(&empty_statuses);

            match tasks_by_execution.get(&execution.id) {
                // Has pending tasks → by definition NOT complete: update task
                // readiness only, and skip the completion query entirely
                // (CLOACI-T-0745 — this was an unconditional per-execution query).
                Some(execution_tasks) => {
                    if let Err(e) = state_manager
                        .update_workflow_task_readiness(execution, execution_tasks, statuses)
                        .await
                    {
                        error!(
                            "Failed to update task readiness for workflow execution {}: {}",
                            execution.id, e
                        );
                        continue;
                    }
                }
                // No pending tasks → candidate for completion. Only here do we
                // run check_workflow_completion (its semantics are unchanged), so
                // completion queries are bounded by idle executions, not total.
                None => {
                    if self
                        .dal
                        .task_execution()
                        .check_workflow_completion(execution.id)
                        .await?
                    {
                        self.complete_execution(execution).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Dispatches all Ready tasks to the executor.
    ///
    /// This method finds tasks that are Ready (either newly marked or from retries)
    /// and dispatches them via the configured dispatcher. Tasks are only dispatched
    /// if their retry_at time has passed (or is null).
    async fn dispatch_ready_tasks(&self) -> Result<(), ValidationError> {
        let dispatcher = match &self.dispatcher {
            Some(d) => d.clone(),
            None => return Ok(()),
        };

        // CLOACINA-T-1045: the waiting set, then the tasks to dispatch in
        // this pass in their order: workflow priority, task priority, time
        // waited, with ageing. A task put off for capacity keeps its place
        // and holds back worse tasks of its pool.
        let waiting = self.dal.task_execution().get_waiting_for_dispatch().await?;
        let ready_tasks = super::priority::dispatch_order(
            waiting,
            chrono::Utc::now(),
            self.priority_aging_interval,
        );

        if ready_tasks.is_empty() {
            metrics::counter!(
                "cloacina_scheduler_claim_attempts_total",
                "outcome" => "empty",
            )
            .increment(1);
            return Ok(());
        }

        // Backend-aware dispatch (CLOACI-T-0745).
        //
        // `dispatcher.dispatch()` blocks until the task COMPLETES (the fleet
        // executor waits on the result channel up to RESULT_WAIT_TIMEOUT).
        //
        // - **Postgres** tolerates many concurrent writers, so dispatch
        //   fire-and-forget: spawn each so the scheduler loop returns immediately
        //   and keeps marking tasks Ready / finalizing completions at full cadence
        //   regardless of how long tasks run — this is what lets the fleet
        //   saturate under load.
        // - **SQLite** is single-writer: concurrent dispatch writes (claim /
        //   mark-running / result) contend with the loop's readiness writes and
        //   surface as "database is locked". So dispatch SERIALLY (one writer at a
        //   time) — the embedded-safe original behavior. Embedded runs are
        //   single-process/low-concurrency, so the serial cost is a non-issue.
        //
        // Exactly-once holds for both: `claim_for_runner`'s atomic CAS
        // (`WHERE claimed_by IS NULL`) is the guard, and `get_ready_for_retry` now
        // excludes claimed (in-flight) tasks. In the small window before a spawned
        // dispatch claims, a re-select could re-spawn it; the CAS lets one claim
        // win, the other no-ops. Spawns/serial dispatch are bounded per tick
        // (`take`); the remainder is picked up on later ticks (backpressure).
        let concurrent = match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => true,
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => false,
        };

        // CLOACINA-T-1045: the spawned dispatches run at the same time, but
        // each waits for the one before it to decide about its slot. So the
        // free slots go to the tasks in the order of the pass.
        if concurrent && self.ordered_pass_in_flight() {
            return Ok(());
        }
        let mut before: Option<tokio::sync::oneshot::Receiver<()>> = None;
        for task in ready_tasks.into_iter().take(MAX_CONCURRENT_DISPATCH) {
            let dispatcher = dispatcher.clone();
            if concurrent {
                let (turn, released) = Admission::new();
                let wait_for = before.replace(released);
                tokio::spawn(async move {
                    if let Some(wait_for) = wait_for {
                        let _ = tokio::time::timeout(ADMISSION_WAIT, wait_for).await;
                    }
                    dispatch_one(&dispatcher, task, turn).await
                });
            } else {
                dispatch_one(&dispatcher, task, Admission::none()).await;
            }
        }
        // The turn of the last dispatch of this pass: the next pass waits
        // for it.
        if let Some(last_turn) = before {
            *self.ordered_pass.lock().unwrap_or_else(|e| e.into_inner()) =
                Some((last_turn, std::time::Instant::now()));
        }

        Ok(())
    }

    /// Completes a workflow execution by updating its final context and marking it as completed.
    ///
    /// Guards against the race where two scheduler ticks both see the workflow execution
    /// as complete and both try to finalise it. Only the first caller (whose
    /// workflow execution is still "Running") will proceed; subsequent calls return early.
    async fn complete_execution(
        &self,
        execution: &WorkflowExecutionRecord,
    ) -> Result<(), ValidationError> {
        // Guard: only proceed if the workflow execution is still running.
        // This prevents duplicate WorkflowCompleted events when two scheduler
        // ticks race through check_workflow_completion concurrently.
        let current = self
            .dal
            .workflow_execution()
            .get_by_id(execution.id)
            .await?;
        if current.status == "Completed" || current.status == "Failed" {
            debug!(
                "Workflow execution {} already in status '{}', skipping completion",
                execution.id, current.status
            );
            return Ok(());
        }

        // Get task summary for logging
        let all_tasks = self
            .dal
            .task_execution()
            .get_all_tasks_for_workflow(execution.id)
            .await?;
        let completed_count = all_tasks.iter().filter(|t| t.status == "Completed").count();
        let failed_count = all_tasks.iter().filter(|t| t.status == "Failed").count();
        let skipped_count = all_tasks.iter().filter(|t| t.status == "Skipped").count();

        // Update the workflow execution's final context before marking complete
        if let Err(e) = self
            .update_execution_final_context(execution.id, &all_tasks)
            .await
        {
            warn!(
                "Failed to update final context for workflow execution {}: {}",
                execution.id, e
            );
        }

        // Determine final workflow execution status based on task outcomes
        if failed_count > 0 {
            let reason = format!(
                "{} task(s) failed, {} completed, {} skipped",
                failed_count, completed_count, skipped_count
            );
            self.dal
                .workflow_execution()
                .mark_failed(execution.id, &reason)
                .await?;
            // Workflow-level failures are always downstream of task failures.
            metrics::counter!(
                "cloacina_workflows_total",
                "status" => "failed",
                "reason" => "dependency_failed",
                "workflow" => crate::metric_labels::workflow_label(
                    self.dal.tenant_label(),
                    &execution.workflow_name,
                ),
                "tenant" => self.dal.tenant_label().to_string(),
            )
            .increment(1);
            info!(
                "Workflow execution failed: {} (name: {}, {})",
                execution.id, execution.workflow_name, reason
            );
        } else {
            self.dal
                .workflow_execution()
                .mark_completed(execution.id)
                .await?;
            metrics::counter!(
                "cloacina_workflows_total",
                "status" => "completed",
                "reason" => "ok",
                "workflow" => crate::metric_labels::workflow_label(
                    self.dal.tenant_label(),
                    &execution.workflow_name,
                ),
                "tenant" => self.dal.tenant_label().to_string(),
            )
            .increment(1);
            info!(
                "Workflow execution completed: {} (name: {}, {} completed, {} skipped)",
                execution.id, execution.workflow_name, completed_count, skipped_count
            );
        }

        // Record workflow execution duration. The active-workflows gauge is
        // SQL-derived in process_active_executions and does NOT need an
        // explicit decrement here — the next tick will re-seed it from the
        // authoritative row count.
        let duration = chrono::Utc::now() - execution.started_at.0;
        if let Ok(secs) = duration.to_std() {
            metrics::histogram!("cloacina_workflow_duration_seconds").record(secs.as_secs_f64());
        }

        Ok(())
    }

    /// Updates the workflow execution's final context when it completes.
    ///
    /// This method finds the context from the final task(s) that produced output
    /// and updates the workflow execution's context_id to point to that final context,
    /// ensuring that WorkflowExecutionResult.final_context returns the correct data.
    ///
    /// **Selection rule (COR-14):** primary sort by `completed_at` (latest
    /// wins), secondary sort by `task_name` (lexicographically largest
    /// wins) so two tasks finishing at the same millisecond don't
    /// produce nondeterministic final contexts across replays.
    async fn update_execution_final_context(
        &self,
        workflow_execution_id: UniversalUuid,
        all_tasks: &[TaskExecution],
    ) -> Result<(), ValidationError> {
        let mut final_context_id: Option<UniversalUuid> = None;
        let mut latest_key: Option<(chrono::DateTime<chrono::Utc>, String)> = None;

        for task in all_tasks {
            // Only consider tasks that have finished execution and might have output context
            if task.status == "Completed" || task.status == "Skipped" {
                if let Some(completed_at) = task.completed_at {
                    // Check if this task has a context stored
                    let task_namespace = crate::task::TaskNamespace::from_string(&task.task_name)
                        .map_err(|_| {
                        crate::error::ValidationError::InvalidTaskName(task.task_name.clone())
                    })?;
                    if let Ok(task_metadata) = self
                        .dal
                        .task_execution_metadata()
                        .get_by_workflow_and_task(workflow_execution_id, &task_namespace)
                        .await
                    {
                        if let Some(context_id) = task_metadata.context_id {
                            let candidate = (completed_at.0, task.task_name.clone());
                            // Lexicographic compare on (completed_at, task_name) —
                            // deterministic tiebreaker per COR-14.
                            let should_update = match latest_key.as_ref() {
                                None => true,
                                Some(cur) => candidate > *cur,
                            };
                            if should_update {
                                final_context_id = Some(context_id);
                                latest_key = Some(candidate);
                            }
                        }
                    }
                }
            }
        }

        // Update the workflow execution's context_id if we found a final context
        if let Some(context_id) = final_context_id {
            debug!(
                "Updating workflow execution {} final context to context_id: {}",
                workflow_execution_id, context_id
            );
            self.dal
                .workflow_execution()
                .update_final_context(workflow_execution_id, context_id)
                .await?;
        } else {
            debug!(
                "No final context found for workflow execution {} - keeping initial context",
                workflow_execution_id
            );
        }

        Ok(())
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod gauge_label_tests {
    //! The gauges that the loop sets from the database carry the `tenant`
    //! label (CLOACINA-T-1024, CLOACINA-T-1036). These tests run one pass of
    //! the loop on SQLite in memory under a thread-local metrics recorder.

    use super::*;
    use crate::database::Database;
    use crate::models::task_execution::NewTaskExecution;
    use crate::models::workflow_execution::NewWorkflowExecution;
    use metrics_util::MetricKind;
    use metrics_util::debugging::{DebugValue, DebuggingRecorder};

    /// Each gauge series of one pass as (name, sorted labels, value).
    type Gauges = Vec<(String, Vec<(String, String)>, f64)>;

    /// Run one pass of the loop over a database with one running workflow
    /// that has one `Running` task and `ready_tasks` `Ready` tasks.
    fn gauges_after_one_pass(ready_tasks: usize) -> Gauges {
        let recorder = DebuggingRecorder::new();
        let snapshotter = recorder.snapshotter();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        metrics::with_local_recorder(&recorder, || {
            rt.block_on(async move {
                let url = format!(
                    "file:gauge_label_test_{}?mode=memory&cache=shared",
                    uuid::Uuid::new_v4()
                );
                let db = Database::new(&url, "", 5);
                db.run_migrations().await.expect("migrations");
                let dal = DAL::new(db);
                let workflow_id = dal
                    .workflow_execution()
                    .create(NewWorkflowExecution {
                        workflow_name: "orders".into(),
                        workflow_version: "1.0".into(),
                        status: "Running".into(),
                        context_id: None,
                    })
                    .await
                    .unwrap()
                    .id;
                let mut tasks = vec![("load".to_string(), "Running")];
                tasks.extend((0..ready_tasks).map(|n| (format!("ready_{n}"), "Ready")));
                for (task_name, status) in tasks {
                    dal.task_execution()
                        .create(NewTaskExecution {
                            workflow_execution_id: workflow_id,
                            task_name,
                            status: status.into(),
                            attempt: 1,
                            max_attempts: 3,
                            trigger_rules: r#"{"type":"Always"}"#.into(),
                            task_configuration: "{}".into(),
                        })
                        .await
                        .unwrap();
                }
                let scheduler = SchedulerLoop::with_dispatcher(
                    &dal,
                    Arc::new(Runtime::new()),
                    Uuid::new_v4(),
                    Duration::from_millis(100),
                    None,
                );
                // The gauges are set at the top of the pass. What the pass
                // does with the workflow after that is not under test.
                let _ = scheduler.process_active_executions().await;
            })
        });
        snapshotter
            .snapshot()
            .into_vec()
            .into_iter()
            .filter(|(key, _, _, _)| key.kind() == MetricKind::Gauge)
            .map(|(key, _, _, value)| {
                let mut labels: Vec<(String, String)> = key
                    .key()
                    .labels()
                    .map(|l| (l.key().to_string(), l.value().to_string()))
                    .collect();
                labels.sort();
                let value = match value {
                    DebugValue::Gauge(v) => v.into_inner(),
                    other => panic!("not a gauge: {other:?}"),
                };
                (key.key().name().to_string(), labels, value)
            })
            .collect()
    }

    /// The one series of `name`, with its labels and value.
    fn series(gauges: &Gauges, name: &str) -> (Vec<(String, String)>, f64) {
        let found: Vec<_> = gauges.iter().filter(|(n, _, _)| n == name).collect();
        assert_eq!(found.len(), 1, "series of {name}: {found:?}");
        (found[0].1.clone(), found[0].2)
    }

    fn tenant_public() -> Vec<(String, String)> {
        vec![("tenant".to_string(), "public".to_string())]
    }

    #[test]
    fn active_gauges_carry_the_tenant_label() {
        let gauges = gauges_after_one_pass(0);
        assert_eq!(
            series(&gauges, "cloacina_active_workflows"),
            (tenant_public(), 1.0)
        );
        assert_eq!(
            series(&gauges, "cloacina_active_tasks"),
            (tenant_public(), 1.0)
        );
    }

    #[test]
    fn waiting_gauges_carry_the_tenant_label_and_count_ready_tasks() {
        let gauges = gauges_after_one_pass(2);
        // CLOACINA-T-1027: the waiting gauges also carry the `pool` label. A
        // task with no pool is in `default`.
        let default_pool = vec![
            ("pool".to_string(), "default".to_string()),
            ("tenant".to_string(), "public".to_string()),
        ];
        assert_eq!(
            series(&gauges, "cloacina_waiting_tasks"),
            (default_pool.clone(), 2.0)
        );
        let (labels, age) = series(&gauges, "cloacina_oldest_waiting_task_age_seconds");
        assert_eq!(labels, default_pool);
        assert!((0.0..60.0).contains(&age), "age was {age}");
    }
}
