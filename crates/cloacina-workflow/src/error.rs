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

//! # Error Types
//!
//! This module defines the minimal error types needed for workflow authoring.
//! These errors do not include database or runtime-specific variants, which
//! are defined in the main `cloacina` crate.
//!
//! ## Error Types
//!
//! - [`ContextError`]: Errors related to context operations
//! - [`TaskError`]: Errors that occur during task execution
//! - [`CheckpointError`]: Errors in task checkpointing

use chrono::{DateTime, Utc};
use thiserror::Error;

/// Errors that can occur during context operations.
///
/// This minimal version only includes errors that can occur without database
/// or runtime dependencies.
#[derive(Debug, Error)]
pub enum ContextError {
    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Key not found in context
    #[error("Key not found: {0}")]
    KeyNotFound(String),

    /// Type mismatch when retrieving a value
    #[error("Type mismatch for key {0}")]
    TypeMismatch(String),

    /// Key already exists when inserting
    #[error("Key already exists: {0}")]
    KeyExists(String),
}

/// Errors that can occur during task execution.
///
/// Task errors encompass execution failures, context issues, and
/// any other problems that prevent a task from completing successfully.
#[derive(Debug, Error)]
pub enum TaskError {
    /// Task execution failed with a message
    #[error("Task execution failed: {message}")]
    ExecutionFailed {
        message: String,
        task_id: String,
        timestamp: DateTime<Utc>,
    },

    /// Task dependency not satisfied
    #[error("Task dependency not satisfied: {dependency} required by {task_id}")]
    DependencyNotSatisfied { dependency: String, task_id: String },

    /// Task exceeded timeout
    #[error("Task timeout: {task_id} exceeded {timeout_seconds}s")]
    Timeout {
        task_id: String,
        timeout_seconds: u64,
    },

    /// Context operation error within a task
    #[error("Context error in task {task_id}: {error}")]
    ContextError {
        task_id: String,
        error: ContextError,
    },

    /// Task validation failed
    #[error("Task validation failed: {message}")]
    ValidationFailed { message: String },

    /// Unknown error
    #[error("Unknown error in task {task_id}: {message}")]
    Unknown { task_id: String, message: String },

    /// Task readiness check failed
    #[error("Task readiness check failed: {task_id}")]
    ReadinessCheckFailed { task_id: String },

    /// Trigger rule evaluation failed
    #[error("Trigger rule evaluation failed: {task_id}")]
    TriggerRuleFailed { task_id: String },
}

impl From<ContextError> for TaskError {
    fn from(error: ContextError) -> Self {
        TaskError::ContextError {
            task_id: "unknown".to_string(),
            error,
        }
    }
}

/// Errors that can occur during task checkpointing.
///
/// Checkpoint errors occur when tasks attempt to save intermediate state
/// for recovery purposes.
#[derive(Debug, Error)]
pub enum CheckpointError {
    /// Failed to save checkpoint
    #[error("Failed to save checkpoint for task {task_id}: {message}")]
    SaveFailed { task_id: String, message: String },

    /// Failed to load checkpoint
    #[error("Failed to load checkpoint for task {task_id}: {message}")]
    LoadFailed { task_id: String, message: String },

    /// Checkpoint serialization error
    #[error("Checkpoint serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Checkpoint storage error
    #[error("Checkpoint storage error: {message}")]
    StorageError { message: String },

    /// Checkpoint validation failed
    #[error("Checkpoint validation failed: {message}")]
    ValidationFailed { message: String },
}
