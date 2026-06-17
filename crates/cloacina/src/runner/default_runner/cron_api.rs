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

//! Cron scheduling API for the DefaultRunner.
//!
//! This module provides methods for managing cron-scheduled workflow executions.

use std::sync::Arc;

use crate::dal::DAL;
use crate::executor::workflow_executor::WorkflowExecutionError;
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
    /// * `Result<UniversalUuid, WorkflowExecutionError>` - The ID of the created schedule or an error
    pub async fn register_cron_workflow(
        &self,
        workflow_name: &str,
        cron_expression: &str,
        timezone: &str,
    ) -> Result<UniversalUuid, WorkflowExecutionError> {
        if !self.config.enable_cron_scheduling() {
            return Err(WorkflowExecutionError::Configuration {
                message: "Cron scheduling not enabled. Use enable_cron_scheduling(true) in config."
                    .to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());

        // Validate cron expression and timezone
        use crate::CronEvaluator;
        CronEvaluator::validate(cron_expression, timezone).map_err(|e| {
            WorkflowExecutionError::Configuration {
                message: format!("Invalid cron expression or timezone: {}", e),
            }
        })?;

        // Calculate initial next run time
        let evaluator = CronEvaluator::new(cron_expression, timezone).map_err(|e| {
            WorkflowExecutionError::Configuration {
                message: format!("Failed to create cron evaluator: {}", e),
            }
        })?;

        let now = chrono::Utc::now();
        let next_run =
            evaluator
                .next_execution(now)
                .map_err(|e| WorkflowExecutionError::Configuration {
                    message: format!("Failed to calculate next execution: {}", e),
                })?;

        // Create the schedule using unified NewSchedule
        use crate::database::universal_types::UniversalTimestamp;
        use crate::models::schedule::NewSchedule;

        let mut new_schedule =
            NewSchedule::cron(workflow_name, cron_expression, UniversalTimestamp(next_run));
        new_schedule.timezone = Some(timezone.to_string());

        let schedule = dal.schedule().create(new_schedule).await.map_err(|e| {
            WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to create cron schedule: {}", e),
            }
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
    /// * `Result<Vec<Schedule>, WorkflowExecutionError>` - List of cron schedules
    pub async fn list_cron_schedules(
        &self,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::models::schedule::Schedule>, WorkflowExecutionError> {
        if !self.config.enable_cron_scheduling() {
            return Err(WorkflowExecutionError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());
        dal.schedule()
            .list(Some("cron"), enabled_only, limit, offset)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
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
    /// * `Result<(), WorkflowExecutionError>` - Success or error
    pub async fn set_cron_schedule_enabled(
        &self,
        schedule_id: UniversalUuid,
        enabled: bool,
    ) -> Result<(), WorkflowExecutionError> {
        if !self.config.enable_cron_scheduling() {
            return Err(WorkflowExecutionError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());

        if enabled {
            dal.schedule().enable(schedule_id).await
        } else {
            dal.schedule().disable(schedule_id).await
        }
        .map_err(|e| WorkflowExecutionError::ExecutionFailed {
            message: format!("Failed to update cron schedule: {}", e),
        })
    }

    /// Delete a cron schedule
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the schedule to delete
    ///
    /// # Returns
    /// * `Result<(), WorkflowExecutionError>` - Success or error
    pub async fn delete_cron_schedule(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<(), WorkflowExecutionError> {
        if !self.config.enable_cron_scheduling() {
            return Err(WorkflowExecutionError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());
        dal.schedule().delete(schedule_id).await.map_err(|e| {
            WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to delete cron schedule: {}", e),
            }
        })
    }

    /// Get a specific cron schedule by ID
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the schedule to retrieve
    ///
    /// # Returns
    /// * `Result<Schedule, WorkflowExecutionError>` - The cron schedule or an error
    pub async fn get_cron_schedule(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<crate::models::schedule::Schedule, WorkflowExecutionError> {
        if !self.config.enable_cron_scheduling() {
            return Err(WorkflowExecutionError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());
        dal.schedule().get_by_id(schedule_id).await.map_err(|e| {
            WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to get cron schedule: {}", e),
            }
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
    /// * `Result<(), WorkflowExecutionError>` - Success or error
    pub async fn update_cron_schedule(
        &self,
        schedule_id: UniversalUuid,
        cron_expression: Option<&str>,
        timezone: Option<&str>,
    ) -> Result<(), WorkflowExecutionError> {
        if !self.config.enable_cron_scheduling() {
            return Err(WorkflowExecutionError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());

        // Get current schedule to fill in missing values
        let schedule = dal.schedule().get_by_id(schedule_id).await.map_err(|e| {
            WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to get cron schedule: {}", e),
            }
        })?;

        let effective_expr = cron_expression
            .or(schedule.cron_expression.as_deref())
            .unwrap_or("* * * * *");
        let effective_tz = timezone.or(schedule.timezone.as_deref()).unwrap_or("UTC");

        // Validate inputs if provided
        if cron_expression.is_some() || timezone.is_some() {
            use crate::CronEvaluator;
            CronEvaluator::validate(effective_expr, effective_tz).map_err(|e| {
                WorkflowExecutionError::Configuration {
                    message: format!("Invalid cron expression or timezone: {}", e),
                }
            })?;
        }

        // Calculate new next run time
        use crate::CronEvaluator;
        let evaluator = CronEvaluator::new(effective_expr, effective_tz).map_err(|e| {
            WorkflowExecutionError::Configuration {
                message: format!("Failed to create cron evaluator: {}", e),
            }
        })?;

        let now = chrono::Utc::now();
        let next_run =
            evaluator
                .next_execution(now)
                .map_err(|e| WorkflowExecutionError::Configuration {
                    message: format!("Failed to calculate next execution: {}", e),
                })?;

        dal.schedule()
            .update_cron_expression_and_timezone(schedule_id, cron_expression, timezone, next_run)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
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
    /// * `Result<Vec<ScheduleExecution>, WorkflowExecutionError>` - List of schedule executions
    pub async fn get_cron_execution_history(
        &self,
        schedule_id: UniversalUuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::models::schedule::ScheduleExecution>, WorkflowExecutionError> {
        if !self.config.enable_cron_scheduling() {
            return Err(WorkflowExecutionError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());
        dal.schedule_execution()
            .list_by_schedule(schedule_id, limit, offset)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to get cron execution history: {}", e),
            })
    }

    /// Get cron execution statistics
    ///
    /// # Arguments
    /// * `since` - Only include executions since this timestamp
    ///
    /// # Returns
    /// * `Result<CronExecutionStats, WorkflowExecutionError>` - Execution statistics
    pub async fn get_cron_execution_stats(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<crate::dal::ScheduleExecutionStats, WorkflowExecutionError> {
        if !self.config.enable_cron_scheduling() {
            return Err(WorkflowExecutionError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.clone());
        dal.schedule_execution()
            .get_execution_stats(since)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to get cron execution stats: {}", e),
            })
    }

    /// Get access to the workflow registry (if enabled)
    ///
    /// # Returns
    /// * `Some(Arc<WorkflowRegistry>)` - If the registry is enabled and initialized
    /// * `None` - If the registry is not enabled or not yet initialized
    pub async fn get_workflow_registry(&self) -> Option<Arc<dyn WorkflowRegistry>> {
        self.service_manager.read().await.workflow_registry.clone()
    }

    /// Check if the registry reconciler is enabled in the configuration
    pub fn is_registry_reconciler_enabled(&self) -> bool {
        self.config.enable_registry_reconciler()
    }
}

/// Adapter that lets the registry reconciler register/unregister cron
/// workflow schedules without holding a `DefaultRunner` reference back
/// (which would form a cycle, given the runner OWNS the reconciler).
/// Holds an `Arc<Database>` and replicates the schedule-CRUD logic
/// from the runner's `register_cron_workflow` / `delete_cron_schedule`
/// methods. Constructed by `services.rs` only when cron scheduling is
/// enabled in the runner config; otherwise the reconciler runs without
/// a registrar and cron triggers warn loudly at load.
pub struct DalCronRegistrar {
    database: crate::database::Database,
}

impl DalCronRegistrar {
    pub fn new(database: crate::database::Database) -> Self {
        Self { database }
    }
}

#[async_trait::async_trait]
impl crate::registry::reconciler::CronWorkflowRegistrar for DalCronRegistrar {
    async fn register_cron_workflow(
        &self,
        workflow_name: &str,
        cron_expression: &str,
        timezone: &str,
    ) -> Result<String, String> {
        use crate::database::universal_types::UniversalTimestamp;
        use crate::models::schedule::NewSchedule;
        use crate::CronEvaluator;

        CronEvaluator::validate(cron_expression, timezone)
            .map_err(|e| format!("Invalid cron expression or timezone: {}", e))?;
        let evaluator = CronEvaluator::new(cron_expression, timezone)
            .map_err(|e| format!("Failed to create cron evaluator: {}", e))?;
        let now = chrono::Utc::now();
        let next_run = evaluator
            .next_execution(now)
            .map_err(|e| format!("Failed to calculate next execution: {}", e))?;

        let mut new_schedule =
            NewSchedule::cron(workflow_name, cron_expression, UniversalTimestamp(next_run));
        new_schedule.timezone = Some(timezone.to_string());

        let dal = DAL::new(self.database.clone());
        // Idempotent on (workflow_name, cron_expression, timezone): the
        // reconciler re-runs this on every package re-load, and a partially-
        // failed load is retried each tick — `create` accumulated a duplicate
        // schedule every time (CLOACI-T-0669). Upsert collapses those to one.
        let schedule = dal
            .schedule()
            .upsert_cron(new_schedule)
            .await
            .map_err(|e| format!("Failed to register cron schedule: {}", e))?;

        Ok(schedule.id.to_string())
    }

    async fn unregister_cron_workflow(&self, schedule_id: &str) -> Result<(), String> {
        let parsed: UniversalUuid = schedule_id
            .parse::<uuid::Uuid>()
            .map_err(|e| format!("invalid schedule id '{}': {}", schedule_id, e))?
            .into();
        let dal = DAL::new(self.database.clone());
        dal.schedule()
            .delete(parsed)
            .await
            .map_err(|e| format!("Failed to delete cron schedule: {}", e))
    }

    async fn register_poll_trigger(
        &self,
        trigger_name: &str,
        workflow_name: &str,
        poll_interval_ms: i32,
        allow_concurrent: bool,
    ) -> Result<String, String> {
        use crate::database::universal_types::UniversalBool;
        use crate::models::schedule::NewSchedule;
        use std::time::Duration;

        let mut new_schedule = NewSchedule::trigger(
            trigger_name,
            workflow_name,
            Duration::from_millis(poll_interval_ms.max(0) as u64),
        );
        new_schedule.allow_concurrent = Some(UniversalBool::new(allow_concurrent));

        let dal = DAL::new(self.database.clone());
        // Upsert so re-loading a package (same trigger name) refreshes the
        // row instead of erroring on a duplicate.
        let schedule = dal
            .schedule()
            .upsert_trigger(new_schedule)
            .await
            .map_err(|e| format!("Failed to create poll-trigger schedule: {}", e))?;
        Ok(schedule.id.to_string())
    }
}
