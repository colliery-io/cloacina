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

//! Execution Event Model
//!
//! This module defines domain structures and types for tracking execution events.
//! Execution events provide a complete audit trail of task and pipeline state
//! transitions for debugging, compliance, and replay capability.
//!
//! These are API-level types; backend-specific models handle database storage.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};

/// Represents an execution event record (domain type).
///
/// Execution events are append-only records tracking all state transitions
/// for tasks and pipelines. Each event captures the transition type, associated
/// context, and ordering information for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    /// Unique identifier for this event
    pub id: UniversalUuid,
    /// The pipeline execution this event belongs to
    pub pipeline_execution_id: UniversalUuid,
    /// The task execution this event relates to (None for pipeline-level events)
    pub task_execution_id: Option<UniversalUuid>,
    /// The type of event (e.g., "task_created", "task_completed")
    pub event_type: String,
    /// JSON-encoded additional data for the event
    pub event_data: Option<String>,
    /// Worker ID that generated this event (for distributed tracing)
    pub worker_id: Option<String>,
    /// When this event was created
    pub created_at: UniversalTimestamp,
    /// Monotonically increasing sequence number for ordering
    pub sequence_num: i64,
}

/// Structure for creating new execution event records (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewExecutionEvent {
    /// The pipeline execution this event belongs to
    pub pipeline_execution_id: UniversalUuid,
    /// The task execution this event relates to (None for pipeline-level events)
    pub task_execution_id: Option<UniversalUuid>,
    /// The type of event
    pub event_type: String,
    /// JSON-encoded additional data for the event
    pub event_data: Option<String>,
    /// Worker ID that generated this event
    pub worker_id: Option<String>,
}

impl NewExecutionEvent {
    /// Creates a new execution event for a pipeline-level transition.
    pub fn pipeline_event(
        pipeline_execution_id: UniversalUuid,
        event_type: ExecutionEventType,
        event_data: Option<String>,
        worker_id: Option<String>,
    ) -> Self {
        Self {
            pipeline_execution_id,
            task_execution_id: None,
            event_type: event_type.as_str().to_string(),
            event_data,
            worker_id,
        }
    }

    /// Creates a new execution event for a task-level transition.
    pub fn task_event(
        pipeline_execution_id: UniversalUuid,
        task_execution_id: UniversalUuid,
        event_type: ExecutionEventType,
        event_data: Option<String>,
        worker_id: Option<String>,
    ) -> Self {
        Self {
            pipeline_execution_id,
            task_execution_id: Some(task_execution_id),
            event_type: event_type.as_str().to_string(),
            event_data,
            worker_id,
        }
    }
}

/// Enumeration of execution event types in the system.
///
/// These cover the full lifecycle of tasks and pipelines, providing
/// complete observability into execution state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionEventType {
    // Task lifecycle events
    /// A new task execution record was created
    TaskCreated,
    /// Task transitioned to Ready status (eligible for claiming)
    TaskMarkedReady,
    /// Task was claimed by a worker
    TaskClaimed,
    /// Task execution started
    TaskStarted,
    /// Task was deferred (waiting for external condition)
    TaskDeferred,
    /// Deferred task resumed execution
    TaskResumed,
    /// Task completed successfully
    TaskCompleted,
    /// Task failed with an error
    TaskFailed,
    /// Task scheduled for retry after failure
    TaskRetryScheduled,
    /// Task was skipped (trigger rules not met)
    TaskSkipped,
    /// Task was abandoned (exceeded max retries or manually cancelled)
    TaskAbandoned,
    /// Task was reset by recovery process
    TaskReset,

    // Pipeline lifecycle events
    /// Pipeline execution started
    PipelineStarted,
    /// Pipeline execution completed successfully
    PipelineCompleted,
    /// Pipeline execution failed
    PipelineFailed,
    /// Pipeline was paused
    PipelinePaused,
    /// Paused pipeline was resumed
    PipelineResumed,
}

