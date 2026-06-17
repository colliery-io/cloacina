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

//! Task Executor Module
//!
//! This module provides the core task execution functionality for the Cloacina workflow system.
//! The ThreadTaskExecutor implements the `TaskExecutor` trait for dispatcher-based execution.
//!
//! The executor is responsible for:
//! - Executing tasks with proper timeout handling
//! - Managing task retries and error handling
//! - Maintaining task execution state
//! - Handling task dependencies and context management
//!
//! ## Dispatcher Integration
//!
//! ThreadTaskExecutor implements the `TaskExecutor` trait, allowing it to be registered
//! with a dispatcher to receive task events directly. The dispatcher routes `TaskReadyEvent`s
//! to the executor based on routing rules.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;

use super::slot_token::SlotToken;
use super::task_handle::{with_task_handle, TaskHandle};
use super::types::{ClaimedTask, ExecutorConfig};
use crate::dal::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::dispatcher::{
    DispatchError, ExecutionResult, ExecutorMetrics, TaskExecutor, TaskReadyEvent,
};
use crate::error::ExecutorError;
use crate::Runtime;
use crate::{parse_namespace, Context, Database, Task, TaskRegistry};
use async_trait::async_trait;

/// Bounded reason value for `cloacina_tasks_total{status="failed", reason=...}`.
///
/// Cardinality is closed: the set of returned values is fixed here so label
/// explosion is impossible. Currently used only by the test that pins the
/// label set; production code path that emitted these labels was removed
/// in T-0563. Kept as a behavioral spec test.
#[cfg(test)]
fn failure_reason(err: &ExecutorError) -> &'static str {
    match err {
        ExecutorError::TaskTimeout => "timeout",
        ExecutorError::TaskExecution(_) => "task_error",
        ExecutorError::Validation(_) => "validation_failed",
        ExecutorError::ClaimLost => "claim_lost",
        ExecutorError::Database(_)
        | ExecutorError::ConnectionPool(_)
        | ExecutorError::Context(_) => "infrastructure",
        // COR-11: ContextLoadFailed now reports as its own bounded
        // reason value so operators can distinguish "task failed
        // because we couldn't load its dependency context" from
        // generic infrastructure issues.
        ExecutorError::ContextLoadFailed(_) => "context_load_failed",
        ExecutorError::TaskNotFound(_) | ExecutorError::WorkflowExecutionNotFound(_) => {
            "task_not_found"
        }
        ExecutorError::Serialization(_)
        | ExecutorError::InvalidScope(_)
        | ExecutorError::Semaphore(_) => "unknown",
    }
}

/// ThreadTaskExecutor is a thread-based implementation of task execution.
///
/// This executor runs tasks in the current thread/process and manages:
/// - Task execution with timeout handling
/// - Context management and dependency resolution
/// - Error handling and retry logic
/// - State persistence
///
/// The executor maintains its own instance ID for tracking and logging purposes
/// and uses a task registry to resolve task implementations.
///
/// ## Dispatcher Integration
///
/// ThreadTaskExecutor implements the `TaskExecutor` trait, allowing it to be
/// registered with a dispatcher to receive task events directly via the
/// `execute()` method.
pub struct ThreadTaskExecutor {
    /// Database connection pool for task state persistence
    database: Database,
    /// Data Access Layer for database operations
    dal: DAL,
    /// Registry of available task implementations
    task_registry: Arc<TaskRegistry>,
    /// Scoped runtime for task lookup (used in dispatcher execute path)
    runtime: Arc<Runtime>,
    /// Unique identifier for this executor instance
    instance_id: UniversalUuid,
    /// Configuration parameters for executor behavior
    config: ExecutorConfig,
    /// Semaphore controlling concurrent task execution slots
    semaphore: Arc<Semaphore>,
    /// Metrics: total tasks executed. `Arc` so clones — and the shared
    /// [`crate::executor::TaskResultHandler`] (T-0630) — see the same counter.
    total_executed: Arc<AtomicU64>,
    /// Metrics: total tasks failed.
    total_failed: Arc<AtomicU64>,
    /// Shared post-execution handler (T-0630). Holds the same DAL, counters,
    /// and runner_id as this executor; the upcoming `FleetExecutor` (T-0633)
    /// will construct an analogous handler so thread and fleet paths share
    /// one state-write sequence.
    result_handler: crate::executor::TaskResultHandler,
}

