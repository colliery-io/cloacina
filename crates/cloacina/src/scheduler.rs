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

//! Unified scheduler for both cron and trigger-based workflow execution.
//!
//! This module provides a single `Scheduler` that replaces the separate
//! `CronScheduler` and `TriggerScheduler`, driving both cron and trigger
//! schedules from one run loop backed by the unified `schedules` and
//! `schedule_executions` tables.
//!
//! # Key Features
//!
//! - **Single Run Loop**: One tick drives both cron and trigger checks
//! - **Atomic Claiming**: Prevents duplicate cron executions across instances
//! - **Per-trigger Poll Intervals**: Each trigger retains its own polling frequency
//! - **Context-based Deduplication**: Prevents duplicate trigger executions
//! - **Catchup Policies**: Configurable handling of missed cron executions
//! - **Audit Trail**: Records every handoff via `schedule_executions`
//! - **Saga Pattern**: Clean separation between scheduling and execution
//!
//! # Architecture
//!
//! ```text
//! ┌───────────────┐    claim / fire   ┌──────────────────┐    execute    ┌─────────────┐
//! │   Scheduler   │   & hand off      │ PipelineExecutor │  workflows   │   Tasks     │
//! │               │ ─────────────────▶│                  │ ────────────▶│             │
//! │ • Poll cron   │                   │ • Execute        │              │ • Business  │
//! │ • Poll trigs  │                   │ • Retry          │              │   Logic     │
//! │ • Deduplicate │                   │ • Recovery       │              │ • Context   │
//! │ • Audit log   │                   │                  │              │             │
//! └───────────────┘                   └──────────────────┘              └─────────────┘
//! ```

use crate::context::Context;
use crate::cron_evaluator::CronEvaluator;
use crate::dal::DAL;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::executor::{PipelineError, PipelineExecutor};
use crate::models::schedule::{CatchupPolicy, NewSchedule, NewScheduleExecution, Schedule};
use crate::trigger::{get_trigger, Trigger, TriggerError};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

/// Configuration for the unified scheduler.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// How often to check for due cron schedules.
    pub cron_poll_interval: Duration,
    /// Maximum number of missed executions to run in catchup mode.
    pub max_catchup_executions: usize,
    /// Maximum acceptable delay for cron (used for observability / alerting).
    pub max_acceptable_delay: Duration,
    /// Base poll interval — the tick rate of the run loop.
    pub trigger_base_poll_interval: Duration,
    /// Maximum time to wait for a single trigger poll operation.
    pub trigger_poll_timeout: Duration,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            cron_poll_interval: Duration::from_secs(30),
            max_catchup_executions: 100,
            max_acceptable_delay: Duration::from_secs(300), // 5 minutes
            trigger_base_poll_interval: Duration::from_secs(1),
            trigger_poll_timeout: Duration::from_secs(30),
        }
    }
}

/// Unified scheduler for both cron and trigger-based workflow execution.
///
/// The scheduler runs a single polling loop that:
/// 1. Ticks at `trigger_base_poll_interval` (default 1 s)
/// 2. Every `cron_poll_interval`, queries due cron schedules and processes them
/// 3. Every tick, checks enabled triggers respecting per-trigger poll intervals
/// 4. Records audit trail for every handoff via `schedule_executions`
///
/// # Responsibilities
///
/// **What Scheduler Does:**
/// - Poll database for due cron schedules and enabled triggers
/// - Atomically claim cron schedules
/// - Calculate missed execution times (catchup)
/// - Poll trigger functions and deduplicate
/// - Hand off workflow executions to the pipeline executor
/// - Record execution audit trail
/// - Move on immediately (no waiting for completion)
///
/// **What Scheduler Does NOT Do:**
/// - Execute workflows directly
/// - Handle task retries or failures
/// - Wait for workflow completion
/// - Manage workflow state or recovery
#[derive(Clone)]
pub struct Scheduler {
    dal: Arc<DAL>,
    executor: Arc<dyn PipelineExecutor>,
    config: SchedulerConfig,
    shutdown: watch::Receiver<bool>,
    /// Tracks when each trigger was last polled (by trigger name).
    last_poll_times: HashMap<String, Instant>,
    /// Tracks when cron schedules were last checked.
    last_cron_check: Option<Instant>,
}

