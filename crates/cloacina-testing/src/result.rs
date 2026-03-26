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

//! Test result types for capturing task execution outcomes.

use cloacina_workflow::{Context, TaskError};
use indexmap::IndexMap;

/// The result of running tasks through a [`TestRunner`](crate::TestRunner).
///
/// Contains the final context after all tasks have executed, plus
/// per-task outcomes preserving execution order.
#[derive(Debug)]
pub struct TestResult {
    /// The final context after all tasks have executed.
    pub context: Context<serde_json::Value>,
    /// Per-task outcomes in execution order.
    pub task_outcomes: IndexMap<String, TaskOutcome>,
}

/// The outcome of a single task execution.
#[derive(Debug)]
pub enum TaskOutcome {
    /// Task executed successfully.
    Completed,
    /// Task execution failed with the given error.
    Failed(TaskError),
    /// Task was skipped because a dependency failed.
    Skipped,
}

impl TaskOutcome {
    /// Returns `true` if the task completed successfully.
    pub fn is_completed(&self) -> bool {
        matches!(self, TaskOutcome::Completed)
    }

    /// Returns `true` if the task failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, TaskOutcome::Failed(_))
    }

    /// Returns `true` if the task was skipped.
    pub fn is_skipped(&self) -> bool {
        matches!(self, TaskOutcome::Skipped)
    }

    /// Returns the error if the task failed, panics otherwise.
    ///
    /// # Panics
    ///
    /// Panics if the outcome is not `Failed`.
    pub fn unwrap_error(&self) -> &TaskError {
        match self {
            TaskOutcome::Failed(e) => e,
            other => panic!(
                "called unwrap_error() on a {:?} outcome",
                outcome_name(other)
            ),
        }
    }
}

impl std::fmt::Display for TaskOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskOutcome::Completed => write!(f, "Completed"),
            TaskOutcome::Failed(e) => write!(f, "Failed({})", e),
            TaskOutcome::Skipped => write!(f, "Skipped"),
        }
    }
}

impl std::ops::Index<&str> for TestResult {
    type Output = TaskOutcome;

    fn index(&self, task_id: &str) -> &Self::Output {
        self.task_outcomes.get(task_id).unwrap_or_else(|| {
            panic!(
                "task '{}' not found in test results. Available tasks: {:?}",
                task_id,
                self.task_outcomes.keys().collect::<Vec<_>>()
            )
        })
    }
}

fn outcome_name(outcome: &TaskOutcome) -> &'static str {
    match outcome {
        TaskOutcome::Completed => "Completed",
        TaskOutcome::Failed(_) => "Failed",
        TaskOutcome::Skipped => "Skipped",
    }
}
