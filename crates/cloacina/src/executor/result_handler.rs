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

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tracing::{debug, info, warn};

use crate::context::Context;
use crate::dal::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::dispatcher::{ExecutionResult, TaskReadyEvent};
use crate::error::ExecutorError;
use crate::executor::types::ClaimedTask;
use crate::retry::{RetryCondition, RetryPolicy};

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
        }
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
        match outcome {
            Ok(result_context) => {
                match self
                    .complete_task_transaction(claimed_task, result_context)
                    .await
                {
                    Ok(()) => {
                        self.total_executed.fetch_add(1, Ordering::SeqCst);
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
                        let error_msg = format!("Failed to save context: {}", e);
                        // Mark failed in DB — executor owns all state transitions
                        let _ = self
                            .dal
                            .task_execution()
                            .mark_failed(event.task_execution_id, &error_msg, self.runner_id)
                            .await;
                        ExecutionResult::failure(event.task_execution_id, error_msg, duration)
                    }
                }
            }
            Err(error) => {
                let should_retry = self
                    .should_retry_task(claimed_task, &error, retry_policy)
                    .await
                    .unwrap_or(false);

                if should_retry {
                    if let Err(e) = self.schedule_task_retry(claimed_task, retry_policy).await {
                        warn!(
                            task_id = %event.task_execution_id,
                            error = %e,
                            "Failed to schedule retry"
                        );
                    }
                    self.total_executed.fetch_add(1, Ordering::SeqCst);
                    ExecutionResult::retry(event.task_execution_id, error.to_string(), duration)
                } else {
                    self.total_failed.fetch_add(1, Ordering::SeqCst);
                    // Mark failed in DB — executor owns all state transitions
                    let _ = self
                        .dal
                        .task_execution()
                        .mark_failed(event.task_execution_id, &error.to_string(), self.runner_id)
                        .await;
                    ExecutionResult::failure(event.task_execution_id, error.to_string(), duration)
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
    ) -> Result<(), ExecutorError> {
        // 1. Save context FIRST. If this fails, the task is still marked
        //    Running with our claim — the heartbeat-loss / claim-sweep
        //    cycle will reset it to Ready and another attempt drives it
        //    through this path again. No "marked Completed but no
        //    context" state is reachable.
        self.save_task_context(claimed_task, context).await?;

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
    /// task_execution_metadata.
    async fn save_task_context(
        &self,
        claimed_task: &ClaimedTask,
        context: Context<serde_json::Value>,
    ) -> Result<(), ExecutorError> {
        use crate::models::task_execution_metadata::NewTaskExecutionMetadata;

        let context_id = self.dal.context().create(&context).await?;

        let task_metadata_record = NewTaskExecutionMetadata {
            task_execution_id: claimed_task.task_execution_id,
            workflow_execution_id: claimed_task.workflow_execution_id,
            task_name: claimed_task.task_name.clone(),
            context_id,
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

        if claimed_task.attempt >= retry_policy.max_attempts {
            debug!(
                "Task {} exceeded max retry attempts ({}/{})",
                claimed_task.task_name, claimed_task.attempt, retry_policy.max_attempts
            );
            return Ok(false);
        }

        let should_retry = retry_policy
            .retry_conditions
            .iter()
            .all(|condition| match condition {
                RetryCondition::Never => false,
                RetryCondition::AllErrors => true,
                RetryCondition::TransientOnly => self.is_transient_error(error),
                RetryCondition::ErrorPattern { patterns } => {
                    let error_msg = error.to_string().to_lowercase();
                    patterns
                        .iter()
                        .any(|pattern| error_msg.contains(&pattern.to_lowercase()))
                }
            });

        debug!(
            "Retry decision for task {}: {} (conditions: {:?}, error: {})",
            claimed_task.task_name, should_retry, retry_policy.retry_conditions, error
        );

        Ok(should_retry)
    }

    /// Classifies whether an error looks transient enough to retry under
    /// `RetryCondition::TransientOnly`. Public so the unit tests that pin the
    /// classification (moved here from ThreadTaskExecutor in T-0630) can
    /// exercise it directly.
    pub fn is_transient_error(&self, error: &ExecutorError) -> bool {
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
        assert!(handler()
            .is_transient_error(&ExecutorError::ConnectionPool("pool exhausted".to_string())));
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
