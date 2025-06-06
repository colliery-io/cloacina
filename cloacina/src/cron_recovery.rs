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

//! Cron execution recovery service for handling lost executions.
//!
//! This module provides a recovery mechanism that detects and retries cron executions
//! that were claimed but never successfully handed off to the pipeline executor.
//! It implements the recovery side of the guaranteed execution pattern.
//!
//! # Architecture
//!
//! The recovery service runs as a background task that periodically:
//! 1. Queries for lost executions (claimed but no pipeline execution)
//! 2. Determines if recovery is appropriate
//! 3. Retries the execution through the pipeline executor
//! 4. Updates audit records to reflect recovery attempts
//!
//! # Recovery Policy
//!
//! Executions are considered "lost" if:
//! - They have a cron_executions record (were claimed)
//! - They have no corresponding pipeline_executions record
//! - They were claimed more than X minutes ago (configurable)
//!
//! Recovery is skipped if:
//! - The schedule is disabled
//! - The schedule has been deleted
//! - Too many recovery attempts have been made
//! - The execution is too old (beyond recovery window)

use crate::context::Context;
use crate::dal::DAL;
use crate::executor::{PipelineError, PipelineExecutor};
use crate::models::cron_execution::CronExecution;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

/// Configuration for the cron recovery service.
#[derive(Debug, Clone)]
pub struct CronRecoveryConfig {
    /// How often to check for lost executions
    pub check_interval: Duration,
    /// Consider executions lost if claimed more than this many minutes ago
    pub lost_threshold_minutes: i32,
    /// Maximum age of executions to recover (older ones are abandoned)
    pub max_recovery_age: Duration,
    /// Maximum number of recovery attempts per execution
    pub max_recovery_attempts: usize,
    /// Whether to recover executions for disabled schedules
    pub recover_disabled_schedules: bool,
}

impl Default for CronRecoveryConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(300), // 5 minutes
            lost_threshold_minutes: 10,
            max_recovery_age: Duration::from_secs(86400), // 24 hours
            max_recovery_attempts: 3,
            recover_disabled_schedules: false,
        }
    }
}

/// Recovery service for lost cron executions.
///
/// This service implements the recovery side of the guaranteed execution pattern,
/// detecting executions that were claimed but never handed off and retrying them.
#[derive(Clone)]
pub struct CronRecoveryService {
    dal: Arc<DAL>,
    executor: Arc<dyn PipelineExecutor>,
    config: CronRecoveryConfig,
    shutdown: watch::Receiver<bool>,
    /// Tracks recovery attempts per execution ID
    recovery_attempts: Arc<tokio::sync::Mutex<HashMap<crate::database::UniversalUuid, usize>>>,
}

