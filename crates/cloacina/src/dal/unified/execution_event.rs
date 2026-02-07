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

//! Unified Execution Event DAL with runtime backend selection
//!
//! This module provides CRUD operations for ExecutionEvent entities that work with
//! both PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.
//!
//! Execution events form an append-only audit trail of all task and pipeline
//! state transitions for debugging, compliance, and replay capability.

use super::models::{NewUnifiedExecutionEvent, UnifiedExecutionEvent};
use super::DAL;
use crate::database::schema::unified::execution_events;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::execution_event::{ExecutionEvent, ExecutionEventType, NewExecutionEvent};
use diesel::prelude::*;

/// Data access layer for execution event operations with runtime backend selection.
///
/// This DAL provides methods for creating and querying execution events,
/// which track all state transitions for tasks and pipelines.
#[derive(Clone)]
pub struct ExecutionEventDAL<'a> {
    dal: &'a DAL,
}

impl<'a> ExecutionEventDAL<'a> {
    /// Creates a new ExecutionEventDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new execution event record.
    ///
    /// Events are append-only and never updated after creation. Each event
    /// receives a monotonically increasing sequence number for ordering.
    pub async fn create(
        &self,
        new_event: NewExecutionEvent,
    ) -> Result<ExecutionEvent, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_event).await,
            self.create_sqlite(new_event).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn create_postgres(
        &self,
        new_event: NewExecutionEvent,
    ) -> Result<ExecutionEvent, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedExecutionEvent {
            id,
            pipeline_execution_id: new_event.pipeline_execution_id,
            task_execution_id: new_event.task_execution_id,
            event_type: new_event.event_type,
            event_data: new_event.event_data,
            worker_id: new_event.worker_id,
            created_at: now,
        };

        let result: UnifiedExecutionEvent = conn
            .interact(move |conn| {
                diesel::insert_into(execution_events::table)
                    .values(&new_unified)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    async fn create_sqlite(
        &self,
        new_event: NewExecutionEvent,
    ) -> Result<ExecutionEvent, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedExecutionEvent {
            id,
            pipeline_execution_id: new_event.pipeline_execution_id,
            task_execution_id: new_event.task_execution_id,
            event_type: new_event.event_type,
            event_data: new_event.event_data,
            worker_id: new_event.worker_id,
            created_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(execution_events::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        // SQLite doesn't support RETURNING, so we need to fetch the inserted row
        let result: UnifiedExecutionEvent = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::id.eq(id))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    /// Gets all execution events for a specific pipeline execution, ordered by sequence.
    pub async fn list_by_pipeline(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_by_pipeline_postgres(pipeline_execution_id).await,
            self.list_by_pipeline_sqlite(pipeline_execution_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_by_pipeline_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::pipeline_execution_id.eq(pipeline_execution_id))
                    .order(execution_events::sequence_num.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn list_by_pipeline_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::pipeline_execution_id.eq(pipeline_execution_id))
                    .order(execution_events::sequence_num.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Gets all execution events for a specific task execution, ordered by sequence.
    pub async fn list_by_task(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_by_task_postgres(task_execution_id).await,
            self.list_by_task_sqlite(task_execution_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_by_task_postgres(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::task_execution_id.eq(task_execution_id))
                    .order(execution_events::sequence_num.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn list_by_task_sqlite(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::task_execution_id.eq(task_execution_id))
                    .order(execution_events::sequence_num.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Gets execution events by type for monitoring and analysis.
    pub async fn list_by_type(
        &self,
        event_type: ExecutionEventType,
        limit: i64,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_by_type_postgres(event_type, limit).await,
            self.list_by_type_sqlite(event_type, limit).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_by_type_postgres(
        &self,
        event_type: ExecutionEventType,
        limit: i64,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let event_type_str = event_type.as_str().to_string();
        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::event_type.eq(event_type_str))
                    .order(execution_events::created_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn list_by_type_sqlite(
        &self,
        event_type: ExecutionEventType,
        limit: i64,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let event_type_str = event_type.as_str().to_string();
        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::event_type.eq(event_type_str))
                    .order(execution_events::created_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Gets recent execution events for monitoring purposes.
    pub async fn get_recent(&self, limit: i64) -> Result<Vec<ExecutionEvent>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_recent_postgres(limit).await,
            self.get_recent_sqlite(limit).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_recent_postgres(
        &self,
        limit: i64,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .order(execution_events::created_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn get_recent_sqlite(&self, limit: i64) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .order(execution_events::created_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Deletes execution events older than the specified timestamp.
    ///
    /// Used for retention policy enforcement to prevent unbounded table growth.
    /// Returns the number of deleted events.
    pub async fn delete_older_than(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<usize, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.delete_older_than_postgres(cutoff).await,
            self.delete_older_than_sqlite(cutoff).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn delete_older_than_postgres(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let deleted: usize = conn
            .interact(move |conn| {
                diesel::delete(
                    execution_events::table.filter(execution_events::created_at.lt(cutoff)),
                )
                .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(deleted)
    }

    #[cfg(feature = "sqlite")]
    async fn delete_older_than_sqlite(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let deleted: usize = conn
            .interact(move |conn| {
                diesel::delete(
                    execution_events::table.filter(execution_events::created_at.lt(cutoff)),
                )
                .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(deleted)
    }

    /// Counts total execution events for a pipeline.
    pub async fn count_by_pipeline(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<i64, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.count_by_pipeline_postgres(pipeline_execution_id).await,
            self.count_by_pipeline_sqlite(pipeline_execution_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn count_by_pipeline_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count: i64 = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::pipeline_execution_id.eq(pipeline_execution_id))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }

    #[cfg(feature = "sqlite")]
    async fn count_by_pipeline_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count: i64 = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::pipeline_execution_id.eq(pipeline_execution_id))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }

    /// Counts execution events older than the specified timestamp.
    ///
    /// Used for dry-run mode to preview how many events would be deleted.
    pub async fn count_older_than(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<i64, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.count_older_than_postgres(cutoff).await,
            self.count_older_than_sqlite(cutoff).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn count_older_than_postgres(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count: i64 = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::created_at.lt(cutoff))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }

    #[cfg(feature = "sqlite")]
    async fn count_older_than_sqlite(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count: i64 = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::created_at.lt(cutoff))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }
}
