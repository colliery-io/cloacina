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

//! Shared post-execution result handler (CLOACI-I-0114 / task T-0630).
//!
//! Extracted from [`super::ThreadTaskExecutor`]'s post-closure block so the
//! upcoming `FleetExecutor` (T-0633) can reconcile agent-reported results
//! through the **same** state-write sequence the thread executor uses.
//! Without this seam the two executors would inevitably drift on status
//! transitions, retry decisions, context persistence, and metric increments —
//! and "did this task get the right state?" would depend on *where* it ran.
//!
//! The handler is intentionally independent of *where* the task closure ran:
//! it takes a `Result<Context, ExecutorError>` (the outcome) plus identity +
//! retry policy + elapsed duration, and applies all persistent-state effects.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use chrono::Utc;
use tracing::{debug, error, info, warn};

/// CLOACI-I-0140: is this a TRANSIENT database-contention error worth retrying?
/// SQLite under concurrent writers surfaces `database is locked` (SQLITE_BUSY,
/// after `busy_timeout` elapses) or `database table is locked` (SQLITE_LOCKED);
/// postgres has its own transient shapes (serialization/deadlock). Terminal
/// task-state transitions must not be dropped on these — a swallowed
/// `mark_failed` left the task Running FOREVER, which is the reproduced
/// scenario-flake HANG (the workflow future never resolves, the Python
/// `execute()` blocks on its oneshot, pytest-timeout is silenced).
pub(crate) fn is_transient_db_error(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    m.contains("database is locked")
        || m.contains("database table is locked")
        || m.contains("deadlock detected")
        || m.contains("could not serialize access")
}

/// Retry an async DB write on transient contention errors: `attempts` tries,
/// linear backoff (`base_delay * attempt`). Non-transient errors return
/// immediately; exhaustion returns the last error.
pub(crate) async fn retry_transient<T, E, F, Fut>(
    attempts: u32,
    base_delay: Duration,
    mut op: F,
) -> Result<T, E>
where
    E: std::fmt::Display,
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let mut last: Option<E> = None;
    for attempt in 1..=attempts {
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) if is_transient_db_error(&e.to_string()) && attempt < attempts => {
                warn!(
                    attempt,
                    attempts,
                    error = %e,
                    "transient DB contention on task-state write — retrying"
                );
                tokio::time::sleep(base_delay * attempt).await;
                last = Some(e);
            }
            Err(e) => return Err(e),
        }
    }
    Err(last.expect("retry_transient: attempts >= 1"))
}

use crate::context::Context;
use crate::dal::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::dispatcher::{ExecutionResult, TaskReadyEvent};
use crate::error::ExecutorError;
use crate::executor::context_size::{ContextKind, ContextSizeLimits, check_context_size};
use crate::executor::types::ClaimedTask;
use crate::retry::{RetryCondition, RetryPolicy};

/// Bounded `reason` value of `cloacina_tasks_total` for a failed or retried
/// task.
///
/// Cardinality is closed: the set of returned values is fixed here, so the
/// label cannot grow with error text.
pub(crate) fn failure_reason(err: &ExecutorError) -> &'static str {
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
        // CLOACINA-T-1047: a context over the hard size limit.
        ExecutorError::ContextTooLarge { .. } => "context_too_large",
        ExecutorError::TaskNotFound(_) | ExecutorError::WorkflowExecutionNotFound(_) => {
            "task_not_found"
        }
        ExecutorError::Serialization(_)
        | ExecutorError::InvalidScope(_)
        | ExecutorError::Semaphore(_) => "unknown",
    }
}

/// Value of the `workflow` label for a task: the workflow part of its
/// `tenant::package::workflow::task` name, or `unknown` when the name does
/// not parse. Capped per tenant by [`crate::metric_labels::workflow_label`].
fn workflow_label(tenant: &str, task_name: &str) -> String {
    match crate::parse_namespace(task_name) {
        Ok(ns) => crate::metric_labels::workflow_label(tenant, &ns.workflow_id),
        Err(_) => "unknown".to_string(),
    }
}

/// Count one finished task attempt in `cloacina_tasks_total`
/// (CLOACINA-T-1024).
///
/// `status` is `completed`, `failed` or `retried`; `reason` is `ok` or a
/// value of [`failure_reason`]. [`TaskResultHandler::handle_outcome`] calls
/// this once per call, so the in-process executor and the fleet executor
/// count the same way. The in-process executor also calls it for the early
/// exits that fail a task before the handler runs.
pub(crate) fn record_task_outcome(
    dal: &DAL,
    task_name: &str,
    status: &'static str,
    reason: &'static str,
) {
    let tenant = dal.tenant_label();
    metrics::counter!(
        "cloacina_tasks_total",
        "status" => status,
        "reason" => reason,
        "workflow" => workflow_label(tenant, task_name),
        "tenant" => tenant.to_string(),
    )
    .increment(1);
}

