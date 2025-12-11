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
//! This module defines comprehensive error types for all components of the Cloacina framework.
//! The error types follow a hierarchical structure that provides detailed context for debugging
//! and error handling.
//!
//! ## Error Categories
//!
//! - [`ContextError`]: Errors related to context operations (insert, update, serialization)
//! - [`TaskError`]: Errors that occur during task execution
//! - [`WorkflowError`]: Errors in workflow construction and validation
//! - [`ValidationError`]: Graph validation and dependency resolution errors
//! - [`CheckpointError`]: Errors in task checkpointing
//! - [`RegistrationError`]: Task registration and ID validation errors
//! - [`SubgraphError`]: Errors when creating Workflow subgraphs
//! - [`ExecutorError`]: Errors during task execution and pipeline management
//!
//! ## Error Handling Patterns
//!
//! ```rust,ignore
//! use cloacina::*;
//!
//! # let mut context = Context::<i32>::new();
//! # let value = 42;
//! match context.insert("key", value) {
//!     Ok(()) => println!("Success"),
//!     Err(ContextError::KeyExists(key)) => {
//!         println!("Key '{}' already exists", key);
//!     }
//!     Err(ContextError::Serialization(e)) => {
//!         eprintln!("Failed to serialize: {}", e);
//!     }
//!     Err(e) => eprintln!("Other error: {}", e),
//! }
//! ```
//!
//! ## Error Type Details
//!
//! ### ContextError
//! Handles errors related to the execution context, including:
//! - Serialization failures
//! - Key management issues (not found, exists, type mismatch)
//! - Database and connection pool errors
//! - Invalid execution scope errors
//!
//! ### TaskError
//! Covers task execution failures, including:
//! - Execution failures with detailed context
//! - Dependency satisfaction issues
//! - Timeout conditions
//! - Context-related errors
//! - Validation and readiness check failures
//! - Trigger rule evaluation issues
//!
//! ### ValidationError
//! Encompasses workflow and graph validation issues:
//! - Circular dependencies
//! - Missing dependencies
//! - Duplicate task IDs
//! - Empty workflow conditions
//! - Invalid graph structures
//! - Database and connection issues
//! - Recovery-specific errors
//!
//! ### ExecutorError
//! Manages errors during task execution and pipeline management:
//! - Database and connection pool issues
//! - Task registry problems
//! - Execution timeouts
//! - Semaphore acquisition failures
//! - Pipeline execution issues
//! - Serialization errors
//! - Scope and validation errors
//!
//! ### CheckpointError
//! Handles task state persistence issues:
//! - Save and load failures
//! - Serialization errors
//! - Storage and validation problems
//!
//! ### RegistrationError
//! Manages task registration issues:
//! - Duplicate task IDs
//! - Invalid task IDs
//! - Registration failures
//!
//! ### WorkflowError
//! Covers workflow construction problems:
//! - Duplicate tasks
//! - Missing tasks
//! - Invalid dependencies
//! - Cyclic dependencies
//! - Unreachable tasks
//!
//! ### SubgraphError
//! Handles subgraph creation issues:
//! - Missing tasks
//! - Unsupported operations
//!
//! All error types implement the standard `Error` trait and provide detailed
//! error messages for debugging and logging purposes. Each error variant includes
//! relevant context information to aid in troubleshooting and recovery.

use thiserror::Error;
use uuid::Uuid;

// Re-export TaskError and CheckpointError from cloacina_workflow
// This ensures type compatibility with macro-generated code
pub use cloacina_workflow::{CheckpointError, TaskError};

/// Errors that can occur during context operations.
///
/// Context errors cover data manipulation, serialization, and key management
/// within the execution context.
#[derive(Debug, Error)]
pub enum ContextError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Type mismatch for key {0}")]
    TypeMismatch(String),

    #[error("Key already exists: {0}")]
    KeyExists(String),

    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),

    #[error("Connection pool error: {0}")]
    ConnectionPool(String),

    #[error("Invalid execution scope: {0}")]
    InvalidScope(String),
}

impl From<cloacina_workflow::ContextError> for ContextError {
    fn from(err: cloacina_workflow::ContextError) -> Self {
        match err {
            cloacina_workflow::ContextError::Serialization(e) => ContextError::Serialization(e),
            cloacina_workflow::ContextError::KeyNotFound(k) => ContextError::KeyNotFound(k),
            cloacina_workflow::ContextError::TypeMismatch(k) => ContextError::TypeMismatch(k),
            cloacina_workflow::ContextError::KeyExists(k) => ContextError::KeyExists(k),
        }
    }
}

/// Errors that can occur during task registration.
///
/// Registration errors prevent tasks from being added to a registry
/// due to validation failures or conflicts.
#[derive(Debug, Error)]
pub enum RegistrationError {
    #[error("Task with id '{id}' already registered")]
    DuplicateTaskId { id: String },

    #[error("Invalid task id: {message}")]
    InvalidTaskId { message: String },

    #[error("Task registration failed: {message}")]
    RegistrationFailed { message: String },
}

