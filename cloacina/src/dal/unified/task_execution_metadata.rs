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

//! Unified Task Execution Metadata DAL with runtime backend selection
//!
//! This module provides CRUD operations for TaskExecutionMetadata entities that work with
//! both PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.

use super::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::database::BackendType;
use crate::error::ValidationError;
use crate::models::task_execution_metadata::{NewTaskExecutionMetadata, TaskExecutionMetadata};
use crate::task::TaskNamespace;
use diesel::prelude::*;

/// Data access layer for task execution metadata operations with runtime backend selection.
#[derive(Clone)]
pub struct TaskExecutionMetadataDAL<'a> {
    dal: &'a DAL,
}

impl<'a> TaskExecutionMetadataDAL<'a> {
    /// Creates a new TaskExecutionMetadataDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new task execution metadata record.
    pub async fn create(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.create_postgres(new_metadata).await,
            BackendType::Sqlite => self.create_sqlite(new_metadata).await,
        }
    }

    async fn create_postgres(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        use crate::dal::postgres_dal::models::{NewPgTaskExecutionMetadata, PgTaskExecutionMetadata};
        use crate::database::schema::postgres::task_execution_metadata;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pg_new = NewPgTaskExecutionMetadata {
            task_execution_id: new_metadata.task_execution_id.0,
            pipeline_execution_id: new_metadata.pipeline_execution_id.0,
            task_name: new_metadata.task_name,
            context_id: new_metadata.context_id.map(|u| u.0),
        };

        let pg_metadata: PgTaskExecutionMetadata = conn
            .interact(move |conn| {
                diesel::insert_into(task_execution_metadata::table)
                    .values(&pg_new)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_metadata.into())
    }

    async fn create_sqlite(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        use crate::dal::sqlite_dal::models::{
            current_timestamp_string, uuid_to_blob, NewSqliteTaskExecutionMetadata,
            SqliteTaskExecutionMetadata,
        };
        use crate::database::schema::sqlite::task_execution_metadata;

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

        let sqlite_new = NewSqliteTaskExecutionMetadata {
            id: id_blob.clone(),
            task_execution_id: uuid_to_blob(&new_metadata.task_execution_id.0),
            pipeline_execution_id: uuid_to_blob(&new_metadata.pipeline_execution_id.0),
            task_name: new_metadata.task_name,
            context_id: new_metadata.context_id.map(|u| uuid_to_blob(&u.0)),
            created_at: now.clone(),
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(task_execution_metadata::table)
                .values(&sqlite_new)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        // Retrieve the inserted record
        let sqlite_metadata: SqliteTaskExecutionMetadata = conn
            .interact(move |conn| task_execution_metadata::table.find(id_blob).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_metadata.into())
    }

    /// Retrieves task execution metadata for a specific pipeline and task.
    pub async fn get_by_pipeline_and_task(
        &self,
        pipeline_id: UniversalUuid,
        task_namespace: &TaskNamespace,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.get_by_pipeline_and_task_postgres(pipeline_id, task_namespace)
                    .await
            }
            BackendType::Sqlite => {
                self.get_by_pipeline_and_task_sqlite(pipeline_id, task_namespace)
                    .await
            }
        }
    }

    async fn get_by_pipeline_and_task_postgres(
        &self,
        pipeline_id: UniversalUuid,
        task_namespace: &TaskNamespace,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        use crate::dal::postgres_dal::models::PgTaskExecutionMetadata;
        use crate::database::schema::postgres::task_execution_metadata;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = pipeline_id.0;
        let task_name_owned = task_namespace.to_string();
        let pg_metadata: PgTaskExecutionMetadata = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::pipeline_execution_id.eq(uuid_id))
                    .filter(task_execution_metadata::task_name.eq(&task_name_owned))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_metadata.into())
    }

    async fn get_by_pipeline_and_task_sqlite(
        &self,
        pipeline_id: UniversalUuid,
        task_namespace: &TaskNamespace,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        use crate::dal::sqlite_dal::models::{uuid_to_blob, SqliteTaskExecutionMetadata};
        use crate::database::schema::sqlite::task_execution_metadata;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pipeline_blob = uuid_to_blob(&pipeline_id.0);
        let task_name = task_namespace.to_string();
        let sqlite_metadata: SqliteTaskExecutionMetadata = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::pipeline_execution_id.eq(pipeline_blob))
                    .filter(task_execution_metadata::task_name.eq(task_name))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_metadata.into())
    }

    /// Retrieves task execution metadata by task execution ID.
    pub async fn get_by_task_execution(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.get_by_task_execution_postgres(task_execution_id).await
            }
            BackendType::Sqlite => self.get_by_task_execution_sqlite(task_execution_id).await,
        }
    }

    async fn get_by_task_execution_postgres(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        use crate::dal::postgres_dal::models::PgTaskExecutionMetadata;
        use crate::database::schema::postgres::task_execution_metadata;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = task_execution_id.0;
        let pg_metadata: PgTaskExecutionMetadata = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::task_execution_id.eq(uuid_id))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_metadata.into())
    }

    async fn get_by_task_execution_sqlite(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        use crate::dal::sqlite_dal::models::{uuid_to_blob, SqliteTaskExecutionMetadata};
        use crate::database::schema::sqlite::task_execution_metadata;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task_blob = uuid_to_blob(&task_execution_id.0);
        let sqlite_metadata: SqliteTaskExecutionMetadata = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::task_execution_id.eq(task_blob))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_metadata.into())
    }

    /// Updates the context ID for a specific task execution.
    pub async fn update_context_id(
        &self,
        task_execution_id: UniversalUuid,
        context_id: Option<UniversalUuid>,
    ) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.update_context_id_postgres(task_execution_id, context_id)
                    .await
            }
            BackendType::Sqlite => {
                self.update_context_id_sqlite(task_execution_id, context_id)
                    .await
            }
        }
    }

    async fn update_context_id_postgres(
        &self,
        task_execution_id: UniversalUuid,
        context_id: Option<UniversalUuid>,
    ) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::task_execution_metadata;
        use uuid::Uuid;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = task_execution_id.0;
        let context_uuid: Option<Uuid> = context_id.map(|id| id.0);
        conn.interact(move |conn| {
            diesel::update(task_execution_metadata::table)
                .filter(task_execution_metadata::task_execution_id.eq(uuid_id))
                .set((
                    task_execution_metadata::context_id.eq(context_uuid),
                    task_execution_metadata::updated_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn update_context_id_sqlite(
        &self,
        task_execution_id: UniversalUuid,
        context_id: Option<UniversalUuid>,
    ) -> Result<(), ValidationError> {
        use crate::dal::sqlite_dal::models::{current_timestamp_string, uuid_to_blob};
        use crate::database::schema::sqlite::task_execution_metadata;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task_blob = uuid_to_blob(&task_execution_id.0);
        let context_blob: Option<Vec<u8>> = context_id.map(|u| uuid_to_blob(&u.0));
        let now = current_timestamp_string();

        conn.interact(move |conn| {
            diesel::update(task_execution_metadata::table)
                .filter(task_execution_metadata::task_execution_id.eq(task_blob))
                .set((
                    task_execution_metadata::context_id.eq(context_blob),
                    task_execution_metadata::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Creates or updates task execution metadata.
    pub async fn upsert_task_execution_metadata(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.upsert_task_execution_metadata_postgres(new_metadata)
                    .await
            }
            BackendType::Sqlite => {
                self.upsert_task_execution_metadata_sqlite(new_metadata)
                    .await
            }
        }
    }

    async fn upsert_task_execution_metadata_postgres(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        use crate::dal::postgres_dal::models::{NewPgTaskExecutionMetadata, PgTaskExecutionMetadata};
        use crate::database::schema::postgres::task_execution_metadata;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let context_uuid = new_metadata.context_id.map(|u| u.0);
        let pg_new = NewPgTaskExecutionMetadata {
            task_execution_id: new_metadata.task_execution_id.0,
            pipeline_execution_id: new_metadata.pipeline_execution_id.0,
            task_name: new_metadata.task_name,
            context_id: context_uuid,
        };

        let pg_metadata: PgTaskExecutionMetadata = conn
            .interact(move |conn| {
                diesel::insert_into(task_execution_metadata::table)
                    .values(&pg_new)
                    .on_conflict(task_execution_metadata::task_execution_id)
                    .do_update()
                    .set((
                        task_execution_metadata::context_id.eq(&pg_new.context_id),
                        task_execution_metadata::updated_at.eq(diesel::dsl::now),
                    ))
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_metadata.into())
    }

    async fn upsert_task_execution_metadata_sqlite(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        use crate::dal::sqlite_dal::models::{
            current_timestamp_string, uuid_to_blob, NewSqliteTaskExecutionMetadata,
            SqliteTaskExecutionMetadata,
        };
        use crate::database::schema::sqlite::task_execution_metadata;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        // SQLite doesn't support ON CONFLICT DO UPDATE with RETURNING
        // Check if the record exists first
        let task_exec_blob = uuid_to_blob(&new_metadata.task_execution_id.0);
        let task_exec_blob_clone = task_exec_blob.clone();
        let existing: Option<SqliteTaskExecutionMetadata> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::task_execution_id.eq(&task_exec_blob_clone))
                    .first::<SqliteTaskExecutionMetadata>(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        match existing {
            Some(_) => {
                // Update existing record
                let task_exec_blob = uuid_to_blob(&new_metadata.task_execution_id.0);
                let context_blob: Option<Vec<u8>> =
                    new_metadata.context_id.map(|u| uuid_to_blob(&u.0));
                let now = current_timestamp_string();

                conn.interact(move |conn| {
                    diesel::update(task_execution_metadata::table)
                        .filter(task_execution_metadata::task_execution_id.eq(&task_exec_blob))
                        .set((
                            task_execution_metadata::context_id.eq(&context_blob),
                            task_execution_metadata::updated_at.eq(&now),
                        ))
                        .execute(conn)
                })
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

                // Retrieve the updated record
                let task_exec_blob = uuid_to_blob(&new_metadata.task_execution_id.0);
                let sqlite_metadata: SqliteTaskExecutionMetadata = conn
                    .interact(move |conn| {
                        task_execution_metadata::table
                            .filter(task_execution_metadata::task_execution_id.eq(&task_exec_blob))
                            .first(conn)
                    })
                    .await
                    .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

                Ok(sqlite_metadata.into())
            }
            None => {
                // Create new record
                let id = UniversalUuid::new_v4();
                let id_blob = uuid_to_blob(&id.0);
                let now = current_timestamp_string();

                let sqlite_new = NewSqliteTaskExecutionMetadata {
                    id: id_blob.clone(),
                    task_execution_id: uuid_to_blob(&new_metadata.task_execution_id.0),
                    pipeline_execution_id: uuid_to_blob(&new_metadata.pipeline_execution_id.0),
                    task_name: new_metadata.task_name,
                    context_id: new_metadata.context_id.map(|u| uuid_to_blob(&u.0)),
                    created_at: now.clone(),
                    updated_at: now,
                };

                conn.interact(move |conn| {
                    diesel::insert_into(task_execution_metadata::table)
                        .values(&sqlite_new)
                        .execute(conn)
                })
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

                // Retrieve the inserted record
                let sqlite_metadata: SqliteTaskExecutionMetadata = conn
                    .interact(move |conn| task_execution_metadata::table.find(id_blob).first(conn))
                    .await
                    .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

                Ok(sqlite_metadata.into())
            }
        }
    }

    /// Retrieves metadata for multiple dependency tasks within a pipeline.
    pub async fn get_dependency_metadata(
        &self,
        pipeline_id: UniversalUuid,
        dependency_task_names: &[String],
    ) -> Result<Vec<TaskExecutionMetadata>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.get_dependency_metadata_postgres(pipeline_id, dependency_task_names)
                    .await
            }
            BackendType::Sqlite => {
                self.get_dependency_metadata_sqlite(pipeline_id, dependency_task_names)
                    .await
            }
        }
    }

    async fn get_dependency_metadata_postgres(
        &self,
        pipeline_id: UniversalUuid,
        dependency_task_names: &[String],
    ) -> Result<Vec<TaskExecutionMetadata>, ValidationError> {
        use crate::dal::postgres_dal::models::PgTaskExecutionMetadata;
        use crate::database::schema::postgres::task_execution_metadata;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = pipeline_id.0;
        let dependency_task_names_owned = dependency_task_names.to_vec();
        let pg_metadata: Vec<PgTaskExecutionMetadata> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::pipeline_execution_id.eq(uuid_id))
                    .filter(task_execution_metadata::task_name.eq_any(&dependency_task_names_owned))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_metadata.into_iter().map(Into::into).collect())
    }

    async fn get_dependency_metadata_sqlite(
        &self,
        pipeline_id: UniversalUuid,
        dependency_task_names: &[String],
    ) -> Result<Vec<TaskExecutionMetadata>, ValidationError> {
        use crate::dal::sqlite_dal::models::{uuid_to_blob, SqliteTaskExecutionMetadata};
        use crate::database::schema::sqlite::task_execution_metadata;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pipeline_blob = uuid_to_blob(&pipeline_id.0);
        let dependency_task_names = dependency_task_names.to_vec();
        let sqlite_metadata: Vec<SqliteTaskExecutionMetadata> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::pipeline_execution_id.eq(pipeline_blob))
                    .filter(task_execution_metadata::task_name.eq_any(dependency_task_names))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_metadata.into_iter().map(Into::into).collect())
    }

    /// Retrieves metadata and context data for multiple dependency tasks in a single query.
    pub async fn get_dependency_metadata_with_contexts(
        &self,
        pipeline_id: UniversalUuid,
        dependency_task_namespaces: &[TaskNamespace],
    ) -> Result<Vec<(TaskExecutionMetadata, Option<String>)>, ValidationError> {
        if dependency_task_namespaces.is_empty() {
            return Ok(Vec::new());
        }

        match self.dal.backend() {
            BackendType::Postgres => {
                self.get_dependency_metadata_with_contexts_postgres(
                    pipeline_id,
                    dependency_task_namespaces,
                )
                .await
            }
            BackendType::Sqlite => {
                self.get_dependency_metadata_with_contexts_sqlite(
                    pipeline_id,
                    dependency_task_namespaces,
                )
                .await
            }
        }
    }

    async fn get_dependency_metadata_with_contexts_postgres(
        &self,
        pipeline_id: UniversalUuid,
        dependency_task_namespaces: &[TaskNamespace],
    ) -> Result<Vec<(TaskExecutionMetadata, Option<String>)>, ValidationError> {
        use crate::dal::postgres_dal::models::PgTaskExecutionMetadata;
        use crate::database::schema::postgres::{contexts, task_execution_metadata};

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = pipeline_id.0;
        let dependency_task_names_owned: Vec<String> = dependency_task_namespaces
            .iter()
            .map(|ns| ns.to_string())
            .collect();

        let results: Vec<(PgTaskExecutionMetadata, Option<String>)> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .left_join(
                        contexts::table
                            .on(task_execution_metadata::context_id.eq(contexts::id.nullable())),
                    )
                    .filter(task_execution_metadata::pipeline_execution_id.eq(uuid_id))
                    .filter(task_execution_metadata::task_name.eq_any(&dependency_task_names_owned))
                    .select((
                        task_execution_metadata::all_columns,
                        contexts::value.nullable(),
                    ))
                    .load::<(PgTaskExecutionMetadata, Option<String>)>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results
            .into_iter()
            .map(|(m, c)| (m.into(), c))
            .collect())
    }

    async fn get_dependency_metadata_with_contexts_sqlite(
        &self,
        pipeline_id: UniversalUuid,
        dependency_task_namespaces: &[TaskNamespace],
    ) -> Result<Vec<(TaskExecutionMetadata, Option<String>)>, ValidationError> {
        use crate::dal::sqlite_dal::models::{uuid_to_blob, SqliteTaskExecutionMetadata};
        use crate::database::schema::sqlite::{contexts, task_execution_metadata};

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pipeline_blob = uuid_to_blob(&pipeline_id.0);
        let dependency_task_names: Vec<String> = dependency_task_namespaces
            .iter()
            .map(|ns| ns.to_string())
            .collect();

        let results: Vec<(SqliteTaskExecutionMetadata, Option<String>)> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .left_join(
                        contexts::table
                            .on(task_execution_metadata::context_id.eq(contexts::id.nullable())),
                    )
                    .filter(task_execution_metadata::pipeline_execution_id.eq(pipeline_blob))
                    .filter(task_execution_metadata::task_name.eq_any(dependency_task_names))
                    .select((
                        task_execution_metadata::all_columns,
                        contexts::value.nullable(),
                    ))
                    .load::<(SqliteTaskExecutionMetadata, Option<String>)>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results
            .into_iter()
            .map(|(m, c)| (m.into(), c))
            .collect())
    }
}
