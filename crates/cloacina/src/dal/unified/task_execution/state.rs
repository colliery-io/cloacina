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

//! State transition operations for task executions.
//!
//! All state transitions are transactional: the status update and execution event
//! are written atomically. If either fails, both are rolled back.

use super::TaskExecutionDAL;
use crate::dal::unified::models::{
    NewUnifiedExecutionEvent, NewUnifiedTaskOutbox, UnifiedTaskExecution,
};
use crate::database::schema::unified::{execution_events, task_executions, task_outbox};
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::execution_event::ExecutionEventType;
use diesel::prelude::*;

impl<'a> TaskExecutionDAL<'a> {
    /// Marks a task execution as completed.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn mark_completed(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.mark_completed_postgres(task_id).await,
            self.mark_completed_sqlite(task_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn mark_completed_postgres(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
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

                // Update task status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Completed"),
                        task_executions::completed_at.eq(Some(now)),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskCompleted.as_str().to_string(),
                    event_data: None,
                    worker_id: None,
                    created_at: now,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn mark_completed_sqlite(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
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

                // Update task status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Completed"),
                        task_executions::completed_at.eq(Some(now)),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskCompleted.as_str().to_string(),
                    event_data: None,
                    worker_id: None,
                    created_at: now,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Marks a task execution as failed with an error message.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn mark_failed(
        &self,
        task_id: UniversalUuid,
        error_message: &str,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.mark_failed_postgres(task_id, error_message).await,
            self.mark_failed_sqlite(task_id, error_message).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn mark_failed_postgres(
        &self,
        task_id: UniversalUuid,
        error_message: &str,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let error_message = error_message.to_string();
        conn.interact(move |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Failed"),
                        task_executions::completed_at.eq(Some(now)),
                        task_executions::last_error.eq(&error_message),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with error details
                let event_data = serde_json::json!({ "error": error_message }).to_string();
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskFailed.as_str().to_string(),
                    event_data: Some(event_data),
                    worker_id: None,
                    created_at: now,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn mark_failed_sqlite(
        &self,
        task_id: UniversalUuid,
        error_message: &str,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let error_message = error_message.to_string();
        conn.interact(move |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Failed"),
                        task_executions::completed_at.eq(Some(now)),
                        task_executions::last_error.eq(&error_message),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with error details
                let event_data = serde_json::json!({ "error": error_message }).to_string();
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskFailed.as_str().to_string(),
                    event_data: Some(event_data),
                    worker_id: None,
                    created_at: now,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Marks a task as ready for execution.
    ///
    /// This operation is transactional: the status update, execution event,
    /// and outbox entry are written atomically. If any fail, all are rolled back.
    ///
    /// The outbox entry enables push-based work distribution (Postgres LISTEN/NOTIFY)
    /// or polling-based distribution (SQLite).
    pub async fn mark_ready(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.mark_ready_postgres(task_id).await,
            self.mark_ready_sqlite(task_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn mark_ready_postgres(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
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

                // Update task status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Ready"),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskMarkedReady.as_str().to_string(),
                    event_data: None,
                    worker_id: None,
                    created_at: now,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                // Insert outbox entry for work distribution
                let outbox_entry = NewUnifiedTaskOutbox {
                    task_execution_id: task_id,
                    created_at: now,
                };
                diesel::insert_into(task_outbox::table)
                    .values(&outbox_entry)
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        tracing::debug!(task_id = %task_id, "Task marked as Ready with outbox entry");
        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn mark_ready_sqlite(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
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

                // Update task status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Ready"),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskMarkedReady.as_str().to_string(),
                    event_data: None,
                    worker_id: None,
                    created_at: now,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                // Insert outbox entry for work distribution
                let outbox_entry = NewUnifiedTaskOutbox {
                    task_execution_id: task_id,
                    created_at: now,
                };
                diesel::insert_into(task_outbox::table)
                    .values(&outbox_entry)
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        tracing::debug!(task_id = %task_id, "Task marked as Ready with outbox entry");
        Ok(())
    }

    /// Marks a task as skipped with a provided reason.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn mark_skipped(
        &self,
        task_id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.mark_skipped_postgres(task_id, reason).await,
            self.mark_skipped_sqlite(task_id, reason).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn mark_skipped_postgres(
        &self,
        task_id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let reason = reason.to_string();
        let reason_log = reason.clone();
        conn.interact(move |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Skipped"),
                        task_executions::error_details.eq(&reason),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with skip reason
                let event_data = serde_json::json!({ "reason": reason }).to_string();
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskSkipped.as_str().to_string(),
                    event_data: Some(event_data),
                    worker_id: None,
                    created_at: now,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        tracing::info!(task_id = %task_id, reason = %reason_log, "Task marked as Skipped");
        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn mark_skipped_sqlite(
        &self,
        task_id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let reason = reason.to_string();
        let reason_log = reason.clone();
        conn.interact(move |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Skipped"),
                        task_executions::error_details.eq(&reason),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with skip reason
                let event_data = serde_json::json!({ "reason": reason }).to_string();
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskSkipped.as_str().to_string(),
                    event_data: Some(event_data),
                    worker_id: None,
                    created_at: now,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        tracing::info!(task_id = %task_id, reason = %reason_log, "Task marked as Skipped");
        Ok(())
    }

    /// Marks a task as permanently abandoned after too many recovery attempts.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn mark_abandoned(
        &self,
        task_id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.mark_abandoned_postgres(task_id, reason).await,
            self.mark_abandoned_sqlite(task_id, reason).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn mark_abandoned_postgres(
        &self,
        task_id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let reason = reason.to_string();
        conn.interact(move |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Failed"),
                        task_executions::completed_at.eq(Some(now)),
                        task_executions::error_details.eq(format!("ABANDONED: {}", reason)),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with abandonment reason
                let event_data = serde_json::json!({ "reason": reason }).to_string();
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskAbandoned.as_str().to_string(),
                    event_data: Some(event_data),
                    worker_id: None,
                    created_at: now,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn mark_abandoned_sqlite(
        &self,
        task_id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let reason = reason.to_string();
        conn.interact(move |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;

                // Update task status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::status.eq("Failed"),
                        task_executions::completed_at.eq(Some(now)),
                        task_executions::error_details.eq(format!("ABANDONED: {}", reason)),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with abandonment reason
                let event_data = serde_json::json!({ "reason": reason }).to_string();
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskAbandoned.as_str().to_string(),
                    event_data: Some(event_data),
                    worker_id: None,
                    created_at: now,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Updates the sub_status of a running task execution.
    ///
    /// Valid values: `Some("Active")`, `Some("Deferred")`, or `None` to clear.
    ///
    /// This operation is transactional when transitioning to/from Deferred state.
    pub async fn set_sub_status(
        &self,
        task_id: UniversalUuid,
        sub_status: Option<&str>,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.set_sub_status_postgres(task_id, sub_status).await,
            self.set_sub_status_sqlite(task_id, sub_status).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn set_sub_status_postgres(
        &self,
        task_id: UniversalUuid,
        sub_status: Option<&str>,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let sub_status_owned = sub_status.map(|s| s.to_string());
        conn.interact(move |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event (need to check previous sub_status)
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;
                let was_deferred = task.sub_status.as_deref() == Some("Deferred");

                // Update task sub_status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::sub_status.eq(&sub_status_owned),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Emit event only for deferred/resumed transitions
                let event_type = match (was_deferred, sub_status_owned.as_deref()) {
                    (false, Some("Deferred")) => Some(ExecutionEventType::TaskDeferred),
                    (true, Some("Active") | None) => Some(ExecutionEventType::TaskResumed),
                    _ => None,
                };

                if let Some(event_type) = event_type {
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        pipeline_execution_id: task.pipeline_execution_id,
                        task_execution_id: Some(task_id),
                        event_type: event_type.as_str().to_string(),
                        event_data: None,
                        worker_id: None,
                        created_at: now,
                    };
                    diesel::insert_into(execution_events::table)
                        .values(&event)
                        .execute(conn)?;
                }

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn set_sub_status_sqlite(
        &self,
        task_id: UniversalUuid,
        sub_status: Option<&str>,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let sub_status_owned = sub_status.map(|s| s.to_string());
        conn.interact(move |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Get task info for event (need to check previous sub_status)
                let task: UnifiedTaskExecution =
                    task_executions::table.find(task_id).first(conn)?;
                let was_deferred = task.sub_status.as_deref() == Some("Deferred");

                // Update task sub_status
                diesel::update(task_executions::table.find(task_id))
                    .set((
                        task_executions::sub_status.eq(&sub_status_owned),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Emit event only for deferred/resumed transitions
                let event_type = match (was_deferred, sub_status_owned.as_deref()) {
                    (false, Some("Deferred")) => Some(ExecutionEventType::TaskDeferred),
                    (true, Some("Active") | None) => Some(ExecutionEventType::TaskResumed),
                    _ => None,
                };

                if let Some(event_type) = event_type {
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        pipeline_execution_id: task.pipeline_execution_id,
                        task_execution_id: Some(task_id),
                        event_type: event_type.as_str().to_string(),
                        event_data: None,
                        worker_id: None,
                        created_at: now,
                    };
                    diesel::insert_into(execution_events::table)
                        .values(&event)
                        .execute(conn)?;
                }

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Resets the retry state for a task to its initial state.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn reset_retry_state(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.reset_retry_state_postgres(task_id).await,
            self.reset_retry_state_sqlite(task_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn reset_retry_state_postgres(
        &self,
        task_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
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

                // Reset task state
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
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskReset.as_str().to_string(),
                    event_data: None,
                    worker_id: None,
                    created_at: now,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn reset_retry_state_sqlite(
        &self,
        task_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
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

                // Reset task state
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
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_execution_id: Some(task_id),
                    event_type: ExecutionEventType::TaskReset.as_str().to_string(),
                    event_data: None,
                    worker_id: None,
                    created_at: now,
                };
                diesel::insert_into(execution_events::table)
                    .values(&event)
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }
}
