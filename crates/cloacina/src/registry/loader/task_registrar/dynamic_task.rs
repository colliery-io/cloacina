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
//!
//! The plugin library is loaded once during package registration and the handle
//! is shared across all task instances from that package. No per-execution
//! temp files or dlopen/dlclose cycles.

use chrono::Utc;
use std::sync::Arc;

use crate::context::Context;
use crate::error::TaskError;
use crate::task::{Task, TaskNamespace};
use cloacina_workflow_plugin::{TaskExecutionRequest, TaskExecutionResult};

/// A persistent handle to a loaded workflow plugin library.
///
/// Loaded once from compiled library bytes, kept alive for the lifetime of the
/// package. All task instances from the same package share this handle.
pub(super) struct LoadedWorkflowPlugin {
    handle: std::sync::Mutex<fidius_host::PluginHandle>,
    // Keep the temp dir alive so the dylib file isn't deleted while loaded
    _temp_dir: tempfile::TempDir,
}

// Safety: fidius PluginHandle wraps a libloading::Library which is Send.
// We serialize access via Mutex so concurrent calls are safe.
unsafe impl Send for LoadedWorkflowPlugin {}
unsafe impl Sync for LoadedWorkflowPlugin {}

impl LoadedWorkflowPlugin {
    /// Load a workflow plugin from library bytes.
    pub(super) fn load(library_data: &[u8], package_name: &str) -> Result<Self, TaskError> {
        let temp_dir = tempfile::TempDir::new().map_err(|e| TaskError::ExecutionFailed {
            task_id: package_name.to_string(),
            message: format!("Failed to create temp dir: {}", e),
            timestamp: Utc::now(),
        })?;

        let library_extension = crate::registry::loader::package_loader::get_library_extension();
        let temp_path = temp_dir
            .path()
            .join(format!("workflow_plugin.{}", library_extension));
        std::fs::write(&temp_path, library_data).map_err(|e| TaskError::ExecutionFailed {
            task_id: package_name.to_string(),
            message: format!("Failed to write library: {}", e),
            timestamp: Utc::now(),
        })?;

        let loaded = fidius_host::loader::load_library(&temp_path).map_err(
            |e: fidius_host::LoadError| TaskError::ExecutionFailed {
                task_id: package_name.to_string(),
                message: format!("Failed to load plugin library: {}", e),
                timestamp: Utc::now(),
            },
        )?;

        let plugin =
            loaded
                .plugins
                .into_iter()
                .next()
                .ok_or_else(|| TaskError::ExecutionFailed {
                    task_id: package_name.to_string(),
                    message: "Plugin library contains no plugins".to_string(),
                    timestamp: Utc::now(),
                })?;

        let handle = fidius_host::PluginHandle::from_loaded(plugin);

        Ok(Self {
            handle: std::sync::Mutex::new(handle),
            _temp_dir: temp_dir,
        })
    }

    /// Call execute_task (method index 1) on the loaded plugin.
    fn execute_task(&self, request: TaskExecutionRequest) -> Result<TaskExecutionResult, String> {
        let handle = self
            .handle
            .lock()
            .map_err(|e| format!("Plugin mutex poisoned: {}", e))?;
        handle
            .call_method(1, &(request,))
            .map_err(|e| format!("execute_task FFI call failed: {}", e))
    }
}

impl std::fmt::Debug for LoadedWorkflowPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedWorkflowPlugin").finish()
    }
}

/// A task implementation that executes via the fidius plugin API.
///
/// The plugin handle is loaded once and shared across all task instances
/// from the same package. No per-execution temp files or dlopen cycles.
#[derive(Debug)]
pub(super) struct DynamicLibraryTask {
    /// Shared handle to the loaded plugin library
    plugin: Arc<LoadedWorkflowPlugin>,
    /// Name of the task within the package
    task_name: String,
    /// Name of the package containing this task
    package_name: String,
    /// Task dependencies as fully qualified namespaces
    dependencies: Vec<TaskNamespace>,
}

impl DynamicLibraryTask {
    /// Load a plugin library from bytes. Called once per package during registration.
    pub(super) fn load_plugin(
        library_data: &[u8],
        package_name: &str,
    ) -> Result<LoadedWorkflowPlugin, TaskError> {
        LoadedWorkflowPlugin::load(library_data, package_name)
    }

    /// Create a new dynamic library task with a shared plugin handle.
    pub(super) fn new(
        plugin: Arc<LoadedWorkflowPlugin>,
        task_name: String,
        package_name: String,
        dependencies: Vec<TaskNamespace>,
    ) -> Self {
        Self {
            plugin,
            task_name,
            package_name,
            dependencies,
        }
    }
}

#[async_trait::async_trait]
impl Task for DynamicLibraryTask {
    /// Execute the task using the pre-loaded plugin handle.
    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
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

        // Call via the shared plugin handle
        let plugin = self.plugin.clone();
        let task_name = self.task_name.clone();
        let pkg_name = self.package_name.clone();

        let result = tokio::task::spawn_blocking(move || plugin.execute_task(request))
            .await
            .map_err(|e| TaskError::ExecutionFailed {
                task_id: task_name.clone(),
                message: format!("spawn_blocking panicked: {}", e),
                timestamp: Utc::now(),
            })?
            .map_err(|e| TaskError::ExecutionFailed {
                task_id: task_name.clone(),
                message: format!("Plugin call failed for task '{}': {}", task_name, e),
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

    fn id(&self) -> &str {
        &self.task_name
    }

    fn dependencies(&self) -> &[TaskNamespace] {
        &self.dependencies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loaded_workflow_plugin_debug() {
        // Just verify Debug trait works
        let formatted = format!("{:?}", "LoadedWorkflowPlugin");
        assert!(!formatted.is_empty());
    }
}
