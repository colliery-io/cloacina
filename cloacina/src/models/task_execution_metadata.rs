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

//! Task Execution Metadata Module
//!
//! This module defines the data structures for managing task execution metadata within the pipeline system.
//! Task execution metadata stores metadata and execution data for individual task executions within a pipeline execution.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a task execution metadata record in the database.
///
/// This struct maps to the `task_execution_metadata` table and contains all metadata and execution data
/// for a specific task execution within a pipeline execution. It includes timestamps for tracking
/// when the metadata was created and last updated.
#[derive(Debug, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::task_execution_metadata)]
#[cfg_attr(feature = "postgres", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "sqlite", diesel(check_for_backend(diesel::sqlite::Sqlite)))]
pub struct TaskExecutionMetadata {
    /// Unique identifier for the task execution metadata
    pub id: UniversalUuid,
    /// Reference to the associated task execution
    pub task_execution_id: UniversalUuid,
    /// Reference to the parent pipeline execution
    pub pipeline_execution_id: UniversalUuid,
    /// Full namespace of the task this metadata belongs to
    pub task_namespace: String,
    /// Reference to the context record containing the task's output data
    pub context_id: Option<UniversalUuid>,
    /// Timestamp when this metadata was created
    pub created_at: UniversalTimestamp,
    /// Timestamp when this metadata was last updated
    pub updated_at: UniversalTimestamp,
}

/// Represents a new task execution metadata to be inserted into the database.
///
/// This struct is used when creating new task execution metadata. It omits the `id`, `created_at`,
/// and `updated_at` fields as these are managed by the database.
#[derive(Debug, Insertable)]
#[diesel(table_name = crate::database::schema::task_execution_metadata)]
pub struct NewTaskExecutionMetadata {
    /// Reference to the associated task execution
    pub task_execution_id: UniversalUuid,
    /// Reference to the parent pipeline execution
    pub pipeline_execution_id: UniversalUuid,
    /// Name of the task this metadata belongs to
    pub task_name: String,
    /// Reference to the context record containing the task's output data
    pub context_id: Option<UniversalUuid>,
}