impl ThreadTaskExecutor {
    /// Creates a new ThreadTaskExecutor instance.
    ///
    /// # Arguments
    /// * `database` - Database connection pool for task state persistence
    /// * `task_registry` - Registry containing available task implementations
    /// * `config` - Configuration parameters for executor behavior
    ///
    /// # Returns
    /// A new TaskExecutor instance with a randomly generated instance ID
    pub fn new(
        database: Database,
        task_registry: Arc<TaskRegistry>,
        config: ExecutorConfig,
    ) -> Self {
        Self::with_runtime_and_registry(database, task_registry, Arc::new(Runtime::new()), config)
    }

    /// Creates a new ThreadTaskExecutor with a specific runtime.
    pub fn with_runtime_and_registry(
        database: Database,
        task_registry: Arc<TaskRegistry>,
        runtime: Arc<Runtime>,
        config: ExecutorConfig,
    ) -> Self {
        let dal = DAL::new(database.clone());
        let max_concurrent = config.max_concurrent_tasks;
        let instance_id = UniversalUuid::new_v4();
        let total_executed = Arc::new(AtomicU64::new(0));
        let total_failed = Arc::new(AtomicU64::new(0));
        // `runner_id` for claim-guarded transitions only applies when claiming
        // is enabled; mirror the same logic the inline `claim_runner_id` had.
        let runner_id = if config.enable_claiming {
            Some(instance_id)
        } else {
            None
        };
        let result_handler = crate::executor::TaskResultHandler::new(
            dal.clone(),
            total_executed.clone(),
            total_failed.clone(),
            runner_id,
        );

        Self {
            database,
            dal,
            task_registry,
            runtime,
            instance_id,
            config,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            total_executed,
            total_failed,
            result_handler,
        }
    }

    /// Sets the runtime for this executor, replacing the default.
    pub fn with_runtime(mut self, runtime: Arc<Runtime>) -> Self {
        self.runtime = runtime;
        self
    }

    /// Returns a reference to the concurrency semaphore.
    ///
    /// Used by TaskHandle to release and reclaim concurrency slots
    /// during deferred execution.
    pub fn semaphore(&self) -> &Arc<Semaphore> {
        &self.semaphore
    }

    /// Builds the execution context for a task by loading its dependencies.
    ///
    /// # Arguments
    /// * `claimed_task` - The task to build context for
    /// * `dependencies` - Task dependencies
    ///
    /// # Returns
    /// Result containing the task's execution context
    async fn build_task_context(
        &self,
        claimed_task: &ClaimedTask,
        dependencies: &[crate::task::TaskNamespace],
    ) -> Result<Context<serde_json::Value>, ExecutorError> {
        // CLOACI-T-0633: delegate to the shared TaskContextBuilder so the
        // thread executor and the fleet executor resolve dependency context
        // identically (same drift-elimination pattern as TaskResultHandler).
        crate::executor::TaskContextBuilder::new(self.dal.clone())
            .build(claimed_task, dependencies)
            .await
    }

    /// Merges two context values using smart merging strategy.
    ///
    /// For arrays: concatenates unique values maintaining order
    /// For objects: merges recursively (latest wins for conflicting keys)
    /// For primitives: latest wins
    ///
    /// # Arguments
    /// * `existing` - The existing value in the context
    /// * `new` - The new value from dependency context
    ///
    /// # Returns
    /// The merged value
    /// CLOACI-T-0633: forwards to the shared
    /// [`crate::executor::TaskContextBuilder::merge_context_values`]. Test-only
    /// wrapper so the existing `merge_*` unit tests below keep exercising the
    /// canonical implementation through the thread executor's surface; the
    /// production path now goes through `TaskContextBuilder` directly.
    #[cfg(test)]
    fn merge_context_values(
        existing: &serde_json::Value,
        new: &serde_json::Value,
    ) -> serde_json::Value {
        crate::executor::TaskContextBuilder::merge_context_values(existing, new)
    }

