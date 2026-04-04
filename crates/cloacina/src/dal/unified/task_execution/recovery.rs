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

#[cfg(test)]
mod tests {
    use crate::dal::DAL;
    use crate::database::universal_types::UniversalUuid;
    use crate::database::Database;
    use crate::models::pipeline_execution::NewPipelineExecution;
    use crate::models::task_execution::NewTaskExecution;

    #[cfg(feature = "sqlite")]
    async fn unique_dal() -> DAL {
        let url = format!(
            "sqlite:///tmp/recovery_test_{}.db?mode=rwc",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    /// Helper: create a pipeline and return its ID.
    #[cfg(feature = "sqlite")]
    async fn create_pipeline(dal: &DAL) -> UniversalUuid {
        dal.pipeline_execution()
            .create(NewPipelineExecution {
                pipeline_name: "recovery_pipeline".into(),
                pipeline_version: "1.0".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap()
            .id
    }

    /// Helper: create a task with a given status, returning its ID.
    #[cfg(feature = "sqlite")]
    async fn create_task(
        dal: &DAL,
        pipeline_id: UniversalUuid,
        name: &str,
        status: &str,
        attempt: i32,
        max_attempts: i32,
    ) -> UniversalUuid {
        dal.task_execution()
            .create(NewTaskExecution {
                pipeline_execution_id: pipeline_id,
                task_name: name.into(),
                status: status.into(),
                attempt,
                max_attempts,
                trigger_rules: r#"{"type":"Always"}"#.into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap()
            .id
    }

    // ── get_orphaned_tasks ─────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_orphaned_tasks_none() {
        let dal = unique_dal().await;
        let pipeline_id = create_pipeline(&dal).await;
        create_task(&dal, pipeline_id, "pending_task", "NotStarted", 1, 3).await;
        create_task(&dal, pipeline_id, "ready_task", "Ready", 1, 3).await;

        let orphaned = dal.task_execution().get_orphaned_tasks().await.unwrap();
        assert!(orphaned.is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_orphaned_tasks_finds_running() {
        let dal = unique_dal().await;
        let pipeline_id = create_pipeline(&dal).await;
        let running_id = create_task(&dal, pipeline_id, "stuck_task", "Running", 1, 3).await;
        create_task(&dal, pipeline_id, "ok_task", "Completed", 1, 3).await;

        let orphaned = dal.task_execution().get_orphaned_tasks().await.unwrap();
        assert_eq!(orphaned.len(), 1);
        assert_eq!(orphaned[0].id, running_id);
        assert_eq!(orphaned[0].status, "Running");
    }

    // ── reset_task_for_recovery ────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_reset_task_for_recovery() {
        let dal = unique_dal().await;
        let pipeline_id = create_pipeline(&dal).await;
        let task_id = create_task(&dal, pipeline_id, "recover_me", "Running", 1, 3).await;

        dal.task_execution()
            .reset_task_for_recovery(task_id)
            .await
            .unwrap();

        let task = dal.task_execution().get_by_id(task_id).await.unwrap();
        assert_eq!(task.status, "Ready");
        assert!(task.started_at.is_none());
        assert_eq!(task.recovery_attempts, 1);
        assert!(task.last_recovery_at.is_some());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_reset_task_increments_recovery_attempts() {
        let dal = unique_dal().await;
        let pipeline_id = create_pipeline(&dal).await;
        let task_id = create_task(&dal, pipeline_id, "multi_recover", "Running", 1, 3).await;

        // First recovery
        dal.task_execution()
            .reset_task_for_recovery(task_id)
            .await
            .unwrap();
        let task = dal.task_execution().get_by_id(task_id).await.unwrap();
        assert_eq!(task.recovery_attempts, 1);

        // Simulate it going back to Running (would normally happen via claiming)
        // We can just reset again since the method only checks the ID
        dal.task_execution()
            .reset_task_for_recovery(task_id)
            .await
            .unwrap();
        let task = dal.task_execution().get_by_id(task_id).await.unwrap();
        assert_eq!(task.recovery_attempts, 2);
    }

    // ── check_pipeline_failure ─────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_check_pipeline_failure_no_abandoned() {
        let dal = unique_dal().await;
        let pipeline_id = create_pipeline(&dal).await;
        create_task(&dal, pipeline_id, "ok", "Completed", 1, 3).await;

        let failed = dal
            .task_execution()
            .check_pipeline_failure(pipeline_id)
            .await
            .unwrap();
        assert!(!failed);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_check_pipeline_failure_with_abandoned() {
        let dal = unique_dal().await;
        let pipeline_id = create_pipeline(&dal).await;
        let task_id = create_task(&dal, pipeline_id, "abandoned", "NotStarted", 1, 3).await;

        // Mark the task as abandoned (which sets status=Failed + error_details=ABANDONED:...)
        dal.task_execution()
            .mark_abandoned(task_id, "worker lost")
            .await
            .unwrap();

        let failed = dal
            .task_execution()
            .check_pipeline_failure(pipeline_id)
            .await
            .unwrap();
        assert!(failed);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_check_pipeline_failure_regular_failure_not_abandoned() {
        let dal = unique_dal().await;
        let pipeline_id = create_pipeline(&dal).await;
        let task_id = create_task(&dal, pipeline_id, "regular_fail", "NotStarted", 1, 3).await;

        // A regular failure (not ABANDONED) should NOT trigger pipeline failure check
        dal.task_execution()
            .mark_failed(task_id, "something broke")
            .await
            .unwrap();

        let failed = dal
            .task_execution()
            .check_pipeline_failure(pipeline_id)
            .await
            .unwrap();
        assert!(!failed);
    }

    // ── get_retry_stats ────────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_retry_stats_no_retries() {
        let dal = unique_dal().await;
        let pipeline_id = create_pipeline(&dal).await;
        create_task(&dal, pipeline_id, "t1", "Completed", 1, 3).await;
        create_task(&dal, pipeline_id, "t2", "Completed", 1, 3).await;

        let stats = dal
            .task_execution()
            .get_retry_stats(pipeline_id)
            .await
            .unwrap();
        assert_eq!(stats.tasks_with_retries, 0);
        assert_eq!(stats.total_retries, 0);
        assert_eq!(stats.max_attempts_used, 1);
        assert_eq!(stats.tasks_exhausted_retries, 0);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_retry_stats_with_retries() {
        let dal = unique_dal().await;
        let pipeline_id = create_pipeline(&dal).await;

        // Task that succeeded on attempt 1
        create_task(&dal, pipeline_id, "first_try", "Completed", 1, 3).await;
        // Task that succeeded on attempt 3
        create_task(&dal, pipeline_id, "third_try", "Completed", 3, 3).await;
        // Task that exhausted retries
        create_task(&dal, pipeline_id, "exhausted", "Failed", 3, 3).await;

        let stats = dal
            .task_execution()
            .get_retry_stats(pipeline_id)
            .await
            .unwrap();
        assert_eq!(stats.tasks_with_retries, 2); // third_try + exhausted
        assert_eq!(stats.total_retries, 4); // (3-1) + (3-1) = 4
        assert_eq!(stats.max_attempts_used, 3);
        assert_eq!(stats.tasks_exhausted_retries, 1); // exhausted
    }

    // ── get_exhausted_retry_tasks ──────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_exhausted_retry_tasks() {
        let dal = unique_dal().await;
        let pipeline_id = create_pipeline(&dal).await;

        create_task(&dal, pipeline_id, "ok", "Completed", 1, 3).await;
        create_task(&dal, pipeline_id, "still_trying", "Failed", 2, 3).await;
        let exhausted_id = create_task(&dal, pipeline_id, "gave_up", "Failed", 3, 3).await;

        let exhausted = dal
            .task_execution()
            .get_exhausted_retry_tasks(pipeline_id)
            .await
            .unwrap();
        assert_eq!(exhausted.len(), 1);
        assert_eq!(exhausted[0].id, exhausted_id);
        assert_eq!(exhausted[0].task_name, "gave_up");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_exhausted_retry_tasks_empty() {
        let dal = unique_dal().await;
        let pipeline_id = create_pipeline(&dal).await;
        create_task(&dal, pipeline_id, "ok", "Completed", 1, 3).await;

        let exhausted = dal
            .task_execution()
            .get_exhausted_retry_tasks(pipeline_id)
            .await
            .unwrap();
        assert!(exhausted.is_empty());
    }
}
