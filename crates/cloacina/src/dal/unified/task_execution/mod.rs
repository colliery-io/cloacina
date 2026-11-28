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

//! Task Execution Data Access Layer for Unified Backend Support
//!
//! This module provides the data access layer for managing task executions in the workflow system
//! with runtime backend selection between PostgreSQL and SQLite.
//!
//! Key features:
//! - Task state management (Ready, Running, Completed, Failed, Skipped)
//! - Retry mechanism with configurable backoff
//! - Recovery system for handling orphaned tasks
//! - Atomic task claiming for distributed execution
//! - Workflow completion and failure detection

mod claiming;
mod crud;
mod placement;
mod queries;
mod recovery;
mod state;

pub use placement::{FleetPlacement, PoolWaitingTasks, WaitingDemand};

use super::DAL;
use crate::database::universal_types::UniversalUuid;

/// Statistics about retry behavior for a workflow execution.
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
    /// ID of the workflow execution this task belongs to
    pub workflow_execution_id: UniversalUuid,
    /// Name of the task that was claimed
    pub task_name: String,
    /// Current attempt number for this task
    pub attempt: i32,
}

/// Result of attempting to claim a task for a specific runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerClaimResult {
    /// Successfully claimed the task.
    Claimed,
    /// Another runner already claimed this task.
    AlreadyClaimed,
}

/// Result of a heartbeat attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeartbeatResult {
    /// Heartbeat updated successfully — this runner still owns the claim.
    Ok,
    /// Claim was lost — another runner has claimed this task (or claim was released).
    ClaimLost,
}

/// A task with a stale claim (heartbeat expired).
#[derive(Debug, Clone)]
pub struct StaleClaim {
    /// Task execution ID.
    pub task_id: UniversalUuid,
    /// The runner that claimed it (now presumed dead).
    pub claimed_by: UniversalUuid,
    /// Last heartbeat timestamp.
    pub heartbeat_at: chrono::DateTime<chrono::Utc>,
    /// How many times this task has already been recovered from a stale
    /// claim. Lets the sweeper cap recycling (CLOACI-T-0914) instead of
    /// re-Readying a runner-killing task forever.
    pub recovery_attempts: i32,
}

/// The tasks that wait for capacity (CLOACINA-T-1024, CLOACINA-T-1036): status
/// `Ready` with a `waiting_since` that is not in the future. Feeds the
/// `cloacina_waiting_tasks` and `cloacina_oldest_waiting_task_age_seconds`
/// gauges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingTasks {
    /// Number of waiting tasks.
    pub count: i64,
    /// The earliest `waiting_since` of them. `None` when no task waits.
    pub oldest_waiting_since: Option<chrono::DateTime<chrono::Utc>>,
}

impl WaitingTasks {
    /// Age of the oldest waiting task at `now`, in seconds. 0 when no task
    /// waits, and never negative.
    pub fn oldest_age_seconds(&self, now: chrono::DateTime<chrono::Utc>) -> f64 {
        match self.oldest_waiting_since {
            Some(since) => ((now - since).num_milliseconds() as f64 / 1000.0).max(0.0),
            None => 0.0,
        }
    }
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