/// Post-execution handler — applies all state writes / retry decisions /
/// metric increments for one task. Cloneable so executors can hold one per
/// instance and pass clones into spawned work where needed.
#[derive(Clone)]
pub struct TaskResultHandler {
    dal: DAL,
    total_executed: Arc<AtomicU64>,
    total_failed: Arc<AtomicU64>,
    /// `runner_id` for claim-guarded state transitions. `None` when claiming
    /// is disabled (or for backends that don't use the per-runner claim
    /// model). The fleet's reconciliation will plumb the agent's owning
    /// runner id here so `mark_completed` and friends stay claim-guarded.
    runner_id: Option<UniversalUuid>,
    /// CLOACINA-T-1047: the warning and the hard limit on the size of the
    /// context a task returns.
    context_limits: ContextSizeLimits,
}

impl TaskResultHandler {
    pub fn new(
        dal: DAL,
        total_executed: Arc<AtomicU64>,
        total_failed: Arc<AtomicU64>,
        runner_id: Option<UniversalUuid>,
    ) -> Self {
        Self {
            dal,
            total_executed,
            total_failed,
            runner_id,
            context_limits: ContextSizeLimits::default(),
        }
    }

    /// The same handler with these limits on the size of a result context
    /// (CLOACINA-T-1047). A new handler has the default limits.
    pub fn with_context_limits(mut self, context_limits: ContextSizeLimits) -> Self {
        self.context_limits = context_limits;
        self
    }

    /// The limits on the size of a result context.
    pub fn context_limits(&self) -> ContextSizeLimits {
        self.context_limits
    }

    /// The same handler, with its terminal writes (`mark_completed`,
    /// `mark_failed`) guarded by the claim of `runner_id`: they apply only
    /// while that runner holds the claim on the task (CLOACINA-T-1013).
    pub fn with_runner_id(mut self, runner_id: UniversalUuid) -> Self {
        self.runner_id = Some(runner_id);
        self
    }

    /// The runner whose claim guards the terminal writes, if any.
    pub fn runner_id(&self) -> Option<UniversalUuid> {
        self.runner_id
    }

