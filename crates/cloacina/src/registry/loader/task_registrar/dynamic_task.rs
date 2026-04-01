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

//! Dynamic library task implementation using fidius-host for task execution.

use chrono::Utc;

use crate::context::Context;
use crate::error::TaskError;
use crate::registry::loader::package_loader::get_library_extension;
use crate::task::{Task, TaskNamespace};
use cloacina_workflow_plugin::{TaskExecutionRequest, TaskExecutionResult};

/// A task implementation that executes via the fidius plugin API.
///
/// This task type represents a task loaded from a packaged workflow cdylib file.
/// Each execution writes the library to a temp file, loads it via `fidius_host::load_library`,
/// and calls `execute_task` (method index 1) through a `PluginHandle`.
#[derive(Debug)]
pub(super) struct DynamicLibraryTask {
    /// Binary data of the library (.so/.dylib/.dll)
    library_data: Vec<u8>,
    /// Name of the task within the package
    task_name: String,
    /// Name of the package containing this task
    package_name: String,
    /// Task dependencies as fully qualified namespaces
    dependencies: Vec<TaskNamespace>,
}

impl DynamicLibraryTask {
    /// Create a new dynamic library task.
    pub(super) fn new(
        library_data: Vec<u8>,
        task_name: String,
        package_name: String,
        dependencies: Vec<TaskNamespace>,
    ) -> Self {
        Self {
            library_data,
            task_name,
            package_name,
            dependencies,
        }
    }
}

#[async_trait::async_trait]
impl Task for DynamicLibraryTask {
    /// Execute the task using the fidius-host plugin API.
    ///
    /// Writes the library to a temporary file, loads it via `fidius_host::load_library`,
    /// and calls `execute_task` (method index 1) with a `TaskExecutionRequest`.
    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        // Write library to a temporary file
        let library_extension = get_library_extension();
        let temp_dir = tempfile::TempDir::new().map_err(|e| TaskError::ExecutionFailed {
            task_id: self.task_name.clone(),
            message: format!(
                "Failed to create temp directory for package '{}': {}",
                self.package_name, e
            ),
            timestamp: Utc::now(),
        })?;

        let temp_path = temp_dir
            .path()
            .join(format!("task_exec.{}", library_extension));
        std::fs::write(&temp_path, &self.library_data).map_err(|e| TaskError::ExecutionFailed {
            task_id: self.task_name.clone(),
            message: format!("Failed to write library to temp file: {}", e),
            timestamp: Utc::now(),
        })?;

        // Load via fidius-host
        let loaded = fidius_host::loader::load_library(&temp_path).map_err(
            |e: fidius_host::LoadError| TaskError::ExecutionFailed {
                task_id: self.task_name.clone(),
                message: format!(
                    "Failed to load plugin library for package '{}': {}",
                    self.package_name, e
                ),
                timestamp: Utc::now(),
            },
        )?;

        let plugin =
            loaded
                .plugins
                .into_iter()
                .next()
                .ok_or_else(|| TaskError::ExecutionFailed {
                    task_id: self.task_name.clone(),
                    message: format!(
                        "Plugin library for package '{}' contains no plugins",
                        self.package_name
                    ),
                    timestamp: Utc::now(),
                })?;

        let handle = fidius_host::PluginHandle::from_loaded(plugin);

        // Serialize current context for the request
        let context_json =
            serde_json::to_string(context.data()).map_err(|e| TaskError::ValidationFailed {
                message: format!(
                    "Failed to serialize context for task '{}': {}",
                    self.task_name, e
                ),
            })?;

        tracing::debug!("Task '{}' input context: {}", self.task_name, context_json);

        let request = TaskExecutionRequest {
            task_name: self.task_name.clone(),
            context_json,
        };

        // Method index 1 = execute_task
        let result: TaskExecutionResult =
            handle
                .call_method(1, &(request,))
                .map_err(|e| TaskError::ExecutionFailed {
                    task_id: self.task_name.clone(),
                    message: format!("Plugin call failed for task '{}': {}", self.task_name, e),
                    timestamp: Utc::now(),
                })?;

        if result.success {
            let mut result_context = context;

            if let Some(result_json) = result.context_json {
                tracing::debug!("Task '{}' output result: {}", self.task_name, result_json);

                let result_value: serde_json::Value =
                    serde_json::from_str(&result_json).map_err(|e| {
                        TaskError::ValidationFailed {
                            message: format!(
                                "Invalid JSON in result for task '{}': {}",
                                self.task_name, e
                            ),
                        }
                    })?;

                // Merge result into context (overwrite existing keys)
                if let serde_json::Value::Object(obj) = result_value {
                    for (key, value) in obj {
                        if result_context.get(&key).is_some() {
                            result_context.update(key, value).map_err(|e| {
                                TaskError::ExecutionFailed {
                                    task_id: self.task_name.clone(),
                                    message: format!("Failed to update result: {}", e),
                                    timestamp: Utc::now(),
                                }
                            })?;
                        } else {
                            result_context.insert(key, value).map_err(|e| {
                                TaskError::ExecutionFailed {
                                    task_id: self.task_name.clone(),
                                    message: format!("Failed to insert result: {}", e),
                                    timestamp: Utc::now(),
                                }
                            })?;
                        }
                    }
                }
            }

            Ok(result_context)
        } else {
            let error_msg = result.error.unwrap_or_else(|| {
                format!("Task '{}' failed with no error message", self.task_name)
            });
            Err(TaskError::ExecutionFailed {
                task_id: self.task_name.clone(),
                message: error_msg,
                timestamp: Utc::now(),
            })
        }
    }

    /// Get the unique identifier for this task.
    fn id(&self) -> &str {
        &self.task_name
    }

    /// Get the list of task dependencies.
    fn dependencies(&self) -> &[TaskNamespace] {
        &self.dependencies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_library_task_creation() {
        let task = DynamicLibraryTask::new(
            vec![0x7f, 0x45, 0x4c, 0x46], // Mock library data
            "test_task".to_string(),
            "test_package".to_string(),
            Vec::new(),
        );

        assert_eq!(task.id(), "test_task");
        assert_eq!(task.dependencies().len(), 0);
    }
}