    /// Executes a task with timeout protection.
    ///
    /// # Arguments
    /// * `task` - The task implementation to execute
    /// * `context` - The execution context
    ///
    /// # Returns
    /// Result containing either the updated context or an error
    async fn execute_with_timeout(
        &self,
        task: &dyn Task,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, ExecutorError> {
        match tokio::time::timeout(self.config.task_timeout, task.execute(context)).await {
            Ok(result) => result.map_err(ExecutorError::TaskExecution),
            Err(_) => Err(ExecutorError::TaskTimeout),
        }
    }

    /// Runs [`execute_with_timeout`] racing against a cancellation signal
    /// fed by the heartbeat loop. If the heartbeat detects `ClaimLost`, it
    /// flips the channel to `true`, the task future is dropped, and this
    /// returns [`ExecutorError::ClaimLost`]. This is the "Layer 1"
    /// cancellation of T-0487 — cooperative observation via `TaskHandle` is
    /// layered on top for tasks that need graceful cleanup.
    async fn execute_with_cancellation(
        &self,
        task: &dyn Task,
        context: Context<serde_json::Value>,
        mut cancel_rx: tokio::sync::watch::Receiver<bool>,
    ) -> Result<Context<serde_json::Value>, ExecutorError> {
        // Convert the watch signal into a bool *before* entering the select!
        // arm body so we don't hold a `watch::Ref` (which is !Send) across
        // the subsequent await.
        let wait_cancelled = async { cancel_rx.wait_for(|&v| v).await.is_ok() };
        // `biased;` gives the task arm priority. When the watch fires, both
        // arms can become ready on the same poll (the task's own
        // `TaskHandle::cancelled()` observes the same signal). Without
        // `biased`, `select!` picks randomly — which races Layer 2's
        // cooperative cleanup against Layer 1's drop. With `biased`, a task
        // that cooperatively handles cancellation runs to completion; a
        // task that ignores the signal still falls through to Layer 1
        // because its arm stays `Pending` while the cancel arm is ready.
        tokio::select! {
            biased;
            r = self.execute_with_timeout(task, context) => r,
            fired = wait_cancelled => {
                if fired {
                    Err(ExecutorError::ClaimLost)
                } else {
                    // Sender dropped without firing — the heartbeat was
                    // aborted via the success/failure path. Never resolve on
                    // this arm so the task future can complete normally.
                    std::future::pending().await
                }
            }
        }
    }

    // CLOACI-T-0630: the post-execution helpers that used to live here —
    // `save_task_context`, `complete_task_transaction`, `should_retry_task`,
    // `is_transient_error`, `schedule_task_retry` — moved to
    // `crate::executor::result_handler::TaskResultHandler` so the upcoming
    // fleet executor can share the same state-write sequence. See
    // `result_handler.rs` for the (verbatim) implementations.
}

impl Clone for ThreadTaskExecutor {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            dal: self.dal.clone(),
            task_registry: Arc::clone(&self.task_registry),
            runtime: Arc::clone(&self.runtime),
            instance_id: self.instance_id,
            config: self.config.clone(),
            // Shared semaphore — clones coordinate on the same concurrency limit
            semaphore: Arc::clone(&self.semaphore),
            // Counters are now Arc<AtomicU64> so clones share the same
            // running totals; T-0630 also shares them with the
            // result_handler.
            total_executed: Arc::clone(&self.total_executed),
            total_failed: Arc::clone(&self.total_failed),
            result_handler: self.result_handler.clone(),
        }
    }
}

