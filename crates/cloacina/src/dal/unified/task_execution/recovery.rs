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

//! Recovery operations for orphaned and failed tasks.

use super::{RetryStats, TaskExecutionDAL};
use crate::dal::unified::models::UnifiedTaskExecution;
use crate::database::schema::unified::task_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::task_execution::TaskExecution;
use diesel::prelude::*;

impl<'a> TaskExecutionDAL<'a> {
    /// Retrieves tasks that are stuck in "Running" state (orphaned tasks).
    pub async fn get_orphaned_tasks(&self) -> Result<Vec<TaskExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_orphaned_tasks_postgres().await,
            self.get_orphaned_tasks_sqlite().await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_orphaned_tasks_postgres(&self) -> Result<Vec<TaskExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let orphaned_tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::status.eq("Running"))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(orphaned_tasks.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn get_orphaned_tasks_sqlite(&self) -> Result<Vec<TaskExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let orphaned_tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::status.eq("Running"))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(orphaned_tasks.into_iter().map(Into::into).collect())
    }

    /// Resets a task from "Running" to "Ready" state for recovery.
    pub async fn reset_task_for_recovery(
        &self,
        task_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.reset_task_for_recovery_postgres(task_id).await,
            self.reset_task_for_recovery_sqlite(task_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn reset_task_for_recovery_postgres(
        &self,
        task_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(task_executions::table.find(task_id))
                .set((
                    task_executions::status.eq("Ready"),
                    task_executions::started_at.eq(None::<UniversalTimestamp>),
                    task_executions::recovery_attempts.eq(task_executions::recovery_attempts + 1),
                    task_executions::last_recovery_at.eq(Some(now)),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn reset_task_for_recovery_sqlite(
        &self,
        task_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(task_executions::table.find(task_id))
                .set((
                    task_executions::status.eq("Ready"),
                    task_executions::started_at.eq(None::<UniversalTimestamp>),
                    task_executions::recovery_attempts.eq(task_executions::recovery_attempts + 1),
                    task_executions::last_recovery_at.eq(Some(now)),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Checks if a pipeline should be marked as failed due to abandoned tasks.
    pub async fn check_pipeline_failure(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<bool, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.check_pipeline_failure_postgres(pipeline_execution_id)
                .await,
            self.check_pipeline_failure_sqlite(pipeline_execution_id)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn check_pipeline_failure_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let failed_count: i64 = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::status.eq("Failed"))
                    .filter(task_executions::error_details.like("ABANDONED:%"))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(failed_count > 0)
    }

    #[cfg(feature = "sqlite")]
    async fn check_pipeline_failure_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let failed_count: i64 = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::status.eq("Failed"))
                    .filter(task_executions::error_details.like("ABANDONED:%"))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(failed_count > 0)
    }

    /// Calculates retry statistics for a specific pipeline execution.
    pub async fn get_retry_stats(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<RetryStats, ValidationError> {
        // This method is backend-agnostic since it processes data in memory
        let tasks = self
            .get_all_tasks_for_pipeline(pipeline_execution_id)
            .await?;

        let mut stats = RetryStats::default();

        for task in tasks {
            if task.attempt > 1 {
                stats.tasks_with_retries += 1;
                stats.total_retries += task.attempt - 1;
            }

            if task.attempt > stats.max_attempts_used {
                stats.max_attempts_used = task.attempt;
            }

            if task.status == "Failed" && task.attempt >= task.max_attempts {
                stats.tasks_exhausted_retries += 1;
            }
        }

        Ok(stats)
    }

    /// Retrieves tasks that have exceeded their retry limit.
    pub async fn get_exhausted_retry_tasks(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        // This method is backend-agnostic since it filters in memory
        let tasks = self
            .get_all_tasks_for_pipeline(pipeline_execution_id)
            .await?;

        let exhausted_tasks: Vec<TaskExecution> = tasks
            .into_iter()
            .filter(|task| task.status == "Failed" && task.attempt >= task.max_attempts)
            .collect();

        Ok(exhausted_tasks)
    }
}
