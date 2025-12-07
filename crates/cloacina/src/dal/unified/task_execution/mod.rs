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

//! Task Execution Data Access Layer for Unified Backend Support
//!
//! This module provides the data access layer for managing task executions in the pipeline system
//! with runtime backend selection between PostgreSQL and SQLite.
//!
//! Key features:
//! - Task state management (Ready, Running, Completed, Failed, Skipped)
//! - Retry mechanism with configurable backoff
//! - Recovery system for handling orphaned tasks
//! - Atomic task claiming for distributed execution
//! - Pipeline completion and failure detection

mod claiming;
mod crud;
mod queries;
mod recovery;
mod state;

use super::DAL;
use crate::database::universal_types::UniversalUuid;

/// Statistics about retry behavior for a pipeline execution.
#[derive(Debug, Default)]
pub struct RetryStats {
    /// Number of tasks that required at least one retry.
    pub tasks_with_retries: i32,
    /// Total number of retry attempts across all tasks.
    pub total_retries: i32,
    /// Maximum number of attempts used by any single task.
    pub max_attempts_used: i32,
    /// Number of tasks that exhausted all retry attempts and failed.
    pub tasks_exhausted_retries: i32,
}

/// Result structure for atomic task claiming operations.
#[derive(Debug)]
pub struct ClaimResult {
    /// Unique identifier of the claimed task
    pub id: UniversalUuid,
    /// ID of the pipeline execution this task belongs to
    pub pipeline_execution_id: UniversalUuid,
    /// Name of the task that was claimed
    pub task_name: String,
    /// Current attempt number for this task
    pub attempt: i32,
}

/// Data access layer for task execution operations with runtime backend selection.
#[derive(Clone)]
pub struct TaskExecutionDAL<'a> {
    dal: &'a DAL,
}

impl<'a> TaskExecutionDAL<'a> {
    /// Creates a new TaskExecutionDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
}
