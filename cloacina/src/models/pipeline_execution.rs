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

//! Pipeline Execution Models
//!
//! This module defines the data structures for tracking pipeline executions in the system.
//! Pipeline executions represent individual runs of data processing pipelines, including
//! their status, timing information, and any error details.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a completed or in-progress pipeline execution in the system.
///
/// This struct maps to the `pipeline_executions` table in the database and includes
/// all fields for tracking the lifecycle of a pipeline run, including timing information,
/// status updates, and error handling.
#[derive(Debug, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::pipeline_executions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PipelineExecution {
    /// Unique identifier for the pipeline execution
    pub id: Uuid,
    /// Name of the pipeline that was executed
    pub pipeline_name: String,
    /// Version of the pipeline that was executed
    pub pipeline_version: String,
    /// Current status of the pipeline execution (e.g., "running", "completed", "failed")
    pub status: String,
    /// Optional context identifier for grouping related pipeline executions
    pub context_id: Option<Uuid>,
    /// Timestamp when the pipeline execution started
    pub started_at: NaiveDateTime,
    /// Timestamp when the pipeline execution completed (if applicable)
    pub completed_at: Option<NaiveDateTime>,
    /// Detailed error information if the pipeline execution failed
    pub error_details: Option<String>,
    /// Number of recovery attempts made for this pipeline execution
    pub recovery_attempts: i32,
    /// Timestamp of the last recovery attempt (if any)
    pub last_recovery_at: Option<NaiveDateTime>,
    /// Timestamp when this record was created
    pub created_at: NaiveDateTime,
    /// Timestamp when this record was last updated
    pub updated_at: NaiveDateTime,
}

/// Represents a new pipeline execution to be inserted into the database.
///
/// This struct contains only the fields required to create a new pipeline execution.
/// Additional fields like timestamps and IDs are managed by the database.
#[derive(Debug, Insertable)]
#[diesel(table_name = crate::database::schema::pipeline_executions)]
pub struct NewPipelineExecution {
    /// Name of the pipeline to be executed
    pub pipeline_name: String,
    /// Version of the pipeline to be executed
    pub pipeline_version: String,
    /// Initial status of the pipeline execution
    pub status: String,
    /// Optional context identifier for grouping related pipeline executions
    pub context_id: Option<Uuid>,
}
