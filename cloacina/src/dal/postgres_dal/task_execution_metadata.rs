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

//! Task Execution Metadata Data Access Layer
//!
//! This module provides the data access layer for managing task execution metadata in the system.
//! It handles CRUD operations for task execution metadata, which includes information about
//! task executions, their context, and relationships within pipeline executions.
//!
//! The metadata includes information such as:
//! - Task execution IDs
//! - Pipeline execution IDs
//! - Task names
//! - Context IDs
//! - Creation and update timestamps

use super::DAL;
use crate::database::schema::task_execution_metadata;
use crate::database::universal_types::UniversalUuid;
use crate::error::ValidationError;
use crate::models::task_execution_metadata::{NewTaskExecutionMetadata, TaskExecutionMetadata};
use diesel::prelude::*;
use uuid::Uuid;

/// Data Access Layer for Task Execution Metadata
///
/// This struct provides methods to interact with the task execution metadata table
/// in the database. It requires a reference to the main DAL instance for database
/// connection management.
pub struct TaskExecutionMetadataDAL<'a> {
    pub dal: &'a DAL,
}

impl<'a> TaskExecutionMetadataDAL<'a> {
    /// Creates a new task execution metadata record
    ///
    /// # Arguments
    /// * `new_metadata` - The new metadata to be inserted into the database
    ///
    /// # Returns
    /// * `Result<TaskExecutionMetadata, ValidationError>` - The created metadata record or an error
    pub async fn create(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let mut conn = self.dal.pool.get().await?;

        let metadata: TaskExecutionMetadata = conn.interact(move |conn| {
            diesel::insert_into(task_execution_metadata::table)
                .values(&new_metadata)
                .get_result(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(metadata)
    }

    /// Retrieves task execution metadata for a specific pipeline and task
    ///
    /// # Arguments
    /// * `pipeline_id` - The UUID of the pipeline execution
    /// * `task_name` - The name of the task
    ///
    /// # Returns
    /// * `Result<TaskExecutionMetadata, ValidationError>` - The metadata record or an error
    pub async fn get_by_pipeline_and_task(
        &self,
        pipeline_id: UniversalUuid,
        task_name: &str,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let mut conn = self.dal.pool.get().await?;

        let task_name_owned = task_name.to_string();
        let metadata = conn.interact(move |conn| {
            task_execution_metadata::table
                .filter(task_execution_metadata::pipeline_execution_id.eq(pipeline_id.0))
                .filter(task_execution_metadata::task_name.eq(&task_name_owned))
                .first(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(metadata)
    }

    /// Retrieves task execution metadata by task execution ID
    ///
    /// # Arguments
    /// * `task_execution_id` - The UUID of the task execution
    ///
    /// # Returns
    /// * `Result<TaskExecutionMetadata, ValidationError>` - The metadata record or an error
    pub async fn get_by_task_execution(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let mut conn = self.dal.pool.get().await?;

        let metadata = conn.interact(move |conn| {
            task_execution_metadata::table
                .filter(task_execution_metadata::task_execution_id.eq(task_execution_id.0))
                .first(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(metadata)
    }

    /// Updates the context ID for a specific task execution
    ///
    /// # Arguments
    /// * `task_execution_id` - The UUID of the task execution to update
    /// * `context_id` - The new context ID (can be None)
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub async fn update_context_id(
        &self,
        task_execution_id: UniversalUuid,
        context_id: Option<UniversalUuid>,
    ) -> Result<(), ValidationError> {
        let mut conn = self.dal.pool.get().await?;

        let context_uuid: Option<Uuid> = context_id.map(|id| id.into());
        conn.interact(move |conn| {
            diesel::update(task_execution_metadata::table)
                .filter(task_execution_metadata::task_execution_id.eq(task_execution_id.0))
                .set((
                    task_execution_metadata::context_id.eq(context_uuid),
                    task_execution_metadata::updated_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Creates or updates task execution metadata
    ///
    /// If a record with the same task execution ID exists, it will be updated.
    /// Otherwise, a new record will be created.
    ///
    /// # Arguments
    /// * `new_metadata` - The metadata to create or update
    ///
    /// # Returns
    /// * `Result<TaskExecutionMetadata, ValidationError>` - The created/updated metadata record or an error
    pub async fn upsert_task_execution_metadata(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let mut conn = self.dal.pool.get().await?;

        let metadata: TaskExecutionMetadata = conn.interact(move |conn| {
            diesel::insert_into(task_execution_metadata::table)
                .values(&new_metadata)
                .on_conflict(task_execution_metadata::task_execution_id)
                .do_update()
                .set((
                    task_execution_metadata::context_id.eq(&new_metadata.context_id),
                    task_execution_metadata::updated_at.eq(diesel::dsl::now),
                ))
                .get_result(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(metadata)
    }

    /// Retrieves metadata for multiple dependency tasks within a pipeline
    ///
    /// # Arguments
    /// * `pipeline_id` - The UUID of the pipeline execution
    /// * `dependency_task_names` - A slice of task names to retrieve metadata for
    ///
    /// # Returns
    /// * `Result<Vec<TaskExecutionMetadata>, ValidationError>` - Vector of metadata records or an error
    pub async fn get_dependency_metadata(
        &self,
        pipeline_id: UniversalUuid,
        dependency_task_names: &[String],
    ) -> Result<Vec<TaskExecutionMetadata>, ValidationError> {
        let mut conn = self.dal.pool.get().await?;

        let dependency_task_names_owned = dependency_task_names.to_vec();
        let metadata = conn.interact(move |conn| {
            task_execution_metadata::table
                .filter(task_execution_metadata::pipeline_execution_id.eq(pipeline_id.0))
                .filter(task_execution_metadata::task_name.eq_any(&dependency_task_names_owned))
                .load(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(metadata)
    }

    /// Retrieves metadata and context data for multiple dependency tasks in a single query.
    ///
    /// This method performs a JOIN between task_execution_metadata and contexts tables
    /// to efficiently fetch both metadata and context data in one database roundtrip.
    ///
    /// # Arguments
    /// * `pipeline_id` - The UUID of the pipeline execution
    /// * `dependency_task_names` - A slice of task names to retrieve data for
    ///
    /// # Returns
    /// * `Result<Vec<(TaskExecutionMetadata, Option<String>)>, ValidationError>` - Vector of (metadata, context_data) tuples
    ///
    /// # Performance
    /// This method replaces N+1 queries (1 for metadata + N for contexts) with a single JOIN query.
    pub async fn get_dependency_metadata_with_contexts(
        &self,
        pipeline_id: UniversalUuid,
        dependency_task_names: &[String],
    ) -> Result<Vec<(TaskExecutionMetadata, Option<String>)>, ValidationError> {
        use crate::database::schema::contexts;

        if dependency_task_names.is_empty() {
            return Ok(Vec::new());
        }

        let mut conn = self.dal.pool.get().await?;

        let dependency_task_names_owned = dependency_task_names.to_vec();
        let results = conn.interact(move |conn| {
            task_execution_metadata::table
                .left_join(
                    contexts::table.on(task_execution_metadata::context_id.eq(contexts::id.nullable())),
                )
                .filter(task_execution_metadata::pipeline_execution_id.eq(pipeline_id.0))
                .filter(task_execution_metadata::task_name.eq_any(&dependency_task_names_owned))
                .select((
                    task_execution_metadata::all_columns,
                    contexts::value.nullable(),
                ))
                .load::<(TaskExecutionMetadata, Option<String>)>(conn)
        }).await.map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results)
    }
}