impl ExecutionEventType {
    /// Returns the string representation of the event type.
    pub fn as_str(&self) -> &'static str {
        match self {
            // Task events
            ExecutionEventType::TaskCreated => "task_created",
            ExecutionEventType::TaskMarkedReady => "task_marked_ready",
            ExecutionEventType::TaskClaimed => "task_claimed",
            ExecutionEventType::TaskStarted => "task_started",
            ExecutionEventType::TaskDeferred => "task_deferred",
            ExecutionEventType::TaskResumed => "task_resumed",
            ExecutionEventType::TaskCompleted => "task_completed",
            ExecutionEventType::TaskFailed => "task_failed",
            ExecutionEventType::TaskRetryScheduled => "task_retry_scheduled",
            ExecutionEventType::TaskSkipped => "task_skipped",
            ExecutionEventType::TaskAbandoned => "task_abandoned",
            ExecutionEventType::TaskReset => "task_reset",
            // Pipeline events
            ExecutionEventType::PipelineStarted => "pipeline_started",
            ExecutionEventType::PipelineCompleted => "pipeline_completed",
            ExecutionEventType::PipelineFailed => "pipeline_failed",
            ExecutionEventType::PipelinePaused => "pipeline_paused",
            ExecutionEventType::PipelineResumed => "pipeline_resumed",
        }
    }

    /// Parses an event type from its string representation.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "task_created" => Some(ExecutionEventType::TaskCreated),
            "task_marked_ready" => Some(ExecutionEventType::TaskMarkedReady),
            "task_claimed" => Some(ExecutionEventType::TaskClaimed),
            "task_started" => Some(ExecutionEventType::TaskStarted),
            "task_deferred" => Some(ExecutionEventType::TaskDeferred),
            "task_resumed" => Some(ExecutionEventType::TaskResumed),
            "task_completed" => Some(ExecutionEventType::TaskCompleted),
            "task_failed" => Some(ExecutionEventType::TaskFailed),
            "task_retry_scheduled" => Some(ExecutionEventType::TaskRetryScheduled),
            "task_skipped" => Some(ExecutionEventType::TaskSkipped),
            "task_abandoned" => Some(ExecutionEventType::TaskAbandoned),
            "task_reset" => Some(ExecutionEventType::TaskReset),
            "pipeline_started" => Some(ExecutionEventType::PipelineStarted),
            "pipeline_completed" => Some(ExecutionEventType::PipelineCompleted),
            "pipeline_failed" => Some(ExecutionEventType::PipelineFailed),
            "pipeline_paused" => Some(ExecutionEventType::PipelinePaused),
            "pipeline_resumed" => Some(ExecutionEventType::PipelineResumed),
            _ => None,
        }
    }

    /// Returns true if this is a task-level event.
    pub fn is_task_event(&self) -> bool {
        matches!(
            self,
            ExecutionEventType::TaskCreated
                | ExecutionEventType::TaskMarkedReady
                | ExecutionEventType::TaskClaimed
                | ExecutionEventType::TaskStarted
                | ExecutionEventType::TaskDeferred
                | ExecutionEventType::TaskResumed
                | ExecutionEventType::TaskCompleted
                | ExecutionEventType::TaskFailed
                | ExecutionEventType::TaskRetryScheduled
                | ExecutionEventType::TaskSkipped
                | ExecutionEventType::TaskAbandoned
                | ExecutionEventType::TaskReset
        )
    }

    /// Returns true if this is a pipeline-level event.
    pub fn is_pipeline_event(&self) -> bool {
        matches!(
            self,
            ExecutionEventType::PipelineStarted
                | ExecutionEventType::PipelineCompleted
                | ExecutionEventType::PipelineFailed
                | ExecutionEventType::PipelinePaused
                | ExecutionEventType::PipelineResumed
        )
    }
}

impl From<ExecutionEventType> for String {
    fn from(event_type: ExecutionEventType) -> Self {
        event_type.as_str().to_string()
    }
}

impl std::fmt::Display for ExecutionEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
