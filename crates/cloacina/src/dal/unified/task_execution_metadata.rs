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

use super::models::{NewUnifiedTaskExecutionMetadata, UnifiedTaskExecutionMetadata};
use super::DAL;
use crate::database::schema::unified::{contexts, task_execution_metadata};
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
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
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_metadata).await,
            self.create_sqlite(new_metadata).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn create_postgres(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedTaskExecutionMetadata {
            id,
            task_execution_id: new_metadata.task_execution_id,
            pipeline_execution_id: new_metadata.pipeline_execution_id,
            task_name: new_metadata.task_name,
            context_id: new_metadata.context_id,
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(task_execution_metadata::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedTaskExecutionMetadata = conn
            .interact(move |conn| task_execution_metadata::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    async fn create_sqlite(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedTaskExecutionMetadata {
            id,
            task_execution_id: new_metadata.task_execution_id,
            pipeline_execution_id: new_metadata.pipeline_execution_id,
            task_name: new_metadata.task_name,
            context_id: new_metadata.context_id,
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(task_execution_metadata::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedTaskExecutionMetadata = conn
            .interact(move |conn| task_execution_metadata::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    /// Retrieves task execution metadata for a specific pipeline and task.
    pub async fn get_by_pipeline_and_task(
        &self,
        pipeline_id: UniversalUuid,
        task_namespace: &TaskNamespace,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_pipeline_and_task_postgres(pipeline_id, task_namespace)
                .await,
            self.get_by_pipeline_and_task_sqlite(pipeline_id, task_namespace)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_by_pipeline_and_task_postgres(
        &self,
        pipeline_id: UniversalUuid,
        task_namespace: &TaskNamespace,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task_name_owned = task_namespace.to_string();
        let result: UnifiedTaskExecutionMetadata = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::pipeline_execution_id.eq(pipeline_id))
                    .filter(task_execution_metadata::task_name.eq(&task_name_owned))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    async fn get_by_pipeline_and_task_sqlite(
        &self,
        pipeline_id: UniversalUuid,
        task_namespace: &TaskNamespace,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task_name = task_namespace.to_string();
        let result: UnifiedTaskExecutionMetadata = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::pipeline_execution_id.eq(pipeline_id))
                    .filter(task_execution_metadata::task_name.eq(task_name))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    /// Retrieves task execution metadata by task execution ID.
    pub async fn get_by_task_execution(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_task_execution_postgres(task_execution_id).await,
            self.get_by_task_execution_sqlite(task_execution_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_by_task_execution_postgres(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedTaskExecutionMetadata = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::task_execution_id.eq(task_execution_id))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    async fn get_by_task_execution_sqlite(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedTaskExecutionMetadata = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::task_execution_id.eq(task_execution_id))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    /// Updates the context ID for a specific task execution.
    pub async fn update_context_id(
        &self,
        task_execution_id: UniversalUuid,
        context_id: Option<UniversalUuid>,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_context_id_postgres(task_execution_id, context_id)
                .await,
            self.update_context_id_sqlite(task_execution_id, context_id)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn update_context_id_postgres(
        &self,
        task_execution_id: UniversalUuid,
        context_id: Option<UniversalUuid>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(task_execution_metadata::table)
                .filter(task_execution_metadata::task_execution_id.eq(task_execution_id))
                .set((
                    task_execution_metadata::context_id.eq(context_id),
                    task_execution_metadata::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn update_context_id_sqlite(
        &self,
        task_execution_id: UniversalUuid,
        context_id: Option<UniversalUuid>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(task_execution_metadata::table)
                .filter(task_execution_metadata::task_execution_id.eq(task_execution_id))
                .set((
                    task_execution_metadata::context_id.eq(context_id),
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
        crate::dispatch_backend!(
            self.dal.backend(),
            self.upsert_task_execution_metadata_postgres(new_metadata)
                .await,
            self.upsert_task_execution_metadata_sqlite(new_metadata)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn upsert_task_execution_metadata_postgres(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedTaskExecutionMetadata {
            id,
            task_execution_id: new_metadata.task_execution_id,
            pipeline_execution_id: new_metadata.pipeline_execution_id,
            task_name: new_metadata.task_name,
            context_id: new_metadata.context_id,
            created_at: now,
            updated_at: now,
        };

        let task_exec_id = new_unified.task_execution_id;
        let context_id = new_unified.context_id;

        let result: UnifiedTaskExecutionMetadata = conn
            .interact(move |conn| {
                diesel::insert_into(task_execution_metadata::table)
                    .values(&new_unified)
                    .on_conflict(task_execution_metadata::task_execution_id)
                    .do_update()
                    .set((
                        task_execution_metadata::context_id.eq(context_id),
                        task_execution_metadata::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Fetch the result
                task_execution_metadata::table
                    .filter(task_execution_metadata::task_execution_id.eq(task_exec_id))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    async fn upsert_task_execution_metadata_sqlite(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        // SQLite doesn't support ON CONFLICT DO UPDATE with RETURNING well
        // Check if the record exists first
        let task_exec_id = new_metadata.task_execution_id;
        let existing: Option<UnifiedTaskExecutionMetadata> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::task_execution_id.eq(task_exec_id))
                    .first::<UnifiedTaskExecutionMetadata>(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        match existing {
            Some(_) => {
                // Update existing record
                let task_exec_id = new_metadata.task_execution_id;
                let context_id = new_metadata.context_id;
                let now = UniversalTimestamp::now();

                conn.interact(move |conn| {
                    diesel::update(task_execution_metadata::table)
                        .filter(task_execution_metadata::task_execution_id.eq(task_exec_id))
                        .set((
                            task_execution_metadata::context_id.eq(context_id),
                            task_execution_metadata::updated_at.eq(now),
                        ))
                        .execute(conn)
                })
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

                // Retrieve the updated record
                let task_exec_id = new_metadata.task_execution_id;
                let result: UnifiedTaskExecutionMetadata = conn
                    .interact(move |conn| {
                        task_execution_metadata::table
                            .filter(task_execution_metadata::task_execution_id.eq(task_exec_id))
                            .first(conn)
                    })
                    .await
                    .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

                Ok(result.into())
            }
            None => {
                // Create new record
                let id = UniversalUuid::new_v4();
                let now = UniversalTimestamp::now();

                let new_unified = NewUnifiedTaskExecutionMetadata {
                    id,
                    task_execution_id: new_metadata.task_execution_id,
                    pipeline_execution_id: new_metadata.pipeline_execution_id,
                    task_name: new_metadata.task_name,
                    context_id: new_metadata.context_id,
                    created_at: now,
                    updated_at: now,
                };

                conn.interact(move |conn| {
                    diesel::insert_into(task_execution_metadata::table)
                        .values(&new_unified)
                        .execute(conn)
                })
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

                let result: UnifiedTaskExecutionMetadata = conn
                    .interact(move |conn| task_execution_metadata::table.find(id).first(conn))
                    .await
                    .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

                Ok(result.into())
            }
        }
    }

    /// Retrieves metadata for multiple dependency tasks within a pipeline.
    pub async fn get_dependency_metadata(
        &self,
        pipeline_id: UniversalUuid,
        dependency_task_names: &[String],
    ) -> Result<Vec<TaskExecutionMetadata>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_dependency_metadata_postgres(pipeline_id, dependency_task_names)
                .await,
            self.get_dependency_metadata_sqlite(pipeline_id, dependency_task_names)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_dependency_metadata_postgres(
        &self,
        pipeline_id: UniversalUuid,
        dependency_task_names: &[String],
    ) -> Result<Vec<TaskExecutionMetadata>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let dependency_task_names_owned = dependency_task_names.to_vec();
        let results: Vec<UnifiedTaskExecutionMetadata> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::pipeline_execution_id.eq(pipeline_id))
                    .filter(task_execution_metadata::task_name.eq_any(&dependency_task_names_owned))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn get_dependency_metadata_sqlite(
        &self,
        pipeline_id: UniversalUuid,
        dependency_task_names: &[String],
    ) -> Result<Vec<TaskExecutionMetadata>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let dependency_task_names = dependency_task_names.to_vec();
        let results: Vec<UnifiedTaskExecutionMetadata> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::pipeline_execution_id.eq(pipeline_id))
                    .filter(task_execution_metadata::task_name.eq_any(dependency_task_names))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
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

        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_dependency_metadata_with_contexts_postgres(
                pipeline_id,
                dependency_task_namespaces
            )
            .await,
            self.get_dependency_metadata_with_contexts_sqlite(
                pipeline_id,
                dependency_task_namespaces
            )
            .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_dependency_metadata_with_contexts_postgres(
        &self,
        pipeline_id: UniversalUuid,
        dependency_task_namespaces: &[TaskNamespace],
    ) -> Result<Vec<(TaskExecutionMetadata, Option<String>)>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let dependency_task_names_owned: Vec<String> = dependency_task_namespaces
            .iter()
            .map(|ns| ns.to_string())
            .collect();

        let results: Vec<(UnifiedTaskExecutionMetadata, Option<String>)> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .left_join(
                        contexts::table
                            .on(task_execution_metadata::context_id.eq(contexts::id.nullable())),
                    )
                    .filter(task_execution_metadata::pipeline_execution_id.eq(pipeline_id))
                    .filter(task_execution_metadata::task_name.eq_any(&dependency_task_names_owned))
                    .select((
                        task_execution_metadata::all_columns,
                        contexts::value.nullable(),
                    ))
                    .load::<(UnifiedTaskExecutionMetadata, Option<String>)>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|(m, c)| (m.into(), c)).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn get_dependency_metadata_with_contexts_sqlite(
        &self,
        pipeline_id: UniversalUuid,
        dependency_task_namespaces: &[TaskNamespace],
    ) -> Result<Vec<(TaskExecutionMetadata, Option<String>)>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let dependency_task_names: Vec<String> = dependency_task_namespaces
            .iter()
            .map(|ns| ns.to_string())
            .collect();

        let results: Vec<(UnifiedTaskExecutionMetadata, Option<String>)> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .left_join(
                        contexts::table
                            .on(task_execution_metadata::context_id.eq(contexts::id.nullable())),
                    )
                    .filter(task_execution_metadata::pipeline_execution_id.eq(pipeline_id))
                    .filter(task_execution_metadata::task_name.eq_any(dependency_task_names))
                    .select((
                        task_execution_metadata::all_columns,
                        contexts::value.nullable(),
                    ))
                    .load::<(UnifiedTaskExecutionMetadata, Option<String>)>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|(m, c)| (m.into(), c)).collect())
    }
}
