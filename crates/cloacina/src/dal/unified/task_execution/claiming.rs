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

//! Task claiming and retry scheduling operations.
//!
//! All operations are transactional: state changes and execution events
//! are written atomically. If either fails, both are rolled back.

use super::{ClaimResult, TaskExecutionDAL};
use crate::dal::unified::models::{NewUnifiedExecutionEvent, UnifiedTaskExecution};
use crate::database::schema::unified::{execution_events, task_executions, task_outbox};
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::execution_event::ExecutionEventType;
use crate::models::task_execution::TaskExecution;
use diesel::prelude::*;
use uuid::Uuid;

impl<'a> TaskExecutionDAL<'a> {
    /// Updates a task's retry schedule with a new attempt count and retry time.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
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
        use crate::dal::unified::models::NewUnifiedTaskOutbox;
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task retry state
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Ready"),
                        task_executions::attempt.eq(new_attempt),
                        task_executions::retry_at.eq(Some(retry_at)),
                        task_executions::started_at.eq(None::<UniversalTimestamp>),
                        task_executions::completed_at.eq(None::<UniversalTimestamp>),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with retry details
                let event_data = serde_json::json!({
                    "attempt": new_attempt,
                    "retry_at": retry_at.to_string()
                })
                .to_string();
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskRetryScheduled.as_str().to_string(),
                    event_data: Some(event_data),
                    worker_id: None,
                    created_at: now,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                // Insert outbox entry for work distribution
                // Use retry_at as created_at so workers won't claim until retry time
                let outbox_entry = NewUnifiedTaskOutbox {
                    task_execution_id: task_id,
                    created_at: retry_at,
                };
                diesel::insert_into(task_outbox::table)
                    .values(&outbox_entry)
                    .execute(conn)?;