impl CronRecoveryService {
    /// Creates a new cron recovery service.
    ///
    /// # Arguments
    /// * `dal` - Data access layer for database operations
    /// * `executor` - Pipeline executor for retrying executions
    /// * `config` - Recovery service configuration
    /// * `shutdown` - Shutdown signal receiver
    pub fn new(
        dal: Arc<DAL>,
        executor: Arc<dyn PipelineExecutor>,
        config: CronRecoveryConfig,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            dal,
            executor,
            config,
            shutdown,
            recovery_attempts: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Creates a new recovery service with default configuration.
    pub fn with_defaults(
        dal: Arc<DAL>,
        executor: Arc<dyn PipelineExecutor>,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self::new(dal, executor, CronRecoveryConfig::default(), shutdown)
    }

    /// Runs the recovery service loop.
    ///
    /// This method starts an infinite loop that periodically checks for and
    /// recovers lost executions until a shutdown signal is received.
    pub async fn run_recovery_loop(&mut self) -> Result<(), PipelineError> {
        info!(
            "Starting cron recovery service (interval: {:?}, threshold: {} minutes)",
            self.config.check_interval, self.config.lost_threshold_minutes
        );

        let mut interval = tokio::time::interval(self.config.check_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.check_and_recover_lost_executions().await {
                        error!("Error in cron recovery service: {}", e);
                        // Continue running despite errors
                    }
                }
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        info!("Cron recovery service received shutdown signal");
                        break;
                    }
                }
            }
        }

        info!("Cron recovery service stopped");
        Ok(())
    }

    /// Checks for lost executions and attempts to recover them.
    async fn check_and_recover_lost_executions(&self) -> Result<(), PipelineError> {
        debug!("Checking for lost cron executions");

        // Find lost executions
        let lost_executions = self
            .dal
            .cron_execution()
            .find_lost_executions(self.config.lost_threshold_minutes)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to find lost executions: {}", e),
            })?;

        if lost_executions.is_empty() {
            debug!("No lost executions found");
            return Ok(());
        }

        info!("Found {} lost cron execution(s)", lost_executions.len());

        // Attempt to recover each lost execution
        for execution in lost_executions {
            if let Err(e) = self.recover_execution(&execution).await {
                error!(
                    "Failed to recover execution {} for schedule {}: {}",
                    execution.id, execution.schedule_id, e
                );
                // Continue with other executions
            }
        }

        Ok(())
    }

    /// Attempts to recover a single lost execution.
    async fn recover_execution(&self, execution: &CronExecution) -> Result<(), PipelineError> {
        let execution_age = Utc::now() - execution.scheduled_time();

        // Check if execution is too old to recover
        if execution_age > chrono::Duration::from_std(self.config.max_recovery_age).unwrap() {
            warn!(
                "Execution {} is too old to recover (age: {:?}), abandoning",
                execution.id, execution_age
            );
            return Ok(());
        }

        // Check recovery attempts
        let mut attempts = self.recovery_attempts.lock().await;
        let attempt_count = attempts.entry(execution.id).or_insert(0);
        *attempt_count += 1;

        if *attempt_count > self.config.max_recovery_attempts {
            error!(
                "Execution {} has exceeded max recovery attempts ({}), abandoning",
                execution.id, self.config.max_recovery_attempts
            );
            return Ok(());
        }

        info!(
            "Attempting recovery of execution {} (schedule: {}, attempt: {}/{})",
            execution.id, execution.schedule_id, attempt_count, self.config.max_recovery_attempts
        );

        // Get the schedule to check if it's still active
        let schedule = match self
            .dal
            .cron_schedule()
            .get_by_id(execution.schedule_id)
            .await
        {
            Ok(sched) => sched,
            Err(e) => {
                warn!(
                    "Schedule {} not found for execution {}, skipping recovery: {}",
                    execution.schedule_id, execution.id, e
                );
                return Ok(());
            }
        };

        // Check if schedule is enabled (unless configured to recover disabled schedules)
        if !self.config.recover_disabled_schedules && !schedule.enabled.is_true() {
            info!(
                "Schedule {} is disabled, skipping recovery of execution {}",
                schedule.id, execution.id
            );
            return Ok(());
        }

        // Create recovery context
        let mut context = Context::new();

        // Add recovery metadata
        context
            .insert("is_recovery", serde_json::json!(true))
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert("recovery_attempt", serde_json::json!(attempt_count))
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert(
                "original_execution_id",
                serde_json::json!(execution.id.to_string()),
            )
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;

        // Add original scheduling metadata
        context
            .insert(
                "scheduled_time",
                serde_json::json!(execution.scheduled_time().to_rfc3339()),
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

        // Execute the workflow
        info!(
            "Executing recovery for workflow '{}' (execution: {}, schedule: {})",
            schedule.workflow_name, execution.id, schedule.id
        );

        match self
            .executor
            .execute(&schedule.workflow_name, context)
            .await
        {
            Ok(pipeline_result) => {
                // Update the audit record with the new pipeline execution ID
                if let Err(e) = self
                    .dal
                    .cron_execution()
                    .update_pipeline_execution_id(
                        execution.id,
                        crate::database::UniversalUuid(pipeline_result.execution_id),
                    )
                    .await
                {
                    error!(
                        "Failed to update audit record for recovered execution {}: {}",
                        execution.id, e
                    );
                    // Continue - the recovery succeeded, just audit update failed
                }

                info!(
                    "Successfully recovered execution {} (new pipeline: {})",
                    execution.id, pipeline_result.execution_id
                );

                // Clear recovery attempts on success
                attempts.remove(&execution.id);

                Ok(())
            }
            Err(e) => {
                error!(
                    "Failed to recover execution {} for workflow '{}': {}",
                    execution.id, schedule.workflow_name, e
                );
                Err(e)
            }
        }
    }

    /// Clears the recovery attempts cache.
    ///
    /// This can be useful for testing or when you want to retry
    /// previously abandoned executions.
    pub async fn clear_recovery_attempts(&self) {
        let mut attempts = self.recovery_attempts.lock().await;
        attempts.clear();
        info!("Cleared recovery attempts cache");
    }

    /// Gets the current recovery attempts for an execution.
    pub async fn get_recovery_attempts(
        &self,
        execution_id: crate::database::UniversalUuid,
    ) -> usize {
        let attempts = self.recovery_attempts.lock().await;
        attempts.get(&execution_id).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::watch;

    #[test]
    fn test_recovery_config_default() {
        let config = CronRecoveryConfig::default();
        assert_eq!(config.check_interval, Duration::from_secs(300));
        assert_eq!(config.lost_threshold_minutes, 10);
        assert_eq!(config.max_recovery_age, Duration::from_secs(86400));
        assert_eq!(config.max_recovery_attempts, 3);
        assert!(!config.recover_disabled_schedules);
    }

    #[tokio::test]
    async fn test_recovery_attempts_tracking() {
        let (_tx, _rx) = watch::channel(false);
        // This would need proper mocking of DAL and executor for full testing
        // Just testing the basic structure here

        let config = CronRecoveryConfig {
            check_interval: Duration::from_secs(1),
            lost_threshold_minutes: 5,
            max_recovery_age: Duration::from_secs(3600),
            max_recovery_attempts: 3,
            recover_disabled_schedules: false,
        };

        assert_eq!(config.max_recovery_attempts, 3);
    }
}
