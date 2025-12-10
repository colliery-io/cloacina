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

//! Cron scheduling API for the DefaultRunner.
//!
//! This module provides methods for managing cron-scheduled workflow executions.

use std::sync::Arc;

use crate::dal::DAL;
use crate::executor::pipeline_executor::PipelineError;
use crate::registry::traits::WorkflowRegistry;
use crate::UniversalUuid;

use super::DefaultRunner;

impl DefaultRunner {
    /// Register a workflow to run on a cron schedule
    ///
    /// # Arguments
    /// * `workflow_name` - Name of the workflow to schedule
    /// * `cron_expression` - Cron expression (e.g., "0 9 * * *" for daily at 9 AM)
    /// * `timezone` - Timezone for interpreting the cron expression (e.g., "UTC", "America/New_York")
    ///
    /// # Returns
    /// * `Result<UniversalUuid, PipelineError>` - The ID of the created schedule or an error
    ///
    /// # Example
    /// ```rust,ignore
    /// let runner = DefaultRunner::new("postgresql://localhost/db").await?;
    ///
    /// // Schedule daily backup at 2 AM UTC
    /// runner.register_cron_workflow("backup_workflow", "0 2 * * *", "UTC").await?;
    ///
    /// // Schedule hourly reports during business hours in Eastern time
    /// runner.register_cron_workflow("hourly_report", "0 9-17 * * 1-5", "America/New_York").await?;
    /// ```
    pub async fn register_cron_workflow(
        &self,
        workflow_name: &str,
        cron_expression: &str,
        timezone: &str,
    ) -> Result<UniversalUuid, PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled. Use enable_cron_scheduling(true) in config."
                    .to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());

        // Validate cron expression and timezone
        use crate::CronEvaluator;
        CronEvaluator::validate(cron_expression, timezone).map_err(|e| {
            PipelineError::Configuration {
                message: format!("Invalid cron expression or timezone: {}", e),
            }
        })?;

        // Calculate initial next run time
        let evaluator = CronEvaluator::new(cron_expression, timezone).map_err(|e| {
            PipelineError::Configuration {
                message: format!("Failed to create cron evaluator: {}", e),
            }
        })?;

        let now = chrono::Utc::now();
        // Calculate next run time from now, ensuring it's in the future
        let next_run = evaluator
            .next_execution(now)
            .map_err(|e| PipelineError::Configuration {
                message: format!("Failed to calculate next execution: {}", e),
            })?;

        // Create the schedule
        use crate::database::universal_types::{UniversalBool, UniversalTimestamp};
        use crate::models::cron_schedule::NewCronSchedule;

        let new_schedule = NewCronSchedule {
            workflow_name: workflow_name.to_string(),
            cron_expression: cron_expression.to_string(),
            timezone: Some(timezone.to_string()),
            enabled: Some(UniversalBool::new(true)),
            catchup_policy: Some("skip".to_string()),
            start_date: None,
            end_date: None,
            next_run_at: UniversalTimestamp(next_run),
        };

        let schedule = dal
            .cron_schedule()
            .create(new_schedule)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to create cron schedule: {}", e),
            })?;

        Ok(schedule.id)
    }

    /// List all registered cron schedules
    ///
    /// # Arguments
    /// * `enabled_only` - If true, only return enabled schedules
    /// * `limit` - Maximum number of schedules to return
    /// * `offset` - Number of schedules to skip for pagination
    ///
    /// # Returns
    /// * `Result<Vec<CronSchedule>, PipelineError>` - List of cron schedules
    pub async fn list_cron_schedules(
        &self,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::models::cron_schedule::CronSchedule>, PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());
        dal.cron_schedule()
            .list(enabled_only, limit, offset)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to list cron schedules: {}", e),
            })
    }

    /// Enable or disable a cron schedule
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the schedule to modify
    /// * `enabled` - Whether to enable (true) or disable (false) the schedule
    ///
    /// # Returns
    /// * `Result<(), PipelineError>` - Success or error
    pub async fn set_cron_schedule_enabled(
        &self,
        schedule_id: UniversalUuid,
        enabled: bool,
    ) -> Result<(), PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());

        if enabled {
            dal.cron_schedule().enable(schedule_id).await
        } else {
            dal.cron_schedule().disable(schedule_id).await
        }
        .map_err(|e| PipelineError::ExecutionFailed {
            message: format!("Failed to update cron schedule: {}", e),
        })
    }

    /// Delete a cron schedule
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the schedule to delete
    ///
    /// # Returns
    /// * `Result<(), PipelineError>` - Success or error
    pub async fn delete_cron_schedule(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<(), PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());
        dal.cron_schedule()
            .delete(schedule_id)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to delete cron schedule: {}", e),
            })
    }

    /// Get a specific cron schedule by ID
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the schedule to retrieve
    ///
    /// # Returns
    /// * `Result<CronSchedule, PipelineError>` - The cron schedule or an error
    pub async fn get_cron_schedule(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<crate::models::cron_schedule::CronSchedule, PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());
        dal.cron_schedule()
            .get_by_id(schedule_id)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get cron schedule: {}", e),
            })
    }

    /// Update a cron schedule's expression and/or timezone
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the schedule to update
    /// * `cron_expression` - New cron expression (optional)
    /// * `timezone` - New timezone (optional)
    ///
    /// # Returns
    /// * `Result<(), PipelineError>` - Success or error
    pub async fn update_cron_schedule(
        &self,
        schedule_id: UniversalUuid,
        cron_expression: Option<&str>,
        timezone: Option<&str>,
    ) -> Result<(), PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());

        // Validate inputs if provided
        if let (Some(expr), Some(tz)) = (cron_expression, timezone) {
            use crate::CronEvaluator;
            CronEvaluator::validate(expr, tz).map_err(|e| PipelineError::Configuration {
                message: format!("Invalid cron expression or timezone: {}", e),
            })?;
        }

        // Get current schedule
        let mut schedule = dal
            .cron_schedule()
            .get_by_id(schedule_id)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get cron schedule: {}", e),
            })?;

        // Update fields if provided
        if let Some(expr) = cron_expression {
            schedule.cron_expression = expr.to_string();
        }
        if let Some(tz) = timezone {
            schedule.timezone = tz.to_string();
        }

        // Calculate new next run time
        use crate::CronEvaluator;
        let evaluator =
            CronEvaluator::new(&schedule.cron_expression, &schedule.timezone).map_err(|e| {
                PipelineError::Configuration {
                    message: format!("Failed to create cron evaluator: {}", e),
                }
            })?;

        let now = chrono::Utc::now();
        let next_run = evaluator
            .next_execution(now)
            .map_err(|e| PipelineError::Configuration {
                message: format!("Failed to calculate next execution: {}", e),
            })?;

        // Update the schedule with new expression, timezone, and next run time
        dal.cron_schedule()
            .update_expression_and_timezone(schedule_id, cron_expression, timezone, next_run)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to update cron schedule: {}", e),
            })?;

        Ok(())
    }

    /// Get execution history for a cron schedule
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the schedule
    /// * `limit` - Maximum number of executions to return
    /// * `offset` - Number of executions to skip for pagination
    ///
    /// # Returns
    /// * `Result<Vec<CronExecution>, PipelineError>` - List of cron executions
    pub async fn get_cron_execution_history(
        &self,
        schedule_id: UniversalUuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::models::cron_execution::CronExecution>, PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());
        dal.cron_execution()
            .get_by_schedule_id(schedule_id, limit, offset)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get cron execution history: {}", e),
            })
    }

    /// Get cron execution statistics
    ///
    /// # Arguments
    /// * `since` - Only include executions since this timestamp
    ///
    /// # Returns
    /// * `Result<CronExecutionStats, PipelineError>` - Execution statistics
    pub async fn get_cron_execution_stats(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<crate::dal::CronExecutionStats, PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());
        dal.cron_execution()
            .get_execution_stats(since)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get cron execution stats: {}", e),
            })
    }

    /// Get access to the workflow registry (if enabled)
    ///
    /// # Returns
    /// * `Some(Arc<WorkflowRegistry>)` - If the registry is enabled and initialized
    /// * `None` - If the registry is not enabled or not yet initialized
    pub async fn get_workflow_registry(&self) -> Option<Arc<dyn WorkflowRegistry>> {
        let registry = self.workflow_registry.read().await;
        registry.clone()
    }

    /// Get the current status of the registry reconciler (if enabled)
    ///
    /// # Returns
    /// * `Some(ReconcilerStatus)` - If the reconciler is enabled and initialized
    /// * `None` - If the reconciler is not enabled or not yet initialized
    pub async fn get_registry_reconciler_status(
        &self,
    ) -> Option<crate::registry::ReconcilerStatus> {
        let reconciler = self.registry_reconciler.read().await;
        if let Some(reconciler) = reconciler.as_ref() {
            Some(reconciler.get_status().await)
        } else {
            None
        }
    }

    /// Check if the registry reconciler is enabled in the configuration
    pub fn is_registry_reconciler_enabled(&self) -> bool {
        self.config.enable_registry_reconciler
    }
}
