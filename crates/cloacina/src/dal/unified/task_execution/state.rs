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

//! State transition operations for task executions.

use super::TaskExecutionDAL;
use crate::database::schema::unified::task_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use diesel::prelude::*;

impl<'a> TaskExecutionDAL<'a> {
    /// Marks a task execution as completed.
    pub async fn mark_completed(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.mark_completed_postgres(task_id).await,
            self.mark_completed_sqlite(task_id).await
        )
    }

    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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

    #[cfg(feature = "sqlite")]
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

    /// Marks a task as ready for execution.
    pub async fn mark_ready(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.mark_ready_postgres(task_id).await,
            self.mark_ready_sqlite(task_id).await
        )
    }

    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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

    #[cfg(feature = "sqlite")]
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

    /// Marks a task as permanently abandoned after too many recovery attempts.
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

    #[cfg(feature = "sqlite")]
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

    /// Resets the retry state for a task to its initial state.
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

    #[cfg(feature = "sqlite")]
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
