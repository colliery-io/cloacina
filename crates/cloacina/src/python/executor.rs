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

//! Python task executor trait and error types.
//!
//! The [`PythonTaskExecutor`] trait abstracts over the PyO3 bridge so
//! that cloacina core does not depend on `pyo3`. The `cloaca-backend`
//! crate provides the concrete implementation.

use std::path::Path;
use thiserror::Error;

/// Errors that can occur during Python task execution.
#[derive(Debug, Error)]
pub enum PythonExecutionError {
    /// Failed to set up the Python environment (sys.path, imports).
    #[error("Python environment setup failed: {reason}")]
    EnvironmentSetup { reason: String },

    /// The requested task was not found in the entry module.
    #[error("Task '{task_id}' not found in module '{module}'")]
    TaskNotFound { task_id: String, module: String },

    /// The task function raised an exception.
    #[error("Task '{task_id}' raised an exception: {message}\n{traceback}")]
    TaskException {
        task_id: String,
        message: String,
        traceback: String,
    },

    /// Context serialization/deserialization failed.
    #[error("Context serialization error for task '{task_id}': {reason}")]
    SerializationError { task_id: String, reason: String },

    /// Import error — likely a missing vendored dependency.
    #[error("Import error in task '{task_id}': {message}")]
    ImportError { task_id: String, message: String },

    /// The GIL could not be acquired (e.g., Python not initialized).
    #[error("Python runtime not available: {reason}")]
    RuntimeUnavailable { reason: String },
}

/// Result of executing a Python task.
#[derive(Debug, Clone)]
pub struct PythonTaskResult {
    /// Task ID that was executed.
    pub task_id: String,
    /// JSON-serialized output from the Python function.
    pub output_json: String,
}

/// Trait for executing Python tasks from extracted packages.
///
/// Implementors (in `cloaca-backend`) use PyO3 to:
/// 1. Configure `sys.path` with vendor and workflow directories
/// 2. Import the entry module to trigger `@task` registration
/// 3. Look up the task function by ID
/// 4. Call the function with a JSON-serialized context dict
/// 5. Return the JSON-serialized result
///
/// All GIL operations must be performed inside `tokio::task::spawn_blocking`
/// to avoid blocking the async runtime.
#[async_trait::async_trait]
pub trait PythonTaskExecutor: Send + Sync {
    /// Execute a single Python task.
    ///
    /// # Arguments
    ///
    /// * `workflow_dir` — Path to the extracted `workflow/` directory.
    /// * `vendor_dir`   — Path to the extracted `vendor/` directory.
    /// * `entry_module` — Dotted module path (e.g., `"workflow.tasks"`).
    /// * `task_id`      — ID of the task to execute.
    /// * `context_json` — JSON-encoded context dict passed to the function.
    async fn execute_task(
        &self,
        workflow_dir: &Path,
        vendor_dir: &Path,
        entry_module: &str,
        task_id: &str,
        context_json: &str,
    ) -> Result<PythonTaskResult, PythonExecutionError>;

    /// Discover task IDs available in the entry module.
    ///
    /// This imports the module to trigger `@task` decorator registration,
    /// then queries the task registry.
    async fn discover_tasks(
        &self,
        workflow_dir: &Path,
        vendor_dir: &Path,
        entry_module: &str,
    ) -> Result<Vec<String>, PythonExecutionError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A mock executor for testing without PyO3.
    struct MockPythonExecutor {
        task_ids: Vec<String>,
    }

    #[async_trait::async_trait]
    impl PythonTaskExecutor for MockPythonExecutor {
        async fn execute_task(
            &self,
            _workflow_dir: &Path,
            _vendor_dir: &Path,
            _entry_module: &str,
            task_id: &str,
            context_json: &str,
        ) -> Result<PythonTaskResult, PythonExecutionError> {
            if !self.task_ids.contains(&task_id.to_string()) {
                return Err(PythonExecutionError::TaskNotFound {
                    task_id: task_id.to_string(),
                    module: "mock".to_string(),
                });
            }
            Ok(PythonTaskResult {
                task_id: task_id.to_string(),
                output_json: context_json.to_string(),
            })
        }

        async fn discover_tasks(
            &self,
            _workflow_dir: &Path,
            _vendor_dir: &Path,
            _entry_module: &str,
        ) -> Result<Vec<String>, PythonExecutionError> {
            Ok(self.task_ids.clone())
        }
    }

    #[tokio::test]
    async fn test_mock_executor_discover() {
        let exec = MockPythonExecutor {
            task_ids: vec!["extract".to_string(), "transform".to_string()],
        };
        let ids = exec
            .discover_tasks(Path::new("/tmp"), Path::new("/tmp"), "workflow.tasks")
            .await
            .unwrap();
        assert_eq!(ids, vec!["extract", "transform"]);
    }

    #[tokio::test]
    async fn test_mock_executor_execute() {
        let exec = MockPythonExecutor {
            task_ids: vec!["extract".to_string()],
        };
        let result = exec
            .execute_task(
                Path::new("/tmp"),
                Path::new("/tmp"),
                "workflow.tasks",
                "extract",
                r#"{"key": "value"}"#,
            )
            .await
            .unwrap();
        assert_eq!(result.task_id, "extract");
        assert_eq!(result.output_json, r#"{"key": "value"}"#);
    }

    #[tokio::test]
    async fn test_mock_executor_task_not_found() {
        let exec = MockPythonExecutor { task_ids: vec![] };
        let err = exec
            .execute_task(
                Path::new("/tmp"),
                Path::new("/tmp"),
                "workflow.tasks",
                "missing",
                "{}",
            )
            .await
            .unwrap_err();
        assert!(matches!(err, PythonExecutionError::TaskNotFound { .. }));
    }

    #[test]
    fn test_error_display() {
        let err = PythonExecutionError::TaskException {
            task_id: "my_task".to_string(),
            message: "ZeroDivisionError: division by zero".to_string(),
            traceback: "  File \"tasks.py\", line 10".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("my_task"));
        assert!(msg.contains("ZeroDivisionError"));
    }
}
