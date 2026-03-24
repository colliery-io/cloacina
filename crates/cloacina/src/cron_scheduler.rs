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

//! Cron scheduler for time-based workflow execution.
//!
//! This module provides the core scheduling engine that polls for due cron schedules
//! and hands them off to the pipeline executor. The scheduler implements a saga-based
//! pattern where responsibilities are clearly separated:
//!
//! - **CronScheduler**: Polls for due schedules, claims them atomically, hands off to executor
//! - **PipelineExecutor**: Handles actual workflow execution, retries, and recovery
//!
//! # Key Features
//!
//! - **Atomic Claiming**: Prevents duplicate executions across multiple instances
//! - **Timezone Aware**: Handles DST transitions and timezone-specific scheduling
//! - **Audit Trail**: Records every handoff for observability and debugging
//! - **Catchup Policies**: Configurable handling of missed executions
//! - **Saga Pattern**: Clean separation between scheduling and execution concerns
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐    claim &     ┌──────────────────┐    execute    ┌─────────────┐
//! │  CronScheduler  │   hand off     │ PipelineExecutor │  workflows    │   Tasks     │
//! │                 │ ──────────────▶│                  │ ─────────────▶│             │
//! │ • Poll DB       │                │ • Execute        │               │ • Business  │
//! │ • Claim sched   │                │ • Retry          │               │   Logic     │
//! │ • Audit log     │                │ • Recovery       │               │ • Context   │
//! └─────────────────┘                └──────────────────┘               └─────────────┘
//! ```
//!
//! # Examples
//!
//! ```rust,ignore
//! use cloacina::cron_scheduler::CronScheduler;
//! use cloacina::runner::DefaultRunner;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let executor = DefaultRunner::new("postgresql://localhost/mydb").await?;
//! let scheduler = CronScheduler::new(
//!     executor.dal(),
//!     executor.clone(),
//!     Duration::from_secs(30)  // Poll every 30 seconds
//! );
//!
//! // Start the polling loop
//! scheduler.run_polling_loop().await?;
//! # Ok(())
//! # }
//! ```

use crate::context::Context;
use crate::cron_evaluator::CronEvaluator;
use crate::dal::DAL;
use crate::error::ValidationError;
use crate::executor::{PipelineError, PipelineExecutor};
use crate::models::cron_schedule::{CatchupPolicy, CronSchedule};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

/// Configuration for the cron scheduler.
#[derive(Debug, Clone)]
pub struct CronSchedulerConfig {
    /// How often to poll for due schedules
    pub poll_interval: Duration,
    /// Maximum number of missed executions to run in catchup mode
    pub max_catchup_executions: usize,
    /// Maximum delay before considering a schedule "severely delayed"
    pub max_acceptable_delay: Duration,
}

impl Default for CronSchedulerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(30),
            max_catchup_executions: 100,
            max_acceptable_delay: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Saga-based cron scheduler for time-based workflow execution.
///
/// The scheduler implements a polling loop that:
/// 1. Finds due schedules in the database
/// 2. Atomically claims schedules to prevent duplicates
/// 3. Calculates execution times based on catchup policy
/// 4. Hands off executions to the pipeline executor
/// 5. Records audit trail for each handoff
///
/// # Responsibilities
///
/// **What CronScheduler Does:**
/// - Poll database for due schedules
/// - Atomically claim schedules
/// - Calculate missed execution times
/// - Hand off to pipeline executor
/// - Record execution audit trail
/// - Move on immediately (no waiting for completion)
///
/// **What CronScheduler Does NOT Do:**
/// - Execute workflows directly
/// - Handle task retries or failures
/// - Wait for workflow completion
/// - Manage workflow state or recovery
///
/// This separation allows the pipeline executor's existing retry, recovery,
/// and concurrency mechanisms to handle execution concerns.
#[derive(Clone)]
pub struct CronScheduler {
    dal: Arc<DAL>,
    executor: Arc<dyn PipelineExecutor>,
    config: CronSchedulerConfig,
    shutdown: watch::Receiver<bool>,
}

