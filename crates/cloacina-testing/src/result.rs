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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_result() -> TestResult {
        let mut outcomes = IndexMap::new();
        outcomes.insert("task_a".to_string(), TaskOutcome::Completed);
        outcomes.insert(
            "task_b".to_string(),
            TaskOutcome::Failed(TaskError::ExecutionFailed {
                message: "boom".to_string(),
                task_id: "task_b".to_string(),
                timestamp: chrono::Utc::now(),
            }),
        );
        outcomes.insert("task_c".to_string(), TaskOutcome::Skipped);

        TestResult {
            context: Context::new(),
            task_outcomes: outcomes,
        }
    }

    #[test]
    fn test_task_outcome_predicates() {
        assert!(TaskOutcome::Completed.is_completed());
        assert!(!TaskOutcome::Completed.is_failed());
        assert!(!TaskOutcome::Completed.is_skipped());

        let failed = TaskOutcome::Failed(TaskError::ExecutionFailed {
            message: "err".to_string(),
            task_id: "t".to_string(),
            timestamp: chrono::Utc::now(),
        });
        assert!(failed.is_failed());
        assert!(!failed.is_completed());

        assert!(TaskOutcome::Skipped.is_skipped());
        assert!(!TaskOutcome::Skipped.is_completed());
    }

    #[test]
    fn test_task_outcome_display() {
        assert_eq!(format!("{}", TaskOutcome::Completed), "Completed");
        assert_eq!(format!("{}", TaskOutcome::Skipped), "Skipped");
        let failed = TaskOutcome::Failed(TaskError::ExecutionFailed {
            message: "oops".to_string(),
            task_id: "t".to_string(),
            timestamp: chrono::Utc::now(),
        });
        let display = format!("{}", failed);
        assert!(display.starts_with("Failed("));
    }

    #[test]
    fn test_unwrap_error() {
        let failed = TaskOutcome::Failed(TaskError::ExecutionFailed {
            message: "test error".to_string(),
            task_id: "t".to_string(),
            timestamp: chrono::Utc::now(),
        });
        let err = failed.unwrap_error();
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    #[should_panic(expected = "unwrap_error")]
    fn test_unwrap_error_on_completed_panics() {
        TaskOutcome::Completed.unwrap_error();
    }

    #[test]
    fn test_index_access() {
        let result = make_test_result();
        assert!(result["task_a"].is_completed());
        assert!(result["task_b"].is_failed());
        assert!(result["task_c"].is_skipped());
    }

    #[test]
    #[should_panic(expected = "not found")]
    fn test_index_missing_task_panics() {
        let result = make_test_result();
        let _ = &result["nonexistent"];
    }

    // Assertion tests (from assertions.rs impl)

    #[test]
    fn test_assert_task_completed() {
        let result = make_test_result();
        result.assert_task_completed("task_a");
    }

    #[test]
    #[should_panic(expected = "expected task")]
    fn test_assert_task_completed_on_failed_panics() {
        let result = make_test_result();
        result.assert_task_completed("task_b");
    }

    #[test]
    fn test_assert_task_failed() {
        let result = make_test_result();
        result.assert_task_failed("task_b");
    }

    #[test]
    fn test_assert_task_skipped() {
        let result = make_test_result();
        result.assert_task_skipped("task_c");
    }

    #[test]
    #[should_panic(expected = "not found")]
    fn test_assert_task_completed_missing_panics() {
        let result = make_test_result();
        result.assert_task_completed("nonexistent");
    }

    #[test]
    #[should_panic(expected = "expected all tasks")]
    fn test_assert_all_completed_with_failures() {
        let result = make_test_result();
        result.assert_all_completed();
    }

    #[test]
    fn test_assert_all_completed_success() {
        let mut outcomes = IndexMap::new();
        outcomes.insert("a".to_string(), TaskOutcome::Completed);
        outcomes.insert("b".to_string(), TaskOutcome::Completed);
        let result = TestResult {
            context: Context::new(),
            task_outcomes: outcomes,
        };
        result.assert_all_completed(); // should not panic
    }
}
