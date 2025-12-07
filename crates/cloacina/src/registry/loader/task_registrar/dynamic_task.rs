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

//! Dynamic library task implementation for FFI-based task execution.

use chrono::Utc;
use libloading::{Library, Symbol};

use crate::context::Context;
use crate::error::TaskError;
use crate::registry::loader::package_loader::get_library_extension;
use crate::task::{Task, TaskNamespace};

/// A task implementation that executes via dynamic library FFI calls.
///
/// This task type represents a task loaded from a packaged workflow .so file,
/// using the host-managed registry approach but executing tasks via the
/// cloacina_execute_task FFI function.
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
    /// Execute the task using the cloacina_execute_task FFI function.
    ///
    /// This loads the library, calls the cloacina_execute_task function with the task name,
    /// and returns the result as a JSON value.
    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        // Write library to temporary file
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

        // Load the library
        let lib = unsafe {
            Library::new(&temp_path).map_err(|e| TaskError::ExecutionFailed {
                task_id: self.task_name.clone(),
                message: format!(
                    "Failed to load library for package '{}' at {}: {}",
                    self.package_name,
                    temp_path.display(),
                    e
                ),
                timestamp: Utc::now(),
            })?
        };

        // Get the execute function symbol
        let execute_task_symbol = b"cloacina_execute_task";
        let execute_task: Symbol<
            unsafe extern "C" fn(
                task_name: *const std::os::raw::c_char,
                task_name_len: u32,
                context_json: *const std::os::raw::c_char,
                context_len: u32,
                result_buffer: *mut u8,
                result_capacity: u32,
                result_len: *mut u32,
            ) -> i32,
        > = unsafe {
            lib.get(execute_task_symbol)
                .map_err(|e| TaskError::ExecutionFailed {
                    task_id: self.task_name.clone(),
                    message: format!(
                        "Symbol 'cloacina_execute_task' not found in library for package '{}': {}",
                        self.package_name, e
                    ),
                    timestamp: Utc::now(),
                })?
        };

        // Prepare input data
        let task_name_cstring = std::ffi::CString::new(self.task_name.clone()).map_err(|e| {
            TaskError::ExecutionFailed {
                task_id: self.task_name.clone(),
                message: format!("Invalid task name: {}", e),
                timestamp: Utc::now(),
            }
        })?;

        let context_json =
            serde_json::to_string(context.data()).map_err(|e| TaskError::ValidationFailed {
                message: format!(
                    "Failed to serialize context for task {}: {}",
                    self.task_name, e
                ),
            })?;

        // Debug: Log the context being passed to the task
        tracing::debug!("Task '{}' input context: {}", self.task_name, context_json);
        eprintln!(
            "DEBUG: Task '{}' input context: {}",
            self.task_name, context_json
        );

        let context_cstring =
            std::ffi::CString::new(context_json).map_err(|e| TaskError::ExecutionFailed {
                task_id: self.task_name.clone(),
                message: format!("Invalid context JSON: {}", e),
                timestamp: Utc::now(),
            })?;

        // Prepare output buffer
        let mut result_buffer = vec![0u8; 10 * 1024 * 1024]; // 10MB buffer (matches database limit)
        let mut result_len = 0u32;

        // Call the FFI function
        let return_code = unsafe {
            execute_task(
                task_name_cstring.as_ptr(),
                task_name_cstring.as_bytes().len() as u32,
                context_cstring.as_ptr(),
                context_cstring.as_bytes().len() as u32,
                result_buffer.as_mut_ptr(),
                result_buffer.len() as u32,
                &mut result_len,
            )
        };

        // Handle the result
        if return_code == 0 {
            // Success - parse the result JSON
            let mut result_context = context;
            if result_len > 0 {
                if result_len > result_buffer.len() as u32 {
                    return Err(TaskError::ExecutionFailed {
                        task_id: self.task_name.clone(),
                        message: format!(
                            "Task execution result too large: {} bytes exceeds maximum buffer size of {} bytes. \
                            This indicates the task context has grown beyond the database storage limit.",
                            result_len,
                            result_buffer.len()
                        ),
                        timestamp: Utc::now(),
                    });
                }
                result_buffer.truncate(result_len as usize);
                let result_str =
                    String::from_utf8(result_buffer).map_err(|e| TaskError::ExecutionFailed {
                        task_id: self.task_name.clone(),
                        message: format!("Invalid UTF-8 in result: {}", e),
                        timestamp: Utc::now(),
                    })?;

                // Debug: Log the result from the task
                tracing::debug!("Task '{}' output result: {}", self.task_name, result_str);
                eprintln!(
                    "DEBUG: Task '{}' output result: {}",
                    self.task_name, result_str
                );

                let result_value: serde_json::Value =
                    serde_json::from_str(&result_str).map_err(|e| TaskError::ValidationFailed {
                        message: format!(
                            "Invalid JSON in result for task {}: {}",
                            self.task_name, e
                        ),
                    })?;
                // Merge result into context (overwrite existing keys)
                if let serde_json::Value::Object(obj) = result_value {
                    for (key, value) in obj {
                        // Check if key exists and use appropriate method
                        if result_context.get(&key).is_some() {
                            // Key exists, update it
                            result_context.update(key, value).map_err(|e| {
                                TaskError::ExecutionFailed {
                                    task_id: self.task_name.clone(),
                                    message: format!("Failed to update result: {}", e),
                                    timestamp: Utc::now(),
                                }
                            })?;
                        } else {
                            // Key doesn't exist, insert it
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
            // Error - try to parse error message from buffer
            let error_msg = if result_len > 0 {
                if result_len > result_buffer.len() as u32 {
                    format!(
                        "Task execution failed (code: {}) with oversized error message: {} bytes exceeds buffer size of {} bytes",
                        return_code, result_len, result_buffer.len()
                    )
                } else {
                    result_buffer.truncate(result_len as usize);
                    String::from_utf8_lossy(&result_buffer).to_string()
                }
            } else {
                format!("Task execution failed with code {}", return_code)
            };
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
            Vec::new(), // No dependencies for test
        );

        assert_eq!(task.id(), "test_task");
        assert_eq!(task.dependencies().len(), 0); // No dependencies provided
    }
}
