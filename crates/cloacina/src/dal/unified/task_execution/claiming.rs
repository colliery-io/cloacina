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

//! Task claiming and retry scheduling operations.

use super::{ClaimResult, TaskExecutionDAL};
use crate::dal::unified::models::UnifiedTaskExecution;
use crate::database::schema::unified::task_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::task_execution::TaskExecution;
use diesel::prelude::*;
use uuid::Uuid;

impl<'a> TaskExecutionDAL<'a> {
    /// Updates a task's retry schedule with a new attempt count and retry time.
    pub async fn schedule_retry(
        &self,
        task_id: UniversalUuid,
        retry_at: UniversalTimestamp,
        new_attempt: i32,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.schedule_retry_postgres(task_id, retry_at, new_attempt)
                .await,
            self.schedule_retry_sqlite(task_id, retry_at, new_attempt)
                .await
        )
    }

    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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
        crate::dispatch_backend!(
            self.dal.backend(),
            self.claim_ready_task_postgres(limit).await,
            self.claim_ready_task_sqlite(limit).await
        )
    }

    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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

    /// Retrieves tasks that are ready for retry (retry_at time has passed).
    pub async fn get_ready_for_retry(&self) -> Result<Vec<TaskExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_ready_for_retry_postgres().await,
            self.get_ready_for_retry_sqlite().await
        )
    }

    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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
}