    /// Apply the post-execution state machine for one task and return the
    /// dispatcher-shaped outcome.
    ///
    /// - `event` is the original [`TaskReadyEvent`] (used for log/metric ids).
    /// - `claimed_task` is the identity the helpers below operate on.
    /// - `outcome` is `Ok(produced_context)` on success or `Err(executor_error)`
    ///   on failure.
    /// - `retry_policy` is consulted only on the error branch.
    /// - `duration` is the wall-clock time the task closure consumed; used
    ///   for logs and embedded into the returned `ExecutionResult`.
    pub async fn handle_outcome(
        &self,
        event: &TaskReadyEvent,
        claimed_task: &ClaimedTask,
        outcome: Result<Context<serde_json::Value>, ExecutorError>,
        retry_policy: &RetryPolicy,
        duration: Duration,
    ) -> ExecutionResult {
        // One histogram for the in-process and the fleet path
        // (CLOACINA-T-1024). `duration` is what the executor measured from
        // the start of its dispatch to the result: for a fleet task that
        // includes the hand-off to the agent and the wait for its report.
        metrics::histogram!("cloacina_task_duration_seconds").record(duration.as_secs_f64());

        // CLOACINA-T-1047: measure the context the task returned, here, so
        // the in-process and the fleet path are measured and limited the same
        // way. Over the hard limit the result becomes a failure: the context
        // is not saved, and the retry decision below refuses a retry.
        let mut context_size_bytes: Option<u64> = None;
        let outcome = match outcome {
            Ok(result_context) => match check_context_size(
                &self.context_limits,
                self.dal.tenant_label(),
                &event.task_name,
                ContextKind::Result,
                result_context.data(),
            ) {
                Ok(size_bytes) => {
                    context_size_bytes = Some(size_bytes);
                    Ok(result_context)
                }
                Err(e) => {
                    error!(
                        task_id = %event.task_execution_id,
                        task_name = %event.task_name,
                        error = %e,
                        "Task result context refused; the task fails and is not retried"
                    );
                    Err(e)
                }
            },
            Err(e) => Err(e),
        };

        match outcome {
            Ok(result_context) => {
                // CLOACI-I-0140: retry transient DB contention — a sqlite-busy
                // context save was failing tasks that SUCCEEDED (the reproduced
                // 'Failed' == 'Completed' assertion flake class).
                match retry_transient(5, Duration::from_millis(100), || {
                    self.complete_task_transaction(
                        claimed_task,
                        result_context.clone_data(),
                        context_size_bytes,
                    )
                })
                .await
                {
                    Ok(()) => {
                        self.total_executed.fetch_add(1, Ordering::SeqCst);
                        record_task_outcome(&self.dal, &event.task_name, "completed", "ok");
                        info!(
                            task_id = %event.task_execution_id,
                            task_name = %event.task_name,
                            duration_ms = duration.as_millis(),
                            "Task executed successfully via dispatcher"
                        );
                        ExecutionResult::success(event.task_execution_id, duration)
                    }
                    Err(e) => {
                        self.total_failed.fetch_add(1, Ordering::SeqCst);
                        record_task_outcome(
                            &self.dal,
                            &event.task_name,
                            "failed",
                            failure_reason(&e),
                        );
                        let error_msg = format!("Failed to save context: {}", e);
                        // Mark failed in DB — executor owns all state transitions.
                        // CLOACI-I-0140: retried + NEVER silently swallowed — a
                        // dropped mark_failed leaves the task Running forever and
                        // hangs the workflow (the reproduced hang class).
                        if let Err(mark_err) =
                            retry_transient(5, Duration::from_millis(100), || async {
                                self.dal
                                    .task_execution()
                                    .mark_failed(
                                        event.task_execution_id,
                                        &error_msg,
                                        self.runner_id,
                                    )
                                    .await
                            })
                            .await
                        {
                            error!(
                                task_id = %event.task_execution_id,
                                error = %mark_err,
                                "mark_failed FAILED after retries — task row stays Running until \
                                 the stale-claim sweeper recovers it; workflow completion is delayed"
                            );
                        }
                        ExecutionResult::failure(event.task_execution_id, error_msg, duration)
                    }
                }
            }
            Err(error) => {
                let reason = failure_reason(&error);
                let should_retry = self
                    .should_retry_task(claimed_task, &error, retry_policy)
                    .await
                    .unwrap_or(false);

                if should_retry {
                    // CLOACI-I-0140: schedule_retry is a terminal state write too —
                    // dropping it on sqlite-busy left the task Running forever (the
                    // scenario-33 hang the first fix round missed). Retry it, and if
                    // it STILL fails, fail the task rather than leave it in limbo.
                    match retry_transient(5, Duration::from_millis(100), || {
                        self.schedule_task_retry(claimed_task, retry_policy)
                    })
                    .await
                    {
                        Ok(()) => {
                            self.total_executed.fetch_add(1, Ordering::SeqCst);
                            record_task_outcome(&self.dal, &event.task_name, "retried", reason);
                            ExecutionResult::retry(
                                event.task_execution_id,
                                error.to_string(),
                                duration,
                            )
                        }
                        Err(sched_err) => {
                            warn!(
                                task_id = %event.task_execution_id,
                                error = %sched_err,
                                "Failed to schedule retry after retries — failing task \
                                 instead of leaving it Running"
                            );
                            self.total_failed.fetch_add(1, Ordering::SeqCst);
                            record_task_outcome(&self.dal, &event.task_name, "failed", reason);
                            let error_str =
                                format!("{} (retry scheduling failed: {})", error, sched_err);
                            if let Err(mark_err) =
                                retry_transient(5, Duration::from_millis(100), || async {
                                    self.dal
                                        .task_execution()
                                        .mark_failed(
                                            event.task_execution_id,
                                            &error_str,
                                            self.runner_id,
                                        )
                                        .await
                                })
                                .await
                            {
                                error!(
                                    task_id = %event.task_execution_id,
                                    error = %mark_err,
                                    "mark_failed FAILED after retries — task row stays Running \
                                     until the stale-claim sweeper recovers it; workflow \
                                     completion is delayed"
                                );
                            }
                            ExecutionResult::failure(event.task_execution_id, error_str, duration)
                        }
                    }
                } else {
                    self.total_failed.fetch_add(1, Ordering::SeqCst);
                    record_task_outcome(&self.dal, &event.task_name, "failed", reason);
                    // Mark failed in DB — executor owns all state transitions.
                    // CLOACI-I-0140: retried + never swallowed (see above) — THIS
                    // was the primary hang site: the callback scenarios' tasks fail
                    // by design, and a sqlite-busy here left them Running forever.
                    let error_str = error.to_string();
                    if let Err(mark_err) =
                        retry_transient(5, Duration::from_millis(100), || async {
                            self.dal
                                .task_execution()
                                .mark_failed(event.task_execution_id, &error_str, self.runner_id)
                                .await
                        })
                        .await
                    {
                        error!(
                            task_id = %event.task_execution_id,
                            error = %mark_err,
                            "mark_failed FAILED after retries — task row stays Running until \
                             the stale-claim sweeper recovers it; workflow completion is delayed"
                        );
                    }
                    ExecutionResult::failure(event.task_execution_id, error_str, duration)
                }
            }
        }
    }

    /// Completes a task by saving its context and then marking it completed.
    ///
    /// **Order (COR-10):** save context first, mark completed second. The old
    /// order (`mark_completed` first) had a CRITICAL hole — if context save
    /// failed after the task was already marked Completed, the task looked
    /// done but downstream consumers saw missing/empty context. The reversed
    /// order is asymmetric in the opposite, harmless direction (orphan
    /// context row, harmless and prunable). See ThreadTaskExecutor history.
    async fn complete_task_transaction(
        &self,
        claimed_task: &ClaimedTask,
        context: Context<serde_json::Value>,
        context_size_bytes: Option<u64>,
    ) -> Result<(), ExecutorError> {
        // 1. Save context FIRST. If this fails, the task is still marked
        //    Running with our claim — the heartbeat-loss / claim-sweep
        //    cycle will reset it to Ready and another attempt drives it
        //    through this path again. No "marked Completed but no
        //    context" state is reachable.
        self.save_task_context(claimed_task, context, context_size_bytes)
            .await?;

        // 2. Mark completed — guarded by claim ownership.
        let applied = self
            .dal
            .task_execution()
            .mark_completed(claimed_task.task_execution_id, self.runner_id)
            .await?;

        if !applied {
            warn!(
                task_id = %claimed_task.task_execution_id,
                task_name = %claimed_task.task_name,
                "Claim lost between context save and mark_completed — context row is orphaned (harmless), another runner now owns this task"
            );
            return Ok(());
        }

        info!(
            task_id = %claimed_task.task_execution_id,
            task_name = %claimed_task.task_name,
            workflow_id = %claimed_task.workflow_execution_id,
            "Task state change: -> Completed"
        );

        Ok(())
    }

