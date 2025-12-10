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

//! Core types and structures for the Cloacina task execution system.
//!
//! This module defines the fundamental types used throughout the executor system,
//! including execution scopes, dependency management, and configuration structures.
//! These types are used to coordinate task execution, manage dependencies between tasks,
//! and configure the behavior of the execution engine.

use crate::dal::DAL;
use crate::database::UniversalUuid;
use crate::error::ExecutorError;
use crate::Database;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Execution scope information for a context
///
/// This structure holds metadata about the current execution context, including
/// identifiers for both pipeline and task executions. It is used to track and
/// correlate execution contexts throughout the system.
#[derive(Debug, Clone)]
pub struct ExecutionScope {
    /// Unique identifier for the pipeline execution
    pub pipeline_execution_id: UniversalUuid,
    /// Optional unique identifier for the specific task execution
    pub task_execution_id: Option<UniversalUuid>,
    /// Optional name of the task being executed
    pub task_name: Option<String>,
}

/// Dependency loader for automatic context merging with lazy loading
///
/// This structure manages the loading and caching of task dependencies,
/// implementing a "latest wins" strategy for context merging. It provides
/// thread-safe access to dependency contexts through a read-write lock.
#[derive(Debug)]
pub struct DependencyLoader {
    /// Database connection for loading dependency data
    database: Database,
    /// ID of the pipeline execution being processed
    pipeline_execution_id: UniversalUuid,
    /// List of task namespaces that this loader depends on
    dependency_tasks: Vec<crate::task::TaskNamespace>,
    /// Thread-safe cache of loaded dependency contexts
    loaded_contexts: RwLock<HashMap<String, HashMap<String, serde_json::Value>>>, // Cache
}

impl DependencyLoader {
    /// Creates a new dependency loader instance
    ///
    /// # Arguments
    /// * `database` - Database connection for loading dependencies
    /// * `pipeline_execution_id` - ID of the pipeline execution
    /// * `dependency_tasks` - List of task namespaces that this loader depends on
    pub fn new(
        database: Database,
        pipeline_execution_id: UniversalUuid,
        dependency_tasks: Vec<crate::task::TaskNamespace>,
    ) -> Self {
        Self {
            database,
            pipeline_execution_id,
            dependency_tasks,
            loaded_contexts: RwLock::new(HashMap::new()),
        }
    }

    /// Loads a value from dependency contexts using a "latest wins" strategy
    ///
    /// This method searches through all dependency contexts in reverse order,
    /// returning the first matching value found. If no value is found, returns None.
    ///
    /// # Arguments
    /// * `key` - The key to look up in the dependency contexts
    ///
    /// # Returns
    /// * `Result<Option<serde_json::Value>, ExecutorError>` - The found value or None if not found
    pub async fn load_from_dependencies(
        &self,
        key: &str,
    ) -> Result<Option<serde_json::Value>, ExecutorError> {
        // Search dependencies in reverse order (latest wins for overwrites)
        for dep_task_namespace in self.dependency_tasks.iter().rev() {
            let dep_task_name = dep_task_namespace.to_string();
            // Check cache first (read lock)
            {
                let cache = self.loaded_contexts.read().await;
                if let Some(context_data) = cache.get(&dep_task_name) {
                    if let Some(value) = context_data.get(key) {
                        return Ok(Some(value.clone())); // Found! (overwrite strategy)
                    }
                }
            }

            // Lazy load dependency context if not cached (write lock)
            {
                let mut cache = self.loaded_contexts.write().await;
                if !cache.contains_key(&dep_task_name) {
                    let dep_context_data = self
                        .load_dependency_context_data(dep_task_namespace)
                        .await?;
                    cache.insert(dep_task_name.clone(), dep_context_data);
                }

                // Check the newly loaded context
                if let Some(context_data) = cache.get(&dep_task_name) {
                    if let Some(value) = context_data.get(key) {
                        return Ok(Some(value.clone())); // Found! (overwrite strategy)
                    }
                }
            }
        }

        Ok(None) // Key not found in any dependency
    }

    /// Loads the context data for a specific dependency task
    ///
    /// # Arguments
    /// * `task_namespace` - Namespace of the task to load context data for
    ///
    /// # Returns
    /// * `Result<HashMap<String, serde_json::Value>, ExecutorError>` - The loaded context data
    async fn load_dependency_context_data(
        &self,
        task_namespace: &crate::task::TaskNamespace,
    ) -> Result<HashMap<String, serde_json::Value>, ExecutorError> {
        let dal = DAL::new(self.database.clone());
        let task_metadata = dal
            .task_execution_metadata()
            .get_by_pipeline_and_task(self.pipeline_execution_id, task_namespace)
            .await?;

        if let Some(context_id) = task_metadata.context_id {
            let context = dal.context().read::<serde_json::Value>(context_id).await?;
            Ok(context.data().clone())
        } else {
            // Task has no output context
            Ok(HashMap::new())
        }
    }
}

/// Configuration settings for the executor
///
/// This structure defines various parameters that control the behavior of the
/// task execution system, including concurrency limits and timing parameters.
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Maximum number of tasks that can run concurrently
    pub max_concurrent_tasks: usize,
    /// Maximum time a task is allowed to run before timing out
    pub task_timeout: std::time::Duration,
}

impl Default for ExecutorConfig {
    /// Creates a new executor configuration with default values
    ///
    /// Default values:
    /// * max_concurrent_tasks: 4
    /// * task_timeout: 5 minutes
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            task_timeout: std::time::Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Represents a task that has been claimed for execution
///
/// This structure contains the metadata for a task that has been claimed
/// by an executor instance and is ready to be processed.
#[derive(Debug)]
pub struct ClaimedTask {
    /// Unique identifier for this task execution
    pub task_execution_id: UniversalUuid,
    /// ID of the pipeline this task belongs to
    pub pipeline_execution_id: UniversalUuid,
    /// Name of the task being executed
    pub task_name: String,
    /// Current attempt number for this task execution
    pub attempt: i32,
}