/// Implementation of the dispatcher's TaskExecutor trait.
///
/// This allows ThreadTaskExecutor to be used with the dispatcher pattern,
/// receiving task events directly instead of polling the database.
#[async_trait]
impl TaskExecutor for ThreadTaskExecutor {
    async fn execute(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError> {
        let start = Instant::now();

        // If claiming is enabled, try to claim the task before executing.
        // If another runner already claimed it, skip silently.
        if self.config.enable_claiming {
            use crate::dal::unified::task_execution::RunnerClaimResult;
            let claim_result = self
                .dal
                .task_execution()
                .claim_for_runner(event.task_execution_id, self.instance_id)
                .await;

            match claim_result {
                Ok(RunnerClaimResult::Claimed) => {
                    metrics::counter!(
                        "cloacina_scheduler_claim_attempts_total",
                        "outcome" => "claimed",
                    )
                    .increment(1);
                    tracing::debug!(
                        task_id = %event.task_execution_id,
                        runner_id = %self.instance_id,
                        "Task claimed for execution"
                    );
                }
                Ok(RunnerClaimResult::AlreadyClaimed) => {
                    metrics::counter!(
                        "cloacina_scheduler_claim_attempts_total",
                        "outcome" => "contended",
                    )
                    .increment(1);
                    tracing::debug!(
                        task_id = %event.task_execution_id,
                        "Task already claimed by another runner — skipping"
                    );
                    return Ok(ExecutionResult::skipped(event.task_execution_id));
                }
                Err(e) => {
                    tracing::warn!(
                        task_id = %event.task_execution_id,
                        error = %e,
                        "Failed to claim task — proceeding without claim"
                    );
                }
            }
        }

        // Surface the workflow execution as Running once a task of it is being
        // executed (parity with the fleet path; CLOACI-T-0639). Workflow
        // executions otherwise go Pending → Completed directly — the completion
        // guards accept any non-terminal status — so a long in-process run would
        // read Pending the whole time. Best-effort + idempotent (only ever
        // Pending→Running or Running→Running, since the scheduler only dispatches
        // tasks for active executions).
        if let Err(e) = self
            .dal
            .workflow_execution()
            .update_status(event.workflow_execution_id, "Running")
            .await
        {
            tracing::warn!(
                workflow_id = %event.workflow_execution_id,
                error = %e,
                "Failed to mark workflow execution Running"
            );
        }

        // Cancellation channel — the heartbeat loop flips this to `true` if
        // it detects `ClaimLost`. The execution future races against it via
        // `execute_with_cancellation` (Layer 1), and tasks holding a
        // `TaskHandle` can observe it cooperatively via
        // `TaskHandle::is_cancelled` / `cancelled()` (Layer 2). See T-0487.
        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

        // If claiming is enabled, start a background heartbeat task.
        let heartbeat_handle = if self.config.enable_claiming {
            let dal = self.dal.clone();
            let task_id = event.task_execution_id;
            let runner_id = self.instance_id;
            let interval = self.config.heartbeat_interval;
            let cancel_tx = cancel_tx.clone();
            Some(tokio::spawn(async move {
                let mut ticker = tokio::time::interval(interval);
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    ticker.tick().await;
                    match dal.task_execution().heartbeat(task_id, runner_id).await {
                        Ok(crate::dal::unified::task_execution::HeartbeatResult::Ok) => {
                            metrics::counter!("cloacina_scheduler_heartbeat_writes_total")
                                .increment(1);
                            tracing::trace!(task_id = %task_id, "Heartbeat sent");
                        }
                        Ok(crate::dal::unified::task_execution::HeartbeatResult::ClaimLost) => {
                            tracing::warn!(
                                task_id = %task_id,
                                "Heartbeat failed — claim lost, signaling cancellation"
                            );
                            let _ = cancel_tx.send(true);
                            break;
                        }
                        Err(e) => {
                            tracing::warn!(
                                task_id = %task_id,
                                error = %e,
                                "Heartbeat error"
                            );
                        }
                    }
                }
            }))
        } else {
            None
        };

        // Acquire a concurrency slot — held for the duration of execution.
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| DispatchError::ExecutorNotFound("semaphore closed".into()))?;

        // Stamp started_at now that the slot is acquired and execution is about
        // to begin — so a task's duration reflects real work, not time spent
        // waiting for a concurrency slot. The claiming path may already have set
        // it; mark_started is a no-op when started_at is non-NULL. (The embedded
        // path otherwise leaves it NULL, breaking the per-task timeline.)
        // Best-effort.
        if let Err(e) = self
            .dal
            .task_execution()
            .mark_started(event.task_execution_id)
            .await
        {
            tracing::warn!(
                task_id = %event.task_execution_id,
                error = %e,
                "Failed to stamp task started_at"
            );
        }

