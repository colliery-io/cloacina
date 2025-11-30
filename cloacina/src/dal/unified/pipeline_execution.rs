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

//! Unified Pipeline Execution DAL with runtime backend selection
//!
//! This module provides CRUD operations for PipelineExecution entities that work with
//! both PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.

use super::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::database::BackendType;
use crate::error::ValidationError;
use crate::models::pipeline_execution::{NewPipelineExecution, PipelineExecution};
use diesel::prelude::*;
use uuid::Uuid;

/// Data access layer for pipeline execution operations with runtime backend selection.
#[derive(Clone)]
pub struct PipelineExecutionDAL<'a> {
    dal: &'a DAL,
}

impl<'a> PipelineExecutionDAL<'a> {
    /// Creates a new PipelineExecutionDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new pipeline execution record in the database.
    pub async fn create(
        &self,
        new_execution: NewPipelineExecution,
    ) -> Result<PipelineExecution, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.create_postgres(new_execution).await,
            BackendType::Sqlite => self.create_sqlite(new_execution).await,
        }
    }

    async fn create_postgres(
        &self,
        new_execution: NewPipelineExecution,
    ) -> Result<PipelineExecution, ValidationError> {
        use crate::dal::postgres_dal::models::{NewPgPipelineExecution, PgPipelineExecution};
        use crate::database::schema::postgres::pipeline_executions;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pg_new = NewPgPipelineExecution {
            pipeline_name: new_execution.pipeline_name,
            pipeline_version: new_execution.pipeline_version,
            status: new_execution.status,
            context_id: new_execution.context_id.map(|u| u.0),
        };

        let pg_execution: PgPipelineExecution = conn
            .interact(move |conn| {
                diesel::insert_into(pipeline_executions::table)
                    .values(&pg_new)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_execution.into())
    }

    async fn create_sqlite(
        &self,
        new_execution: NewPipelineExecution,
    ) -> Result<PipelineExecution, ValidationError> {
        use crate::dal::sqlite_dal::models::{
            current_timestamp_string, uuid_to_blob, NewSqlitePipelineExecution,
            SqlitePipelineExecution,
        };
        use crate::database::schema::sqlite::pipeline_executions;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        // For SQLite, generate UUID and timestamps client-side
        let id = UniversalUuid::new_v4();
        let now = current_timestamp_string();
        let id_blob = uuid_to_blob(&id.0);

        let sqlite_new = NewSqlitePipelineExecution {
            id: id_blob.clone(),
            pipeline_name: new_execution.pipeline_name,
            pipeline_version: new_execution.pipeline_version,
            status: new_execution.status,
            context_id: new_execution.context_id.map(|u| uuid_to_blob(&u.0)),
            started_at: now.clone(),
            created_at: now.clone(),
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(pipeline_executions::table)
                .values(&sqlite_new)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        // Retrieve the inserted record
        let sqlite_execution: SqlitePipelineExecution = conn
            .interact(move |conn| pipeline_executions::table.find(id_blob).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_execution.into())
    }

    /// Retrieves a pipeline execution by its unique identifier.
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<PipelineExecution, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.get_by_id_postgres(id).await,
            BackendType::Sqlite => self.get_by_id_sqlite(id).await,
        }
    }

    async fn get_by_id_postgres(
        &self,
        id: UniversalUuid,
    ) -> Result<PipelineExecution, ValidationError> {
        use crate::dal::postgres_dal::models::PgPipelineExecution;
        use crate::database::schema::postgres::pipeline_executions;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id: Uuid = id.0;
        let execution: PgPipelineExecution = conn
            .interact(move |conn| pipeline_executions::table.find(uuid_id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(execution.into())
    }

    async fn get_by_id_sqlite(
        &self,
        id: UniversalUuid,
    ) -> Result<PipelineExecution, ValidationError> {
        use crate::dal::sqlite_dal::models::{uuid_to_blob, SqlitePipelineExecution};
        use crate::database::schema::sqlite::pipeline_executions;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let execution: SqlitePipelineExecution = conn
            .interact(move |conn| pipeline_executions::table.find(id_blob).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(execution.into())
    }

    /// Retrieves all active pipeline executions (status is either "Pending" or "Running").
    pub async fn get_active_executions(&self) -> Result<Vec<PipelineExecution>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.get_active_executions_postgres().await,
            BackendType::Sqlite => self.get_active_executions_sqlite().await,
        }
    }

    async fn get_active_executions_postgres(
        &self,
    ) -> Result<Vec<PipelineExecution>, ValidationError> {
        use crate::dal::postgres_dal::models::PgPipelineExecution;
        use crate::database::schema::postgres::pipeline_executions;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pg_executions: Vec<PgPipelineExecution> = conn
            .interact(move |conn| {
                pipeline_executions::table
                    .filter(pipeline_executions::status.eq_any(vec!["Pending", "Running"]))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_executions.into_iter().map(Into::into).collect())
    }

    async fn get_active_executions_sqlite(&self) -> Result<Vec<PipelineExecution>, ValidationError> {
        use crate::dal::sqlite_dal::models::SqlitePipelineExecution;
        use crate::database::schema::sqlite::pipeline_executions;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let sqlite_executions: Vec<SqlitePipelineExecution> = conn
            .interact(move |conn| {
                pipeline_executions::table
                    .filter(pipeline_executions::status.eq_any(vec!["Pending", "Running"]))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_executions.into_iter().map(Into::into).collect())
    }

    /// Updates the status of a pipeline execution.
    pub async fn update_status(
        &self,
        id: UniversalUuid,
        status: &str,
    ) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.update_status_postgres(id, status).await,
            BackendType::Sqlite => self.update_status_sqlite(id, status).await,
        }
    }

    async fn update_status_postgres(
        &self,
        id: UniversalUuid,
        status: &str,
    ) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::pipeline_executions;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let status = status.to_string();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id.0))
                .set(pipeline_executions::status.eq(status))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn update_status_sqlite(
        &self,
        id: UniversalUuid,
        status: &str,
    ) -> Result<(), ValidationError> {
        use crate::dal::sqlite_dal::models::uuid_to_blob;
        use crate::database::schema::sqlite::pipeline_executions;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let status = status.to_string();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id_blob))
                .set(pipeline_executions::status.eq(status))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Marks a pipeline execution as completed and sets the completion timestamp.
    pub async fn mark_completed(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.mark_completed_postgres(id).await,
            BackendType::Sqlite => self.mark_completed_sqlite(id).await,
        }
    }

    async fn mark_completed_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::pipeline_executions;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id.0))
                .set((
                    pipeline_executions::status.eq("Completed"),
                    pipeline_executions::completed_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn mark_completed_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        use crate::dal::sqlite_dal::models::{current_timestamp_string, uuid_to_blob};
        use crate::database::schema::sqlite::pipeline_executions;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let now = current_timestamp_string();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id_blob))
                .set((
                    pipeline_executions::status.eq("Completed"),
                    pipeline_executions::completed_at.eq(Some(now)),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Retrieves the most recent version of a pipeline by name.
    pub async fn get_last_version(
        &self,
        pipeline_name: &str,
    ) -> Result<Option<String>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.get_last_version_postgres(pipeline_name).await,
            BackendType::Sqlite => self.get_last_version_sqlite(pipeline_name).await,
        }
    }

    async fn get_last_version_postgres(
        &self,
        pipeline_name: &str,
    ) -> Result<Option<String>, ValidationError> {
        use crate::database::schema::postgres::pipeline_executions;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
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

    async fn get_last_version_sqlite(
        &self,
        pipeline_name: &str,
    ) -> Result<Option<String>, ValidationError> {
        use crate::database::schema::sqlite::pipeline_executions;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
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

    /// Marks a pipeline as failed and records the failure reason.
    pub async fn mark_failed(
        &self,
        id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.mark_failed_postgres(id, reason).await,
            BackendType::Sqlite => self.mark_failed_sqlite(id, reason).await,
        }
    }

    async fn mark_failed_postgres(
        &self,
        id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::pipeline_executions;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let reason = reason.to_string();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id.0))
                .set((
                    pipeline_executions::status.eq("Failed"),
                    pipeline_executions::completed_at.eq(diesel::dsl::now),
                    pipeline_executions::error_details.eq(reason),
                    pipeline_executions::updated_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn mark_failed_sqlite(
        &self,
        id: UniversalUuid,
        reason: &str,
    ) -> Result<(), ValidationError> {
        use crate::dal::sqlite_dal::models::{current_timestamp_string, uuid_to_blob};
        use crate::database::schema::sqlite::pipeline_executions;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let reason = reason.to_string();
        let now = current_timestamp_string();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id_blob))
                .set((
                    pipeline_executions::status.eq("Failed"),
                    pipeline_executions::completed_at.eq(Some(now.clone())),
                    pipeline_executions::error_details.eq(reason),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Increments the recovery attempt counter for a pipeline execution.
    pub async fn increment_recovery_attempts(
        &self,
        id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.increment_recovery_attempts_postgres(id).await,
            BackendType::Sqlite => self.increment_recovery_attempts_sqlite(id).await,
        }
    }

    async fn increment_recovery_attempts_postgres(
        &self,
        id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::pipeline_executions;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id.0))
                .set((
                    pipeline_executions::recovery_attempts
                        .eq(pipeline_executions::recovery_attempts + 1),
                    pipeline_executions::last_recovery_at.eq(diesel::dsl::now),
                    pipeline_executions::updated_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn increment_recovery_attempts_sqlite(
        &self,
        id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        use crate::dal::sqlite_dal::models::{current_timestamp_string, uuid_to_blob};
        use crate::database::schema::sqlite::pipeline_executions;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let now = current_timestamp_string();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id_blob))
                .set((
                    pipeline_executions::recovery_attempts
                        .eq(pipeline_executions::recovery_attempts + 1),
                    pipeline_executions::last_recovery_at.eq(Some(now.clone())),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Cancels a pipeline execution and marks it as cancelled.
    pub async fn cancel(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.cancel_postgres(id).await,
            BackendType::Sqlite => self.cancel_sqlite(id).await,
        }
    }

    async fn cancel_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::pipeline_executions;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id.0))
                .set((
                    pipeline_executions::status.eq("Cancelled"),
                    pipeline_executions::completed_at.eq(diesel::dsl::now),
                    pipeline_executions::updated_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn cancel_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        use crate::dal::sqlite_dal::models::{current_timestamp_string, uuid_to_blob};
        use crate::database::schema::sqlite::pipeline_executions;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let now = current_timestamp_string();
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id_blob))
                .set((
                    pipeline_executions::status.eq("Cancelled"),
                    pipeline_executions::completed_at.eq(Some(now.clone())),
                    pipeline_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Updates the final context ID for a pipeline execution.
    pub async fn update_final_context(
        &self,
        id: UniversalUuid,
        final_context_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.update_final_context_postgres(id, final_context_id)
                    .await
            }
            BackendType::Sqlite => {
                self.update_final_context_sqlite(id, final_context_id)
                    .await
            }
        }
    }

    async fn update_final_context_postgres(
        &self,
        id: UniversalUuid,
        final_context_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::pipeline_executions;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id.0))
                .set(pipeline_executions::context_id.eq(Some(final_context_id.0)))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn update_final_context_sqlite(
        &self,
        id: UniversalUuid,
        final_context_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        use crate::dal::sqlite_dal::models::uuid_to_blob;
        use crate::database::schema::sqlite::pipeline_executions;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let context_blob = uuid_to_blob(&final_context_id.0);
        conn.interact(move |conn| {
            diesel::update(pipeline_executions::table.find(id_blob))
                .set(pipeline_executions::context_id.eq(Some(context_blob)))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Retrieves a list of recent pipeline executions, ordered by start time.
    pub async fn list_recent(&self, limit: i64) -> Result<Vec<PipelineExecution>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.list_recent_postgres(limit).await,
            BackendType::Sqlite => self.list_recent_sqlite(limit).await,
        }
    }

    async fn list_recent_postgres(
        &self,
        limit: i64,
    ) -> Result<Vec<PipelineExecution>, ValidationError> {
        use crate::dal::postgres_dal::models::PgPipelineExecution;
        use crate::database::schema::postgres::pipeline_executions;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pg_executions: Vec<PgPipelineExecution> = conn
            .interact(move |conn| {
                pipeline_executions::table
                    .order(pipeline_executions::started_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_executions.into_iter().map(Into::into).collect())
    }

    async fn list_recent_sqlite(
        &self,
        limit: i64,
    ) -> Result<Vec<PipelineExecution>, ValidationError> {
        use crate::dal::sqlite_dal::models::SqlitePipelineExecution;
        use crate::database::schema::sqlite::pipeline_executions;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let sqlite_executions: Vec<SqlitePipelineExecution> = conn
            .interact(move |conn| {
                pipeline_executions::table
                    .order(pipeline_executions::started_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_executions.into_iter().map(Into::into).collect())
    }
}