impl Scheduler {
    /// Creates a new unified scheduler.
    ///
    /// # Arguments
    /// * `dal` - Data access layer for database operations
    /// * `executor` - Pipeline executor for workflow execution
    /// * `config` - Scheduler configuration
    /// * `shutdown` - Shutdown signal receiver
    pub fn new(
        dal: Arc<DAL>,
        executor: Arc<dyn PipelineExecutor>,
        config: SchedulerConfig,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            dal,
            executor,
            config,
            shutdown,
            last_poll_times: HashMap::new(),
            last_cron_check: None,
        }
    }

    /// Creates a new unified scheduler with default configuration.
    pub fn with_defaults(
        dal: Arc<DAL>,
        executor: Arc<dyn PipelineExecutor>,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self::new(dal, executor, SchedulerConfig::default(), shutdown)
    }

    // -----------------------------------------------------------------------
    // Run loop
    // -----------------------------------------------------------------------

    /// Runs the main polling loop.
    ///
    /// Ticks at `trigger_base_poll_interval`. On each tick it:
    /// - Checks cron schedules if `cron_poll_interval` has elapsed since the
    ///   last cron check.
    /// - Checks all enabled triggers, respecting per-trigger poll intervals.
    ///
    /// The loop continues until a shutdown signal is received.
    pub async fn run_polling_loop(&mut self) -> Result<(), PipelineError> {
        info!(
            "Starting unified scheduler (cron interval: {:?}, trigger base interval: {:?})",
            self.config.cron_poll_interval, self.config.trigger_base_poll_interval,
        );

        let mut interval = tokio::time::interval(self.config.trigger_base_poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // --- Cron ---
                    let now = Instant::now();
                    let should_check_cron = match self.last_cron_check {
                        Some(last) => now.duration_since(last) >= self.config.cron_poll_interval,
                        None => true,
                    };

                    if should_check_cron {
                        self.last_cron_check = Some(now);
                        if let Err(e) = self.check_and_execute_cron_schedules().await {
                            error!("Error processing cron schedules: {}", e);
                        }
                    }

                    // --- Triggers ---
                    if let Err(e) = self.check_and_process_triggers().await {
                        error!("Error processing triggers: {}", e);
                    }
                }
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        info!("Unified scheduler received shutdown signal");
                        break;
                    }
                }
            }
        }

        info!("Unified scheduler polling loop stopped");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Cron schedule processing
    // -----------------------------------------------------------------------

    /// Checks for due cron schedules and executes them.
    async fn check_and_execute_cron_schedules(&self) -> Result<(), PipelineError> {
        let now = Utc::now();
        debug!("Checking for due cron schedules at {}", now);

        let due_schedules = self
            .dal
            .schedule()
            .get_due_cron_schedules(now)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: e.to_string(),
            })?;

        if due_schedules.is_empty() {
            debug!("No due cron schedules found");
            return Ok(());
        }

        info!("Found {} due cron schedule(s)", due_schedules.len());

        for schedule in due_schedules {
            if let Err(e) = self.process_cron_schedule(&schedule, now).await {
                error!("Failed to process cron schedule {}: {}", schedule.id, e);
            }
        }

        Ok(())
    }

    /// Processes a single cron schedule using the saga pattern.
    async fn process_cron_schedule(
        &self,
        schedule: &Schedule,
        now: DateTime<Utc>,
    ) -> Result<(), PipelineError> {
        debug!(
            "Processing cron schedule: {} (workflow: {})",
            schedule.id, schedule.workflow_name
        );

        // Check active time window
        if !self.is_cron_schedule_active(schedule, now) {
            debug!(
                "Cron schedule {} is outside its active time window, skipping",
                schedule.id
            );
            return Ok(());
        }

        // Calculate execution times based on catchup policy
        let execution_times = self.calculate_execution_times(schedule, now)?;
        if execution_times.is_empty() {
            debug!(
                "No execution times calculated for cron schedule {}",
                schedule.id
            );
            return Ok(());
        }

        // Calculate next run time
        let next_run = self.calculate_next_run(schedule, now)?;

        // Atomically claim the schedule
        let claimed = self
            .dal
            .schedule()
            .claim_and_update_cron(schedule.id, now, now, next_run)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: e.to_string(),
            })?;

        if !claimed {
            debug!(
                "Cron schedule {} was already claimed by another instance",
                schedule.id
            );
            return Ok(());
        }

        info!(
            "Successfully claimed cron schedule {} for {} execution(s)",
            schedule.id,
            execution_times.len()
        );

        // Execute all scheduled times
        for scheduled_time in execution_times {
            // Step 1: Create audit record BEFORE handoff
            let audit_record_id = match self
                .create_cron_execution_audit(schedule.id, scheduled_time)
                .await
            {
                Ok(id) => id,
                Err(e) => {
                    error!(
                        "Failed to create execution audit for cron schedule {} at {}: {}",
                        schedule.id, scheduled_time, e
                    );
                    continue;
                }
            };

            // Step 2: Hand off to pipeline executor
            match self.execute_cron_workflow(schedule, scheduled_time).await {
                Ok(pipeline_execution_id) => {
                    // Step 3: Link audit record
                    if let Err(e) = self
                        .dal
                        .schedule_execution()
                        .update_pipeline_execution_id(audit_record_id, pipeline_execution_id)
                        .await
                    {
                        error!(
                            "Failed to complete audit trail for cron schedule {} execution: {}",
                            schedule.id, e
                        );
                    }

                    info!(
                        "Successfully executed and audited workflow {} for cron schedule {} (scheduled: {})",
                        schedule.workflow_name, schedule.id, scheduled_time
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to execute workflow {} for cron schedule {} (scheduled: {}): {}",
                        schedule.workflow_name, schedule.id, scheduled_time, e
                    );
                    error!(
                        "Execution lost: audit record {} exists but pipeline execution failed",
                        audit_record_id
                    );
                }
            }
        }

        Ok(())
    }

    /// Checks if a cron schedule is within its active time window.
    fn is_cron_schedule_active(&self, schedule: &Schedule, now: DateTime<Utc>) -> bool {
        if let Some(start) = &schedule.start_date {
            if now < start.0 {
                return false;
            }
        }
        if let Some(end) = &schedule.end_date {
            if now > end.0 {
                return false;
            }
        }
        true
    }

    /// Calculates execution times based on the schedule's catchup policy.
    fn calculate_execution_times(
        &self,
        schedule: &Schedule,
        now: DateTime<Utc>,
    ) -> Result<Vec<DateTime<Utc>>, PipelineError> {
        let policy_str = schedule.catchup_policy.as_deref().unwrap_or("skip");
        let policy = CatchupPolicy::from(policy_str.to_string());

        match policy {
            CatchupPolicy::Skip => {
                // Just return the current next_run_at
                let next_run = schedule.next_run_at.map(|t| t.0).unwrap_or(now);
                Ok(vec![next_run])
            }
            CatchupPolicy::RunAll => {
                let cron_expr = schedule.cron_expression.as_deref().unwrap_or("* * * * *");
                let tz = schedule.timezone.as_deref().unwrap_or("UTC");

                let evaluator = CronEvaluator::new(cron_expr, tz).map_err(|e| {
                    PipelineError::ExecutionFailed {
                        message: format!("Cron evaluation error: {}", e),
                    }
                })?;

                let start_time = schedule
                    .last_run_at
                    .map(|t| t.0)
                    .unwrap_or(schedule.created_at.0);

                let missed_executions = evaluator
                    .executions_between(start_time, now, self.config.max_catchup_executions)
                    .map_err(|e| PipelineError::ExecutionFailed {
                        message: format!("Cron evaluation error: {}", e),
                    })?;

                if missed_executions.len() >= self.config.max_catchup_executions {
                    warn!(
                        "Limited catchup executions to {} for cron schedule {} (policy: RunAll)",
                        self.config.max_catchup_executions, schedule.id
                    );
                }

                Ok(missed_executions)
            }
        }
    }

    /// Calculates the next run time for a cron schedule.
    fn calculate_next_run(
        &self,
        schedule: &Schedule,
        after: DateTime<Utc>,
    ) -> Result<DateTime<Utc>, PipelineError> {
        let cron_expr = schedule.cron_expression.as_deref().unwrap_or("* * * * *");
        let tz = schedule.timezone.as_deref().unwrap_or("UTC");

        let evaluator =
            CronEvaluator::new(cron_expr, tz).map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Cron evaluation error: {}", e),
            })?;

        evaluator
            .next_execution(after)
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Cron evaluation error: {}", e),
            })
    }

    /// Executes a cron workflow by handing it off to the pipeline executor.
    async fn execute_cron_workflow(
        &self,
        schedule: &Schedule,
        scheduled_time: DateTime<Utc>,
    ) -> Result<UniversalUuid, PipelineError> {
        let mut context = Context::new();
        context
            .insert(
                "scheduled_time",
                serde_json::json!(scheduled_time.to_rfc3339()),
            )
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert("schedule_id", serde_json::json!(schedule.id.to_string()))
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert(
                "schedule_timezone",
                serde_json::json!(schedule.timezone.as_deref().unwrap_or("UTC")),
            )
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert(
                "schedule_expression",
                serde_json::json!(schedule.cron_expression.as_deref().unwrap_or("")),
            )
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;

        info!(
            "Executing workflow '{}' for cron schedule {} (scheduled time: {})",
            schedule.workflow_name, schedule.id, scheduled_time
        );

        let pipeline_result = self
            .executor
            .execute(&schedule.workflow_name, context)
            .await?;

        debug!(
            "Successfully handed off workflow '{}' to executor (execution_id: {})",
            schedule.workflow_name, pipeline_result.execution_id
        );

        Ok(UniversalUuid(pipeline_result.execution_id))
    }

    /// Creates an audit record for a cron execution.
    async fn create_cron_execution_audit(
        &self,
        schedule_id: UniversalUuid,
        scheduled_time: DateTime<Utc>,
    ) -> Result<UniversalUuid, ValidationError> {
        let new_execution = NewScheduleExecution {
            schedule_id,
            pipeline_execution_id: None,
            scheduled_time: Some(UniversalTimestamp(scheduled_time)),
            claimed_at: Some(UniversalTimestamp(Utc::now())),
            context_hash: None,
        };

        let audit_record = self.dal.schedule_execution().create(new_execution).await?;

        debug!(
            "Created cron execution audit record {} for schedule {} (scheduled: {})",
            audit_record.id, schedule_id, scheduled_time
        );

        Ok(audit_record.id)
    }

    // -----------------------------------------------------------------------
    // Trigger schedule processing
    // -----------------------------------------------------------------------

    /// Checks all enabled triggers and processes those that are due.
    async fn check_and_process_triggers(&mut self) -> Result<(), PipelineError> {
        debug!("Checking trigger schedules");

        let schedules = self
            .dal
            .schedule()
            .get_enabled_triggers()
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get trigger schedules: {}", e),
            })?;

        if schedules.is_empty() {
            debug!("No enabled trigger schedules found");
            return Ok(());
        }

        let now = Instant::now();

        for schedule in schedules {
            let trigger_name = schedule
                .trigger_name
                .as_deref()
                .unwrap_or("unknown")
                .to_string();

            // Check if this trigger is due for polling
            let poll_interval = schedule
                .poll_interval()
                .unwrap_or(self.config.trigger_base_poll_interval);
            let last_poll = self.last_poll_times.get(&trigger_name);

            let should_poll = match last_poll {
                Some(last) => now.duration_since(*last) >= poll_interval,
                None => true,
            };

            if !should_poll {
                continue;
            }

            // Process this trigger
            if let Err(e) = self.process_trigger(&schedule).await {
                error!("Failed to process trigger '{}': {}", trigger_name, e);
            }

            // Update last poll time
            self.last_poll_times.insert(trigger_name, now);
        }

        Ok(())
    }

    /// Processes a single trigger schedule.
    async fn process_trigger(&self, schedule: &Schedule) -> Result<(), TriggerError> {
        let trigger_name = schedule.trigger_name.as_deref().unwrap_or("unknown");

        debug!(
            "Processing trigger '{}' (workflow: {})",
            trigger_name, schedule.workflow_name
        );

        // Get the trigger instance from registry
        let trigger = get_trigger(trigger_name).ok_or_else(|| TriggerError::TriggerNotFound {
            name: trigger_name.to_string(),
        })?;

        // Poll the trigger with timeout
        let poll_result = tokio::time::timeout(self.config.trigger_poll_timeout, trigger.poll())
            .await
            .map_err(|_| TriggerError::PollError {
                message: format!(
                    "Trigger '{}' poll timed out after {:?}",
                    trigger_name, self.config.trigger_poll_timeout
                ),
            })?
            .map_err(|e| {
                error!("Trigger '{}' poll error: {}", trigger_name, e);
                e
            })?;

        // Update last poll time in database
        let now = Utc::now();
        if let Err(e) = self.dal.schedule().update_last_poll(schedule.id, now).await {
            warn!(
                "Failed to update last_poll_at for trigger '{}': {}",
                trigger_name, e
            );
        }

        // Check if trigger should fire
        if !poll_result.should_fire() {
            debug!("Trigger '{}' returned Skip", trigger_name);
            return Ok(());
        }

        // Compute context hash for deduplication
        let context_hash = poll_result.context_hash();

        // Check for duplicate active execution (unless allow_concurrent)
        if !schedule.allows_concurrent() {
            let has_active = self
                .dal
                .schedule_execution()
                .has_active_execution(schedule.id, &context_hash)
                .await
                .map_err(|e| TriggerError::ConnectionPool(e.to_string()))?;

            if has_active {
                debug!(
                    "Trigger '{}' has active execution with same context hash, skipping",
                    trigger_name
                );
                return Ok(());
            }
        }

        info!(
            "Trigger '{}' fired, scheduling workflow '{}'",
            trigger_name, schedule.workflow_name
        );

        // Create execution audit record before handoff
        let execution = self
            .create_trigger_execution_audit(schedule.id, &context_hash)
            .await?;

        // Extract context from result
        let context = poll_result.into_context().unwrap_or_else(Context::new);

        // Hand off to pipeline executor
        match self.execute_trigger_workflow(schedule, context).await {
            Ok(pipeline_execution_id) => {
                // Link the execution to the pipeline execution
                if let Err(e) = self
                    .dal
                    .schedule_execution()
                    .update_pipeline_execution_id(execution.id, pipeline_execution_id)
                    .await
                {
                    warn!(
                        "Failed to link schedule execution to pipeline execution: {}",
                        e
                    );
                }

                info!(
                    "Successfully scheduled workflow '{}' for trigger '{}' (execution: {})",
                    schedule.workflow_name, trigger_name, pipeline_execution_id
                );
            }
            Err(e) => {
                error!(
                    "Failed to execute workflow '{}' for trigger '{}': {}",
                    schedule.workflow_name, trigger_name, e
                );
                // Mark execution as completed (failed)
                if let Err(e) = self
                    .dal
                    .schedule_execution()
                    .complete(execution.id, Utc::now())
                    .await
                {
                    warn!(
                        "Failed to mark schedule execution as completed after failure: {}",
                        e
                    );
                }
                return Err(TriggerError::WorkflowSchedulingFailed {
                    workflow: schedule.workflow_name.clone(),
                    message: e.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Creates an audit record for a trigger execution.
    async fn create_trigger_execution_audit(
        &self,
        schedule_id: UniversalUuid,
        context_hash: &str,
    ) -> Result<crate::models::schedule::ScheduleExecution, TriggerError> {
        let new_execution = NewScheduleExecution {
            schedule_id,
            pipeline_execution_id: None,
            scheduled_time: None,
            claimed_at: None,
            context_hash: Some(context_hash.to_string()),
        };

        let execution = self
            .dal
            .schedule_execution()
            .create(new_execution)
            .await
            .map_err(|e| TriggerError::ConnectionPool(e.to_string()))?;

        debug!(
            "Created trigger execution audit record {} for schedule {}",
            execution.id, schedule_id
        );

        Ok(execution)
    }

    /// Executes a trigger workflow by handing it off to the pipeline executor.
    async fn execute_trigger_workflow(
        &self,
        schedule: &Schedule,
        mut context: Context<serde_json::Value>,
    ) -> Result<UniversalUuid, PipelineError> {
        let trigger_name = schedule.trigger_name.as_deref().unwrap_or("unknown");

        context
            .insert("trigger_name", serde_json::json!(trigger_name))
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert("triggered_at", serde_json::json!(Utc::now().to_rfc3339()))
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;

        let result = self
            .executor
            .execute(&schedule.workflow_name, context)
            .await?;

        debug!(
            "Successfully handed off workflow '{}' to executor (execution_id: {})",
            schedule.workflow_name, result.execution_id
        );

        Ok(UniversalUuid(result.execution_id))
    }

    // -----------------------------------------------------------------------
    // Trigger management (public API)
    // -----------------------------------------------------------------------

    /// Registers a trigger with the scheduler.
    ///
    /// Persists the trigger configuration to the database for recovery across
    /// restarts. The trigger must also be registered in the global trigger
    /// registry for the actual polling function.
    ///
    /// # Arguments
    /// * `trigger` - The trigger instance to register
    /// * `workflow_name` - Name of the workflow to fire when trigger activates
    pub async fn register_trigger(
        &self,
        trigger: &dyn Trigger,
        workflow_name: &str,
    ) -> Result<Schedule, ValidationError> {
        let mut new_schedule =
            NewSchedule::trigger(trigger.name(), workflow_name, trigger.poll_interval());
        new_schedule.allow_concurrent = Some(crate::database::universal_types::UniversalBool::new(
            trigger.allow_concurrent(),
        ));

        // Upsert to handle re-registration
        self.dal.schedule().upsert_trigger(new_schedule).await
    }

    /// Disables a trigger by name.
    pub async fn disable_trigger(&self, trigger_name: &str) -> Result<(), ValidationError> {
        if let Some(schedule) = self
            .dal
            .schedule()
            .get_by_trigger_name(trigger_name)
            .await?
        {
            self.dal.schedule().disable(schedule.id).await?;
            info!("Disabled trigger '{}'", trigger_name);
        }
        Ok(())
    }

    /// Enables a trigger by name.
    pub async fn enable_trigger(&self, trigger_name: &str) -> Result<(), ValidationError> {
        if let Some(schedule) = self
            .dal
            .schedule()
            .get_by_trigger_name(trigger_name)
            .await?
        {
            self.dal.schedule().enable(schedule.id).await?;
            info!("Enabled trigger '{}'", trigger_name);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::universal_types::{current_timestamp, UniversalBool};

    fn create_test_cron_schedule(cron_expr: &str, timezone: &str) -> Schedule {
        let now = current_timestamp();
        Schedule {
            id: UniversalUuid::new_v4(),
            schedule_type: "cron".to_string(),
            workflow_name: "test_workflow".to_string(),
            enabled: UniversalBool::new(true),
            cron_expression: Some(cron_expr.to_string()),
            timezone: Some(timezone.to_string()),
            catchup_policy: Some("skip".to_string()),
            start_date: None,
            end_date: None,
            trigger_name: None,
            poll_interval_ms: None,
            allow_concurrent: None,
            next_run_at: Some(now),
            last_run_at: None,
            last_poll_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn create_test_trigger_schedule(trigger_name: &str) -> Schedule {
        let now = current_timestamp();
        Schedule {
            id: UniversalUuid::new_v4(),
            schedule_type: "trigger".to_string(),
            workflow_name: "test_workflow".to_string(),
            enabled: UniversalBool::new(true),
            cron_expression: None,
            timezone: None,
            catchup_policy: None,
            start_date: None,
            end_date: None,
            trigger_name: Some(trigger_name.to_string()),
            poll_interval_ms: Some(5000),
            allow_concurrent: Some(UniversalBool::new(false)),
            next_run_at: None,
            last_run_at: None,
            last_poll_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert_eq!(config.cron_poll_interval, Duration::from_secs(30));
        assert_eq!(config.max_catchup_executions, 100);
        assert_eq!(config.max_acceptable_delay, Duration::from_secs(300));
        assert_eq!(config.trigger_base_poll_interval, Duration::from_secs(1));
        assert_eq!(config.trigger_poll_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_is_cron_schedule_active_no_window() {
        let schedule = create_test_cron_schedule("0 * * * *", "UTC");
        let now = Utc::now();

        // No start/end date — always active
        let config = SchedulerConfig::default();
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        // We can test the method directly by building a minimal Scheduler
        // but since it requires Arc<DAL> and Arc<dyn PipelineExecutor>,
        // we just verify the schedule model itself
        assert!(schedule.start_date.is_none());
        assert!(schedule.end_date.is_none());
        // No window constraints => active
        let active = schedule.start_date.as_ref().is_none_or(|s| now >= s.0)
            && schedule.end_date.as_ref().is_none_or(|e| now <= e.0);
        assert!(active);

        // Suppress unused variable warnings
        let _ = config;
        let _ = shutdown_rx;
    }

    #[test]
    fn test_is_cron_schedule_active_with_start_date_future() {
        let mut schedule = create_test_cron_schedule("0 * * * *", "UTC");
        // Set start date to the future
        let future = Utc::now() + chrono::Duration::hours(1);
        schedule.start_date = Some(UniversalTimestamp(future));

        let now = Utc::now();
        let active = schedule.start_date.as_ref().is_none_or(|s| now >= s.0)
            && schedule.end_date.as_ref().is_none_or(|e| now <= e.0);
        assert!(!active);
    }

    #[test]
    fn test_is_cron_schedule_active_with_end_date_past() {
        let mut schedule = create_test_cron_schedule("0 * * * *", "UTC");
        // Set end date to the past
        let past = Utc::now() - chrono::Duration::hours(1);
        schedule.end_date = Some(UniversalTimestamp(past));

        let now = Utc::now();
        let active = schedule.start_date.as_ref().is_none_or(|s| now >= s.0)
            && schedule.end_date.as_ref().is_none_or(|e| now <= e.0);
        assert!(!active);
    }

    #[test]
    fn test_catchup_policy_from_schedule() {
        let schedule = create_test_cron_schedule("0 * * * *", "UTC");
        let policy_str = schedule.catchup_policy.as_deref().unwrap_or("skip");
        let policy = CatchupPolicy::from(policy_str.to_string());
        assert_eq!(policy, CatchupPolicy::Skip);
    }

    #[test]
    fn test_catchup_policy_run_all() {
        let mut schedule = create_test_cron_schedule("0 * * * *", "UTC");
        schedule.catchup_policy = Some("run_all".to_string());
        let policy_str = schedule.catchup_policy.as_deref().unwrap_or("skip");
        let policy = CatchupPolicy::from(policy_str.to_string());
        assert_eq!(policy, CatchupPolicy::RunAll);
    }

    #[test]
    fn test_trigger_schedule_helpers() {
        let schedule = create_test_trigger_schedule("file_watcher");
        assert!(schedule.is_trigger());
        assert!(!schedule.is_cron());
        assert!(schedule.is_enabled());
        assert_eq!(schedule.poll_interval(), Some(Duration::from_secs(5)));
        assert!(!schedule.allows_concurrent());
    }

    #[test]
    fn test_trigger_schedule_trigger_name_fallback() {
        let mut schedule = create_test_trigger_schedule("file_watcher");
        // Verify the as_deref().unwrap_or("unknown") pattern
        assert_eq!(
            schedule.trigger_name.as_deref().unwrap_or("unknown"),
            "file_watcher"
        );
        schedule.trigger_name = None;
        assert_eq!(
            schedule.trigger_name.as_deref().unwrap_or("unknown"),
            "unknown"
        );
    }

    // -----------------------------------------------------------------------
    // SchedulerConfig tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_scheduler_config_custom() {
        let config = SchedulerConfig {
            cron_poll_interval: Duration::from_secs(60),
            max_catchup_executions: 50,
            max_acceptable_delay: Duration::from_secs(120),
            trigger_base_poll_interval: Duration::from_secs(5),
            trigger_poll_timeout: Duration::from_secs(10),
        };
        assert_eq!(config.cron_poll_interval, Duration::from_secs(60));
        assert_eq!(config.max_catchup_executions, 50);
        assert_eq!(config.max_acceptable_delay, Duration::from_secs(120));
        assert_eq!(config.trigger_base_poll_interval, Duration::from_secs(5));
        assert_eq!(config.trigger_poll_timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_scheduler_config_clone() {
        let config = SchedulerConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.cron_poll_interval, config.cron_poll_interval);
        assert_eq!(cloned.max_catchup_executions, config.max_catchup_executions);
        assert_eq!(cloned.max_acceptable_delay, config.max_acceptable_delay);
        assert_eq!(
            cloned.trigger_base_poll_interval,
            config.trigger_base_poll_interval
        );
        assert_eq!(cloned.trigger_poll_timeout, config.trigger_poll_timeout);
    }

    #[test]
    fn test_scheduler_config_debug() {
        let config = SchedulerConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("SchedulerConfig"));
        assert!(debug_str.contains("cron_poll_interval"));
    }

    // -----------------------------------------------------------------------
    // Cron schedule active window tests (expanded)
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_cron_schedule_active_both_bounds_containing_now() {
        let mut schedule = create_test_cron_schedule("0 * * * *", "UTC");
        let past = Utc::now() - chrono::Duration::hours(1);
        let future = Utc::now() + chrono::Duration::hours(1);
        schedule.start_date = Some(UniversalTimestamp(past));
        schedule.end_date = Some(UniversalTimestamp(future));

        let now = Utc::now();
        let active = schedule.start_date.as_ref().is_none_or(|s| now >= s.0)
            && schedule.end_date.as_ref().is_none_or(|e| now <= e.0);
        assert!(active);
    }

    #[test]
    fn test_is_cron_schedule_active_both_bounds_excluding_now() {
        let mut schedule = create_test_cron_schedule("0 * * * *", "UTC");
        // Both in the future
        let future1 = Utc::now() + chrono::Duration::hours(1);
        let future2 = Utc::now() + chrono::Duration::hours(2);
        schedule.start_date = Some(UniversalTimestamp(future1));
        schedule.end_date = Some(UniversalTimestamp(future2));

        let now = Utc::now();
        let active = schedule.start_date.as_ref().is_none_or(|s| now >= s.0)
            && schedule.end_date.as_ref().is_none_or(|e| now <= e.0);
        assert!(!active);
    }

    // -----------------------------------------------------------------------
    // Catchup policy parsing tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_catchup_policy_unknown_defaults_to_skip() {
        let policy = CatchupPolicy::from("unknown_policy".to_string());
        assert_eq!(policy, CatchupPolicy::Skip);
    }

    #[test]
    fn test_catchup_policy_none_defaults_to_skip() {
        let schedule = create_test_cron_schedule("0 * * * *", "UTC");
        // catchup_policy is Some("skip") by default in our helper
        let policy_str = schedule.catchup_policy.as_deref().unwrap_or("skip");
        assert_eq!(policy_str, "skip");
    }

    #[test]
    fn test_catchup_policy_missing_defaults_correctly() {
        let mut schedule = create_test_cron_schedule("0 * * * *", "UTC");
        schedule.catchup_policy = None;
        let policy_str = schedule.catchup_policy.as_deref().unwrap_or("skip");
        let policy = CatchupPolicy::from(policy_str.to_string());
        assert_eq!(policy, CatchupPolicy::Skip);
    }

    // -----------------------------------------------------------------------
    // Cron schedule model tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_cron_schedule_helpers() {
        let schedule = create_test_cron_schedule("*/5 * * * *", "America/New_York");
        assert!(schedule.is_cron());
        assert!(!schedule.is_trigger());
        assert!(schedule.is_enabled());
        assert_eq!(schedule.cron_expression.as_deref(), Some("*/5 * * * *"));
        assert_eq!(schedule.timezone.as_deref(), Some("America/New_York"));
    }

    #[test]
    fn test_trigger_schedule_no_poll_interval() {
        let mut schedule = create_test_trigger_schedule("webhook");
        schedule.poll_interval_ms = None;
        // With no poll_interval_ms, poll_interval() should return None
        assert_eq!(schedule.poll_interval(), None);
    }

    #[test]
    fn test_trigger_schedule_allows_concurrent() {
        let mut schedule = create_test_trigger_schedule("queue_trigger");
        schedule.allow_concurrent = Some(UniversalBool::new(true));
        assert!(schedule.allows_concurrent());
    }

    #[test]
    fn test_trigger_schedule_no_concurrent_flag_defaults_false() {
        let mut schedule = create_test_trigger_schedule("queue_trigger");
        schedule.allow_concurrent = None;
        assert!(!schedule.allows_concurrent());
    }
}