        // Compute runner_id for claim-guarded state transitions
        let claim_runner_id = if self.config.enable_claiming {
            Some(self.instance_id)
        } else {
            None
        };

        // Convert TaskReadyEvent to ClaimedTask format
        let claimed_task = ClaimedTask {
            task_execution_id: event.task_execution_id,
            workflow_execution_id: event.workflow_execution_id,
            task_name: event.task_name.clone(),
            attempt: event.attempt,
        };

        // Resolve task from global registry
        let namespace = match parse_namespace(&claimed_task.task_name) {
            Ok(ns) => ns,
            Err(e) => {
                self.total_failed.fetch_add(1, Ordering::SeqCst);
                let error_msg = format!("Invalid namespace: {}", e);
                let _ = self
                    .dal
                    .task_execution()
                    .mark_failed(event.task_execution_id, &error_msg, claim_runner_id)
                    .await;
                return Ok(ExecutionResult::failure(
                    event.task_execution_id,
                    error_msg,
                    start.elapsed(),
                ));
            }
        };

        let task = match self.runtime.get_task(&namespace) {
            Some(t) => t,
            None => {
                self.total_failed.fetch_add(1, Ordering::SeqCst);
                let error_msg = format!("Task not found: {}", claimed_task.task_name);
                let _ = self
                    .dal
                    .task_execution()
                    .mark_failed(event.task_execution_id, &error_msg, claim_runner_id)
                    .await;
                return Ok(ExecutionResult::failure(
                    event.task_execution_id,
                    error_msg,
                    start.elapsed(),
                ));
            }
        };

        // Build context for execution
        let dependencies = task.dependencies();
        let context = match self.build_task_context(&claimed_task, dependencies).await {
            Ok(ctx) => ctx,
            Err(e) => {
                self.total_failed.fetch_add(1, Ordering::SeqCst);
                let error_msg = format!("Context build failed: {}", e);
                let _ = self
                    .dal
                    .task_execution()
                    .mark_failed(event.task_execution_id, &error_msg, claim_runner_id)
                    .await;
                return Ok(ExecutionResult::failure(
                    event.task_execution_id,
                    error_msg,
                    start.elapsed(),
                ));
            }
        };

        // `cloacina_active_tasks` is SQL-derived in the scheduler tick
        // (see `SchedulerLoop::process_active_executions`); no
        // increment/decrement here because a panic between the two would
        // leak the gauge permanently. CLOACI-T-0589 / mirrors T-0534.

        // Execute the task — if it requires a handle, wrap execution with
        // task-local storage so the macro-generated code can access it.
        let execution_result = if task.requires_handle() {
            let slot_token = SlotToken::new(permit, self.semaphore.clone());
            let handle = TaskHandle::with_dal_and_cancel(
                slot_token,
                event.task_execution_id,
                self.dal.clone(),
                cancel_rx.clone(),
            );

            // Set initial sub_status to Active
            if let Err(e) = self
                .dal
                .task_execution()
                .set_sub_status(event.task_execution_id, Some("Active"))
                .await
            {
                tracing::warn!(
                    task_execution_id = %event.task_execution_id,
                    error = %e,
                    "Failed to set initial sub_status to Active"
                );
            }

            let (result, _returned_handle) = with_task_handle(
                handle,
                self.execute_with_cancellation(task.as_ref(), context, cancel_rx.clone()),
            )
            .await;

            // Clear sub_status when task completes
            if let Err(e) = self
                .dal
                .task_execution()
                .set_sub_status(event.task_execution_id, None)
                .await
            {
                tracing::warn!(
                    task_execution_id = %event.task_execution_id,
                    error = %e,
                    "Failed to clear sub_status after execution"
                );
            }

            // The returned handle (and its slot token) is dropped here,
            // releasing the permit if still held.
            result
        } else {
            // No handle needed — permit is held as _permit for the duration.
            let _permit = permit;
            self.execute_with_cancellation(task.as_ref(), context, cancel_rx.clone())
                .await
        };
        // Drop the local cancel sender so that, once the heartbeat task's
        // clone is aborted, receivers can observe the channel close rather
        // than hang forever. (Not strictly required since the select! arm
        // already holds the last ref during execution, but makes the
        // post-execution state tidier for debugging.)
        drop(cancel_tx);
        let duration = start.elapsed();
        metrics::histogram!("cloacina_task_duration_seconds").record(duration.as_secs_f64());
        // No `cloacina_active_tasks.decrement()` — SQL-derived in the
        // scheduler tick. See CLOACI-T-0589.

