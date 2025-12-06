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

//! Task Execution Data Access Layer for Unified Backend Support
//!
//! This module provides the data access layer for managing task executions in the pipeline system
//! with runtime backend selection between PostgreSQL and SQLite.
//!
//! Key features:
//! - Task state management (Ready, Running, Completed, Failed, Skipped)
//! - Retry mechanism with configurable backoff
//! - Recovery system for handling orphaned tasks
//! - Atomic task claiming for distributed execution
//! - Pipeline completion and failure detection

use super::models::{NewUnifiedTaskExecution, UnifiedTaskExecution};
use super::DAL;
use crate::database::schema::unified::task_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::database::BackendType;
use crate::error::ValidationError;
use crate::models::task_execution::{NewTaskExecution, TaskExecution};
use diesel::prelude::*;

use uuid::Uuid;

/// Statistics about retry behavior for a pipeline execution.
#[derive(Debug, Default)]
pub struct RetryStats {
    /// Number of tasks that required at least one retry.
    pub tasks_with_retries: i32,
    /// Total number of retry attempts across all tasks.
    pub total_retries: i32,
    /// Maximum number of attempts used by any single task.
    pub max_attempts_used: i32,
    /// Number of tasks that exhausted all retry attempts and failed.
    pub tasks_exhausted_retries: i32,
}

/// Result structure for atomic task claiming operations.
#[derive(Debug)]
pub struct ClaimResult {
    /// Unique identifier of the claimed task
    pub id: UniversalUuid,
    /// ID of the pipeline execution this task belongs to
    pub pipeline_execution_id: UniversalUuid,
    /// Name of the task that was claimed
    pub task_name: String,
    /// Current attempt number for this task
    pub attempt: i32,
}

/// Data access layer for task execution operations with runtime backend selection.
#[derive(Clone)]
pub struct TaskExecutionDAL<'a> {
    dal: &'a DAL,
}

