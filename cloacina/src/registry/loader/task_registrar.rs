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

//! Task registrar for integrating packaged workflow tasks with the global registry.
//!
//! This module provides functionality to register tasks from dynamically loaded
//! library packages with cloacina's global task registry, ensuring proper namespace
//! isolation and task lifecycle management.

use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::ffi::CStr;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tempfile::TempDir;
use tokio::fs;

use crate::context::Context;
use crate::error::TaskError;
use crate::registry::error::LoaderError;
use crate::registry::loader::package_loader::{get_library_extension, PackageMetadata};
use crate::task::{register_task_constructor, Task, TaskNamespace};
use chrono::Utc;

/// C-compatible task metadata structure for FFI (from packaged_workflow macro)
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TaskMetadata {
    /// Local task ID (e.g., "collect_data")
    pub local_id: *const std::os::raw::c_char,
    /// Template for namespaced ID (e.g., "{tenant}::simple_demo::data_processing::collect_data")
    pub namespaced_id_template: *const std::os::raw::c_char,
    /// JSON string of task dependencies
    pub dependencies_json: *const std::os::raw::c_char,
    /// Name of the task constructor function in the library
    pub constructor_fn_name: *const std::os::raw::c_char,
    /// Task description
    pub description: *const std::os::raw::c_char,
}

/// C-compatible collection of task metadata for FFI (from packaged_workflow macro)
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TaskMetadataCollection {
    /// Number of tasks in this package
    pub task_count: u32,
    /// Array of task metadata
    pub tasks: *const TaskMetadata,
    /// Name of the workflow (e.g., "data_processing")
    pub workflow_name: *const std::os::raw::c_char,
    /// Name of the package (e.g., "simple_demo")
    pub package_name: *const std::os::raw::c_char,
}

/// Task registrar for managing dynamically loaded package tasks.
///
/// This registrar integrates packaged workflow tasks with cloacina's global
/// task registry while maintaining proper namespace isolation and lifecycle
/// management for dynamic libraries.
pub struct TaskRegistrar {
    /// Temporary directory for library file operations
    temp_dir: TempDir,
    /// Map of package IDs to registered task namespaces for cleanup tracking
    registered_tasks: Arc<RwLock<HashMap<String, Vec<TaskNamespace>>>>,
    /// Map of package IDs to loaded libraries (kept alive)
    loaded_libraries: Arc<RwLock<HashMap<String, Library>>>,
}