    /// Saves a task's produced context to the contexts table and links it via
    /// task_execution_metadata, with its serialized size (CLOACINA-T-1047).
    async fn save_task_context(
        &self,
        claimed_task: &ClaimedTask,
        context: Context<serde_json::Value>,
        context_size_bytes: Option<u64>,
    ) -> Result<(), ExecutorError> {
        use crate::models::task_execution_metadata::NewTaskExecutionMetadata;

        let context_id = self.dal.context().create(&context).await?;

        let task_metadata_record = NewTaskExecutionMetadata {
            task_execution_id: claimed_task.task_execution_id,
            workflow_execution_id: claimed_task.workflow_execution_id,
            task_name: claimed_task.task_name.clone(),
            context_id,
            context_size_bytes: context_size_bytes.map(|n| i64::try_from(n).unwrap_or(i64::MAX)),
        };

        self.dal
            .task_execution_metadata()
            .upsert_task_execution_metadata(task_metadata_record)
            .await?;

        let key_count = context.data().len();
        let keys: Vec<_> = context.data().keys().collect();
        info!(
            "Context saved: {} (workflow: {}, {} keys: {:?}, context_id: {:?})",
            claimed_task.task_name, claimed_task.workflow_execution_id, key_count, keys, context_id
        );
        Ok(())
    }

    /// Determines if a failed task should be retried, considering max-attempts,
    /// `RetryPolicy` conditions, and error classification.
    async fn should_retry_task(
        &self,
        claimed_task: &ClaimedTask,
        error: &ExecutorError,
        retry_policy: &RetryPolicy,
    ) -> Result<bool, ExecutorError> {
        // Claim loss means another runner owns the task now; retrying from
        // this runner would either fight for the claim or spawn a duplicate
        // attempt. Let the owning runner drive the outcome.
        if matches!(error, ExecutorError::ClaimLost) {
            return Ok(false);
        }

        // CLOACINA-T-1047: a context over the size limit is never retried.
        // The next attempt would build the same context.
        if matches!(error, ExecutorError::ContextTooLarge { .. }) {
            return Ok(false);
        }

        if claimed_task.attempt >= retry_policy.max_attempts {
            debug!(
                "Task {} exceeded max retry attempts ({}/{})",
                claimed_task.task_name, claimed_task.attempt, retry_policy.max_attempts
            );
            return Ok(false);
        }

        let should_retry = Self::retry_allowed(retry_policy, claimed_task.attempt, error);

        debug!(
            "Retry decision for task {}: {} (conditions: {:?}, error: {})",
            claimed_task.task_name, should_retry, retry_policy.retry_conditions, error
        );

        Ok(should_retry)
    }

    /// The retry decision for one failed attempt, with no I/O: `true` when the
    /// policy allows another attempt after `attempt` failed with `error`.
    ///
    /// Public so the fleet executor's tests can show that the error it builds
    /// from an agent's report reaches the retry conditions (CLOACINA-T-1014).
    /// A condition is a test on the error text, so the text is part of the
    /// contract.
    pub fn retry_allowed(retry_policy: &RetryPolicy, attempt: i32, error: &ExecutorError) -> bool {
        if matches!(error, ExecutorError::ClaimLost) {
            return false;
        }
        // CLOACINA-T-1047: not retryable under any policy, `AllErrors` and a
        // matching `ErrorPattern` included.
        if matches!(error, ExecutorError::ContextTooLarge { .. }) {
            return false;
        }
        if attempt >= retry_policy.max_attempts {
            return false;
        }
        retry_policy
            .retry_conditions
            .iter()
            .all(|condition| match condition {
                RetryCondition::Never => false,
                RetryCondition::AllErrors => true,
                RetryCondition::TransientOnly => Self::error_is_transient(error),
                RetryCondition::ErrorPattern { patterns } => {
                    let error_msg = error.to_string().to_lowercase();
                    patterns
                        .iter()
                        .any(|pattern| error_msg.contains(&pattern.to_lowercase()))
                }
            })
    }

    /// Classifies whether an error looks transient enough to retry under
    /// `RetryCondition::TransientOnly`. Public so the unit tests that pin the
    /// classification (moved here from ThreadTaskExecutor in T-0630) can
    /// exercise it directly.
    pub fn is_transient_error(&self, error: &ExecutorError) -> bool {
        Self::error_is_transient(error)
    }

    /// [`Self::is_transient_error`] with no handler: the classification reads
    /// only the error.
    pub fn error_is_transient(error: &ExecutorError) -> bool {
        match error {
            ExecutorError::TaskTimeout => true,
            ExecutorError::Database(_) => true,
            ExecutorError::ConnectionPool(_) => true,
            ExecutorError::TaskNotFound(_) => false,
            ExecutorError::TaskExecution(task_error) => {
                let error_msg = task_error.to_string().to_lowercase();
                error_msg.contains("timeout")
                    || error_msg.contains("connection")
                    || error_msg.contains("network")
                    || error_msg.contains("temporary")
                    || error_msg.contains("unavailable")
            }
            _ => false,
        }
    }

