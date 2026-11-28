/*
 *  Copyright 2025-2026 Colliery Software
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
    /// Parent workflow execution ID
    pub workflow_execution_id: UniversalUuid,
    /// Fully qualified task name (namespace::task)
    pub task_name: String,
    /// Current attempt number (starts at 1)
    pub attempt: i32,
    /// The turn of this dispatch among the dispatches of one scheduler pass
    /// (CLOACINA-T-1045). An executor calls [`Admission::release`] when it
    /// has decided whether the task gets a slot.
    pub admission: Admission,
}

impl TaskReadyEvent {
    /// Creates a new TaskReadyEvent.
    pub fn new(
        task_execution_id: UniversalUuid,
        workflow_execution_id: UniversalUuid,
        task_name: String,
        attempt: i32,
    ) -> Self {
        Self {
            task_execution_id,
            workflow_execution_id,
            task_name,
            attempt,
            admission: Admission::none(),
        }
    }

    /// The same event with a turn in the order of a scheduler pass.
    pub fn with_admission(mut self, admission: Admission) -> Self {
        self.admission = admission;
        self
    }
}

/// The turn of one dispatch in the order of a scheduler pass
/// (CLOACINA-T-1045).
///
/// The scheduler loop dispatches the waiting tasks in priority order. On a
/// backend that dispatches them at the same time, each dispatch waits for the
/// one before it to decide about its slot, so the slots that are free go to
/// the tasks in that order. The executor releases the turn as soon as it has
/// decided: the task has a slot, or it has none and waits. The turn is also
/// released when the dispatch ends. A dispatch with no turn (`none`) releases
/// nothing.
#[derive(Clone, Default)]
pub struct Admission {
    released: Option<std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>>,
}

impl Admission {
    /// No turn: the dispatch is not in an ordered pass.
    pub fn none() -> Self {
        Self { released: None }
    }

    /// A turn, and the receiver that resolves when the turn is released.
    pub fn new() -> (Self, tokio::sync::oneshot::Receiver<()>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        (
            Self {
                released: Some(std::sync::Arc::new(std::sync::Mutex::new(Some(tx)))),
            },
            rx,
        )
    }

    /// Let the next dispatch of the pass decide. Safe to call more than once.
    pub fn release(&self) {
        if let Some(released) = &self.released {
            let sender = released.lock().unwrap_or_else(|e| e.into_inner()).take();
            if let Some(sender) = sender {
                let _ = sender.send(());
            }
        }
    }
}

impl std::fmt::Debug for Admission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.released.is_some() {
            "Admission(turn)"
        } else {
            "Admission(none)"
        })
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
    /// Task was skipped (e.g., claimed by another runner)
    Skipped,
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

    /// Creates a skipped execution result (task claimed by another runner).
    pub fn skipped(task_execution_id: UniversalUuid) -> Self {
        Self {
            task_execution_id,
            status: ExecutionStatus::Skipped,
            error: None,
            duration: Duration::ZERO,
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
