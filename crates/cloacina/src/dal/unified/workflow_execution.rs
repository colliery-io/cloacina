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

//! Unified Workflow Execution DAL with compile-time backend selection
//!
//! All state transitions are transactional: the status update and execution event
//! are written atomically. If either fails, both are rolled back.

use super::models::{
    NewUnifiedExecutionEvent, NewUnifiedWorkflowExecution, UnifiedWorkflowExecution,
};
use super::DAL;
use crate::database::schema::unified::{execution_events, workflow_executions};
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::execution_event::ExecutionEventType;
use crate::models::workflow_execution::{NewWorkflowExecution, WorkflowExecutionRecord};
use diesel::prelude::*;

/// Data access layer for workflow execution operations with compile-time backend selection.
#[derive(Clone)]
pub struct WorkflowExecutionDAL<'a> {
    dal: &'a DAL,
}

impl<'a> WorkflowExecutionDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new workflow execution record in the database.
    ///
    /// This operation is transactional: the workflow record and execution event
    /// are written atomically.
    pub async fn create(
        &self,
        new_execution: NewWorkflowExecution,
    ) -> Result<WorkflowExecutionRecord, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_execution).await,
            self.create_sqlite(new_execution).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn create_postgres(
        &self,
        new_execution: NewWorkflowExecution,
    ) -> Result<WorkflowExecutionRecord, ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let execution: UnifiedWorkflowExecution = conn
            .interact(move |conn| {
                conn.transaction::<_, diesel::result::Error, _>(|conn| {
                    let id = UniversalUuid::new_v4();
                    let now = UniversalTimestamp::now();

                    let unified_new = NewUnifiedWorkflowExecution {
                        id,
                        workflow_name: new_execution.workflow_name,
                        workflow_version: new_execution.workflow_version,
                        status: new_execution.status,
                        context_id: new_execution.context_id,
                        started_at: now,
                        created_at: now,
                        updated_at: now,
                    };

                    // Insert workflow record
                    diesel::insert_into(workflow_executions::table)
                        .values(&unified_new)
                        .execute(conn)?;

                    // Retrieve the created record
                    let execution: UnifiedWorkflowExecution =
                        workflow_executions::table.find(id).first(conn)?;

                    // Insert execution event for workflow start
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        workflow_execution_id: execution.id,
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
        new_execution: NewWorkflowExecution,
    ) -> Result<WorkflowExecutionRecord, ValidationError> {
        use diesel::connection::Connection;

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let execution: UnifiedWorkflowExecution = conn
            .interact(move |conn| {
                conn.transaction::<_, diesel::result::Error, _>(|conn| {
                    let id = UniversalUuid::new_v4();
                    let now = UniversalTimestamp::now();

                    let unified_new = NewUnifiedWorkflowExecution {
                        id,
                        workflow_name: new_execution.workflow_name,
                        workflow_version: new_execution.workflow_version,
                        status: new_execution.status,
                        context_id: new_execution.context_id,
                        started_at: now,
                        created_at: now,
                        updated_at: now,
                    };

                    // Insert workflow record
                    diesel::insert_into(workflow_executions::table)
                        .values(&unified_new)
                        .execute(conn)?;

                    // Retrieve the created record
                    let execution: UnifiedWorkflowExecution =
                        workflow_executions::table.find(id).first(conn)?;

                    // Insert execution event for workflow start
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        workflow_execution_id: execution.id,
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

    pub async fn get_by_id(
        &self,
        id: UniversalUuid,
    ) -> Result<WorkflowExecutionRecord, ValidationError> {
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
    ) -> Result<WorkflowExecutionRecord, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let execution: UnifiedWorkflowExecution = conn
            .interact(move |conn| workflow_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(execution.into())
    }

    #[cfg(feature = "sqlite")]
    async fn get_by_id_sqlite(
        &self,
        id: UniversalUuid,
    ) -> Result<WorkflowExecutionRecord, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let execution: UnifiedWorkflowExecution = conn
            .interact(move |conn| workflow_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(execution.into())
    }

    pub async fn get_active_executions(
        &self,
    ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_active_executions_postgres().await,
            self.get_active_executions_sqlite().await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_active_executions_postgres(
        &self,
    ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let executions: Vec<UnifiedWorkflowExecution> = conn
            .interact(move |conn| {
                workflow_executions::table
                    .filter(workflow_executions::status.eq_any(vec!["Pending", "Running"]))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(executions.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn get_active_executions_sqlite(
        &self,
    ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let executions: Vec<UnifiedWorkflowExecution> = conn
            .interact(move |conn| {
                workflow_executions::table
                    .filter(workflow_executions::status.eq_any(vec!["Pending", "Running"]))
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
            diesel::update(workflow_executions::table.find(id))
                .set((
                    workflow_executions::status.eq(status),
                    workflow_executions::updated_at.eq(now),
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
            diesel::update(workflow_executions::table.find(id))
                .set((
                    workflow_executions::status.eq(status),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Marks a workflow execution as completed.
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

                // Only transition from Running to Completed — prevents duplicate
                // PipelineCompleted events when two scheduler ticks race.
                let rows = diesel::update(
                    workflow_executions::table
                        .find(id)
                        .filter(workflow_executions::status.ne_all(vec!["Completed", "Failed"])),
                )
                .set((
                    workflow_executions::status.eq("Completed"),
                    workflow_executions::completed_at.eq(Some(now)),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)?;

                // Only insert event if we actually transitioned
                if rows > 0 {
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        workflow_execution_id: id,
                        task_execution_id: None,
                        event_type: ExecutionEventType::PipelineCompleted.as_str().to_string(),
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

                // Only transition from Running to Completed — prevents duplicate
                // PipelineCompleted events when two scheduler ticks race.
                let rows = diesel::update(
                    workflow_executions::table
                        .find(id)
                        .filter(workflow_executions::status.ne_all(vec!["Completed", "Failed"])),
                )
                .set((
                    workflow_executions::status.eq("Completed"),
                    workflow_executions::completed_at.eq(Some(now)),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)?;

                // Only insert event if we actually transitioned
                if rows > 0 {
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        workflow_execution_id: id,
                        task_execution_id: None,
                        event_type: ExecutionEventType::PipelineCompleted.as_str().to_string(),
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

    pub async fn get_last_version(
        &self,
        workflow_name: &str,
    ) -> Result<Option<String>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_last_version_postgres(workflow_name).await,
            self.get_last_version_sqlite(workflow_name).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_last_version_postgres(
        &self,
        workflow_name: &str,
    ) -> Result<Option<String>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let workflow_name = workflow_name.to_string();
        let version: Option<String> = conn
            .interact(move |conn| {
                workflow_executions::table
                    .filter(workflow_executions::workflow_name.eq(workflow_name))
                    .order(workflow_executions::started_at.desc())
                    .select(workflow_executions::workflow_version)
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
        workflow_name: &str,
    ) -> Result<Option<String>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let workflow_name = workflow_name.to_string();
        let version: Option<String> = conn
            .interact(move |conn| {
                workflow_executions::table
                    .filter(workflow_executions::workflow_name.eq(workflow_name))
                    .order(workflow_executions::started_at.desc())
                    .select(workflow_executions::workflow_version)
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(version)
    }

    /// Marks a workflow execution as failed with an error reason.
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

                // Only transition from Running to Failed — prevents duplicate
                // PipelineFailed events when two scheduler ticks race.
                let rows = diesel::update(
                    workflow_executions::table
                        .find(id)
                        .filter(workflow_executions::status.ne_all(vec!["Completed", "Failed"])),
                )
                .set((
                    workflow_executions::status.eq("Failed"),
                    workflow_executions::completed_at.eq(Some(now)),
                    workflow_executions::error_details.eq(&reason),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)?;

                // Only insert event if we actually transitioned
                if rows > 0 {
                    let event_data = serde_json::json!({ "reason": reason }).to_string();
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        workflow_execution_id: id,
                        task_execution_id: None,
                        event_type: ExecutionEventType::PipelineFailed.as_str().to_string(),
                        event_data: Some(event_data),
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

                // Only transition from Running to Failed — prevents duplicate
                // PipelineFailed events when two scheduler ticks race.
                let rows = diesel::update(
                    workflow_executions::table
                        .find(id)
                        .filter(workflow_executions::status.ne_all(vec!["Completed", "Failed"])),
                )
                .set((
                    workflow_executions::status.eq("Failed"),
                    workflow_executions::completed_at.eq(Some(now)),
                    workflow_executions::error_details.eq(&reason),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)?;

                // Only insert event if we actually transitioned
                if rows > 0 {
                    let event_data = serde_json::json!({ "reason": reason }).to_string();
                    let event = NewUnifiedExecutionEvent {
                        id: UniversalUuid::new_v4(),
                        workflow_execution_id: id,
                        task_execution_id: None,
                        event_type: ExecutionEventType::PipelineFailed.as_str().to_string(),
                        event_data: Some(event_data),
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
            diesel::update(workflow_executions::table.find(id))
                .set((
                    workflow_executions::recovery_attempts
                        .eq(workflow_executions::recovery_attempts + 1),
                    workflow_executions::last_recovery_at.eq(Some(now)),
                    workflow_executions::updated_at.eq(now),
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
            diesel::update(workflow_executions::table.find(id))
                .set((
                    workflow_executions::recovery_attempts
                        .eq(workflow_executions::recovery_attempts + 1),
                    workflow_executions::last_recovery_at.eq(Some(now)),
                    workflow_executions::updated_at.eq(now),
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

    /// Pauses a running workflow execution.
    ///
    /// Sets the workflow status to 'Paused', records the pause timestamp,
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

                // Update workflow status
                diesel::update(workflow_executions::table.find(id))
                    .set((
                        workflow_executions::status.eq("Paused"),
                        workflow_executions::paused_at.eq(Some(now)),
                        workflow_executions::pause_reason.eq(&reason),
                        workflow_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with pause reason
                let event_data = reason.map(|r| serde_json::json!({ "reason": r }).to_string());
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: id,
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

                // Update workflow status
                diesel::update(workflow_executions::table.find(id))
                    .set((
                        workflow_executions::status.eq("Paused"),
                        workflow_executions::paused_at.eq(Some(now)),
                        workflow_executions::pause_reason.eq(&reason),
                        workflow_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event with pause reason
                let event_data = reason.map(|r| serde_json::json!({ "reason": r }).to_string());
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: id,
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

    /// Resumes a paused workflow execution.
    ///
    /// Sets the workflow status back to 'Running' and clears the pause metadata.
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

                // Update workflow status
                diesel::update(workflow_executions::table.find(id))
                    .set((
                        workflow_executions::status.eq("Running"),
                        workflow_executions::paused_at.eq(None::<UniversalTimestamp>),
                        workflow_executions::pause_reason.eq(None::<String>),
                        workflow_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: id,
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

                // Update workflow status
                diesel::update(workflow_executions::table.find(id))
                    .set((
                        workflow_executions::status.eq("Running"),
                        workflow_executions::paused_at.eq(None::<UniversalTimestamp>),
                        workflow_executions::pause_reason.eq(None::<String>),
                        workflow_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Insert execution event
                let event = NewUnifiedExecutionEvent {
                    id: UniversalUuid::new_v4(),
                    workflow_execution_id: id,
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
            diesel::update(workflow_executions::table.find(id))
                .set((
                    workflow_executions::status.eq("Cancelled"),
                    workflow_executions::completed_at.eq(Some(now)),
                    workflow_executions::updated_at.eq(now),
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
            diesel::update(workflow_executions::table.find(id))
                .set((
                    workflow_executions::status.eq("Cancelled"),
                    workflow_executions::completed_at.eq(Some(now)),
                    workflow_executions::updated_at.eq(now),
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
            diesel::update(workflow_executions::table.find(id))
                .set((
                    workflow_executions::context_id.eq(Some(final_context_id)),
                    workflow_executions::updated_at.eq(now),
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
            diesel::update(workflow_executions::table.find(id))
                .set((
                    workflow_executions::context_id.eq(Some(final_context_id)),
                    workflow_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    pub async fn list_recent(
        &self,
        limit: i64,
    ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError> {
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
    ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let executions: Vec<UnifiedWorkflowExecution> = conn
            .interact(move |conn| {
                workflow_executions::table
                    .order(workflow_executions::started_at.desc())
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
    ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let executions: Vec<UnifiedWorkflowExecution> = conn
            .interact(move |conn| {
                workflow_executions::table
                    .order(workflow_executions::started_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(executions.into_iter().map(Into::into).collect())
    }
}