        // Stop heartbeat and release claim after execution (success or failure).
        // COR-08: actually wait for the heartbeat task to finish so the
        // synchronous-close contract holds. Without the bounded await,
        // an in-flight `dal.task_execution().heartbeat(...)` could still
        // be racing the final `mark_completed` write — confusing the
        // claim-loss path. 100ms is plenty: the heartbeat loop's only
        // await point is the DAL call, and after `abort()` it cooperates
        // immediately at the next `tokio::select!` poll.
        if let Some(handle) = heartbeat_handle {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_millis(100), handle).await;
        }

        // Delegate post-execution handling (status writes, retry decision,
        // context persistence, counters, logging) to the shared
        // `TaskResultHandler` (T-0630). The fleet executor (T-0633) will use
        // an analogous handler to reconcile agent-reported results so the
        // two paths share one state-write sequence by construction.
        let retry_policy = task.retry_policy();
        let result = Ok(self
            .result_handler
            .handle_outcome(
                &event,
                &claimed_task,
                execution_result,
                &retry_policy,
                duration,
            )
            .await);

        // Release runner claim (on success, failure, or retry)
        if self.config.enable_claiming {
            if let Err(e) = self
                .dal
                .task_execution()
                .release_runner_claim(event.task_execution_id)
                .await
            {
                tracing::warn!(
                    task_id = %event.task_execution_id,
                    error = %e,
                    "Failed to release runner claim"
                );
            }
        }

        result
    }

    fn has_capacity(&self) -> bool {
        self.semaphore.available_permits() > 0
    }

    fn metrics(&self) -> ExecutorMetrics {
        let available = self.semaphore.available_permits();
        let active = self.config.max_concurrent_tasks.saturating_sub(available);
        ExecutorMetrics {
            active_tasks: active,
            max_concurrent: self.config.max_concurrent_tasks,
            total_executed: self.total_executed.load(Ordering::SeqCst),
            total_failed: self.total_failed.load(Ordering::SeqCst),
            avg_duration_ms: 0, // TODO: track moving average
        }
    }

    fn name(&self) -> &str {
        "ThreadTaskExecutor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // -----------------------------------------------------------------------
    // failure_reason — bounded reason label for cloacina_tasks_total
    // -----------------------------------------------------------------------

    #[test]
    fn failure_reason_covers_every_variant_with_bounded_values() {
        use crate::error::TaskError;

        let cases: Vec<(ExecutorError, &str)> = vec![
            (ExecutorError::TaskTimeout, "timeout"),
            (
                ExecutorError::TaskExecution(TaskError::ExecutionFailed {
                    message: "boom".into(),
                    task_id: "t".into(),
                    timestamp: chrono::Utc::now(),
                }),
                "task_error",
            ),
            (
                ExecutorError::Validation(crate::error::ValidationError::InvalidTaskName(
                    "x".into(),
                )),
                "validation_failed",
            ),
            (
                ExecutorError::ConnectionPool("pool exhausted".into()),
                "infrastructure",
            ),
            (
                // COR-11: ContextLoadFailed now surfaces under its own
                // bounded reason value, distinct from the generic
                // `infrastructure` bucket.
                ExecutorError::ContextLoadFailed("bad".into()),
                "context_load_failed",
            ),
            (
                ExecutorError::TaskNotFound("missing".into()),
                "task_not_found",
            ),
            (ExecutorError::ClaimLost, "claim_lost"),
            (ExecutorError::InvalidScope("scope".into()), "unknown"),
        ];

        let allowed: std::collections::HashSet<&'static str> = [
            "timeout",
            "task_error",
            "validation_failed",
            "infrastructure",
            "context_load_failed",
            "task_not_found",
            "claim_lost",
            "unknown",
        ]
        .into_iter()
        .collect();

        for (err, expected) in cases {
            let got = failure_reason(&err);
            assert_eq!(got, expected, "wrong reason for {:?}", err);
            assert!(
                allowed.contains(got),
                "reason {} is not in the bounded set",
                got
            );
        }
    }

    // -----------------------------------------------------------------------
    // merge_context_values tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_merge_primitives_latest_wins() {
        let existing = json!(42);
        let new = json!(99);
        let merged = ThreadTaskExecutor::merge_context_values(&existing, &new);
        assert_eq!(merged, json!(99));
    }

    #[test]
    fn test_merge_string_latest_wins() {
        let existing = json!("old");
        let new = json!("new");
        let merged = ThreadTaskExecutor::merge_context_values(&existing, &new);
        assert_eq!(merged, json!("new"));
    }

    #[test]
    fn test_merge_different_types_latest_wins() {
        let existing = json!(42);
        let new = json!("now_a_string");
        let merged = ThreadTaskExecutor::merge_context_values(&existing, &new);
        assert_eq!(merged, json!("now_a_string"));
    }

    #[test]
    fn test_merge_arrays_deduplicates() {
        let existing = json!([1, 2, 3]);
        let new = json!([2, 3, 4, 5]);
        let merged = ThreadTaskExecutor::merge_context_values(&existing, &new);
        assert_eq!(merged, json!([1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_merge_arrays_no_overlap() {
        let existing = json!(["a", "b"]);
        let new = json!(["c", "d"]);
        let merged = ThreadTaskExecutor::merge_context_values(&existing, &new);
        assert_eq!(merged, json!(["a", "b", "c", "d"]));
    }

    #[test]
    fn test_merge_arrays_complete_overlap() {
        let existing = json!([1, 2, 3]);
        let new = json!([1, 2, 3]);
        let merged = ThreadTaskExecutor::merge_context_values(&existing, &new);
        assert_eq!(merged, json!([1, 2, 3]));
    }

    #[test]
    fn test_merge_objects_no_conflict() {
        let existing = json!({"a": 1, "b": 2});
        let new = json!({"c": 3, "d": 4});
        let merged = ThreadTaskExecutor::merge_context_values(&existing, &new);
        assert_eq!(merged, json!({"a": 1, "b": 2, "c": 3, "d": 4}));
    }

    #[test]
    fn test_merge_objects_conflicting_keys() {
        let existing = json!({"a": 1, "b": "old"});
        let new = json!({"b": "new", "c": 3});
        let merged = ThreadTaskExecutor::merge_context_values(&existing, &new);
        assert_eq!(merged, json!({"a": 1, "b": "new", "c": 3}));
    }

    #[test]
    fn test_merge_objects_recursive() {
        let existing = json!({"nested": {"x": 1, "y": 2}});
        let new = json!({"nested": {"y": 99, "z": 3}});
        let merged = ThreadTaskExecutor::merge_context_values(&existing, &new);
        assert_eq!(merged, json!({"nested": {"x": 1, "y": 99, "z": 3}}));
    }

    #[test]
    fn test_merge_nested_arrays_in_objects() {
        let existing = json!({"items": [1, 2]});
        let new = json!({"items": [2, 3]});
        let merged = ThreadTaskExecutor::merge_context_values(&existing, &new);
        assert_eq!(merged, json!({"items": [1, 2, 3]}));
    }

    #[test]
    fn test_merge_null_latest_wins() {
        let existing = json!(42);
        let new = json!(null);
        let merged = ThreadTaskExecutor::merge_context_values(&existing, &new);
        assert_eq!(merged, json!(null));
    }

    #[test]
    fn test_merge_bool_latest_wins() {
        let existing = json!(true);
        let new = json!(false);
        let merged = ThreadTaskExecutor::merge_context_values(&existing, &new);
        assert_eq!(merged, json!(false));
    }

    // -----------------------------------------------------------------------
    // Tests requiring SQLite (executor construction uses in-memory SQLite)
    // -----------------------------------------------------------------------
    #[cfg(feature = "sqlite")]
    mod sqlite_tests {
        use super::*;

        fn test_executor() -> ThreadTaskExecutor {
            let db = Database::new("sqlite://:memory:", "", 1);
            let registry = Arc::new(TaskRegistry::new());
            let config = ExecutorConfig::default();
            ThreadTaskExecutor::new(db, registry, config)
        }

        // is_transient_* tests moved to result_handler.rs in T-0630 (the
        // function lives on `TaskResultHandler` now).

        // -----------------------------------------------------------------------
        // ThreadTaskExecutor construction and metrics tests
        // -----------------------------------------------------------------------

        #[test]
        fn test_executor_has_capacity_initially() {
            let exec = test_executor();
            assert!(exec.has_capacity());
        }

        #[test]
        fn test_executor_metrics_initial() {
            let exec = test_executor();
            let metrics = exec.metrics();
            assert_eq!(metrics.active_tasks, 0);
            assert_eq!(metrics.max_concurrent, 4);
            assert_eq!(metrics.total_executed, 0);
            assert_eq!(metrics.total_failed, 0);
        }

        #[test]
        fn test_executor_name() {
            let exec = test_executor();
            assert_eq!(exec.name(), "ThreadTaskExecutor");
        }

        #[test]
        fn test_executor_clone_shares_semaphore() {
            let exec = test_executor();
            let cloned = exec.clone();
            // Both should share the same semaphore, so available permits should match
            assert_eq!(
                exec.semaphore().available_permits(),
                cloned.semaphore().available_permits()
            );
        }

        #[test]
        fn test_executor_custom_config() {
            let db = Database::new("sqlite://:memory:", "", 1);
            let registry = Arc::new(TaskRegistry::new());
            let config = ExecutorConfig {
                max_concurrent_tasks: 8,
                task_timeout: std::time::Duration::from_secs(60),
                enable_claiming: false,
                heartbeat_interval: std::time::Duration::from_secs(5),
            };
            let exec = ThreadTaskExecutor::new(db, registry, config);
            let metrics = exec.metrics();
            assert_eq!(metrics.max_concurrent, 8);
            assert_eq!(exec.semaphore().available_permits(), 8);
        }
    } // mod sqlite_tests

    // -----------------------------------------------------------------------
    // Runtime isolation tests (deadlock prevention)
    // -----------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_new_uses_empty_runtime_not_from_global() {
        // ThreadTaskExecutor::new() must NOT call Runtime::from_global() — that
        // was the cause of the deadlock when #[ctor] constructors blocked.
        // Verify the runtime is empty (use_globals = false, no workflows).
        let db = Database::new("sqlite://:memory:", "test", 1);
        let config = ExecutorConfig::default();
        let exec = ThreadTaskExecutor::new(db, Arc::new(TaskRegistry::new()), config);

        // The runtime should be isolated (Runtime::new(), not from_global())
        assert!(
            exec.runtime.workflow_names().is_empty(),
            "new() executor should have an empty runtime with no workflows"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_with_runtime_and_registry_uses_provided_runtime() {
        let db = Database::new("sqlite://:memory:", "test", 1);
        let config = ExecutorConfig::default();

        // Create a runtime with a workflow
        let runtime = Arc::new(Runtime::new());
        let wf = crate::workflow::Workflow::new("test_wf");
        runtime.register_workflow("test_wf".to_string(), move || wf.clone());

        let exec = ThreadTaskExecutor::with_runtime_and_registry(
            db,
            Arc::new(TaskRegistry::new()),
            runtime,
            config,
        );

        // Executor should see the workflow via the provided runtime
        assert!(
            exec.runtime.get_workflow("test_wf").is_some(),
            "Executor should use the provided runtime"
        );
    }
}