impl TaskRegistrar {
    /// Create a new task registrar with a temporary directory for operations.
    pub fn new() -> Result<Self, LoaderError> {
        let temp_dir = TempDir::new().map_err(|e| LoaderError::TempDirectory {
            error: e.to_string(),
        })?;

        Ok(Self {
            temp_dir,
            registered_tasks: Arc::new(RwLock::new(HashMap::new())),
            loaded_libraries: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Register package tasks with the global task registry using new host-managed approach.
    ///
    /// # Arguments
    ///
    /// * `package_id` - Unique identifier for the package (for cleanup tracking)
    /// * `package_data` - Binary data of the library package
    /// * `metadata` - Package metadata containing task information (legacy, for compatibility)
    /// * `tenant_id` - Tenant ID for namespace isolation (default: "public")
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<TaskNamespace>)` - List of registered task namespaces
    /// * `Err(LoaderError)` - If registration fails
    pub async fn register_package_tasks(
        &self,
        package_id: &str,
        package_data: &[u8],
        _metadata: &PackageMetadata,
        tenant_id: Option<&str>,
    ) -> Result<Vec<TaskNamespace>, LoaderError> {
        let tenant_id = tenant_id.unwrap_or("public");

        // Extract task metadata from library using new FFI approach
        let task_metadata = self
            .extract_task_metadata_from_library(package_data)
            .await?;

        // Register tasks in HOST global registry using metadata
        let mut registered_namespaces = Vec::new();

        let workflow_name = unsafe { CStr::from_ptr(task_metadata.workflow_name) }
            .to_str()
            .map_err(|e| LoaderError::MetadataExtraction {
                reason: format!("Invalid workflow name: {}", e),
            })?;

        let package_name = unsafe { CStr::from_ptr(task_metadata.package_name) }
            .to_str()
            .map_err(|e| LoaderError::MetadataExtraction {
                reason: format!("Invalid package name: {}", e),
            })?;

        let tasks_slice = unsafe {
            std::slice::from_raw_parts(task_metadata.tasks, task_metadata.task_count as usize)
        };

        for task in tasks_slice {
            let task_id = unsafe { CStr::from_ptr(task.local_id) }
                .to_str()
                .map_err(|e| LoaderError::MetadataExtraction {
                    reason: format!("Invalid task ID: {}", e),
                })?;

            let _constructor_fn_name = unsafe { CStr::from_ptr(task.constructor_fn_name) }
                .to_str()
                .map_err(|e| LoaderError::MetadataExtraction {
                    reason: format!("Invalid constructor function name: {}", e),
                })?;

            let dependencies_json = unsafe { CStr::from_ptr(task.dependencies_json) }
                .to_str()
                .map_err(|e| LoaderError::MetadataExtraction {
                    reason: format!("Invalid dependencies JSON: {}", e),
                })?;

            // Parse dependencies JSON to get dependency namespaces
            let dependency_namespaces: Vec<TaskNamespace> = if dependencies_json.trim() == "[]" {
                Vec::new()
            } else {
                let dep_names: Vec<String> =
                    serde_json::from_str(dependencies_json).map_err(|e| {
                        LoaderError::MetadataExtraction {
                            reason: format!(
                                "Failed to parse dependencies JSON '{}': {}",
                                dependencies_json, e
                            ),
                        }
                    })?;

                dep_names
                    .into_iter()
                    .map(|dep_name| {
                        // Dependencies are stored as fully qualified namespaces with {tenant} placeholder
                        // Replace {tenant} with actual tenant_id
                        let full_name = dep_name.replace("{tenant}", tenant_id);
                        // Parse the namespace
                        crate::parse_namespace(&full_name).map_err(|e| {
                            LoaderError::MetadataExtraction {
                                reason: format!(
                                    "Invalid dependency namespace '{}': {}",
                                    full_name, e
                                ),
                            }
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?
            };

            // Create namespace for this task
            let namespace = TaskNamespace::new(tenant_id, package_name, workflow_name, task_id);

            // Create task constructor that creates a dynamic task
            // The constructor name is just metadata - actual execution happens via execute FFI
            let library_data = package_data.to_vec();
            let task_name = task_id.to_string();
            let pkg_name = package_name.to_string();
            let deps = dependency_namespaces.clone();

            let constructor = Box::new(move || {
                Arc::new(DynamicLibraryTask::new(
                    library_data.clone(),
                    task_name.clone(),
                    pkg_name.clone(),
                    deps.clone(),
                )) as Arc<dyn Task>
            });

            // Register in HOST global task registry
            register_task_constructor(namespace.clone(), constructor);

            registered_namespaces.push(namespace);
        }

        // Track registered tasks for cleanup
        {
            let mut registered =
                self.registered_tasks
                    .write()
                    .map_err(|e| LoaderError::TaskRegistration {
                        reason: format!("Failed to acquire registration lock: {}", e),
                    })?;
            registered.insert(package_id.to_string(), registered_namespaces.clone());
        }

        tracing::info!(
            "Successfully registered {} tasks for package {} using host-managed approach",
            registered_namespaces.len(),
            package_name
        );

        Ok(registered_namespaces)
    }

    /// Extract task metadata from library using get_task_metadata() FFI function.
    async fn extract_task_metadata_from_library(
        &self,
        package_data: &[u8],
    ) -> Result<TaskMetadataCollection, LoaderError> {
        // Write package to temporary file with correct extension
        let library_extension = get_library_extension();
        let temp_path = self
            .temp_dir
            .path()
            .join(format!("metadata_extract.{}", library_extension));
        fs::write(&temp_path, package_data)
            .await
            .map_err(|e| LoaderError::FileSystem {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?;

        // Load the library
        let lib = unsafe {
            Library::new(&temp_path).map_err(|e| LoaderError::LibraryLoad {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?
        };

        // Get the get_task_metadata function
        let get_metadata = unsafe {
            lib.get::<unsafe extern "C" fn() -> *const TaskMetadataCollection>(b"get_task_metadata")
                .map_err(|e| LoaderError::SymbolNotFound {
                    symbol: "get_task_metadata".to_string(),
                    error: e.to_string(),
                })?
        };

        // Call the FFI function to get metadata
        let metadata_ptr = unsafe { get_metadata() };
        if metadata_ptr.is_null() {
            return Err(LoaderError::MetadataExtraction {
                reason: "get_task_metadata() returned null pointer".to_string(),
            });
        }

        // Copy the metadata structure (since we need to return owned data)
        let metadata = unsafe { &*metadata_ptr };

        // Create owned copies of the string data for safety
        let workflow_name = unsafe { CStr::from_ptr(metadata.workflow_name) }
            .to_str()
            .map_err(|e| LoaderError::MetadataExtraction {
                reason: format!("Invalid workflow name: {}", e),
            })?
            .to_string();

        let package_name = unsafe { CStr::from_ptr(metadata.package_name) }
            .to_str()
            .map_err(|e| LoaderError::MetadataExtraction {
                reason: format!("Invalid package name: {}", e),
            })?
            .to_string();

        tracing::debug!(
            "Extracted metadata: package={}, workflow={}, task_count={}",
            package_name,
            workflow_name,
            metadata.task_count
        );

        // Return the metadata collection (the caller will handle the task array)
        Ok(unsafe { (*metadata_ptr).clone() })
    }

    /// Unregister package tasks from the global registry.
    ///
    /// # Arguments
    ///
    /// * `package_id` - Package identifier used during registration
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Tasks successfully unregistered
    /// * `Err(LoaderError)` - If unregistration fails
    pub fn unregister_package_tasks(&self, package_id: &str) -> Result<(), LoaderError> {
        // Remove from tracked registrations
        let namespaces = {
            let mut registered =
                self.registered_tasks
                    .write()
                    .map_err(|e| LoaderError::TaskRegistration {
                        reason: format!("Failed to acquire registration lock: {}", e),
                    })?;
            registered.remove(package_id)
        };

        if let Some(namespaces) = namespaces {
            // Unregister tasks from global registry
            // Note: The global registry doesn't currently support removal,
            // so we'll track this for future implementation
            tracing::warn!(
                "Task unregistration requested for package '{}' with {} tasks, but global registry doesn't support removal yet",
                package_id,
                namespaces.len()
            );
        }

        // Remove library reference
        {
            let mut libraries =
                self.loaded_libraries
                    .write()
                    .map_err(|e| LoaderError::TaskRegistration {
                        reason: format!("Failed to acquire library lock: {}", e),
                    })?;
            libraries.remove(package_id);
        }

        Ok(())
    }

    /// Get the list of task namespaces registered for a package.
    pub fn get_registered_namespaces(&self, package_id: &str) -> Vec<TaskNamespace> {
        let registered = self
            .registered_tasks
            .read()
            .unwrap_or_else(|e| e.into_inner());
        registered.get(package_id).cloned().unwrap_or_default()
    }

    /// Get the number of currently loaded packages.
    pub fn loaded_package_count(&self) -> usize {
        let libraries = self
            .loaded_libraries
            .read()
            .unwrap_or_else(|e| e.into_inner());
        libraries.len()
    }

    /// Get the total number of registered tasks across all packages.
    pub fn total_registered_tasks(&self) -> usize {
        let registered = self
            .registered_tasks
            .read()
            .unwrap_or_else(|e| e.into_inner());
        registered.values().map(|tasks| tasks.len()).sum()
    }

    /// Get the temporary directory path for manual operations.
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }
}

impl Default for TaskRegistrar {
    fn default() -> Self {
        Self::new().expect("Failed to create default TaskRegistrar")
    }
}

/// A task implementation that executes via dynamic library FFI calls.
///
/// This task type represents a task loaded from a packaged workflow .so file,
/// using the host-managed registry approach but executing tasks via the
/// cloacina_execute_task FFI function.
#[derive(Debug)]
struct DynamicLibraryTask {
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
    fn new(
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
            message: format!("Failed to create temp directory: {}", e),
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
                message: format!("Failed to load library {}: {}", temp_path.display(), e),
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
                    message: format!("Symbol 'cloacina_execute_task' not found in library: {}", e),
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
        let mut result_buffer = vec![0u8; 64 * 1024]; // 64KB buffer
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
                result_buffer.truncate(result_len as usize);
                String::from_utf8_lossy(&result_buffer).to_string()
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
    use crate::registry::loader::package_loader::{PackageMetadata, TaskMetadata};

    /// Helper to create mock package metadata for testing
    fn create_mock_package_metadata(package_name: &str, task_count: usize) -> PackageMetadata {
        let tasks: Vec<TaskMetadata> = (0..task_count)
            .map(|i| TaskMetadata {
                index: i as u32,
                local_id: format!("task_{}", i),
                namespaced_id_template: format!(
                    "{{tenant_id}}/{{package_name}}/{}",
                    format!("task_{}", i)
                ),
                dependencies: Vec::new(),
                description: format!("Test task {}", i),
                source_location: "test.rs:1".to_string(),
            })
            .collect();

        PackageMetadata {
            package_name: package_name.to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test package".to_string()),
            author: Some("Test Author".to_string()),
            tasks,
            graph_data: None,
            architecture: "x86_64".to_string(),
            symbols: vec![
                "cloacina_execute_task".to_string(),
                "cloacina_get_task_metadata".to_string(),
            ],
        }
    }

    /// Helper to create mock binary data (not a real .so file)
    fn create_mock_binary_data() -> Vec<u8> {
        // This is just mock data - in real tests we'd need actual compiled .so files
        vec![0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00] // ELF header start
    }

    #[tokio::test]
    async fn test_task_registrar_creation() {
        let registrar = TaskRegistrar::new().expect("Failed to create TaskRegistrar");

        // Verify initial state
        assert_eq!(registrar.loaded_package_count(), 0);
        assert_eq!(registrar.total_registered_tasks(), 0);
        assert!(registrar.temp_dir().exists());
    }

    #[tokio::test]
    async fn test_task_registrar_default() {
        let registrar = TaskRegistrar::default();
        assert_eq!(registrar.loaded_package_count(), 0);
        assert!(registrar.temp_dir().exists());
    }

    #[tokio::test]
    async fn test_register_package_tasks_with_invalid_binary() {
        let registrar = TaskRegistrar::new().unwrap();
        let metadata = create_mock_package_metadata("test_package", 2);
        let invalid_data = b"not a valid library".to_vec();

        let result = registrar
            .register_package_tasks("test_id", &invalid_data, &metadata, Some("test_tenant"))
            .await;

        // Should fail because the binary is invalid
        assert!(result.is_err());
        match result.unwrap_err() {
            LoaderError::LibraryLoad { .. } => {
                // Expected error type
            }
            other => panic!("Expected LibraryLoad error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_register_package_tasks_with_missing_symbols() {
        let registrar = TaskRegistrar::new().unwrap();
        let metadata = create_mock_package_metadata("test_package", 1);
        let mock_data = create_mock_binary_data();

        let result = registrar
            .register_package_tasks("test_id", &mock_data, &metadata, Some("test_tenant"))
            .await;

        // Should fail because mock data doesn't have required symbols
        assert!(result.is_err());
        match result.unwrap_err() {
            LoaderError::LibraryLoad { .. } | LoaderError::SymbolNotFound { .. } => {
                // Expected error types
            }
            other => panic!(
                "Expected LibraryLoad or SymbolNotFound error, got: {:?}",
                other
            ),
        }
    }

    #[tokio::test]
    async fn test_register_package_tasks_empty_metadata() {
        let registrar = TaskRegistrar::new().unwrap();
        let metadata = create_mock_package_metadata("empty_package", 0);
        let mock_data = create_mock_binary_data();

        let result = registrar
            .register_package_tasks("empty_id", &mock_data, &metadata, Some("test_tenant"))
            .await;

        // Should still fail due to invalid binary, but test that empty task list is handled
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unregister_nonexistent_package() {
        let registrar = TaskRegistrar::new().unwrap();

        let result = registrar.unregister_package_tasks("nonexistent_package");

        // Should succeed (idempotent operation)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_registered_namespaces_empty() {
        let registrar = TaskRegistrar::new().unwrap();

        let namespaces = registrar.get_registered_namespaces("nonexistent_package");

        assert!(namespaces.is_empty());
    }

    #[tokio::test]
    async fn test_registrar_metrics() {
        let registrar = TaskRegistrar::new().unwrap();

        // Initial state
        assert_eq!(registrar.loaded_package_count(), 0);
        assert_eq!(registrar.total_registered_tasks(), 0);

        // These counts won't change with failed registrations
        let metadata = create_mock_package_metadata("test", 3);
        let invalid_data = b"invalid".to_vec();
        let _ = registrar
            .register_package_tasks("test", &invalid_data, &metadata, None)
            .await;

        assert_eq!(registrar.loaded_package_count(), 0);
        assert_eq!(registrar.total_registered_tasks(), 0);
    }

    #[tokio::test]
    async fn test_concurrent_registrar_operations() {
        use std::sync::Arc;
        use tokio::task;

        let registrar = Arc::new(TaskRegistrar::new().unwrap());
        let mut handles = Vec::new();

        // Start multiple concurrent operations
        for i in 0..5 {
            let registrar_clone = Arc::clone(&registrar);
            let handle = task::spawn(async move {
                let metadata = create_mock_package_metadata(&format!("package_{}", i), 2);
                let mock_data = create_mock_binary_data();

                // All will fail but shouldn't cause race conditions
                let _ = registrar_clone
                    .register_package_tasks(
                        &format!("id_{}", i),
                        &mock_data,
                        &metadata,
                        Some("tenant"),
                    )
                    .await;

                // Test unregistration too
                let _ = registrar_clone.unregister_package_tasks(&format!("id_{}", i));

                i
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            let task_id = handle.await.expect("Task should complete");
            assert!(task_id < 5);
        }

        // Registrar should still be in consistent state
        assert_eq!(registrar.loaded_package_count(), 0);
    }

    #[tokio::test]
    async fn test_temp_directory_isolation() {
        let registrar1 = TaskRegistrar::new().unwrap();
        let registrar2 = TaskRegistrar::new().unwrap();

        // Each registrar should have its own temp directory
        assert_ne!(registrar1.temp_dir(), registrar2.temp_dir());
        assert!(registrar1.temp_dir().exists());
        assert!(registrar2.temp_dir().exists());
    }

    #[tokio::test]
    async fn test_package_id_tracking() {
        let registrar = TaskRegistrar::new().unwrap();

        // Test multiple unregistrations of the same package
        for _ in 0..3 {
            let result = registrar.unregister_package_tasks("same_package_id");
            assert!(result.is_ok());
        }

        // Should remain consistent
        assert_eq!(registrar.loaded_package_count(), 0);
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let registrar = TaskRegistrar::new().unwrap();
        let metadata = create_mock_package_metadata("shared_package", 1);
        let mock_data = create_mock_binary_data();

        // Try registering the same package for different tenants
        let result1 = registrar
            .register_package_tasks("pkg1", &mock_data, &metadata, Some("tenant_a"))
            .await;
        let result2 = registrar
            .register_package_tasks("pkg2", &mock_data, &metadata, Some("tenant_b"))
            .await;

        // Both should fail due to invalid binary, but test that tenant isolation is attempted
        assert!(result1.is_err());
        assert!(result2.is_err());
    }

    #[tokio::test]
    async fn test_default_tenant() {
        let registrar = TaskRegistrar::new().unwrap();
        let metadata = create_mock_package_metadata("test_package", 1);
        let mock_data = create_mock_binary_data();

        // Test with None tenant (should default to "public")
        let result = registrar
            .register_package_tasks("test", &mock_data, &metadata, None)
            .await;

        assert!(result.is_err()); // Will fail due to invalid binary
    }

    #[tokio::test]
    async fn test_large_package_metadata() {
        let registrar = TaskRegistrar::new().unwrap();

        // Test with a package that has many tasks
        let metadata = create_mock_package_metadata("large_package", 100);
        let mock_data = create_mock_binary_data();

        let result = registrar
            .register_package_tasks("large", &mock_data, &metadata, Some("test"))
            .await;

        // Should handle large metadata gracefully (though will fail due to invalid binary)
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_error_message_quality() {
        let registrar = TaskRegistrar::new().unwrap();
        let metadata = create_mock_package_metadata("test", 1);
        let invalid_data = b"definitely not a library".to_vec();

        let result = registrar
            .register_package_tasks("test", &invalid_data, &metadata, Some("test"))
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_string = format!("{}", error);

        // Error message should be informative
        assert!(!error_string.is_empty());
        assert!(error_string.contains("Failed to load library") || error_string.contains("Symbol"));
    }

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

    #[test]
    fn test_registrar_sync_creation() {
        // Test that we can create a registrar in non-async context
        let result = TaskRegistrar::new();
        assert!(result.is_ok());

        let registrar = result.unwrap();
        assert!(registrar.temp_dir().exists());
    }
}