impl<'a> TaskExecutionDAL<'a> {
    /// Creates a new TaskExecutionDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new task execution record in the database.
    pub async fn create(
        &self,
        new_task: NewTaskExecution,
    ) -> Result<TaskExecution, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.create_postgres(new_task).await,
            BackendType::Sqlite => self.create_sqlite(new_task).await,
        }
    }

    async fn create_postgres(
        &self,
        new_task: NewTaskExecution,
    ) -> Result<TaskExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified_task = NewUnifiedTaskExecution {
            id,
            pipeline_execution_id: new_task.pipeline_execution_id,
            task_name: new_task.task_name,
            status: new_task.status,
            attempt: new_task.attempt,
            max_attempts: new_task.max_attempts,
            trigger_rules: new_task.trigger_rules,
            task_configuration: new_task.task_configuration,
            created_at: now,
            updated_at: now,
        };

        let task: UnifiedTaskExecution = conn
            .interact(move |conn| {
                diesel::insert_into(task_executions::table)
                    .values(&new_unified_task)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(task.into())
    }

    async fn create_sqlite(
        &self,
        new_task: NewTaskExecution,
    ) -> Result<TaskExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified_task = NewUnifiedTaskExecution {
            id,
            pipeline_execution_id: new_task.pipeline_execution_id,
            task_name: new_task.task_name,
            status: new_task.status,
            attempt: new_task.attempt,
            max_attempts: new_task.max_attempts,
            trigger_rules: new_task.trigger_rules,
            task_configuration: new_task.task_configuration,
            created_at: now,
            updated_at: now,
        };

        let task: UnifiedTaskExecution = conn
            .interact(move |conn| {
                diesel::insert_into(task_executions::table)
                    .values(&new_unified_task)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(task.into())
    }

    /// Retrieves a specific task execution by its ID.
    pub async fn get_by_id(
        &self,
        task_id: UniversalUuid,
    ) -> Result<TaskExecution, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.get_by_id_postgres(task_id).await,
            BackendType::Sqlite => self.get_by_id_sqlite(task_id).await,
        }
    }

    async fn get_by_id_postgres(
        &self,
        task_id: UniversalUuid,
    ) -> Result<TaskExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task: UnifiedTaskExecution = conn
            .interact(move |conn| task_executions::table.find(task_id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(task.into())
    }

    async fn get_by_id_sqlite(
        &self,
        task_id: UniversalUuid,
    ) -> Result<TaskExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task: UnifiedTaskExecution = conn
            .interact(move |conn| task_executions::table.find(task_id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(task.into())
    }

    /// Retrieves all pending (NotStarted) tasks for a specific pipeline execution.
    pub async fn get_pending_tasks(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.get_pending_tasks_postgres(pipeline_execution_id).await,
            BackendType::Sqlite => self.get_pending_tasks_sqlite(pipeline_execution_id).await,
        }
    }

    async fn get_pending_tasks_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::status.eq("NotStarted"))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    async fn get_pending_tasks_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::status.eq("NotStarted"))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    /// Gets all pending tasks for multiple pipelines in a single query.
    pub async fn get_pending_tasks_batch(
        &self,
        pipeline_execution_ids: Vec<UniversalUuid>,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.get_pending_tasks_batch_postgres(pipeline_execution_ids)
                    .await
            }
            BackendType::Sqlite => {
                self.get_pending_tasks_batch_sqlite(pipeline_execution_ids)
                    .await
            }
        }
    }

    async fn get_pending_tasks_batch_postgres(
        &self,
        pipeline_execution_ids: Vec<UniversalUuid>,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        if pipeline_execution_ids.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq_any(&pipeline_execution_ids))
                    .filter(task_executions::status.eq_any(vec!["NotStarted", "Pending"]))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    async fn get_pending_tasks_batch_sqlite(
        &self,
        pipeline_execution_ids: Vec<UniversalUuid>,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        if pipeline_execution_ids.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq_any(&pipeline_execution_ids))
                    .filter(task_executions::status.eq_any(vec!["NotStarted", "Pending"]))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    /// Checks if all tasks in a pipeline have reached a terminal state.
    pub async fn check_pipeline_completion(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<bool, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.check_pipeline_completion_postgres(pipeline_execution_id)
                    .await
            }
            BackendType::Sqlite => {
                self.check_pipeline_completion_sqlite(pipeline_execution_id)
                    .await
            }
        }
    }

    async fn check_pipeline_completion_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let incomplete_count: i64 = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::status.ne_all(vec!["Completed", "Failed", "Skipped"]))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(incomplete_count == 0)
    }

    async fn check_pipeline_completion_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let incomplete_count: i64 = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::status.ne_all(vec!["Completed", "Failed", "Skipped"]))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(incomplete_count == 0)
    }

    /// Retrieves all tasks associated with a pipeline execution.
    pub async fn get_all_tasks_for_pipeline(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.get_all_tasks_for_pipeline_postgres(pipeline_execution_id)
                    .await
            }
            BackendType::Sqlite => {
                self.get_all_tasks_for_pipeline_sqlite(pipeline_execution_id)
                    .await
            }
        }
    }

    async fn get_all_tasks_for_pipeline_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    async fn get_all_tasks_for_pipeline_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks.into_iter().map(Into::into).collect())
    }

    /// Marks a task execution as completed.
    pub async fn mark_completed(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.mark_completed_postgres(task_id).await,
            BackendType::Sqlite => self.mark_completed_sqlite(task_id).await,
        }
    }

    async fn mark_completed_postgres(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
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
                    task_executions::status.eq("Completed"),
                    task_executions::completed_at.eq(Some(now)),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn mark_completed_sqlite(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
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
                    task_executions::status.eq("Completed"),
                    task_executions::completed_at.eq(Some(now)),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Marks a task execution as failed with an error message.
    pub async fn mark_failed(
        &self,
        task_id: UniversalUuid,
        error_message: &str,
    ) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.mark_failed_postgres(task_id, error_message).await,
            BackendType::Sqlite => self.mark_failed_sqlite(task_id, error_message).await,
        }
    }

    async fn mark_failed_postgres(
        &self,
        task_id: UniversalUuid,
        error_message: &str,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let error_message_owned = error_message.to_string();
        conn.interact(move |conn| {
            diesel::update(task_executions::table.find(task_id))
                .set((
                    task_executions::status.eq("Failed"),
                    task_executions::completed_at.eq(Some(now)),
                    task_executions::last_error.eq(&error_message_owned),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn mark_failed_sqlite(
        &self,
        task_id: UniversalUuid,
        error_message: &str,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let error_message_owned = error_message.to_string();
        conn.interact(move |conn| {
            diesel::update(task_executions::table.find(task_id))
                .set((
                    task_executions::status.eq("Failed"),
                    task_executions::completed_at.eq(Some(now)),
                    task_executions::last_error.eq(&error_message_owned),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Updates a task's retry schedule with a new attempt count and retry time.
    pub async fn schedule_retry(
        &self,
        task_id: UniversalUuid,
        retry_at: UniversalTimestamp,
        new_attempt: i32,
    ) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.schedule_retry_postgres(task_id, retry_at, new_attempt)
                    .await
            }
            BackendType::Sqlite => {
                self.schedule_retry_sqlite(task_id, retry_at, new_attempt)
                    .await
            }
        }
    }

    async fn schedule_retry_postgres(
        &self,
        task_id: UniversalUuid,
        retry_at: UniversalTimestamp,
        new_attempt: i32,
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
                    task_executions::attempt.eq(new_attempt),
                    task_executions::retry_at.eq(Some(retry_at)),
                    task_executions::started_at.eq(None::<UniversalTimestamp>),
                    task_executions::completed_at.eq(None::<UniversalTimestamp>),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn schedule_retry_sqlite(
        &self,
        task_id: UniversalUuid,
        retry_at: UniversalTimestamp,
        new_attempt: i32,
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
                    task_executions::attempt.eq(new_attempt),
                    task_executions::retry_at.eq(Some(retry_at)),
                    task_executions::started_at.eq(None::<UniversalTimestamp>),
                    task_executions::completed_at.eq(None::<UniversalTimestamp>),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Atomically claims up to `limit` ready tasks for execution.
    pub async fn claim_ready_task(
        &self,
        limit: usize,
    ) -> Result<Vec<ClaimResult>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.claim_ready_task_postgres(limit).await,
            BackendType::Sqlite => self.claim_ready_task_sqlite(limit).await,
        }
    }

    async fn claim_ready_task_postgres(
        &self,
        limit: usize,
    ) -> Result<Vec<ClaimResult>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let limit = limit as i64;

        #[derive(Debug, QueryableByName)]
        #[diesel(check_for_backend(diesel::pg::Pg))]
        struct PgClaimResult {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            pipeline_execution_id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Text)]
            task_name: String,
            #[diesel(sql_type = diesel::sql_types::Integer)]
            attempt: i32,
        }

        let pg_results: Vec<PgClaimResult> = conn
            .interact(move |conn| {
                diesel::sql_query(format!(
                    r#"
                WITH ready_tasks AS (
                    SELECT id, pipeline_execution_id, task_name, attempt
                    FROM task_executions
                    WHERE status = 'Ready'
                    AND (retry_at IS NULL OR retry_at <= NOW())
                    ORDER BY id ASC
                    LIMIT {}
                    FOR UPDATE SKIP LOCKED
                )
                UPDATE task_executions
                SET status = 'Running', started_at = NOW()
                FROM ready_tasks
                WHERE task_executions.id = ready_tasks.id
                RETURNING task_executions.id, task_executions.pipeline_execution_id, task_executions.task_name, task_executions.attempt
                "#,
                    limit
                ))
                .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_results
            .into_iter()
            .map(|pg| ClaimResult {
                id: UniversalUuid(pg.id),
                pipeline_execution_id: UniversalUuid(pg.pipeline_execution_id),
                task_name: pg.task_name,
                attempt: pg.attempt,
            })
            .collect())
    }

    async fn claim_ready_task_sqlite(
        &self,
        limit: usize,
    ) -> Result<Vec<ClaimResult>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let limit = limit as i64;
        let now = UniversalTimestamp::now();

        // SQLite doesn't support FOR UPDATE SKIP LOCKED, so we use a simpler approach
        // This is less concurrent-safe but sufficient for single-node SQLite usage
        let tasks: Vec<UnifiedTaskExecution> = conn
            .interact(
                move |conn| -> Result<Vec<UnifiedTaskExecution>, diesel::result::Error> {
                    // First, select ready tasks
                    let ready_tasks: Vec<UnifiedTaskExecution> = task_executions::table
                        .filter(task_executions::status.eq("Ready"))
                        .filter(
                            task_executions::retry_at
                                .is_null()
                                .or(task_executions::retry_at.le(now)),
                        )
                        .limit(limit)
                        .load(conn)?;

                    // Update them to Running
                    for task in &ready_tasks {
                        diesel::update(task_executions::table.find(task.id))
                            .set((
                                task_executions::status.eq("Running"),
                                task_executions::started_at.eq(Some(now)),
                            ))
                            .execute(conn)?;
                    }

                    Ok(ready_tasks)
                },
            )
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(tasks
            .into_iter()
            .map(|task| ClaimResult {
                id: task.id,
                pipeline_execution_id: task.pipeline_execution_id,
                task_name: task.task_name,
                attempt: task.attempt,
            })
            .collect())
    }

    /// Marks a task as ready for execution.
    pub async fn mark_ready(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.mark_ready_postgres(task_id).await,
            BackendType::Sqlite => self.mark_ready_sqlite(task_id).await,
        }
    }

    async fn mark_ready_postgres(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
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
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        tracing::debug!(task_id = %task_id, "Task marked as Ready");
        Ok(())
    }

    async fn mark_ready_sqlite(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
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
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        tracing::debug!(task_id = %task_id, "Task marked as Ready");
        Ok(())
    }

    /// Marks a task as skipped with a provided reason.
    pub async fn mark_skipped(
        &self,
        task_id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.mark_skipped_postgres(task_id, reason).await,
            BackendType::Sqlite => self.mark_skipped_sqlite(task_id, reason).await,
        }
    }

    async fn mark_skipped_postgres(
        &self,
        task_id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let reason_owned = reason.to_string();
        let reason_log = reason.to_string();
        conn.interact(move |conn| {
            diesel::update(task_executions::table.find(task_id))
                .set((
                    task_executions::status.eq("Skipped"),
                    task_executions::error_details.eq(&reason_owned),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        tracing::info!(task_id = %task_id, reason = %reason_log, "Task marked as Skipped");
        Ok(())
    }

    async fn mark_skipped_sqlite(
        &self,
        task_id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let reason_owned = reason.to_string();
        let reason_log = reason.to_string();
        conn.interact(move |conn| {
            diesel::update(task_executions::table.find(task_id))
                .set((
                    task_executions::status.eq("Skipped"),
                    task_executions::error_details.eq(&reason_owned),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        tracing::info!(task_id = %task_id, reason = %reason_log, "Task marked as Skipped");
        Ok(())
    }

    /// Gets the current status of a specific task in a pipeline.
    pub async fn get_task_status(
        &self,
        pipeline_execution_id: UniversalUuid,
        task_name: &str,
    ) -> Result<String, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.get_task_status_postgres(pipeline_execution_id, task_name)
                    .await
            }
            BackendType::Sqlite => {
                self.get_task_status_sqlite(pipeline_execution_id, task_name)
                    .await
            }
        }
    }

    async fn get_task_status_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
        task_name: &str,
    ) -> Result<String, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task_name_owned = task_name.to_string();
        let status: String = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::task_name.eq(&task_name_owned))
                    .select(task_executions::status)
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(status)
    }

    async fn get_task_status_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
        task_name: &str,
    ) -> Result<String, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task_name_owned = task_name.to_string();
        let status: String = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::task_name.eq(&task_name_owned))
                    .select(task_executions::status)
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(status)
    }

    /// Gets the status of multiple tasks in a single database query.
    pub async fn get_task_statuses_batch(
        &self,
        pipeline_execution_id: UniversalUuid,
        task_names: Vec<String>,
    ) -> Result<std::collections::HashMap<String, String>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.get_task_statuses_batch_postgres(pipeline_execution_id, task_names)
                    .await
            }
            BackendType::Sqlite => {
                self.get_task_statuses_batch_sqlite(pipeline_execution_id, task_names)
                    .await
            }
        }
    }

    async fn get_task_statuses_batch_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
        task_names: Vec<String>,
    ) -> Result<std::collections::HashMap<String, String>, ValidationError> {
        use std::collections::HashMap;

        if task_names.is_empty() {
            return Ok(HashMap::new());
        }

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<(String, String)> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::task_name.eq_any(&task_names))
                    .select((task_executions::task_name, task_executions::status))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().collect())
    }

    async fn get_task_statuses_batch_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
        task_names: Vec<String>,
    ) -> Result<std::collections::HashMap<String, String>, ValidationError> {
        use std::collections::HashMap;

        if task_names.is_empty() {
            return Ok(HashMap::new());
        }

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<(String, String)> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
                    .filter(task_executions::task_name.eq_any(&task_names))
                    .select((task_executions::task_name, task_executions::status))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().collect())
    }

    /// Retrieves tasks that are stuck in "Running" state (orphaned tasks).
    pub async fn get_orphaned_tasks(&self) -> Result<Vec<TaskExecution>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.get_orphaned_tasks_postgres().await,
            BackendType::Sqlite => self.get_orphaned_tasks_sqlite().await,
        }
    }

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
        match self.dal.backend() {
            BackendType::Postgres => self.reset_task_for_recovery_postgres(task_id).await,
            BackendType::Sqlite => self.reset_task_for_recovery_sqlite(task_id).await,
        }
    }

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

    /// Marks a task as permanently abandoned after too many recovery attempts.
    pub async fn mark_abandoned(
        &self,
        task_id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.mark_abandoned_postgres(task_id, reason).await,
            BackendType::Sqlite => self.mark_abandoned_sqlite(task_id, reason).await,
        }
    }

    async fn mark_abandoned_postgres(
        &self,
        task_id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let reason_owned = reason.to_string();
        conn.interact(move |conn| {
            diesel::update(task_executions::table.find(task_id))
                .set((
                    task_executions::status.eq("Failed"),
                    task_executions::completed_at.eq(Some(now)),
                    task_executions::error_details.eq(format!("ABANDONED: {}", reason_owned)),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn mark_abandoned_sqlite(
        &self,
        task_id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let reason_owned = reason.to_string();
        conn.interact(move |conn| {
            diesel::update(task_executions::table.find(task_id))
                .set((
                    task_executions::status.eq("Failed"),
                    task_executions::completed_at.eq(Some(now)),
                    task_executions::error_details.eq(format!("ABANDONED: {}", reason_owned)),
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
        match self.dal.backend() {
            BackendType::Postgres => {
                self.check_pipeline_failure_postgres(pipeline_execution_id)
                    .await
            }
            BackendType::Sqlite => {
                self.check_pipeline_failure_sqlite(pipeline_execution_id)
                    .await
            }
        }
    }

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

    /// Retrieves tasks that are ready for retry (retry_at time has passed).
    pub async fn get_ready_for_retry(&self) -> Result<Vec<TaskExecution>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.get_ready_for_retry_postgres().await,
            BackendType::Sqlite => self.get_ready_for_retry_sqlite().await,
        }
    }

    async fn get_ready_for_retry_postgres(&self) -> Result<Vec<TaskExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let ready_tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::status.eq("Ready"))
                    .filter(
                        task_executions::retry_at
                            .is_null()
                            .or(task_executions::retry_at.le(now)),
                    )
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(ready_tasks.into_iter().map(Into::into).collect())
    }

    async fn get_ready_for_retry_sqlite(&self) -> Result<Vec<TaskExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let ready_tasks: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::status.eq("Ready"))
                    .filter(
                        task_executions::retry_at
                            .is_null()
                            .or(task_executions::retry_at.le(now)),
                    )
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(ready_tasks.into_iter().map(Into::into).collect())
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

    /// Resets the retry state for a task to its initial state.
    pub async fn reset_retry_state(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.reset_retry_state_postgres(task_id).await,
            BackendType::Sqlite => self.reset_retry_state_sqlite(task_id).await,
        }
    }

    async fn reset_retry_state_postgres(
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
                    task_executions::attempt.eq(1),
                    task_executions::retry_at.eq(None::<UniversalTimestamp>),
                    task_executions::started_at.eq(None::<UniversalTimestamp>),
                    task_executions::completed_at.eq(None::<UniversalTimestamp>),
                    task_executions::last_error.eq(None::<String>),
                    task_executions::status.eq("Ready"),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn reset_retry_state_sqlite(
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
                    task_executions::attempt.eq(1),
                    task_executions::retry_at.eq(None::<UniversalTimestamp>),
                    task_executions::started_at.eq(None::<UniversalTimestamp>),
                    task_executions::completed_at.eq(None::<UniversalTimestamp>),
                    task_executions::last_error.eq(None::<String>),
                    task_executions::status.eq("Ready"),
                    task_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }
}
