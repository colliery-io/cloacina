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

//! Unified Pipeline Execution DAL with compile-time backend selection

use super::models::{NewUnifiedPipelineExecution, UnifiedPipelineExecution};
use super::DAL;
use crate::database::schema::unified::pipeline_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
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
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

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

        conn.interact(move |conn| {
            diesel::insert_into(pipeline_executions::table)
                .values(&unified_new)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        self.get_by_id_postgres(id).await
    }

    #[cfg(feature = "sqlite")]
    async fn create_sqlite(
        &self,
        new_execution: NewPipelineExecution,
    ) -> Result<PipelineExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

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

        conn.interact(move |conn| {
            diesel::insert_into(pipeline_executions::table)
                .values(&unified_new)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        // IMPORTANT: Drop connection before calling get_by_id_sqlite to avoid deadlock
        // SQLite pool has size 1, so we must return this connection before acquiring another
        drop(conn);

        self.get_by_id_sqlite(id).await
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

    pub async fn mark_completed(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.mark_completed_postgres(id).await,
            self.mark_completed_sqlite(id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn mark_completed_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
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
                    pipeline_executions::status.eq("Completed"),
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
    async fn mark_completed_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
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
                    pipeline_executions::status.eq("Completed"),
                    pipeline_executions::completed_at.eq(Some(now)),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
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
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let reason = reason.to_string();
        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id))
                .set((
                    pipeline_executions::status.eq("Failed"),
                    pipeline_executions::completed_at.eq(Some(now)),
                    pipeline_executions::error_details.eq(reason),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
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
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let reason = reason.to_string();
        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id))
                .set((
                    pipeline_executions::status.eq("Failed"),
                    pipeline_executions::completed_at.eq(Some(now)),
                    pipeline_executions::error_details.eq(reason),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
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
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let reason = reason.map(|r| r.to_string());
        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id))
                .set((
                    pipeline_executions::status.eq("Paused"),
                    pipeline_executions::paused_at.eq(Some(now)),
                    pipeline_executions::pause_reason.eq(reason),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
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
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let reason = reason.map(|r| r.to_string());
        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id))
                .set((
                    pipeline_executions::status.eq("Paused"),
                    pipeline_executions::paused_at.eq(Some(now)),
                    pipeline_executions::pause_reason.eq(reason),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Resumes a paused pipeline execution.
    ///
    /// Sets the pipeline status back to 'Running' and clears the pause metadata.
    pub async fn resume(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.resume_postgres(id).await,
            self.resume_sqlite(id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn resume_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
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
                    pipeline_executions::status.eq("Running"),
                    pipeline_executions::paused_at.eq(None::<UniversalTimestamp>),
                    pipeline_executions::pause_reason.eq(None::<String>),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn resume_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
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
                    pipeline_executions::status.eq("Running"),
                    pipeline_executions::paused_at.eq(None::<UniversalTimestamp>),
                    pipeline_executions::pause_reason.eq(None::<String>),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
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
}
