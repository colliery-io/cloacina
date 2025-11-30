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

//! Unified Recovery Event DAL with runtime backend selection
//!
//! This module provides CRUD operations for RecoveryEvent entities that work with
//! both PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.

use super::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::database::BackendType;
use crate::error::ValidationError;
use crate::models::recovery_event::{NewRecoveryEvent, RecoveryEvent, RecoveryType};
use diesel::prelude::*;

/// Data access layer for recovery event operations with runtime backend selection.
///
/// This DAL provides methods for creating and querying recovery events,
/// which track recovery operations performed on tasks and pipelines.
#[derive(Clone)]
pub struct RecoveryEventDAL<'a> {
    dal: &'a DAL,
}

impl<'a> RecoveryEventDAL<'a> {
    /// Creates a new RecoveryEventDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new recovery event record.
    pub async fn create(
        &self,
        new_event: NewRecoveryEvent,
    ) -> Result<RecoveryEvent, ValidationError> {
        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => self.create_postgres(new_event).await,
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => self.create_sqlite(new_event).await,
        }
    }

    #[cfg(feature = "postgres")]
    async fn create_postgres(
        &self,
        new_event: NewRecoveryEvent,
    ) -> Result<RecoveryEvent, ValidationError> {
        use crate::database::schema::postgres::recovery_events;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result = conn
            .interact(move |conn| {
                diesel::insert_into(recovery_events::table)
                    .values(&new_event)
                    .returning(RecoveryEvent::as_returning())
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result)
    }

    #[cfg(feature = "sqlite")]
    async fn create_sqlite(
        &self,
        new_event: NewRecoveryEvent,
    ) -> Result<RecoveryEvent, ValidationError> {
        use crate::database::schema::sqlite::recovery_events;
        use crate::database::universal_types::current_timestamp;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        // For SQLite, generate UUID and timestamps client-side
        let id = UniversalUuid::new_v4();
        let now = current_timestamp();

        conn.interact(move |conn| {
            diesel::insert_into(recovery_events::table)
                .values((
                    recovery_events::id.eq(&id),
                    recovery_events::pipeline_execution_id.eq(&new_event.pipeline_execution_id),
                    recovery_events::task_execution_id.eq(&new_event.task_execution_id),
                    recovery_events::recovery_type.eq(&new_event.recovery_type),
                    recovery_events::recovered_at.eq(&now),
                    recovery_events::details.eq(&new_event.details),
                    recovery_events::created_at.eq(&now),
                    recovery_events::updated_at.eq(&now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        // Retrieve the inserted record
        let result = conn
            .interact(move |conn| recovery_events::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result)
    }

    /// Gets all recovery events for a specific pipeline execution.
    pub async fn get_by_pipeline(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => self.get_by_pipeline_postgres(pipeline_execution_id).await,
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => self.get_by_pipeline_sqlite(pipeline_execution_id).await,
        }
    }

    #[cfg(feature = "postgres")]
    async fn get_by_pipeline_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::database::schema::postgres::recovery_events;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let events = conn
            .interact(move |conn| {
                recovery_events::table
                    .filter(recovery_events::pipeline_execution_id.eq(pipeline_execution_id.0))
                    .order(recovery_events::recovered_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(events)
    }

    #[cfg(feature = "sqlite")]
    async fn get_by_pipeline_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::database::schema::sqlite::recovery_events;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let events = conn
            .interact(move |conn| {
                recovery_events::table
                    .filter(recovery_events::pipeline_execution_id.eq(pipeline_execution_id))
                    .order(recovery_events::recovered_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(events)
    }

    /// Gets all recovery events for a specific task execution.
    pub async fn get_by_task(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => self.get_by_task_postgres(task_execution_id).await,
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => self.get_by_task_sqlite(task_execution_id).await,
        }
    }

    #[cfg(feature = "postgres")]
    async fn get_by_task_postgres(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::database::schema::postgres::recovery_events;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let events = conn
            .interact(move |conn| {
                recovery_events::table
                    .filter(recovery_events::task_execution_id.eq(task_execution_id.0))
                    .order(recovery_events::recovered_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(events)
    }

    #[cfg(feature = "sqlite")]
    async fn get_by_task_sqlite(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::database::schema::sqlite::recovery_events;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let events = conn
            .interact(move |conn| {
                recovery_events::table
                    .filter(recovery_events::task_execution_id.eq(task_execution_id))
                    .order(recovery_events::recovered_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(events)
    }

    /// Gets recovery events by type for monitoring and analysis.
    pub async fn get_by_type(
        &self,
        recovery_type: &str,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => self.get_by_type_postgres(recovery_type).await,
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => self.get_by_type_sqlite(recovery_type).await,
        }
    }

    #[cfg(feature = "postgres")]
    async fn get_by_type_postgres(
        &self,
        recovery_type: &str,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::database::schema::postgres::recovery_events;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let recovery_type = recovery_type.to_string();
        let events = conn
            .interact(move |conn| {
                recovery_events::table
                    .filter(recovery_events::recovery_type.eq(recovery_type))
                    .order(recovery_events::recovered_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(events)
    }

    #[cfg(feature = "sqlite")]
    async fn get_by_type_sqlite(
        &self,
        recovery_type: &str,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::database::schema::sqlite::recovery_events;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let recovery_type = recovery_type.to_string();
        let events = conn
            .interact(move |conn| {
                recovery_events::table
                    .filter(recovery_events::recovery_type.eq(recovery_type))
                    .order(recovery_events::recovered_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(events)
    }

    /// Gets all workflow unavailability events for monitoring unknown workflow cleanup.
    pub async fn get_workflow_unavailable_events(
        &self,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        self.get_by_type(&RecoveryType::WorkflowUnavailable.as_str())
            .await
    }

    /// Gets recent recovery events for monitoring purposes.
    pub async fn get_recent(&self, limit: i64) -> Result<Vec<RecoveryEvent>, ValidationError> {
        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => self.get_recent_postgres(limit).await,
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => self.get_recent_sqlite(limit).await,
        }
    }

    #[cfg(feature = "postgres")]
    async fn get_recent_postgres(
        &self,
        limit: i64,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::database::schema::postgres::recovery_events;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let events = conn
            .interact(move |conn| {
                recovery_events::table
                    .order(recovery_events::recovered_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(events)
    }

    #[cfg(feature = "sqlite")]
    async fn get_recent_sqlite(&self, limit: i64) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::database::schema::sqlite::recovery_events;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let events = conn
            .interact(move |conn| {
                recovery_events::table
                    .order(recovery_events::recovered_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(events)
    }
}
