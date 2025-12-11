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

//! Core types for the dispatcher system.
//!
//! This module defines the fundamental data structures used for dispatching
//! tasks from the scheduler to executors.

use crate::database::UniversalUuid;
use std::time::Duration;
use thiserror::Error;

/// Event emitted when a task becomes ready for execution.
///
/// This event contains all the information needed to identify and route a task.
/// The actual context loading is deferred to execution time.
#[derive(Debug, Clone)]
pub struct TaskReadyEvent {
    /// Unique identifier for this task execution
    pub task_execution_id: UniversalUuid,
    /// Parent pipeline execution ID
    pub pipeline_execution_id: UniversalUuid,
    /// Fully qualified task name (namespace::task)
    pub task_name: String,
    /// Current attempt number (starts at 1)
    pub attempt: i32,
}

impl TaskReadyEvent {
    /// Creates a new TaskReadyEvent.
    pub fn new(
        task_execution_id: UniversalUuid,
        pipeline_execution_id: UniversalUuid,
        task_name: String,
        attempt: i32,
    ) -> Self {
        Self {
            task_execution_id,
            pipeline_execution_id,
            task_name,
            attempt,
        }
    }
}

/// Simplified status for execution results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task should be retried
    Retry,
}

/// Result of task execution from an executor.
///
/// This structure contains the outcome of a task execution, including
/// the final status, any error message, and execution metrics.
#[derive(Debug)]
pub struct ExecutionResult {
    /// The task execution ID
    pub task_execution_id: UniversalUuid,
    /// Final execution status
    pub status: ExecutionStatus,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Time taken to execute the task
    pub duration: Duration,
}

impl ExecutionResult {
    /// Creates a successful execution result.
    pub fn success(task_execution_id: UniversalUuid, duration: Duration) -> Self {
        Self {
            task_execution_id,
            status: ExecutionStatus::Completed,
            error: None,
            duration,
        }
    }

    /// Creates a failed execution result.
    pub fn failure(
        task_execution_id: UniversalUuid,
        error: impl Into<String>,
        duration: Duration,
    ) -> Self {
        Self {
            task_execution_id,
            status: ExecutionStatus::Failed,
            error: Some(error.into()),
            duration,
        }
    }

    /// Creates a retry execution result.
    pub fn retry(
        task_execution_id: UniversalUuid,
        error: impl Into<String>,
        duration: Duration,
    ) -> Self {
        Self {
            task_execution_id,
            status: ExecutionStatus::Retry,
            error: Some(error.into()),
            duration,
        }
    }
}

/// Metrics for monitoring executor performance.
#[derive(Debug, Clone, Default)]
pub struct ExecutorMetrics {
    /// Number of tasks currently executing
    pub active_tasks: usize,
    /// Maximum concurrent tasks allowed
    pub max_concurrent: usize,
    /// Total tasks executed since startup
    pub total_executed: u64,
    /// Total tasks that failed
    pub total_failed: u64,
    /// Average task duration in milliseconds
    pub avg_duration_ms: u64,
}

impl ExecutorMetrics {
    /// Returns the current capacity (available slots).
    pub fn available_capacity(&self) -> usize {
        self.max_concurrent.saturating_sub(self.active_tasks)
    }
}

/// Configuration for task routing.
///
/// Defines how tasks are routed to different executor backends based on
/// pattern matching rules.
#[derive(Debug, Clone)]
pub struct RoutingConfig {
    /// Default executor key when no rules match
    pub default_executor: String,
    /// Routing rules evaluated in order
    pub rules: Vec<RoutingRule>,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            default_executor: "default".to_string(),
            rules: Vec::new(),
        }
    }
}

impl RoutingConfig {
    /// Creates a new routing configuration with a default executor.
    pub fn new(default_executor: impl Into<String>) -> Self {
        Self {
            default_executor: default_executor.into(),
            rules: Vec::new(),
        }
    }

    /// Adds a routing rule.
    pub fn with_rule(mut self, rule: RoutingRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Adds multiple routing rules.
    pub fn with_rules(mut self, rules: impl IntoIterator<Item = RoutingRule>) -> Self {
        self.rules.extend(rules);
        self
    }
}

/// A routing rule for directing tasks to specific executors.
///
/// Rules are evaluated in order, and the first matching rule determines
/// which executor handles the task.
#[derive(Debug, Clone)]
pub struct RoutingRule {
    /// Glob pattern to match task names (e.g., "ml::*", "heavy::*")
    pub task_pattern: String,
    /// Executor key to route matching tasks to
    pub executor: String,
}

impl RoutingRule {
    /// Creates a new routing rule.
    pub fn new(task_pattern: impl Into<String>, executor: impl Into<String>) -> Self {
        Self {
            task_pattern: task_pattern.into(),
            executor: executor.into(),
        }
    }
}

/// Errors that can occur during dispatch operations.
#[derive(Debug, Error)]
pub enum DispatchError {
    /// The specified executor was not found
    #[error("Executor not found: {0}")]
    ExecutorNotFound(String),

    /// Task execution failed
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    /// Database operation failed
    #[error("Database error: {0}")]
    DatabaseError(#[from] crate::error::ExecutorError),

    /// Context operation failed
    #[error("Context error: {0}")]
    ContextError(#[from] crate::error::ContextError),

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(#[from] crate::error::ValidationError),

    /// The executor has no capacity
    #[error("Executor has no capacity: {0}")]
    NoCapacity(String),

    /// Task not found for dispatch
    #[error("Task not found: {0}")]
    TaskNotFound(UniversalUuid),
}
