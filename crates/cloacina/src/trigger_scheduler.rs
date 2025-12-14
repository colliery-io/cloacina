/*
 *  Copyright 2025 Colliery Software
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

//! Trigger scheduler for event-based workflow execution.
//!
//! This module provides the core scheduling engine that polls user-defined triggers
//! and hands off workflow executions to the pipeline executor. The scheduler implements
//! a polling-based pattern where triggers are checked at their configured intervals.
//!
//! # Key Features
//!
//! - **Per-trigger Poll Intervals**: Each trigger has its own polling frequency
//! - **Context-based Deduplication**: Prevents duplicate executions based on context hash
//! - **Audit Trail**: Records every trigger fire and workflow handoff
//! - **Saga Pattern**: Clean separation between trigger polling and workflow execution
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────┐    fire &     ┌──────────────────┐    execute    ┌─────────────┐
//! │  TriggerScheduler  │   hand off    │ PipelineExecutor │  workflows    │   Tasks     │
//! │                    │ ─────────────▶│                  │ ─────────────▶│             │
//! │ • Poll triggers    │               │ • Execute        │               │ • Business  │
//! │ • Deduplicate      │               │ • Retry          │               │   Logic     │
//! │ • Audit log        │               │ • Recovery       │               │ • Context   │
//! └────────────────────┘               └──────────────────┘               └─────────────┘
//! ```
//!
//! # Examples
//!
//! ```rust,ignore
//! use cloacina::trigger_scheduler::TriggerScheduler;
//! use cloacina::runner::DefaultRunner;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let executor = DefaultRunner::new("postgresql://localhost/mydb").await?;
//! let scheduler = TriggerScheduler::new(
//!     executor.dal(),
//!     executor.clone(),
//!     TriggerSchedulerConfig::default()
//! );
//!
//! // Start the polling loop
//! scheduler.run_polling_loop().await?;
//! # Ok(())
//! # }
//! ```

use crate::context::Context;
use crate::dal::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::error::ValidationError;
use crate::executor::{PipelineError, PipelineExecutor};
use crate::models::trigger_execution::NewTriggerExecution;
use crate::models::trigger_schedule::{NewTriggerSchedule, TriggerSchedule};
use crate::trigger::{get_trigger, Trigger, TriggerError};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

/// Configuration for the trigger scheduler.
#[derive(Debug, Clone)]
pub struct TriggerSchedulerConfig {
    /// Base poll interval - how often to check all triggers for readiness
    pub base_poll_interval: Duration,
    /// Maximum time to wait for a trigger poll operation
    pub poll_timeout: Duration,
}

impl Default for TriggerSchedulerConfig {
    fn default() -> Self {
        Self {
            base_poll_interval: Duration::from_secs(1),
            poll_timeout: Duration::from_secs(30),
        }
    }
}

/// Event-based trigger scheduler for workflow execution.
///
/// The scheduler implements a polling loop that:
/// 1. Checks all registered triggers at their individual intervals
/// 2. Calls the trigger's poll() function to determine if it should fire
/// 3. Deduplicates executions based on context hash (unless allow_concurrent)
/// 4. Hands off workflow executions to the pipeline executor
/// 5. Records audit trail for each trigger fire
///
/// # Responsibilities
///
/// **What TriggerScheduler Does:**
/// - Poll registered triggers at their configured intervals
/// - Check for duplicate active executions (context-based deduplication)
/// - Hand off workflow executions to pipeline executor
/// - Record trigger execution audit trail
/// - Move on immediately (no waiting for workflow completion)
///
/// **What TriggerScheduler Does NOT Do:**
/// - Execute workflows directly
/// - Handle task retries or failures
/// - Wait for workflow completion
/// - Manage workflow state or recovery
#[derive(Clone)]
pub struct TriggerScheduler {
    dal: Arc<DAL>,
    executor: Arc<dyn PipelineExecutor>,
    config: TriggerSchedulerConfig,
    shutdown: watch::Receiver<bool>,
    /// Tracks when each trigger was last polled
    last_poll_times: HashMap<String, Instant>,
}

impl TriggerScheduler {
    /// Creates a new trigger scheduler.
    ///
    /// # Arguments
    /// * `dal` - Data access layer for database operations
    /// * `executor` - Pipeline executor for workflow execution
    /// * `config` - Scheduler configuration
    /// * `shutdown` - Shutdown signal receiver
    pub fn new(
        dal: Arc<DAL>,
        executor: Arc<dyn PipelineExecutor>,
        config: TriggerSchedulerConfig,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            dal,
            executor,
            config,
            shutdown,
            last_poll_times: HashMap::new(),
        }
    }

    /// Creates a new trigger scheduler with default configuration.
    pub fn with_defaults(
        dal: Arc<DAL>,
        executor: Arc<dyn PipelineExecutor>,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self::new(dal, executor, TriggerSchedulerConfig::default(), shutdown)
    }

    /// Runs the main polling loop for trigger processing.
    ///
    /// This method starts a loop that:
    /// 1. Checks all enabled triggers at their individual intervals
    /// 2. Processes each trigger that is due for polling
    /// 3. Handles shutdown gracefully when signaled
    ///
    /// The loop continues until a shutdown signal is received.
    ///
    /// # Returns
    /// * `Result<(), PipelineError>` - Success or error from the polling loop
    pub async fn run_polling_loop(&mut self) -> Result<(), PipelineError> {
        info!(
            "Starting trigger scheduler polling loop (base interval: {:?})",
            self.config.base_poll_interval
        );

        let mut interval = tokio::time::interval(self.config.base_poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.check_and_process_triggers().await {
                        error!("Error processing triggers: {}", e);
                        // Continue running despite errors
                    }
                }
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        info!("Trigger scheduler received shutdown signal");
                        break;
                    }
                }
            }
        }

        info!("Trigger scheduler polling loop stopped");
        Ok(())
    }

    /// Checks all registered triggers and processes those that are due.
    async fn check_and_process_triggers(&mut self) -> Result<(), PipelineError> {
        debug!("Checking trigger schedules");

        // Get all enabled trigger schedules from database
        let schedules = self
            .dal
            .trigger_schedule()
            .get_enabled()
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
            // Check if this trigger is due for polling
            let poll_interval = Duration::from_millis(schedule.poll_interval_ms as u64);
            let last_poll = self.last_poll_times.get(&schedule.trigger_name);

            let should_poll = match last_poll {
                Some(last) => now.duration_since(*last) >= poll_interval,
                None => true, // Never polled before
            };

            if !should_poll {
                continue;
            }

            // Process this trigger
            if let Err(e) = self.process_trigger(&schedule).await {
                error!(
                    "Failed to process trigger '{}': {}",
                    schedule.trigger_name, e
                );
                // Continue with other triggers
            }

            // Update last poll time
            self.last_poll_times
                .insert(schedule.trigger_name.clone(), now);
        }

        Ok(())
    }

    /// Processes a single trigger schedule.
    ///
    /// Steps:
    /// 1. Get the trigger instance from registry
    /// 2. Call the trigger's poll() function
    /// 3. If Fire result, check deduplication
    /// 4. If not duplicate (or allow_concurrent), schedule workflow
    /// 5. Record audit trail
    async fn process_trigger(&self, schedule: &TriggerSchedule) -> Result<(), TriggerError> {
        debug!(
            "Processing trigger '{}' (workflow: {})",
            schedule.trigger_name, schedule.workflow_name
        );

        // Get the trigger instance from registry
        let trigger =
            get_trigger(&schedule.trigger_name).ok_or_else(|| TriggerError::TriggerNotFound {
                name: schedule.trigger_name.clone(),
            })?;

        // Poll the trigger with timeout
        let poll_result = tokio::time::timeout(self.config.poll_timeout, trigger.poll())
            .await
            .map_err(|_| TriggerError::PollError {
                message: format!(
                    "Trigger '{}' poll timed out after {:?}",
                    schedule.trigger_name, self.config.poll_timeout
                ),
            })?
            .map_err(|e| {
                error!("Trigger '{}' poll error: {}", schedule.trigger_name, e);
                e
            })?;

        // Update last poll time in database
        let now = Utc::now();
        if let Err(e) = self
            .dal
            .trigger_schedule()
            .update_last_poll(schedule.id, now)
            .await
        {
            warn!(
                "Failed to update last_poll_at for trigger '{}': {}",
                schedule.trigger_name, e
            );
        }

        // Check if trigger should fire
        if !poll_result.should_fire() {
            debug!("Trigger '{}' returned Skip", schedule.trigger_name);
            return Ok(());
        }

        // Compute context hash for deduplication
        let context_hash = poll_result.context_hash();

        // Check for duplicate active execution (unless allow_concurrent)
        if !schedule.allows_concurrent() {
            let has_active = self
                .dal
                .trigger_execution()
                .has_active_execution(&schedule.trigger_name, &context_hash)
                .await
                .map_err(|e| TriggerError::ConnectionPool(e.to_string()))?;

            if has_active {
                debug!(
                    "Trigger '{}' has active execution with same context hash, skipping",
                    schedule.trigger_name
                );
                return Ok(());
            }
        }

        info!(
            "Trigger '{}' fired, scheduling workflow '{}'",
            schedule.trigger_name, schedule.workflow_name
        );

        // Create execution audit record before handoff
        let execution = self
            .create_execution_audit(&schedule.trigger_name, &context_hash)
            .await?;

        // Extract context from result
        let context = poll_result.into_context().unwrap_or_else(Context::new);

        // Hand off to pipeline executor
        match self.execute_workflow(schedule, context).await {
            Ok(pipeline_execution_id) => {
                // Link the trigger execution to the pipeline execution
                if let Err(e) = self
                    .dal
                    .trigger_execution()
                    .link_pipeline_execution(execution.id, pipeline_execution_id)
                    .await
                {
                    warn!(
                        "Failed to link trigger execution to pipeline execution: {}",
                        e
                    );
                }

                info!(
                    "Successfully scheduled workflow '{}' for trigger '{}' (execution: {})",
                    schedule.workflow_name, schedule.trigger_name, pipeline_execution_id
                );
            }
            Err(e) => {
                error!(
                    "Failed to execute workflow '{}' for trigger '{}': {}",
                    schedule.workflow_name, schedule.trigger_name, e
                );
                // Mark execution as completed (failed)
                if let Err(e) = self
                    .dal
                    .trigger_execution()
                    .complete(execution.id, Utc::now())
                    .await
                {
                    warn!(
                        "Failed to mark trigger execution as completed after failure: {}",
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
    async fn create_execution_audit(
        &self,
        trigger_name: &str,
        context_hash: &str,
    ) -> Result<crate::models::trigger_execution::TriggerExecution, TriggerError> {
        let new_execution = NewTriggerExecution::new(trigger_name, context_hash);

        let execution = self
            .dal
            .trigger_execution()
            .create(new_execution)
            .await
            .map_err(|e| TriggerError::ConnectionPool(e.to_string()))?;

        debug!(
            "Created trigger execution audit record {} for trigger '{}'",
            execution.id, trigger_name
        );

        Ok(execution)
    }

    /// Executes a workflow by handing it off to the pipeline executor.
    async fn execute_workflow(
        &self,
        schedule: &TriggerSchedule,
        mut context: Context<serde_json::Value>,
    ) -> Result<UniversalUuid, PipelineError> {
        // Add trigger metadata to context
        context
            .insert(
                "trigger_name",
                serde_json::json!(schedule.trigger_name.clone()),
            )
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert("triggered_at", serde_json::json!(Utc::now().to_rfc3339()))
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;

        // Hand off to pipeline executor
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

    /// Registers a trigger with the scheduler.
    ///
    /// This persists the trigger configuration to the database for recovery
    /// across restarts. The trigger must also be registered in the global
    /// trigger registry for the actual polling function.
    ///
    /// # Arguments
    /// * `trigger` - The trigger instance to register
    /// * `workflow_name` - Name of the workflow to fire when trigger activates
    pub async fn register_trigger(
        &self,
        trigger: &dyn Trigger,
        workflow_name: &str,
    ) -> Result<TriggerSchedule, ValidationError> {
        let new_schedule =
            NewTriggerSchedule::new(trigger.name(), workflow_name, trigger.poll_interval())
                .with_allow_concurrent(trigger.allow_concurrent());

        // Upsert to handle re-registration
        self.dal.trigger_schedule().upsert(new_schedule).await
    }

    /// Disables a trigger by name.
    pub async fn disable_trigger(&self, trigger_name: &str) -> Result<(), ValidationError> {
        if let Some(schedule) = self
            .dal
            .trigger_schedule()
            .get_by_name(trigger_name)
            .await?
        {
            self.dal.trigger_schedule().disable(schedule.id).await?;
            info!("Disabled trigger '{}'", trigger_name);
        }
        Ok(())
    }

    /// Enables a trigger by name.
    pub async fn enable_trigger(&self, trigger_name: &str) -> Result<(), ValidationError> {
        if let Some(schedule) = self
            .dal
            .trigger_schedule()
            .get_by_name(trigger_name)
            .await?
        {
            self.dal.trigger_schedule().enable(schedule.id).await?;
            info!("Enabled trigger '{}'", trigger_name);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_scheduler_config_default() {
        let config = TriggerSchedulerConfig::default();
        assert_eq!(config.base_poll_interval, Duration::from_secs(1));
        assert_eq!(config.poll_timeout, Duration::from_secs(30));
    }
}