    /// Schedules a task for retry via the DAL, computing the delay from the
    /// retry policy's backoff strategy.
    async fn schedule_task_retry(
        &self,
        claimed_task: &ClaimedTask,
        retry_policy: &RetryPolicy,
    ) -> Result<(), ExecutorError> {
        let retry_delay = retry_policy.calculate_delay(claimed_task.attempt);
        let retry_at = Utc::now() + retry_delay;

        self.dal
            .task_execution()
            .schedule_retry(
                claimed_task.task_execution_id,
                crate::database::UniversalTimestamp(retry_at),
                claimed_task.attempt + 1,
            )
            .await?;

        info!(
            "Scheduled retry for task {} in {:?} (attempt {})",
            claimed_task.task_name,
            retry_delay,
            claimed_task.attempt + 1
        );

        Ok(())
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod is_transient_tests {
    //! Moved from `thread_task_executor.rs` in T-0630 (the classification
    //! function now lives on `TaskResultHandler`). The original assertions
    //! are preserved verbatim.

    use super::*;
    use crate::database::Database;

    fn handler() -> TaskResultHandler {
        let db = Database::new("sqlite://:memory:", "", 1);
        let dal = DAL::new(db);
        TaskResultHandler::new(
            dal,
            Arc::new(AtomicU64::new(0)),
            Arc::new(AtomicU64::new(0)),
            None,
        )
    }

    #[test]
    fn test_is_transient_timeout() {
        assert!(handler().is_transient_error(&ExecutorError::TaskTimeout));
    }

    #[test]
    fn test_is_transient_task_not_found() {
        assert!(!handler().is_transient_error(&ExecutorError::TaskNotFound("missing".to_string())));
    }

    #[test]
    fn test_is_transient_connection_pool() {
        assert!(
            handler()
                .is_transient_error(&ExecutorError::ConnectionPool("pool exhausted".to_string()))
        );
    }

    #[test]
    fn test_is_transient_task_execution_with_timeout_msg() {
        let task_err = crate::error::TaskError::ExecutionFailed {
            message: "connection timeout while waiting".to_string(),
            task_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert!(handler().is_transient_error(&ExecutorError::TaskExecution(task_err)));
    }

    #[test]
    fn test_is_transient_task_execution_permanent() {
        let task_err = crate::error::TaskError::ExecutionFailed {
            message: "invalid input data".to_string(),
            task_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert!(!handler().is_transient_error(&ExecutorError::TaskExecution(task_err)));
    }

    #[test]
    fn test_is_transient_task_execution_network() {
        let task_err = crate::error::TaskError::ExecutionFailed {
            message: "network unreachable".to_string(),
            task_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert!(handler().is_transient_error(&ExecutorError::TaskExecution(task_err)));
    }

    #[test]
    fn test_is_transient_task_execution_unavailable() {
        let task_err = crate::error::TaskError::ExecutionFailed {
            message: "service temporarily unavailable".to_string(),
            task_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert!(handler().is_transient_error(&ExecutorError::TaskExecution(task_err)));
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod outcome_metrics_tests {
    //! `handle_outcome` is the one place that counts a task attempt and
    //! records its duration (CLOACINA-T-1024). These tests run it on SQLite
    //! in memory under a thread-local metrics recorder.

    use super::*;
    use crate::database::Database;
    use crate::models::task_execution::NewTaskExecution;
    use crate::models::workflow_execution::NewWorkflowExecution;
    use metrics_util::debugging::{DebugValue, DebuggingRecorder};
    use metrics_util::{CompositeKey, MetricKind};

    type Snapshot = Vec<(CompositeKey, DebugValue)>;

    /// Run one `handle_outcome` call for a new task named `task_name` at
    /// `attempt` and return the metrics it emitted.
    fn run_outcome(
        task_name: &str,
        attempt: i32,
        outcome: Result<Context<serde_json::Value>, ExecutorError>,
    ) -> (ExecutionResult, Snapshot) {
        let recorder = DebuggingRecorder::new();
        let snapshotter = recorder.snapshotter();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let task_name = task_name.to_string();
        let result = metrics::with_local_recorder(&recorder, || {
            rt.block_on(async move {
                let url = format!(
                    "file:outcome_metrics_test_{}?mode=memory&cache=shared",
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
                let task_id = dal
                    .task_execution()
                    .create(NewTaskExecution {
                        workflow_execution_id: workflow_id,
                        task_name: task_name.clone(),
                        status: "Running".into(),
                        attempt,
                        max_attempts: 3,
                        trigger_rules: r#"{"type":"Always"}"#.into(),
                        task_configuration: "{}".into(),
                    })
                    .await
                    .unwrap()
                    .id;
                let handler = TaskResultHandler::new(
                    dal,
                    Arc::new(AtomicU64::new(0)),
                    Arc::new(AtomicU64::new(0)),
                    None,
                );
                let event = TaskReadyEvent::new(task_id, workflow_id, task_name.clone(), attempt);
                let claimed = ClaimedTask {
                    task_execution_id: task_id,
                    workflow_execution_id: workflow_id,
                    task_name,
                    attempt,
                };
                handler
                    .handle_outcome(
                        &event,
                        &claimed,
                        outcome,
                        &RetryPolicy::default(),
                        Duration::from_millis(250),
                    )
                    .await
            })
        });
        let snapshot = snapshotter
            .snapshot()
            .into_vec()
            .into_iter()
            .map(|(key, _, _, value)| (key, value))
            .collect();
        (result, snapshot)
    }

    /// Each `cloacina_tasks_total` series as (sorted labels, count).
    fn task_counters(snapshot: &Snapshot) -> Vec<(Vec<(String, String)>, u64)> {
        snapshot
            .iter()
            .filter(|(key, _)| {
                key.kind() == MetricKind::Counter && key.key().name() == "cloacina_tasks_total"
            })
            .map(|(key, value)| {
                let mut labels: Vec<(String, String)> = key
                    .key()
                    .labels()
                    .map(|l| (l.key().to_string(), l.value().to_string()))
                    .collect();
                labels.sort();
                let count = match value {
                    DebugValue::Counter(n) => *n,
                    other => panic!("not a counter: {other:?}"),
                };
                (labels, count)
            })
            .collect()
    }

    fn labels(status: &str, reason: &str, workflow: &str) -> Vec<(String, String)> {
        vec![
            ("reason".to_string(), reason.to_string()),
            ("status".to_string(), status.to_string()),
            ("tenant".to_string(), "public".to_string()),
            ("workflow".to_string(), workflow.to_string()),
        ]
    }

    fn task_error() -> ExecutorError {
        ExecutorError::TaskExecution(crate::error::TaskError::ExecutionFailed {
            message: "boom".to_string(),
            task_id: "load".to_string(),
            timestamp: chrono::Utc::now(),
        })
    }

    fn duration_samples(snapshot: &Snapshot) -> Vec<f64> {
        snapshot
            .iter()
            .filter(|(key, _)| {
                key.kind() == MetricKind::Histogram
                    && key.key().name() == "cloacina_task_duration_seconds"
            })
            .flat_map(|(_, value)| match value {
                DebugValue::Histogram(samples) => {
                    samples.iter().map(|s| s.into_inner()).collect::<Vec<_>>()
                }
                other => panic!("not a histogram: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn handle_outcome_counts_a_completed_task_once_with_its_workflow() {
        let (result, snapshot) = run_outcome(
            "public::embedded::orders::load",
            1,
            Ok(Context::<serde_json::Value>::new()),
        );
        assert!(
            matches!(result.status, crate::dispatcher::ExecutionStatus::Completed),
            "status was {:?}",
            result.status
        );
        assert_eq!(
            task_counters(&snapshot),
            vec![(labels("completed", "ok", "orders"), 1)]
        );
        assert_eq!(duration_samples(&snapshot), vec![0.25]);
    }

    #[test]
    fn handle_outcome_counts_a_failed_task_with_a_bounded_reason() {
        // Attempt 3 of 3: no retry is left.
        let (_, snapshot) = run_outcome("public::embedded::orders::load", 3, Err(task_error()));
        assert_eq!(
            task_counters(&snapshot),
            vec![(labels("failed", "task_error", "orders"), 1)]
        );
        assert_eq!(duration_samples(&snapshot), vec![0.25]);
    }

    #[test]
    fn handle_outcome_counts_a_scheduled_retry_as_retried() {
        let (_, snapshot) = run_outcome(
            "public::embedded::orders::load",
            1,
            Err(ExecutorError::TaskTimeout),
        );
        assert_eq!(
            task_counters(&snapshot),
            vec![(labels("retried", "timeout", "orders"), 1)]
        );
    }

    #[test]
    fn handle_outcome_labels_a_task_name_that_does_not_parse_as_unknown() {
        let (_, snapshot) = run_outcome("not_a_namespace", 3, Err(task_error()));
        assert_eq!(
            task_counters(&snapshot),
            vec![(labels("failed", "task_error", "unknown"), 1)]
        );
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod context_size_tests {
    //! `handle_outcome` measures the context a task returns, saves the size
    //! with the task's metadata row, warns over the warning limit and fails
    //! the task with no retry over the hard limit (CLOACINA-T-1047). These
    //! tests run it on SQLite in memory under a thread-local metrics
    //! recorder.

    use super::*;
    use crate::database::Database;
    use crate::dispatcher::ExecutionStatus;
    use crate::models::task_execution::{NewTaskExecution, TaskExecution};
    use crate::models::workflow_execution::NewWorkflowExecution;
    use metrics_util::debugging::{DebugValue, DebuggingRecorder};
    use metrics_util::{CompositeKey, MetricKind};

    type Snapshot = Vec<(CompositeKey, DebugValue)>;
    type Labels = Vec<(String, String)>;

    const TASK: &str = "public::embedded::orders::load";

    /// What one `handle_outcome` call left behind.
    struct Outcome {
        result: ExecutionResult,
        snapshot: Snapshot,
        /// The task row after the call.
        task: TaskExecution,
        /// `context_size_bytes` of the task's metadata row; `None` when the
        /// task has no metadata row.
        saved_size: Option<Option<i64>>,
        /// `context_id` of the task's metadata row, when the row exists.
        saved_context_id: Option<UniversalUuid>,
    }

    /// Run one `handle_outcome` call for a new task at attempt 1 with a retry
    /// policy of 3 attempts on all errors.
    fn run(
        limits: ContextSizeLimits,
        outcome: Result<Context<serde_json::Value>, ExecutorError>,
    ) -> Outcome {
        let recorder = DebuggingRecorder::new();
        let snapshotter = recorder.snapshotter();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (result, task, saved_size, saved_context_id) =
            metrics::with_local_recorder(&recorder, || {
                rt.block_on(async move {
                    let url = format!(
                        "file:context_size_test_{}?mode=memory&cache=shared",
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
                    let task_id = dal
                        .task_execution()
                        .create(NewTaskExecution {
                            workflow_execution_id: workflow_id,
                            task_name: TASK.into(),
                            status: "Running".into(),
                            attempt: 1,
                            max_attempts: 3,
                            trigger_rules: r#"{"type":"Always"}"#.into(),
                            task_configuration: "{}".into(),
                        })
                        .await
                        .unwrap()
                        .id;
                    let handler = TaskResultHandler::new(
                        dal.clone(),
                        Arc::new(AtomicU64::new(0)),
                        Arc::new(AtomicU64::new(0)),
                        None,
                    )
                    .with_context_limits(limits);
                    let event = TaskReadyEvent::new(task_id, workflow_id, TASK.to_string(), 1);
                    let claimed = ClaimedTask {
                        task_execution_id: task_id,
                        workflow_execution_id: workflow_id,
                        task_name: TASK.to_string(),
                        attempt: 1,
                    };
                    let retry_policy = RetryPolicy::builder()
                        .max_attempts(3)
                        .retry_condition(RetryCondition::AllErrors)
                        .build();
                    let result = handler
                        .handle_outcome(
                            &event,
                            &claimed,
                            outcome,
                            &retry_policy,
                            Duration::from_millis(10),
                        )
                        .await;
                    let task = dal.task_execution().get_by_id(task_id).await.unwrap();
                    let metadata = dal
                        .task_execution_metadata()
                        .get_by_task_execution(task_id)
                        .await
                        .ok();
                    let sizes = dal
                        .task_execution_metadata()
                        .context_sizes_for_workflow(workflow_id)
                        .await
                        .unwrap();
                    // The lookup the API uses gives the same size as the row.
                    assert_eq!(
                        sizes.get(&task_id).copied(),
                        metadata.as_ref().and_then(|m| m.context_size_bytes)
                    );
                    (
                        result,
                        task,
                        metadata.as_ref().map(|m| m.context_size_bytes),
                        metadata.and_then(|m| m.context_id),
                    )
                })
            });
        let snapshot = snapshotter
            .snapshot()
            .into_vec()
            .into_iter()
            .map(|(key, _, _, value)| (key, value))
            .collect();
        Outcome {
            result,
            snapshot,
            task,
            saved_size,
            saved_context_id,
        }
    }

    fn sorted_labels(key: &CompositeKey) -> Labels {
        let mut labels: Labels = key
            .key()
            .labels()
            .map(|l| (l.key().to_string(), l.value().to_string()))
            .collect();
        labels.sort();
        labels
    }

    /// Each `cloacina_context_size_bytes` series as (sorted labels, samples).
    fn size_samples(snapshot: &Snapshot) -> Vec<(Labels, Vec<f64>)> {
        snapshot
            .iter()
            .filter(|(key, _)| {
                key.kind() == MetricKind::Histogram
                    && key.key().name() == "cloacina_context_size_bytes"
            })
            .map(|(key, value)| match value {
                DebugValue::Histogram(samples) => (
                    sorted_labels(key),
                    samples.iter().map(|s| s.into_inner()).collect(),
                ),
                other => panic!("not a histogram: {other:?}"),
            })
            .collect()
    }

    /// Each series of the counter `name` as (sorted labels, count).
    fn counters(snapshot: &Snapshot, name: &str) -> Vec<(Labels, u64)> {
        snapshot
            .iter()
            .filter(|(key, _)| key.kind() == MetricKind::Counter && key.key().name() == name)
            .map(|(key, value)| match value {
                DebugValue::Counter(n) => (sorted_labels(key), *n),
                other => panic!("not a counter: {other:?}"),
            })
            .collect()
    }

    fn result_labels() -> Labels {
        vec![
            ("kind".to_string(), "result".to_string()),
            ("tenant".to_string(), "public".to_string()),
        ]
    }

    fn tenant_label() -> Labels {
        vec![("tenant".to_string(), "public".to_string())]
    }

    /// A context with a string of `rows_len` characters under `rows` and a
    /// small `meta` key.
    fn context(rows_len: usize) -> Context<serde_json::Value> {
        let mut ctx = Context::<serde_json::Value>::new();
        ctx.insert("rows", serde_json::json!("x".repeat(rows_len)))
            .unwrap();
        ctx.insert("meta", serde_json::json!({"n": 1})).unwrap();
        ctx
    }

    fn size_of(ctx: &Context<serde_json::Value>) -> u64 {
        ctx.to_json().unwrap().len() as u64
    }

    #[test]
    fn a_result_under_the_limits_is_saved_with_its_size_and_measured() {
        let ctx = context(100);
        let size = size_of(&ctx);
        let out = run(ContextSizeLimits::default(), Ok(ctx));

        assert!(
            matches!(out.result.status, ExecutionStatus::Completed),
            "status was {:?}",
            out.result.status
        );
        assert_eq!(out.task.status, "Completed");
        assert_eq!(out.saved_size, Some(Some(size as i64)));
        assert!(out.saved_context_id.is_some(), "the context is saved");
        assert_eq!(
            size_samples(&out.snapshot),
            vec![(result_labels(), vec![size as f64])]
        );
        assert_eq!(
            counters(&out.snapshot, "cloacina_context_size_warnings_total"),
            vec![]
        );
    }

    #[test]
    fn a_result_between_the_limits_is_saved_and_counted_as_a_warning() {
        let ctx = context(2_000);
        let size = size_of(&ctx);
        let limits = ContextSizeLimits {
            warn_bytes: 1_000,
            max_bytes: 10_000,
        };
        let out = run(limits, Ok(ctx));

        assert_eq!(out.task.status, "Completed");
        assert_eq!(out.saved_size, Some(Some(size as i64)));
        assert_eq!(
            counters(&out.snapshot, "cloacina_context_size_warnings_total"),
            vec![(tenant_label(), 1)]
        );
        assert_eq!(
            size_samples(&out.snapshot),
            vec![(result_labels(), vec![size as f64])]
        );
    }

    #[test]
    fn a_result_over_the_hard_limit_fails_once_and_is_not_retried() {
        let ctx = context(20_000);
        let size = size_of(&ctx);
        let limits = ContextSizeLimits {
            warn_bytes: 1_000,
            max_bytes: 10_000,
        };
        // Attempt 1 of a policy with 3 attempts on all errors.
        let out = run(limits, Ok(ctx));

        assert!(
            matches!(out.result.status, ExecutionStatus::Failed),
            "status was {:?}",
            out.result.status
        );
        let expected = format!(
            "Context too large: the result context of task '{TASK}' is {size} bytes, over the \
             limit of 10000 bytes. Largest keys: rows (20002 bytes), meta (7 bytes). Write \
             large data to storage in the task and pass a reference in the context."
        );
        assert_eq!(out.result.error.as_deref(), Some(expected.as_str()));

        // The task row: failed, no retry scheduled, the error on the row.
        assert_eq!(out.task.status, "Failed");
        assert_eq!(out.task.attempt, 1);
        assert!(out.task.retry_at.is_none(), "no retry is scheduled");
        assert_eq!(out.task.last_error.as_deref(), Some(expected.as_str()));

        // The context is not saved.
        assert_eq!(out.saved_size, None, "no metadata row is written");

        // Counted once as failed, never as retried; measured; no warning.
        assert_eq!(
            counters(&out.snapshot, "cloacina_tasks_total"),
            vec![(
                vec![
                    ("reason".to_string(), "context_too_large".to_string()),
                    ("status".to_string(), "failed".to_string()),
                    ("tenant".to_string(), "public".to_string()),
                    ("workflow".to_string(), "orders".to_string()),
                ],
                1
            )]
        );
        assert_eq!(
            size_samples(&out.snapshot),
            vec![(result_labels(), vec![size as f64])]
        );
        assert_eq!(
            counters(&out.snapshot, "cloacina_context_size_warnings_total"),
            vec![]
        );
    }

    #[test]
    fn the_same_policy_retries_an_ordinary_task_error() {
        // The control for the test above: with this policy a task error at
        // attempt 1 is retried.
        let err = ExecutorError::TaskExecution(crate::error::TaskError::ExecutionFailed {
            message: "boom".to_string(),
            task_id: "load".to_string(),
            timestamp: chrono::Utc::now(),
        });
        let out = run(ContextSizeLimits::default(), Err(err));
        assert!(
            matches!(out.result.status, ExecutionStatus::Retry),
            "status was {:?}",
            out.result.status
        );
        assert!(size_samples(&out.snapshot).is_empty());
    }

    #[test]
    fn limits_of_zero_accept_a_large_result() {
        let ctx = context(20_000);
        let size = size_of(&ctx);
        let out = run(ContextSizeLimits::OFF, Ok(ctx));
        assert_eq!(out.task.status, "Completed");
        assert_eq!(out.saved_size, Some(Some(size as i64)));
        assert_eq!(
            counters(&out.snapshot, "cloacina_context_size_warnings_total"),
            vec![]
        );
    }

    #[test]
    fn no_retry_policy_allows_a_retry_of_a_context_too_large_error() {
        let error = ExecutorError::ContextTooLarge {
            kind: "result",
            task: TASK.to_string(),
            size_bytes: 20,
            limit_bytes: 10,
            largest_keys: "rows (18 bytes)".to_string(),
        };
        for condition in [
            RetryCondition::AllErrors,
            RetryCondition::TransientOnly,
            RetryCondition::ErrorPattern {
                patterns: vec!["context too large".to_string()],
            },
        ] {
            let policy = RetryPolicy::builder()
                .max_attempts(3)
                .retry_condition(condition.clone())
                .build();
            assert!(
                !TaskResultHandler::retry_allowed(&policy, 1, &error),
                "retried under {condition:?}"
            );
        }
        assert!(!TaskResultHandler::error_is_transient(&error));
        assert_eq!(failure_reason(&error), "context_too_large");
    }
}
