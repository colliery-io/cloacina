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

//! Unified Pipeline Execution DAL with compile-time backend selection
//!
//! All state transitions are transactional: the status update and execution event
//! are written atomically. If either fails, both are rolled back.

use super::models::{
    NewUnifiedExecutionEvent, NewUnifiedPipelineExecution, NewUnifiedPipelineOutbox,
    UnifiedPipelineExecution, UnifiedPipelineOutbox,
};
use super::DAL;
use crate::database::schema::unified::{execution_events, pipeline_executions, pipeline_outbox};
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::execution_event::ExecutionEventType;
use crate::models::pipeline_execution::{NewPipelineExecution, PipelineExecution};
use diesel::prelude::*;

/// Data access layer for pipeline execution operations with compile-time backend selection.
#[derive(Clone)]
pub struct PipelineExecutionDAL<'a> {
    dal: &'a DAL,
}

impl<'a> PipelineExecutionDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new pipeline execution record in the database.
    ///
    /// This operation is transactional: the pipeline record and execution event
    /// are written atomically.
    pub async fn create(
        &self,
        new_execution: NewPipelineExecution,
    ) -> Result<PipelineExecution, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_execution).await,
            self.create_sqlite(new_execution).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn create_postgres(
        &self,
        new_execution: NewPipelineExecution,
    ) -> Result<PipelineExecution, ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let execution: UnifiedPipelineExecution = conn
            .interact(move |conn| {
                conn.transaction::<_, diesel::result::Error, _>(|conn| {
                    let id = UniversalUuid::new_v4();
                    let now = UniversalTimestamp::now();

                    let unified_new = NewUnifiedPipelineExecution {
                        id,
                        pipeline_name: new_execution.pipeline_name,
                        pipeline_version: new_execution.pipeline_version,
                        status: new_execution.status,
                        context_id: new_execution.context_id,
                        started_at: now,
                        created_at: now,
                        updated_at: now,
                    };

                    // Insert pipeline record
                    diesel::insert_into(pipeline_executions::table)
                        .values(&unified_new)
                        .execute(conn)?;

                    // Retrieve the created record
                    let execution: UnifiedPipelineExecution =
                        pipeline_executions::table.find(id).first(conn)?;

                    // Insert execution event for pipeline start
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        pipeline_execution_id: execution.id,
                        task_execution_id: None,
                        event_type: ExecutionEventType::PipelineStarted.as_str().to_string(),
                        event_data: None,
                        worker_id: None,
                        created_at: now,
                    };
                    diesel::insert_into(execution_events::table)
                        .values(&event)
                        .execute(conn)?;

                    Ok(execution)
                })
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(execution.into())
    }

    #[cfg(feature = "sqlite")]
    async fn create_sqlite(
        &self,
        new_execution: NewPipelineExecution,
    ) -> Result<PipelineExecution, ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let execution: UnifiedPipelineExecution = conn
            .interact(move |conn| {
                conn.transaction::<_, diesel::result::Error, _>(|conn| {
                    let id = UniversalUuid::new_v4();
                    let now = UniversalTimestamp::now();

                    let unified_new = NewUnifiedPipelineExecution {
                        id,
                        pipeline_name: new_execution.pipeline_name,
                        pipeline_version: new_execution.pipeline_version,
                        status: new_execution.status,
                        context_id: new_execution.context_id,
                        started_at: now,
                        created_at: now,
                        updated_at: now,
                    };

                    // Insert pipeline record
                    diesel::insert_into(pipeline_executions::table)
                        .values(&unified_new)
                        .execute(conn)?;

                    // Retrieve the created record
                    let execution: UnifiedPipelineExecution =
                        pipeline_executions::table.find(id).first(conn)?;

                    // Insert execution event for pipeline start
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        pipeline_execution_id: execution.id,
                        task_execution_id: None,
                        event_type: ExecutionEventType::PipelineStarted.as_str().to_string(),
                        event_data: None,
                        worker_id: None,
                        created_at: now,
                    };
                    diesel::insert_into(execution_events::table)
                        .values(&event)
                        .execute(conn)?;

                    Ok(execution)
                })
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(execution.into())
    }

    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<PipelineExecution, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_id_postgres(id).await,
            self.get_by_id_sqlite(id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_by_id_postgres(
        &self,
        id: UniversalUuid,
    ) -> Result<PipelineExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let execution: UnifiedPipelineExecution = conn
            .interact(move |conn| pipeline_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(execution.into())
    }

    #[cfg(feature = "sqlite")]
    async fn get_by_id_sqlite(
        &self,
        id: UniversalUuid,
    ) -> Result<PipelineExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let execution: UnifiedPipelineExecution = conn
            .interact(move |conn| pipeline_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(execution.into())
    }

    pub async fn get_active_executions(&self) -> Result<Vec<PipelineExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_active_executions_postgres().await,
            self.get_active_executions_sqlite().await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_active_executions_postgres(
        &self,
    ) -> Result<Vec<PipelineExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let executions: Vec<UnifiedPipelineExecution> = conn
            .interact(move |conn| {
                pipeline_executions::table
                    .filter(pipeline_executions::status.eq_any(vec!["Pending", "Running"]))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(executions.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn get_active_executions_sqlite(
        &self,
    ) -> Result<Vec<PipelineExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let executions: Vec<UnifiedPipelineExecution> = conn
            .interact(move |conn| {
                pipeline_executions::table
                    .filter(pipeline_executions::status.eq_any(vec!["Pending", "Running"]))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(executions.into_iter().map(Into::into).collect())
    }

    pub async fn update_status(
        &self,
        id: UniversalUuid,
        status: &str,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_status_postgres(id, status).await,
            self.update_status_sqlite(id, status).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn update_status_postgres(
        &self,
        id: UniversalUuid,
        status: &str,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let status = status.to_string();
        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id))
                .set((
                    pipeline_executions::status.eq(status),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn update_status_sqlite(
        &self,
        id: UniversalUuid,
        status: &str,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let status = status.to_string();
        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id))
                .set((
                    pipeline_executions::status.eq(status),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Marks a pipeline execution as completed.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn mark_completed(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.mark_completed_postgres(id).await,
            self.mark_completed_sqlite(id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn mark_completed_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
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

                // Update pipeline status
                diesel::update(pipeline_executions::table.find(id))
                    .set((
                        pipeline_executions::status.eq("Completed"),
                        pipeline_executions::completed_at.eq(Some(now)),
                        pipeline_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: id,
                    task_execution_id: None,
                    event_type: ExecutionEventType::PipelineCompleted.as_str().to_string(),
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
    async fn mark_completed_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
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

                // Update pipeline status
                diesel::update(pipeline_executions::table.find(id))
                    .set((
                        pipeline_executions::status.eq("Completed"),
                        pipeline_executions::completed_at.eq(Some(now)),
                        pipeline_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: id,
                    task_execution_id: None,
                    event_type: ExecutionEventType::PipelineCompleted.as_str().to_string(),
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

    pub async fn get_last_version(
        &self,
        pipeline_name: &str,
    ) -> Result<Option<String>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_last_version_postgres(pipeline_name).await,
            self.get_last_version_sqlite(pipeline_name).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_last_version_postgres(
        &self,
        pipeline_name: &str,
    ) -> Result<Option<String>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pipeline_name = pipeline_name.to_string();
        let version: Option<String> = conn
            .interact(move |conn| {
                pipeline_executions::table
                    .filter(pipeline_executions::pipeline_name.eq(pipeline_name))
                    .order(pipeline_executions::started_at.desc())
                    .select(pipeline_executions::pipeline_version)
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(version)
    }

    #[cfg(feature = "sqlite")]
    async fn get_last_version_sqlite(
        &self,
        pipeline_name: &str,
    ) -> Result<Option<String>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pipeline_name = pipeline_name.to_string();
        let version: Option<String> = conn
            .interact(move |conn| {
                pipeline_executions::table
                    .filter(pipeline_executions::pipeline_name.eq(pipeline_name))
                    .order(pipeline_executions::started_at.desc())
                    .select(pipeline_executions::pipeline_version)
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(version)
    }

    /// Marks a pipeline execution as failed with an error reason.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn mark_failed(
        &self,
        id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.mark_failed_postgres(id, reason).await,
            self.mark_failed_sqlite(id, reason).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn mark_failed_postgres(
        &self,
        id: UniversalUuid,
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

                // Update pipeline status
                diesel::update(pipeline_executions::table.find(id))
                    .set((
                        pipeline_executions::status.eq("Failed"),
                        pipeline_executions::completed_at.eq(Some(now)),
                        pipeline_executions::error_details.eq(&reason),
                        pipeline_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with error details
                let event_data = serde_json::json!({ "reason": reason }).to_string();
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: id,
                    task_execution_id: None,
                    event_type: ExecutionEventType::PipelineFailed.as_str().to_string(),
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
        id: UniversalUuid,
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

                // Update pipeline status
                diesel::update(pipeline_executions::table.find(id))
                    .set((
                        pipeline_executions::status.eq("Failed"),
                        pipeline_executions::completed_at.eq(Some(now)),
                        pipeline_executions::error_details.eq(&reason),
                        pipeline_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with error details
                let event_data = serde_json::json!({ "reason": reason }).to_string();
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: id,
                    task_execution_id: None,
                    event_type: ExecutionEventType::PipelineFailed.as_str().to_string(),
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

    pub async fn increment_recovery_attempts(
        &self,
        id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.increment_recovery_attempts_postgres(id).await,
            self.increment_recovery_attempts_sqlite(id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn increment_recovery_attempts_postgres(
        &self,
        id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id))
                .set((
                    pipeline_executions::recovery_attempts
                        .eq(pipeline_executions::recovery_attempts + 1),
                    pipeline_executions::last_recovery_at.eq(Some(now)),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn increment_recovery_attempts_sqlite(
        &self,
        id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id))
                .set((
                    pipeline_executions::recovery_attempts
                        .eq(pipeline_executions::recovery_attempts + 1),
                    pipeline_executions::last_recovery_at.eq(Some(now)),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    pub async fn cancel(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.cancel_postgres(id).await,
            self.cancel_sqlite(id).await
        )
    }

    /// Pauses a running pipeline execution.
    ///
    /// Sets the pipeline status to 'Paused', records the pause timestamp,
    /// and optionally stores a reason for the pause.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn pause(
        &self,
        id: UniversalUuid,
        reason: Option<&str>,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.pause_postgres(id, reason).await,
            self.pause_sqlite(id, reason).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn pause_postgres(
        &self,
        id: UniversalUuid,
        reason: Option<&str>,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let reason = reason.map(|r| r.to_string());
        conn.interact(move |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Update pipeline status
                diesel::update(pipeline_executions::table.find(id))
                    .set((
                        pipeline_executions::status.eq("Paused"),
                        pipeline_executions::paused_at.eq(Some(now)),
                        pipeline_executions::pause_reason.eq(&reason),
                        pipeline_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with pause reason
                let event_data = reason.map(|r| serde_json::json!({ "reason": r }).to_string());
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: id,
                    task_execution_id: None,
                    event_type: ExecutionEventType::PipelinePaused.as_str().to_string(),
                    event_data,
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
    async fn pause_sqlite(
        &self,
        id: UniversalUuid,
        reason: Option<&str>,
    ) -> Result<(), ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let reason = reason.map(|r| r.to_string());
        conn.interact(move |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();

                // Update pipeline status
                diesel::update(pipeline_executions::table.find(id))
                    .set((
                        pipeline_executions::status.eq("Paused"),
                        pipeline_executions::paused_at.eq(Some(now)),
                        pipeline_executions::pause_reason.eq(&reason),
                        pipeline_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with pause reason
                let event_data = reason.map(|r| serde_json::json!({ "reason": r }).to_string());
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: id,
                    task_execution_id: None,
                    event_type: ExecutionEventType::PipelinePaused.as_str().to_string(),
                    event_data,
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

    /// Resumes a paused pipeline execution.
    ///
    /// Sets the pipeline status back to 'Running' and clears the pause metadata.
    ///
    /// This operation is transactional: the status update and execution event
    /// are written atomically.
    pub async fn resume(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.resume_postgres(id).await,
            self.resume_sqlite(id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn resume_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
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

                // Update pipeline status
                diesel::update(pipeline_executions::table.find(id))
                    .set((
                        pipeline_executions::status.eq("Running"),
                        pipeline_executions::paused_at.eq(None::<UniversalTimestamp>),
                        pipeline_executions::pause_reason.eq(None::<String>),
                        pipeline_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: id,
                    task_execution_id: None,
                    event_type: ExecutionEventType::PipelineResumed.as_str().to_string(),
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
    async fn resume_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
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

                // Update pipeline status
                diesel::update(pipeline_executions::table.find(id))
                    .set((
                        pipeline_executions::status.eq("Running"),
                        pipeline_executions::paused_at.eq(None::<UniversalTimestamp>),
                        pipeline_executions::pause_reason.eq(None::<String>),
                        pipeline_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    pipeline_execution_id: id,
                    task_execution_id: None,
                    event_type: ExecutionEventType::PipelineResumed.as_str().to_string(),
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

    #[cfg(feature = "postgres")]
    async fn cancel_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id))
                .set((
                    pipeline_executions::status.eq("Cancelled"),
                    pipeline_executions::completed_at.eq(Some(now)),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn cancel_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id))
                .set((
                    pipeline_executions::status.eq("Cancelled"),
                    pipeline_executions::completed_at.eq(Some(now)),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    pub async fn update_final_context(
        &self,
        id: UniversalUuid,
        final_context_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_final_context_postgres(id, final_context_id)
                .await,
            self.update_final_context_sqlite(id, final_context_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn update_final_context_postgres(
        &self,
        id: UniversalUuid,
        final_context_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id))
                .set((
                    pipeline_executions::context_id.eq(Some(final_context_id)),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn update_final_context_sqlite(
        &self,
        id: UniversalUuid,
        final_context_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id))
                .set((
                    pipeline_executions::context_id.eq(Some(final_context_id)),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    pub async fn list_recent(&self, limit: i64) -> Result<Vec<PipelineExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_recent_postgres(limit).await,
            self.list_recent_sqlite(limit).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_recent_postgres(
        &self,
        limit: i64,
    ) -> Result<Vec<PipelineExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let executions: Vec<UnifiedPipelineExecution> = conn
            .interact(move |conn| {
                pipeline_executions::table
                    .order(pipeline_executions::started_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(executions.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn list_recent_sqlite(
        &self,
        limit: i64,
    ) -> Result<Vec<PipelineExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let executions: Vec<UnifiedPipelineExecution> = conn
            .interact(move |conn| {
                pipeline_executions::table
                    .order(pipeline_executions::started_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(executions.into_iter().map(Into::into).collect())
    }

    // =========================================================================
    // Pipeline Outbox Operations
    // =========================================================================

    /// Inserts a pipeline execution into the outbox for work distribution.
    pub async fn insert_outbox(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.insert_outbox_postgres(pipeline_execution_id).await,
            self.insert_outbox_sqlite(pipeline_execution_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn insert_outbox_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let new_entry = NewUnifiedPipelineOutbox {
            pipeline_execution_id,
        };

        conn.interact(move |conn| {
            diesel::insert_into(pipeline_outbox::table)
                .values(&new_entry)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn insert_outbox_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let new_entry = NewUnifiedPipelineOutbox {
            pipeline_execution_id,
        };

        conn.interact(move |conn| {
            diesel::insert_into(pipeline_outbox::table)
                .values(&new_entry)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Atomically claims up to `limit` pipeline executions from the outbox.
    ///
    /// Uses FOR UPDATE SKIP LOCKED on Postgres to prevent duplicate claiming
    /// across concurrent scheduler instances. Returns the joined pipeline
    /// execution records with status Pending or Running.
    pub async fn claim_pipeline_batch(
        &self,
        limit: i64,
    ) -> Result<Vec<PipelineExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.claim_pipeline_batch_postgres(limit).await,
            self.claim_pipeline_batch_sqlite(limit).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn claim_pipeline_batch_postgres(
        &self,
        limit: i64,
    ) -> Result<Vec<PipelineExecution>, ValidationError> {
        use uuid::Uuid;

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        #[derive(Debug, QueryableByName, Clone)]
        #[diesel(check_for_backend(diesel::pg::Pg))]
        struct PgClaimedPipeline {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Text)]
            pipeline_name: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            pipeline_version: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            status: String,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
            context_id: Option<Uuid>,
            #[diesel(sql_type = diesel::sql_types::Timestamp)]
            started_at: chrono::NaiveDateTime,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamp>)]
            completed_at: Option<chrono::NaiveDateTime>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            error_details: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Integer)]
            recovery_attempts: i32,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamp>)]
            last_recovery_at: Option<chrono::NaiveDateTime>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamp>)]
            paused_at: Option<chrono::NaiveDateTime>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            pause_reason: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Timestamp)]
            created_at: chrono::NaiveDateTime,
            #[diesel(sql_type = diesel::sql_types::Timestamp)]
            updated_at: chrono::NaiveDateTime,
        }

        let pg_results: Vec<PgClaimedPipeline> = conn
            .interact(move |conn| {
                diesel::sql_query(format!(
                    r#"
                    WITH claimed AS (
                        DELETE FROM pipeline_outbox
                        WHERE id IN (
                            SELECT id FROM pipeline_outbox
                            ORDER BY created_at ASC
                            LIMIT {}
                            FOR UPDATE SKIP LOCKED
                        )
                        RETURNING pipeline_execution_id
                    )
                    SELECT pe.*
                    FROM pipeline_executions pe
                    INNER JOIN claimed c ON pe.id = c.pipeline_execution_id
                    WHERE pe.status IN ('Pending', 'Running')
                    "#,
                    limit
                ))
                .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_results
            .into_iter()
            .map(|pg| PipelineExecution {
                id: UniversalUuid(pg.id),
                pipeline_name: pg.pipeline_name,
                pipeline_version: pg.pipeline_version,
                status: pg.status,
                context_id: pg.context_id.map(UniversalUuid),
                started_at: UniversalTimestamp(chrono::DateTime::from_naive_utc_and_offset(
                    pg.started_at,
                    chrono::Utc,
                )),
                completed_at: pg.completed_at.map(|t| {
                    UniversalTimestamp(chrono::DateTime::from_naive_utc_and_offset(t, chrono::Utc))
                }),
                error_details: pg.error_details,
                recovery_attempts: pg.recovery_attempts,
                last_recovery_at: pg.last_recovery_at.map(|t| {
                    UniversalTimestamp(chrono::DateTime::from_naive_utc_and_offset(t, chrono::Utc))
                }),
                paused_at: pg.paused_at.map(|t| {
                    UniversalTimestamp(chrono::DateTime::from_naive_utc_and_offset(t, chrono::Utc))
                }),
                pause_reason: pg.pause_reason,
                created_at: UniversalTimestamp(chrono::DateTime::from_naive_utc_and_offset(
                    pg.created_at,
                    chrono::Utc,
                )),
                updated_at: UniversalTimestamp(chrono::DateTime::from_naive_utc_and_offset(
                    pg.updated_at,
                    chrono::Utc,
                )),
            })
            .collect())
    }

    #[cfg(feature = "sqlite")]
    async fn claim_pipeline_batch_sqlite(
        &self,
        limit: i64,
    ) -> Result<Vec<PipelineExecution>, ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let executions: Vec<UnifiedPipelineExecution> = conn
            .interact(
                move |conn| -> Result<Vec<UnifiedPipelineExecution>, diesel::result::Error> {
                    // Use IMMEDIATE transaction to acquire write lock immediately
                    conn.transaction::<Vec<UnifiedPipelineExecution>, diesel::result::Error, _>(
                        |conn| {
                            // Select oldest outbox entries
                            let outbox_entries: Vec<UnifiedPipelineOutbox> = pipeline_outbox::table
                                .order(pipeline_outbox::created_at.asc())
                                .limit(limit)
                                .load(conn)?;

                            if outbox_entries.is_empty() {
                                return Ok(Vec::new());
                            }

                            // Collect IDs
                            let pipeline_ids: Vec<_> = outbox_entries
                                .iter()
                                .map(|o| o.pipeline_execution_id)
                                .collect();
                            let outbox_ids: Vec<_> = outbox_entries.iter().map(|o| o.id).collect();

                            // Delete outbox entries
                            diesel::delete(pipeline_outbox::table)
                                .filter(pipeline_outbox::id.eq_any(&outbox_ids))
                                .execute(conn)?;

                            // Load pipeline executions for the claimed entries
                            let claimed: Vec<UnifiedPipelineExecution> = pipeline_executions::table
                                .filter(pipeline_executions::id.eq_any(&pipeline_ids))
                                .filter(
                                    pipeline_executions::status.eq_any(vec!["Pending", "Running"]),
                                )
                                .load(conn)?;

                            Ok(claimed)
                        },
                    )
                },
            )
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(executions.into_iter().map(Into::into).collect())
    }

    /// Re-inserts a pipeline into the outbox for continued processing.
    ///
    /// Used after processing a pipeline that is still active (Pending/Running)
    /// to ensure it gets picked up again in the next scheduling cycle.
    pub async fn requeue_pipeline(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        self.insert_outbox(pipeline_execution_id).await
    }
}
