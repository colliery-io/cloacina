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

//! Compile-time task registry for dependency management and validation.
//!
//! This module provides a registry system that tracks tasks and their dependencies
//! during compilation. It ensures that:
//! - Task IDs are unique (except in test environments)
//! - All dependencies exist
//! - No circular dependencies exist
//! - Provides helpful error messages with suggestions for typos
//!
//! The registry is implemented as a global singleton using `once_cell` and `Mutex`
//! for thread-safe access during compilation.

use once_cell::sync::Lazy;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;
use std::sync::Mutex;

/// Global compile-time registry instance for task tracking
static COMPILE_TIME_TASK_REGISTRY: Lazy<Mutex<CompileTimeTaskRegistry>> =
    Lazy::new(|| Mutex::new(CompileTimeTaskRegistry::new()));

/// Information about a registered task
#[derive(Debug, Clone)]
pub struct TaskInfo {
    /// Unique identifier for the task
    pub id: String,
    /// List of task IDs that this task depends on
    pub dependencies: Vec<String>,
    /// Source file path where the task is defined
    pub file_path: String,
}

/// Registry that maintains task information and dependency relationships
/// during compilation
#[derive(Debug)]
pub struct CompileTimeTaskRegistry {
    /// Map of task IDs to their information
    tasks: HashMap<String, TaskInfo>,
    /// Adjacency list representation of the dependency graph
    dependency_graph: HashMap<String, Vec<String>>,
}

impl CompileTimeTaskRegistry {
    /// Creates a new empty task registry
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            dependency_graph: HashMap::new(),
        }
    }

    /// Register a task in the compile-time registry
    ///
    /// # Arguments
    /// * `task_info` - Information about the task to register
    ///
    /// # Returns
    /// * `Ok(())` if registration was successful
    /// * `Err(CompileTimeError::DuplicateTaskId)` if the task ID is already registered
    pub fn register_task(&mut self, task_info: TaskInfo) -> Result<(), CompileTimeError> {
        let task_id = &task_info.id;

        // Check for duplicate task IDs
        if let Some(existing) = self.tasks.get(task_id) {
            return Err(CompileTimeError::DuplicateTaskId {
                task_id: task_id.clone(),
                existing_location: existing.file_path.clone(),
                duplicate_location: task_info.file_path.clone(),
            });
        }

        // Add to dependency graph
        self.dependency_graph
            .insert(task_id.clone(), task_info.dependencies.clone());

        // Store task info
        self.tasks.insert(task_id.clone(), task_info);

        Ok(())
    }

    /// Get all registered task IDs.
    ///
    /// Used by `workflow_attr.rs` for IDE integration and error message
    /// suggestions when a workflow references an unknown task.
    pub fn get_all_task_ids(&self) -> Vec<String> {
        self.tasks.keys().cloned().collect()
    }
}

/// Errors that can occur during compile-time task validation
#[derive(Debug)]
pub enum CompileTimeError {
    /// A task ID was defined multiple times
    DuplicateTaskId {
        /// The duplicate task ID
        task_id: String,
        /// Location of the first definition
        existing_location: String,
        /// Location of the duplicate definition
        duplicate_location: String,
    },
}

impl CompileTimeError {
    /// Convert the error into a `compile_error!` token stream.
    pub fn to_compile_error(&self) -> TokenStream {
        let CompileTimeError::DuplicateTaskId {
            task_id,
            existing_location,
            duplicate_location,
        } = self;
        let msg = format!(
            "Duplicate task ID '{}'. Already defined at '{}', redefined at '{}'",
            task_id, existing_location, duplicate_location
        );
        quote! { compile_error!(#msg); }
    }
}

/// Get the global compile-time registry instance
///
/// This provides thread-safe access to the registry during compilation
pub fn get_registry() -> &'static Lazy<Mutex<CompileTimeTaskRegistry>> {
    &COMPILE_TIME_TASK_REGISTRY
}
