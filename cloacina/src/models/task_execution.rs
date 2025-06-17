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

//! Task Execution Model
//!
//! This module defines the data structures for tracking task executions within pipeline runs.
//! It provides models for both querying existing task executions and creating new ones.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a task execution record in the database.
///
/// This struct maps to the `task_executions` table and contains all information about
/// a single task's execution within a pipeline run, including its status, timing,
/// and configuration details.
#[derive(Debug, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::task_executions)]
#[cfg_attr(feature = "postgres", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "sqlite", diesel(check_for_backend(diesel::sqlite::Sqlite)))]
pub struct TaskExecution {
    /// Unique identifier for the task execution
    pub id: UniversalUuid,
    /// Reference to the parent pipeline execution
    pub pipeline_execution_id: UniversalUuid,
    /// Full namespace of the task being executed
    pub task_namespace: String,
    /// Current status of the task execution (e.g., "pending", "running", "completed", "failed")
    pub status: String,
    /// Timestamp when the task execution started
    pub started_at: Option<UniversalTimestamp>,
    /// Timestamp when the task execution completed
    pub completed_at: Option<UniversalTimestamp>,
    /// Current attempt number for this task execution
    pub attempt: i32,
    /// Maximum number of attempts allowed for this task
    pub max_attempts: i32,
    /// Detailed error information if the task failed
    pub error_details: Option<String>,
    /// JSON string containing rules that determine when this task should be triggered
    pub trigger_rules: String,
    /// JSON string containing the task's configuration parameters
    pub task_configuration: String,
    /// Timestamp when the task should be retried (if applicable)
    pub retry_at: Option<UniversalTimestamp>,
    /// Most recent error message encountered
    pub last_error: Option<String>,
    /// Number of recovery attempts made for this task
    pub recovery_attempts: i32,
    /// Timestamp of the last recovery attempt
    pub last_recovery_at: Option<UniversalTimestamp>,
    /// Timestamp when the task execution record was created
    pub created_at: UniversalTimestamp,
    /// Timestamp when the task execution record was last updated
    pub updated_at: UniversalTimestamp,
}

/// Represents a new task execution to be inserted into the database.
///
/// This struct contains the minimum required fields for creating a new task execution
/// record. Additional fields will be populated by the database or during execution.
#[derive(Debug, Insertable)]
#[diesel(table_name = crate::database::schema::task_executions)]
pub struct NewTaskExecution {
    /// Reference to the parent pipeline execution
    pub pipeline_execution_id: UniversalUuid,
    /// Name of the task being executed
    pub task_name: String,
    /// Initial status of the task execution
    pub status: String,
    /// Initial attempt number (typically 1)
    pub attempt: i32,
    /// Maximum number of attempts allowed for this task
    pub max_attempts: i32,
    /// JSON string containing rules that determine when this task should be triggered
    pub trigger_rules: String,
    /// JSON string containing the task's configuration parameters
    pub task_configuration: String,
}
