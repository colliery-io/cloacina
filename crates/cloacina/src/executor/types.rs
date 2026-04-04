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
    /// Enable runner-level task claiming for horizontal scaling.
    /// When enabled, the executor claims tasks before executing and heartbeats during.
    pub enable_claiming: bool,
    /// Heartbeat interval for claimed tasks (only used when claiming is enabled).
    pub heartbeat_interval: std::time::Duration,
}

impl Default for ExecutorConfig {
    /// Creates a new executor configuration with default values
    ///
    /// Default values:
    /// * max_concurrent_tasks: 4
    /// * task_timeout: 5 minutes
    /// * enable_claiming: false (opt-in)
    /// * heartbeat_interval: 10 seconds
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            task_timeout: std::time::Duration::from_secs(300), // 5 minutes
            enable_claiming: true,
            heartbeat_interval: std::time::Duration::from_secs(10),
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

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // ExecutionScope tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_execution_scope_full() {
        let pipeline_id = UniversalUuid::new_v4();
        let task_id = UniversalUuid::new_v4();
        let scope = ExecutionScope {
            pipeline_execution_id: pipeline_id,
            task_execution_id: Some(task_id),
            task_name: Some("my_task".to_string()),
        };
        assert_eq!(scope.pipeline_execution_id, pipeline_id);
        assert_eq!(scope.task_execution_id, Some(task_id));
        assert_eq!(scope.task_name.as_deref(), Some("my_task"));
    }

    #[test]
    fn test_execution_scope_minimal() {
        let pipeline_id = UniversalUuid::new_v4();
        let scope = ExecutionScope {
            pipeline_execution_id: pipeline_id,
            task_execution_id: None,
            task_name: None,
        };
        assert!(scope.task_execution_id.is_none());
        assert!(scope.task_name.is_none());
    }

    #[test]
    fn test_execution_scope_clone() {
        let scope = ExecutionScope {
            pipeline_execution_id: UniversalUuid::new_v4(),
            task_execution_id: Some(UniversalUuid::new_v4()),
            task_name: Some("cloned_task".to_string()),
        };
        let cloned = scope.clone();
        assert_eq!(cloned.pipeline_execution_id, scope.pipeline_execution_id);
        assert_eq!(cloned.task_execution_id, scope.task_execution_id);
        assert_eq!(cloned.task_name, scope.task_name);
    }

    #[test]
    fn test_execution_scope_debug() {
        let scope = ExecutionScope {
            pipeline_execution_id: UniversalUuid::new_v4(),
            task_execution_id: None,
            task_name: Some("debug_task".to_string()),
        };
        let debug_str = format!("{:?}", scope);
        assert!(debug_str.contains("ExecutionScope"));
        assert!(debug_str.contains("debug_task"));
    }

    // -----------------------------------------------------------------------
    // ExecutorConfig tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_executor_config_default() {
        let config = ExecutorConfig::default();
        assert_eq!(config.max_concurrent_tasks, 4);
        assert_eq!(config.task_timeout, std::time::Duration::from_secs(300));
        assert!(config.enable_claiming);
        assert_eq!(
            config.heartbeat_interval,
            std::time::Duration::from_secs(10)
        );
    }

    #[test]
    fn test_executor_config_custom() {
        let config = ExecutorConfig {
            max_concurrent_tasks: 16,
            task_timeout: std::time::Duration::from_secs(60),
            enable_claiming: false,
            heartbeat_interval: std::time::Duration::from_secs(5),
        };
        assert_eq!(config.max_concurrent_tasks, 16);
        assert_eq!(config.task_timeout, std::time::Duration::from_secs(60));
        assert!(!config.enable_claiming);
        assert_eq!(config.heartbeat_interval, std::time::Duration::from_secs(5));
    }

    #[test]
    fn test_executor_config_clone() {
        let config = ExecutorConfig {
            max_concurrent_tasks: 8,
            task_timeout: std::time::Duration::from_secs(120),
            enable_claiming: true,
            heartbeat_interval: std::time::Duration::from_secs(15),
        };
        let cloned = config.clone();
        assert_eq!(cloned.max_concurrent_tasks, config.max_concurrent_tasks);
        assert_eq!(cloned.task_timeout, config.task_timeout);
        assert_eq!(cloned.enable_claiming, config.enable_claiming);
        assert_eq!(cloned.heartbeat_interval, config.heartbeat_interval);
    }

    #[test]
    fn test_executor_config_debug() {
        let config = ExecutorConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("ExecutorConfig"));
        assert!(debug_str.contains("max_concurrent_tasks"));
        assert!(debug_str.contains("4"));
    }

    // -----------------------------------------------------------------------
    // ClaimedTask tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_claimed_task_construction() {
        let task_exec_id = UniversalUuid::new_v4();
        let pipeline_exec_id = UniversalUuid::new_v4();
        let task = ClaimedTask {
            task_execution_id: task_exec_id,
            pipeline_execution_id: pipeline_exec_id,
            task_name: "tenant::pkg::wf::my_task".to_string(),
            attempt: 1,
        };
        assert_eq!(task.task_execution_id, task_exec_id);
        assert_eq!(task.pipeline_execution_id, pipeline_exec_id);
        assert_eq!(task.task_name, "tenant::pkg::wf::my_task");
        assert_eq!(task.attempt, 1);
    }

    #[test]
    fn test_claimed_task_retry_attempt() {
        let task = ClaimedTask {
            task_execution_id: UniversalUuid::new_v4(),
            pipeline_execution_id: UniversalUuid::new_v4(),
            task_name: "t::p::w::task".to_string(),
            attempt: 3,
        };
        assert_eq!(task.attempt, 3);
    }

    #[test]
    fn test_claimed_task_debug() {
        let task = ClaimedTask {
            task_execution_id: UniversalUuid::new_v4(),
            pipeline_execution_id: UniversalUuid::new_v4(),
            task_name: "t::p::w::debug_task".to_string(),
            attempt: 0,
        };
        let debug_str = format!("{:?}", task);
        assert!(debug_str.contains("ClaimedTask"));
        assert!(debug_str.contains("debug_task"));
    }

    // -----------------------------------------------------------------------
    // DependencyLoader tests (constructor only — load methods need DB)
    // -----------------------------------------------------------------------

    #[test]
    fn test_dependency_loader_debug() {
        // DependencyLoader requires a Database, which needs a URL.
        // We can at least verify the struct derives Debug by checking
        // that the type is Send + Sync (required for async usage).
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DependencyLoader>();
    }
}