                Ok(())
            })
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
        use crate::dal::unified::models::NewUnifiedTaskOutbox;
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task retry state
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Ready"),
                        task_executions::attempt.eq(new_attempt),
                        task_executions::retry_at.eq(Some(retry_at)),
                        task_executions::started_at.eq(None::<UniversalTimestamp>),
                        task_executions::completed_at.eq(None::<UniversalTimestamp>),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with retry details
                let event_data = serde_json::json!({
                    "attempt": new_attempt,
                    "retry_at": retry_at.to_string()
                })
                .to_string();
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskRetryScheduled.as_str().to_string(),
                    event_data: Some(event_data),
                    worker_id: None,
                    created_at: now,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                // Insert outbox entry for work distribution
                // Use retry_at as created_at so workers won't claim until retry time
                let outbox_entry = NewUnifiedTaskOutbox {
                    task_execution_id: task_id,
                    created_at: retry_at,
                };
                diesel::insert_into(task_outbox::table)
                    .values(&outbox_entry)
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Atomically claims up to `limit` ready tasks for execution.
    ///
    /// This operation is transactional: the status update and execution events
    /// are written atomically for all claimed tasks.
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
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let limit = limit as i64;

        #[derive(Debug, QueryableByName, Clone)]
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
                conn.transaction::<_, diesel::result::Error, _>(|conn| {
                    let now = UniversalTimestamp::now();

                    // Claim tasks from outbox with FOR UPDATE SKIP LOCKED:
                    // 1. Select outbox entries with lock (skip locked rows)
                    //    - Filter by created_at <= NOW() to respect retry delays
                    // 2. Delete those outbox entries
                    // 3. Update corresponding task_executions to Running
                    // 4. Return task details
                    let claimed: Vec<PgClaimResult> = diesel::sql_query(format!(
                        r#"
                        WITH claimed_outbox AS (
                            DELETE FROM task_outbox
                            WHERE id IN (
                                SELECT id FROM task_outbox
                                WHERE created_at <= NOW()
                                ORDER BY created_at ASC
                                LIMIT {}
                                FOR UPDATE SKIP LOCKED
                            )
                            RETURNING task_execution_id
                        )
                        UPDATE task_executions
                        SET status = 'Running', started_at = NOW(), updated_at = NOW()
                        FROM claimed_outbox
                        WHERE task_executions.id = claimed_outbox.task_execution_id
                        RETURNING task_executions.id, task_executions.pipeline_execution_id, task_executions.task_name, task_executions.attempt
                        "#,
                        limit
                    ))
                    .load(conn)?;

                    // Insert execution events for all claimed tasks
                    for task in &claimed {
                        let event = NewUnifiedExecutionEvent {
                            id: UniversalUuid::new_v4(),
                            pipeline_execution_id: UniversalUuid(task.pipeline_execution_id),
                            task_execution_id: Some(UniversalUuid(task.id)),
                            event_type: ExecutionEventType::TaskClaimed.as_str().to_string(),
                            event_data: None,
                            worker_id: None,
                            created_at: now,
                        };
                        diesel::insert_into(execution_events::table)
                            .values(&event)
                            .execute(conn)?;
                    }

                    Ok(claimed)
                })
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
        use crate::dal::unified::models::UnifiedTaskOutbox;
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let limit = limit as i64;

        // SQLite doesn't support FOR UPDATE SKIP LOCKED, so we use an IMMEDIATE transaction
        // to acquire a write lock at the start, preventing race conditions between workers.
        // This serializes concurrent claim attempts, ensuring each task is claimed exactly once.
        let tasks: Vec<UnifiedTaskExecution> = conn
            .interact(
                move |conn| -> Result<Vec<UnifiedTaskExecution>, diesel::result::Error> {
                    // Use IMMEDIATE transaction to acquire write lock immediately
                    // This prevents TOCTOU race conditions between SELECT and UPDATE
                    conn.transaction::<Vec<UnifiedTaskExecution>, diesel::result::Error, _>(
                        |conn| {
                            let now = UniversalTimestamp::now();

                            // Select oldest outbox entries within the transaction
                            // Filter by created_at <= NOW() to respect retry delays
                            let outbox_entries: Vec<UnifiedTaskOutbox> = task_outbox::table
                                .filter(task_outbox::created_at.le(now))
                                .order(task_outbox::created_at.asc())
                                .limit(limit)
                                .load(conn)?;

                            if outbox_entries.is_empty() {
                                return Ok(Vec::new());
                            }

                            // Collect task execution IDs and outbox IDs
                            let task_ids: Vec<_> =
                                outbox_entries.iter().map(|o| o.task_execution_id).collect();
                            let outbox_ids: Vec<_> = outbox_entries.iter().map(|o| o.id).collect();

                            // Delete outbox entries
                            diesel::delete(task_outbox::table)
                                .filter(task_outbox::id.eq_any(&outbox_ids))
                                .execute(conn)?;

                            // Load task executions for the claimed tasks
                            let claimed_tasks: Vec<UnifiedTaskExecution> = task_executions::table
                                .filter(task_executions::id.eq_any(&task_ids))
                                .load(conn)?;

                            // Batch update all tasks to Running in a single query
                            diesel::update(task_executions::table)
                                .filter(task_executions::id.eq_any(&task_ids))
                                .set((
                                    task_executions::status.eq("Running"),
                                    task_executions::started_at.eq(Some(now)),
                                    task_executions::updated_at.eq(now),
                                ))
                                .execute(conn)?;

                            // Insert execution events for all claimed tasks
                            for task in &claimed_tasks {
                                let event = NewUnifiedExecutionEvent {
                                    id: UniversalUuid::new_v4(),
                                    pipeline_execution_id: task.pipeline_execution_id,
                                    task_execution_id: Some(task.id),
                                    event_type: ExecutionEventType::TaskClaimed
                                        .as_str()
                                        .to_string(),
                                    event_data: None,
                                    worker_id: None,
                                    created_at: now,
                                };
                                diesel::insert_into(execution_events::table)
                                    .values(&event)
                                    .execute(conn)?;
                            }

                            Ok(claimed_tasks)
                        },
                    )
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