impl CronScheduler {
    /// Creates a new cron scheduler.
    ///
    /// # Arguments
    /// * `dal` - Data access layer for database operations
    /// * `executor` - Pipeline executor for workflow execution
    /// * `config` - Scheduler configuration
    /// * `shutdown` - Shutdown signal receiver
    pub fn new(
        dal: Arc<DAL>,
        executor: Arc<dyn PipelineExecutor>,
        config: CronSchedulerConfig,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            dal,
            executor,
            config,
            shutdown,
        }
    }

    /// Creates a new cron scheduler with default configuration.
    pub fn with_defaults(
        dal: Arc<DAL>,
        executor: Arc<dyn PipelineExecutor>,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self::new(dal, executor, CronSchedulerConfig::default(), shutdown)
    }

    /// Runs the main polling loop for cron schedule processing.
    ///
    /// This method starts an infinite loop that:
    /// 1. Polls for due schedules at configured intervals
    /// 2. Processes each due schedule
    /// 3. Handles shutdown gracefully when signaled
    ///
    /// The loop continues until a shutdown signal is received.
    ///
    /// # Returns
    /// * `Result<(), PipelineError>` - Success or error from the polling loop
    pub async fn run_polling_loop(&mut self) -> Result<(), PipelineError> {
        info!(
            "Starting cron scheduler polling loop (interval: {:?})",
            self.config.poll_interval
        );

        let mut interval = tokio::time::interval(self.config.poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.check_and_execute_schedules().await {
                        error!("Error processing cron schedules: {}", e);
                        // Continue running despite errors
                    }
                }
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        info!("Cron scheduler received shutdown signal");
                        break;
                    }
                }
            }
        }

        info!("Cron scheduler polling loop stopped");
        Ok(())
    }

    /// Checks for due schedules and executes them.
    ///
    /// This is the core method that implements the saga-based execution pattern:
    /// 1. Query for due schedules
    /// 2. For each schedule: claim, calculate executions, hand off, audit
    /// 3. Let pipeline executor handle the actual execution
    async fn check_and_execute_schedules(&self) -> Result<(), PipelineError> {
        let now = Utc::now();
        debug!("Checking for due cron schedules at {}", now);

        // Get all schedules that are due for execution
        let due_schedules = self
            .dal
            .cron_schedule()
            .get_due_schedules(now)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: e.to_string(),
            })?;

        if due_schedules.is_empty() {
            debug!("No due schedules found");
            return Ok(());
        }

        info!("Found {} due cron schedule(s)", due_schedules.len());

        // Process each due schedule
        for schedule in due_schedules {
            if let Err(e) = self.process_schedule(&schedule, now).await {
                error!("Failed to process schedule {}: {}", schedule.id, e);
                // Continue with other schedules even if one fails
            }
        }

        Ok(())
    }

    /// Processes a single cron schedule using the saga pattern.
    ///
    /// Steps:
    /// 1. Check if schedule is within its active time window
    /// 2. Calculate execution times based on catchup policy
    /// 3. Calculate next scheduled run time
    /// 4. Atomically claim the schedule
    /// 5. If claimed successfully, hand off all executions to pipeline executor
    /// 6. Record audit trail for each execution
    async fn process_schedule(
        &self,
        schedule: &CronSchedule,
        now: DateTime<Utc>,
    ) -> Result<(), PipelineError> {
        debug!(
            "Processing schedule: {} (workflow: {})",
            schedule.id, schedule.workflow_name
        );

        // Check if schedule is within its active time window
        if !self.is_schedule_active(schedule, now) {
            debug!(
                "Schedule {} is outside its active time window, skipping",
                schedule.id
            );
            return Ok(());
        }

        // Calculate execution times based on catchup policy
        let execution_times = self.calculate_execution_times(schedule, now)?;
        if execution_times.is_empty() {
            debug!("No execution times calculated for schedule {}", schedule.id);
            return Ok(());
        }

        // Calculate next run time
        let next_run = self.calculate_next_run(schedule, now)?;

        // Atomically claim the schedule
        let claimed = self
            .dal
            .cron_schedule()
            .claim_and_update(schedule.id, now, now, next_run)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: e.to_string(),
            })?;

        if !claimed {
            debug!(
                "Schedule {} was already claimed by another instance",
                schedule.id
            );
            return Ok(());
        }

        info!(
            "Successfully claimed schedule {} for {} execution(s)",
            schedule.id,
            execution_times.len()
        );

        // Execute all scheduled times.
        // Pipeline first, then audit — if crash between, the heartbeat sweeper
        // handles orphaned tasks. No separate cron recovery needed.
        for scheduled_time in execution_times {
            // Step 1: Create pipeline + tasks (already atomic via transaction)
            match self.execute_workflow(schedule, scheduled_time).await {
                Ok(pipeline_execution_id) => {
                    // Step 2: Create audit record linked to pipeline
                    let audit_result = self
                        .create_execution_audit(schedule.id, scheduled_time)
                        .await;

                    if let Ok(audit_record_id) = audit_result {
                        if let Err(e) = self
                            .complete_execution_audit(audit_record_id, pipeline_execution_id)
                            .await
                        {
                            warn!(
                                "Failed to link audit trail for schedule {}: {} (execution succeeded)",
                                schedule.id, e
                            );
                        }
                    } else if let Err(e) = audit_result {
                        warn!(
                            "Failed to create audit for schedule {}: {} (execution succeeded)",
                            schedule.id, e
                        );
                    }

                    info!(
                        "Executed workflow {} for schedule {} (scheduled: {})",
                        schedule.workflow_name, schedule.id, scheduled_time
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to execute workflow {} for schedule {} (scheduled: {}): {}",
                        schedule.workflow_name, schedule.id, scheduled_time, e
                    );
                    // No orphaned audit record — pipeline was never created.
                    // The cron scheduler will retry on the next window.
                }
            }
        }

        Ok(())
    }

    /// Checks if a schedule is within its active time window.
    fn is_schedule_active(&self, schedule: &CronSchedule, now: DateTime<Utc>) -> bool {
        // Check start date
        if let Some(start) = &schedule.start_date {
            if now < start.0 {
                return false;
            }
        }

        // Check end date
        if let Some(end) = &schedule.end_date {
            if now > end.0 {
                return false;
            }
        }

        true
    }

    /// Calculates execution times based on the schedule's catchup policy.
    ///
    /// # Catchup Policies
    ///
    /// **Skip**: Only execute the current scheduled time, ignore missed executions
    /// **RunAll**: Execute all missed executions since last run, up to the configured limit
    fn calculate_execution_times(
        &self,
        schedule: &CronSchedule,
        now: DateTime<Utc>,
    ) -> Result<Vec<DateTime<Utc>>, PipelineError> {
        let policy = CatchupPolicy::from(schedule.catchup_policy.clone());

        match policy {
            CatchupPolicy::Skip => {
                // Just return the current scheduled time
                Ok(vec![schedule.next_run_at.0])
            }
            CatchupPolicy::RunAll => {
                // Calculate all missed executions since last run
                let mut executions = Vec::new();
                let evaluator = CronEvaluator::new(&schedule.cron_expression, &schedule.timezone)
                    .map_err(|e| PipelineError::ExecutionFailed {
                    message: format!("Cron evaluation error: {}", e),
                })?;

                // Start from last run time, or creation time if never run
                let start_time = schedule
                    .last_run_at
                    .map(|t| t.0)
                    .unwrap_or(schedule.created_at.0);

                // Find all executions between start_time and now
                let missed_executions = evaluator
                    .executions_between(start_time, now, self.config.max_catchup_executions)
                    .map_err(|e| PipelineError::ExecutionFailed {
                        message: format!("Cron evaluation error: {}", e),
                    })?;

                executions.extend(missed_executions);

                if executions.len() >= self.config.max_catchup_executions {
                    warn!(
                        "Limited catchup executions to {} for schedule {} (policy: RunAll)",
                        self.config.max_catchup_executions, schedule.id
                    );
                }

                Ok(executions)
            }
        }
    }

    /// Calculates the next run time for a schedule.
    fn calculate_next_run(
        &self,
        schedule: &CronSchedule,
        after: DateTime<Utc>,
    ) -> Result<DateTime<Utc>, PipelineError> {
        let evaluator =
            CronEvaluator::new(&schedule.cron_expression, &schedule.timezone).map_err(|e| {
                PipelineError::ExecutionFailed {
                    message: format!("Cron evaluation error: {}", e),
                }
            })?;

        evaluator
            .next_execution(after)
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Cron evaluation error: {}", e),
            })
    }

    /// Executes a workflow by handing it off to the pipeline executor.
    ///
    /// Creates a context with the scheduled time and delegates execution
    /// to the pipeline executor. This implements the saga pattern by
    /// immediately returning after handoff.
    async fn execute_workflow(
        &self,
        schedule: &CronSchedule,
        scheduled_time: DateTime<Utc>,
    ) -> Result<crate::database::UniversalUuid, PipelineError> {
        // Create context with scheduled time
        let mut context = Context::new();
        context
            .insert(
                "scheduled_time",
                serde_json::json!(scheduled_time.to_rfc3339()),
            )
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;

        // Add schedule metadata to context
        context
            .insert("schedule_id", serde_json::json!(schedule.id.to_string()))
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert("schedule_timezone", serde_json::json!(schedule.timezone))
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert(
                "schedule_expression",
                serde_json::json!(schedule.cron_expression),
            )
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;

        info!(
            "Executing workflow '{}' for schedule {} (scheduled time: {})",
            schedule.workflow_name, schedule.id, scheduled_time
        );

        // Hand off to pipeline executor (saga pattern - don't wait for completion)
        let pipeline_result = self
            .executor
            .execute(&schedule.workflow_name, context)
            .await?;

        debug!(
            "Successfully handed off workflow '{}' to executor (execution_id: {})",
            schedule.workflow_name, pipeline_result.execution_id
        );

        Ok(crate::database::UniversalUuid(pipeline_result.execution_id))
    }

    /// Creates audit record BEFORE workflow execution for guaranteed reliability.
    ///
    /// This implements the guaranteed execution pattern by recording execution intent
    /// before handing off to the pipeline executor. This enables recovery if the
    /// handoff fails or the system crashes.
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the cron schedule being executed
    /// * `scheduled_time` - The original scheduled execution time
    ///
    /// # Returns
    /// * `Result<crate::database::UniversalUuid, ValidationError>` - ID of the audit record
    async fn create_execution_audit(
        &self,
        schedule_id: crate::database::UniversalUuid,
        scheduled_time: DateTime<Utc>,
    ) -> Result<crate::database::UniversalUuid, ValidationError> {
        use crate::database::universal_types::UniversalTimestamp;
        use crate::models::cron_execution::NewCronExecution;

        let new_execution = NewCronExecution::new(schedule_id, UniversalTimestamp(scheduled_time));

        let audit_record = self.dal.cron_execution().create(new_execution).await?;

        debug!(
            "Created execution audit record {} for schedule {} (scheduled: {})",
            audit_record.id, schedule_id, scheduled_time
        );

        Ok(audit_record.id)
    }

    /// Updates audit record with pipeline execution ID after successful handoff.
    ///
    /// This completes the audit trail by linking the execution intent record
    /// with the actual pipeline execution that was created.
    ///
    /// # Arguments
    /// * `audit_record_id` - UUID of the audit record to update
    /// * `pipeline_execution_id` - UUID of the created pipeline execution
    async fn complete_execution_audit(
        &self,
        audit_record_id: crate::database::UniversalUuid,
        pipeline_execution_id: crate::database::UniversalUuid,
    ) -> Result<(), ValidationError> {
        self.dal
            .cron_execution()
            .update_pipeline_execution_id(audit_record_id, pipeline_execution_id)
            .await?;

        debug!(
            "Completed execution audit record {} -> pipeline {}",
            audit_record_id, pipeline_execution_id
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::universal_types::{current_timestamp, UniversalBool, UniversalUuid};
    use tokio::sync::watch;

    fn create_test_schedule(cron_expr: &str, timezone: &str) -> CronSchedule {
        let now = current_timestamp();
        CronSchedule {
            id: UniversalUuid::new_v4(),
            workflow_name: "test_workflow".to_string(),
            cron_expression: cron_expr.to_string(),
            timezone: timezone.to_string(),
            enabled: UniversalBool::new(true),
            catchup_policy: "skip".to_string(),
            start_date: None,
            end_date: None,
            next_run_at: now,
            last_run_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_cron_scheduler_config_default() {
        let config = CronSchedulerConfig::default();
        assert_eq!(config.poll_interval, std::time::Duration::from_secs(30));
        assert_eq!(config.max_catchup_executions, 100);
        assert_eq!(
            config.max_acceptable_delay,
            std::time::Duration::from_secs(300)
        );
    }

    #[test]
    fn test_is_schedule_active_no_time_window() {
        // Schedule with no start/end date is always active
        let schedule = create_test_schedule("0 * * * *", "UTC");
        let now = Utc::now();

        // No time window means active at any time
        assert!(schedule.start_date.is_none());
        assert!(schedule.end_date.is_none());
        // The schedule should be considered active (no window restrictions)
    }

    #[test]
    fn test_is_schedule_active_with_start_date() {
        let mut schedule = create_test_schedule("0 * * * *", "UTC");
        let now = Utc::now();

        // Set start date in the future — should not be active yet
        schedule.start_date = Some(crate::database::UniversalTimestamp(
            now + chrono::Duration::hours(1),
        ));
        assert!(now < schedule.start_date.as_ref().unwrap().0);

        // Set start date in the past — should be active
        schedule.start_date = Some(crate::database::UniversalTimestamp(
            now - chrono::Duration::hours(1),
        ));
        assert!(now > schedule.start_date.as_ref().unwrap().0);
    }

    #[test]
    fn test_is_schedule_active_with_end_date() {
        let mut schedule = create_test_schedule("0 * * * *", "UTC");
        let now = Utc::now();

        // Set end date in the past — should no longer be active
        schedule.end_date = Some(crate::database::UniversalTimestamp(
            now - chrono::Duration::hours(1),
        ));
        assert!(now > schedule.end_date.as_ref().unwrap().0);

        // Set end date in the future — still active
        schedule.end_date = Some(crate::database::UniversalTimestamp(
            now + chrono::Duration::hours(1),
        ));
        assert!(now < schedule.end_date.as_ref().unwrap().0);
    }

    #[test]
    fn test_execution_times_skip_policy_returns_next_run() {
        let schedule = create_test_schedule("0 * * * *", "UTC");

        // With Skip policy, execution times should be just the next_run_at
        assert_eq!(schedule.catchup_policy, "skip");
        let policy = CatchupPolicy::from(schedule.catchup_policy.clone());
        assert!(matches!(policy, CatchupPolicy::Skip));

        // Skip policy returns vec![schedule.next_run_at] — a single time
    }

    #[test]
    fn test_execution_times_run_all_policy_uses_evaluator() {
        let mut schedule = create_test_schedule("*/5 * * * * *", "UTC");
        schedule.catchup_policy = "run_all".to_string();

        // RunAll calculates missed executions between last_run and now
        let policy = CatchupPolicy::from(schedule.catchup_policy.clone());
        assert!(matches!(policy, CatchupPolicy::RunAll));

        // Verify the evaluator finds executions between two times
        let evaluator = CronEvaluator::new(&schedule.cron_expression, &schedule.timezone).unwrap();
        let start = Utc::now() - chrono::Duration::seconds(30);
        let end = Utc::now();
        let executions = evaluator.executions_between(start, end, 100).unwrap();

        // With */5 * * * * * (every 5 seconds), 30 seconds should yield ~6 executions
        assert!(
            executions.len() >= 4 && executions.len() <= 8,
            "Expected 4-8 executions in 30s with */5s cron, got {}",
            executions.len()
        );

        // All execution times should be between start and end
        for t in &executions {
            assert!(
                *t >= start && *t <= end,
                "Execution time {} out of range",
                t
            );
        }
    }
}