/// Errors that can occur during Workflow and dependency validation.
///
/// Validation errors indicate structural problems with the task graph
/// that prevent safe execution.
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Circular dependency detected: {cycle:?}")]
    CyclicDependency { cycle: Vec<String> },

    #[error("Missing dependency: task '{task}' depends on '{dependency}' which is not registered")]
    MissingDependency { task: String, dependency: String },

    #[error(
        "Missing dependency: task '{task_id}' depends on '{dependency}' which is not registered"
    )]
    MissingDependencyOld { task_id: String, dependency: String },

    #[error("Circular dependency detected: {cycle}")]
    CircularDependency { cycle: String },

    #[error("Duplicate task ID: {0}")]
    DuplicateTaskId(String),

    #[error("Workflow cannot be empty")]
    EmptyWorkflow,

    #[error("Invalid dependency graph: {message}")]
    InvalidGraph { message: String },

    #[error("Workflow not found in registry: {0}")]
    WorkflowNotFound(String),

    #[error("Pipeline execution failed: {message}")]
    ExecutionFailed { message: String },

    #[error("Task scheduling failed: {task_id}")]
    TaskSchedulingFailed { task_id: String },

    #[error("Invalid trigger rule format: {0}")]
    InvalidTriggerRule(String),

    #[error("Invalid task name format: {0}")]
    InvalidTaskName(String),

    #[error("Context value evaluation failed: {key}")]
    ContextEvaluationFailed { key: String },

    // Recovery-specific errors
    #[error("Recovery operation failed: {message}")]
    RecoveryFailed { message: String },

    #[error("Task recovery abandoned: {task_id} after {attempts} attempts")]
    TaskRecoveryAbandoned { task_id: String, attempts: i32 },

    #[error("Pipeline recovery failed: {pipeline_id}")]
    PipelineRecoveryFailed { pipeline_id: uuid::Uuid },

    #[error("Database connection error: {message}")]
    DatabaseConnection { message: String },

    #[error("Database query error: {message}")]
    DatabaseQuery { message: String },

    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),

    #[error("Connection pool error: {0}")]
    ConnectionPool(String),

    #[error("Context error: {0}")]
    Context(#[from] ContextError),
}

impl From<deadpool::managed::PoolError<deadpool_diesel::Error>> for ValidationError {
    fn from(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self {
        ValidationError::ConnectionPool(err.to_string())
    }
}

impl From<deadpool::managed::PoolError<deadpool_diesel::Error>> for ContextError {
    fn from(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self {
        ContextError::ConnectionPool(err.to_string())
    }
}

/// Errors that can occur during task execution.
#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),

    #[error("Connection pool error: {0}")]
    ConnectionPool(String),

    #[error("Task not found in registry: {0}")]
    TaskNotFound(String),

    #[error("Task execution error: {0}")]
    TaskExecution(#[from] TaskError),

    #[error("Context error: {0}")]
    Context(#[from] ContextError),

    #[error("Task execution timeout")]
    TaskTimeout,

    #[error("Semaphore acquisition error: {0}")]
    Semaphore(#[from] tokio::sync::AcquireError),

    #[error("Pipeline execution not found: {0}")]
    PipelineNotFound(Uuid),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid context scope: {0}")]
    InvalidScope(String),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
}

impl From<deadpool::managed::PoolError<deadpool_diesel::Error>> for ExecutorError {
    fn from(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self {
        ExecutorError::ConnectionPool(err.to_string())
    }
}

/// Errors that can occur during workflow construction and management.
///
/// Workflow errors occur when building or modifying workflows.
#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("Duplicate task: {0}")]
    DuplicateTask(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Invalid dependency: {0}")]
    InvalidDependency(String),

    #[error("Cyclic dependency: {0:?}")]
    CyclicDependency(Vec<String>),

    #[error("Unreachable task: {0}")]
    UnreachableTask(String),

    #[error("Registry error: {0}")]
    RegistryError(String),

    #[error("Task error: {0}")]
    TaskError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// Errors that can occur when creating Workflow subgraphs.
///
/// Subgraph errors occur when extracting portions of a Workflow for
/// partial execution or analysis.
#[derive(Debug, Error)]
pub enum SubgraphError {
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
}

// Conversion implementations
impl From<ContextError> for TaskError {
    fn from(error: ContextError) -> Self {
        // Convert to cloacina_workflow::ContextError first
        let workflow_error = match error {
            ContextError::Serialization(e) => cloacina_workflow::ContextError::Serialization(e),
            ContextError::KeyNotFound(k) => cloacina_workflow::ContextError::KeyNotFound(k),
            ContextError::TypeMismatch(k) => cloacina_workflow::ContextError::TypeMismatch(k),
            ContextError::KeyExists(k) => cloacina_workflow::ContextError::KeyExists(k),
            // Database and ConnectionPool errors don't have workflow equivalents,
            // so convert them to a generic message
            ContextError::Database(e) => {
                cloacina_workflow::ContextError::KeyNotFound(format!("Database error: {}", e))
            }
            ContextError::ConnectionPool(msg) => cloacina_workflow::ContextError::KeyNotFound(
                format!("Connection pool error: {}", msg),
            ),
            ContextError::InvalidScope(msg) => {
                cloacina_workflow::ContextError::KeyNotFound(format!("Invalid scope: {}", msg))
            }
        };
        TaskError::ContextError {
            task_id: "unknown".to_string(),
            error: workflow_error,
        }
    }
}
