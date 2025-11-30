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
            BackendType::Postgres => self.create_postgres(new_event).await,
            BackendType::Sqlite => self.create_sqlite(new_event).await,
        }
    }

    async fn create_postgres(
        &self,
        new_event: NewRecoveryEvent,
    ) -> Result<RecoveryEvent, ValidationError> {
        use crate::dal::postgres_dal::models::{NewPgRecoveryEvent, PgRecoveryEvent};
        use crate::database::schema::postgres::recovery_events;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pg_new = NewPgRecoveryEvent {
            pipeline_execution_id: new_event.pipeline_execution_id.0,
            task_execution_id: new_event.task_execution_id.map(|u| u.0),
            recovery_type: new_event.recovery_type,
            details: new_event.details,
        };

        let pg_result: PgRecoveryEvent = conn
            .interact(move |conn| {
                diesel::insert_into(recovery_events::table)
                    .values(&pg_new)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_result.into())
    }

    async fn create_sqlite(
        &self,
        new_event: NewRecoveryEvent,
    ) -> Result<RecoveryEvent, ValidationError> {
        use crate::dal::sqlite_dal::models::{
            current_timestamp_string, uuid_to_blob, NewSqliteRecoveryEvent, SqliteRecoveryEvent,
        };
        use crate::database::schema::sqlite::recovery_events;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        // For SQLite, generate UUID and timestamps client-side
        let id = UniversalUuid::new_v4();
        let id_blob = uuid_to_blob(&id.0);
        let now = current_timestamp_string();

        let sqlite_new = NewSqliteRecoveryEvent {
            id: id_blob.clone(),
            pipeline_execution_id: uuid_to_blob(&new_event.pipeline_execution_id.0),
            task_execution_id: new_event.task_execution_id.map(|u| uuid_to_blob(&u.0)),
            recovery_type: new_event.recovery_type,
            recovered_at: now.clone(),
            details: new_event.details,
            created_at: now.clone(),
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(recovery_events::table)
                .values(&sqlite_new)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        // Retrieve the inserted record
        let sqlite_result: SqliteRecoveryEvent = conn
            .interact(move |conn| recovery_events::table.find(id_blob).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_result.into())
    }

    /// Gets all recovery events for a specific pipeline execution.
    pub async fn get_by_pipeline(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.get_by_pipeline_postgres(pipeline_execution_id).await,
            BackendType::Sqlite => self.get_by_pipeline_sqlite(pipeline_execution_id).await,
        }
    }

    async fn get_by_pipeline_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::dal::postgres_dal::models::PgRecoveryEvent;
        use crate::database::schema::postgres::recovery_events;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = pipeline_execution_id.0;
        let pg_events: Vec<PgRecoveryEvent> = conn
            .interact(move |conn| {
                recovery_events::table
                    .filter(recovery_events::pipeline_execution_id.eq(uuid_id))
                    .order(recovery_events::recovered_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_events.into_iter().map(Into::into).collect())
    }

    async fn get_by_pipeline_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::dal::sqlite_dal::models::{uuid_to_blob, SqliteRecoveryEvent};
        use crate::database::schema::sqlite::recovery_events;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pipeline_blob = uuid_to_blob(&pipeline_execution_id.0);
        let sqlite_events: Vec<SqliteRecoveryEvent> = conn
            .interact(move |conn| {
                recovery_events::table
                    .filter(recovery_events::pipeline_execution_id.eq(pipeline_blob))
                    .order(recovery_events::recovered_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_events.into_iter().map(Into::into).collect())
    }

    /// Gets all recovery events for a specific task execution.
    pub async fn get_by_task(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.get_by_task_postgres(task_execution_id).await,
            BackendType::Sqlite => self.get_by_task_sqlite(task_execution_id).await,
        }
    }

    async fn get_by_task_postgres(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::dal::postgres_dal::models::PgRecoveryEvent;
        use crate::database::schema::postgres::recovery_events;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = task_execution_id.0;
        let pg_events: Vec<PgRecoveryEvent> = conn
            .interact(move |conn| {
                recovery_events::table
                    .filter(recovery_events::task_execution_id.eq(uuid_id))
                    .order(recovery_events::recovered_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_events.into_iter().map(Into::into).collect())
    }

    async fn get_by_task_sqlite(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::dal::sqlite_dal::models::{uuid_to_blob, SqliteRecoveryEvent};
        use crate::database::schema::sqlite::recovery_events;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task_blob = uuid_to_blob(&task_execution_id.0);
        let sqlite_events: Vec<SqliteRecoveryEvent> = conn
            .interact(move |conn| {
                recovery_events::table
                    .filter(recovery_events::task_execution_id.eq(task_blob))
                    .order(recovery_events::recovered_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_events.into_iter().map(Into::into).collect())
    }

    /// Gets recovery events by type for monitoring and analysis.
    pub async fn get_by_type(
        &self,
        recovery_type: &str,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.get_by_type_postgres(recovery_type).await,
            BackendType::Sqlite => self.get_by_type_sqlite(recovery_type).await,
        }
    }

    async fn get_by_type_postgres(
        &self,
        recovery_type: &str,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::dal::postgres_dal::models::PgRecoveryEvent;
        use crate::database::schema::postgres::recovery_events;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let recovery_type = recovery_type.to_string();
        let pg_events: Vec<PgRecoveryEvent> = conn
            .interact(move |conn| {
                recovery_events::table
                    .filter(recovery_events::recovery_type.eq(recovery_type))
                    .order(recovery_events::recovered_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_events.into_iter().map(Into::into).collect())
    }

    async fn get_by_type_sqlite(
        &self,
        recovery_type: &str,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::dal::sqlite_dal::models::SqliteRecoveryEvent;
        use crate::database::schema::sqlite::recovery_events;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let recovery_type = recovery_type.to_string();
        let sqlite_events: Vec<SqliteRecoveryEvent> = conn
            .interact(move |conn| {
                recovery_events::table
                    .filter(recovery_events::recovery_type.eq(recovery_type))
                    .order(recovery_events::recovered_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_events.into_iter().map(Into::into).collect())
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
            BackendType::Postgres => self.get_recent_postgres(limit).await,
            BackendType::Sqlite => self.get_recent_sqlite(limit).await,
        }
    }

    async fn get_recent_postgres(
        &self,
        limit: i64,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::dal::postgres_dal::models::PgRecoveryEvent;
        use crate::database::schema::postgres::recovery_events;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pg_events: Vec<PgRecoveryEvent> = conn
            .interact(move |conn| {
                recovery_events::table
                    .order(recovery_events::recovered_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_events.into_iter().map(Into::into).collect())
    }

    async fn get_recent_sqlite(&self, limit: i64) -> Result<Vec<RecoveryEvent>, ValidationError> {
        use crate::dal::sqlite_dal::models::SqliteRecoveryEvent;
        use crate::database::schema::sqlite::recovery_events;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let sqlite_events: Vec<SqliteRecoveryEvent> = conn
            .interact(move |conn| {
                recovery_events::table
                    .order(recovery_events::recovered_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_events.into_iter().map(Into::into).collect())
    }
}
