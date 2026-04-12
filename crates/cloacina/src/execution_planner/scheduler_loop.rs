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

//! Scheduler loop and pipeline processing.
//!
//! This module contains the main scheduling loop that continuously processes
//! active pipeline executions and manages task readiness.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::time;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::dal::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::dispatcher::{Dispatcher, TaskReadyEvent};
use crate::error::ValidationError;
use crate::models::pipeline_execution::WorkflowExecutionRecord;
use crate::models::task_execution::TaskExecution;
use crate::Runtime;

use super::state_manager::StateManager;

/// Maximum backoff interval during sustained errors (30 seconds).
const MAX_BACKOFF: Duration = Duration::from_secs(30);

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
}

impl<'a> SchedulerLoop<'a> {
    /// Creates a new SchedulerLoop.
    #[allow(dead_code)]
    pub fn new(
        dal: &'a DAL,
        runtime: Arc<Runtime>,
        instance_id: Uuid,
        poll_interval: Duration,
    ) -> Self {
        Self {
            dal,
            runtime,
            instance_id,
            poll_interval,
            dispatcher: None,
            shutdown_rx: None,
            consecutive_errors: 0,
        }
    }

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
        }
    }

    /// Set the shutdown receiver for graceful termination.
    pub fn with_shutdown(mut self, shutdown_rx: tokio::sync::watch::Receiver<bool>) -> Self {
        self.shutdown_rx = Some(shutdown_rx);
        self
    }

    /// Runs the main scheduling loop that continuously processes active pipeline executions.
    ///
    /// This loop:
    /// 1. Checks for active pipeline executions
    /// 2. Updates task readiness based on dependencies and trigger rules
    /// 3. Marks completed pipelines
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

            match self.process_active_pipelines().await {
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
                    } else if self.consecutive_errors % 10 == 0 {
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

    /// Processes all active pipeline executions to update task readiness.
    pub async fn process_active_pipelines(&self) -> Result<(), ValidationError> {
        let active_executions = self
            .dal
            .workflow_execution()
            .get_active_executions()
            .await?;

        if active_executions.is_empty() {
            // Even with no active pipelines, dispatch any Ready tasks (e.g., retries)
            if self.dispatcher.is_some() {
                self.dispatch_ready_tasks().await?;
            }
            return Ok(());
        }

        // Batch process all active pipelines (evaluates pending tasks, marks them Ready)
        self.process_pipelines_batch(active_executions).await?;

        // Dispatch all Ready tasks (including newly marked and retry tasks)
        if self.dispatcher.is_some() {
            self.dispatch_ready_tasks().await?;
        }

        Ok(())
    }

    /// Processes multiple pipelines in batch for better performance.
    ///
    /// This method optimizes pipeline processing by:
    /// 1. Loading all pending tasks across all pipelines in one query
    /// 2. Grouping tasks by pipeline for processing
    /// 3. Batch checking pipeline completion
    async fn process_pipelines_batch(
        &self,
        active_executions: Vec<WorkflowExecutionRecord>,
    ) -> Result<(), ValidationError> {
        let pipeline_ids: Vec<UniversalUuid> = active_executions.iter().map(|e| e.id).collect();

        // Batch load all pending tasks across all active pipelines
        let all_pending_tasks = self
            .dal
            .task_execution()
            .get_pending_tasks_batch(pipeline_ids)
            .await?;

        // Group tasks by pipeline ID for processing
        let mut tasks_by_pipeline: HashMap<UniversalUuid, Vec<TaskExecution>> = HashMap::new();
        for task in all_pending_tasks {
            tasks_by_pipeline
                .entry(task.pipeline_execution_id)
                .or_default()
                .push(task);
        }

        let state_manager = StateManager::new(self.dal, self.runtime.clone());

        // Process each pipeline's tasks
        for execution in &active_executions {
            if let Some(pipeline_tasks) = tasks_by_pipeline.get(&execution.id) {
                if let Err(e) = state_manager
                    .update_pipeline_task_readiness(execution.id, pipeline_tasks)
                    .await
                {
                    error!(
                        "Failed to update task readiness for pipeline {}: {}",
                        execution.id, e
                    );
                    continue;
                }
            }

            // Check if pipeline is complete
            if self
                .dal
                .task_execution()
                .check_pipeline_completion(execution.id)
                .await?
            {
                self.complete_pipeline(execution).await?;
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
            Some(d) => d,
            None => return Ok(()),
        };

        // Get all Ready tasks where retry_at has passed (or is null)
        let ready_tasks = self.dal.task_execution().get_ready_for_retry().await?;

        for task in ready_tasks {
            let event = TaskReadyEvent::new(
                task.id,
                task.pipeline_execution_id,
                task.task_name.clone(),
                task.attempt,
            );

            if let Err(e) = dispatcher.dispatch(event).await {
                warn!(
                    task_id = %task.id,
                    task_name = %task.task_name,
                    error = %e,
                    "Failed to dispatch ready task"
                );
            }
        }

        Ok(())
    }

    /// Completes a pipeline by updating its final context and marking it as completed.
    ///
    /// Guards against the race where two scheduler ticks both see the pipeline
    /// as complete and both try to finalise it. Only the first caller (whose
    /// pipeline is still "Running") will proceed; subsequent calls return early.
    async fn complete_pipeline(
        &self,
        execution: &WorkflowExecutionRecord,
    ) -> Result<(), ValidationError> {
        // Guard: only proceed if the pipeline is still running.
        // This prevents duplicate PipelineCompleted events when two scheduler
        // ticks race through check_pipeline_completion concurrently.
        let current = self
            .dal
            .workflow_execution()
            .get_by_id(execution.id)
            .await?;
        if current.status == "Completed" || current.status == "Failed" {
            debug!(
                "Pipeline {} already in status '{}', skipping completion",
                execution.id, current.status
            );
            return Ok(());
        }

        // Get task summary for logging
        let all_tasks = self
            .dal
            .task_execution()
            .get_all_tasks_for_pipeline(execution.id)
            .await?;
        let completed_count = all_tasks.iter().filter(|t| t.status == "Completed").count();
        let failed_count = all_tasks.iter().filter(|t| t.status == "Failed").count();
        let skipped_count = all_tasks.iter().filter(|t| t.status == "Skipped").count();

        // Update the pipeline's final context before marking complete
        if let Err(e) = self
            .update_pipeline_final_context(execution.id, &all_tasks)
            .await
        {
            warn!(
                "Failed to update final context for pipeline {}: {}",
                execution.id, e
            );
        }

        // Determine final pipeline status based on task outcomes
        if failed_count > 0 {
            let reason = format!(
                "{} task(s) failed, {} completed, {} skipped",
                failed_count, completed_count, skipped_count
            );
            self.dal
                .workflow_execution()
                .mark_failed(execution.id, &reason)
                .await?;
            metrics::counter!("cloacina_pipelines_total", "status" => "failed").increment(1);
            info!(
                "Pipeline failed: {} (name: {}, {})",
                execution.id, execution.pipeline_name, reason
            );
        } else {
            self.dal
                .workflow_execution()
                .mark_completed(execution.id)
                .await?;
            metrics::counter!("cloacina_pipelines_total", "status" => "completed").increment(1);
            info!(
                "Pipeline completed: {} (name: {}, {} completed, {} skipped)",
                execution.id, execution.pipeline_name, completed_count, skipped_count
            );
        }

        Ok(())
    }

    /// Updates the pipeline's final context when it completes.
    ///
    /// This method finds the context from the final task(s) that produced output
    /// and updates the pipeline's context_id to point to that final context,
    /// ensuring that WorkflowExecutionResult.final_context returns the correct data.
    async fn update_pipeline_final_context(
        &self,
        pipeline_execution_id: UniversalUuid,
        all_tasks: &[TaskExecution],
    ) -> Result<(), ValidationError> {
        // Find the final context by looking at the last task that completed with context
        // Priority order: Completed > Skipped, then by completion time (latest first)
        let mut final_context_id: Option<UniversalUuid> = None;
        let mut latest_completion_time: Option<chrono::DateTime<chrono::Utc>> = None;

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
                        .get_by_pipeline_and_task(pipeline_execution_id, &task_namespace)
                        .await
                    {
                        if let Some(context_id) = task_metadata.context_id {
                            // Use this context if it's the latest completion time or we haven't found one yet
                            if latest_completion_time.is_none()
                                || completed_at.0 > latest_completion_time.unwrap()
                            {
                                final_context_id = Some(context_id);
                                latest_completion_time = Some(completed_at.0);
                            }
                        }
                    }
                }
            }
        }

        // Update the pipeline's context_id if we found a final context
        if let Some(context_id) = final_context_id {
            debug!(
                "Updating pipeline {} final context to context_id: {}",
                pipeline_execution_id, context_id
            );
            self.dal
                .workflow_execution()
                .update_final_context(pipeline_execution_id, context_id)
                .await?;
        } else {
            debug!(
                "No final context found for pipeline {} - keeping initial context",
                pipeline_execution_id
            );
        }

        Ok(())
    }
}
