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

//! # Task Management
//!
//! This module provides the core task execution framework. Tasks are the fundamental building
//! blocks of Cloacina pipelines - they represent individual units of work that can be executed
//! with dependencies, retry policies, and persistent state management.
//!
//! ## Core Concepts
//!
//! - **Tasks**: Individual units of work that can be executed asynchronously
//! - **Context**: Shared state container for passing data between tasks
//! - **Dependencies**: Directed acyclic graph (DAG) of task relationships
//! - **State Management**: Tracking task execution progress and status
//! - **Retry Policies**: Configurable error handling and retry strategies
//! - **Trigger Rules**: Conditional execution based on context data
//!
//! ## Task Lifecycle
//!
//! Tasks progress through these states during execution:
//!
//! ```mermaid
//! stateDiagram-v2
//!     [*] --> Pending
//!     Pending --> Running : scheduler starts task
//!     Running --> Completed : task succeeds
//!     Running --> Failed : task fails
//!     Failed --> Running : retry attempt
//!     Failed --> [*] : max retries exceeded
//!     Completed --> [*]
//! ```
//!
//! ## State Management
//!
//! Each task maintains its execution state for monitoring and recovery:
//!
//! - **Pending**: Task is registered but not yet started
//! - **Running**: Task is currently executing
//! - **Completed**: Task finished successfully
//! - **Failed**: Task encountered an error
//!
//! ## Error Handling
//!
//! Tasks can handle errors in several ways:
//!
//! - **Transient Failures**: Temporary issues that may succeed on retry
//! - **Permanent Failures**: Issues that won't succeed on retry
//! - **Retry Policies**: Configurable retry strategies with backoff
//!
//! ## Context Management
//!
//! The Context provides a thread-safe way to share data between tasks:
//!
//! - **Type Safety**: Generic type parameter ensures data consistency
//! - **Serialization**: Automatic serialization for persistence
//! - **Thread Safety**: Safe concurrent access to shared data
//!
//! ## Tutorial: Your First Task
//!
//! The easiest way to create tasks is with the `#[task]` macro:
//!
//! ```rust
//! use cloacina::*;
//!
//! #[task(
//!     id = "hello_world",
//!     dependencies = []
//! )]
//! async fn hello_world(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     println!("Hello from Cloacina!");
//!     context.insert("greeting", serde_json::json!("Hello World!"))?;
//!     Ok(())
//! }
//! ```
//!
//! That's it! The macro automatically:
//! - Implements the [`Task`] trait
//! - Registers the task globally for use in workflows
//! - Generates a unique code fingerprint for versioning
//! - Handles context serialization
//!
//! ## Tutorial: Tasks with Dependencies
//!
//! Tasks can depend on other tasks, forming a directed acyclic graph (DAG):
//!
//! ```rust
//! use cloacina::*;
//!
//! #[task(id = "fetch_data", dependencies = [])]
//! async fn fetch_data(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     // Simulate fetching data
//!     ctx.insert("raw_data", serde_json::json!({"users": [1, 2, 3]}))?;
//!     println!("Data fetched");
//!     Ok(())
//! }
//!
//! #[task(id = "process_data", dependencies = ["fetch_data"])]
//! async fn process_data(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     // This task runs after fetch_data completes
//!     if let Some(raw_data) = ctx.get("raw_data") {
//!         ctx.insert("processed_data", serde_json::json!({"processed": raw_data}))?;
//!         println!("Data processed");
//!     }
//!     Ok(())
//! }
//!
//! #[task(id = "save_data", dependencies = ["process_data"])]
//! async fn save_data(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     if let Some(processed_data) = ctx.get("processed_data") {
//!         println!("Saving: {:?}", processed_data);
//!     }
//!     Ok(())
//! }
//!
//! // Execution order: fetch_data → process_data → save_data
//! ```
//!
//! ## Task Lifecycle
//!
//! Tasks progress through these states during execution:
//!
//! ```mermaid
//! stateDiagram-v2
//!     [*] --> Pending
//!     Pending --> Running : scheduler starts task
//!     Running --> Completed : task succeeds
//!     Running --> Failed : task fails
//!     Failed --> Running : retry attempt
//!     Failed --> [*] : max retries exceeded
//!     Completed --> [*]
//! ```
//!
//! ## How-To Guide: Error Handling and Retries
//!
//! Configure retry policies for resilient task execution:
//!
//! ```rust
//! use cloacina::*;
//! use std::time::Duration;
//!
//! #[task(
//!     id = "network_request",
//!     dependencies = [],
//!     retry_policy = RetryPolicy::builder()
//!         .max_attempts(3)
//!         .initial_delay(Duration::from_secs(1))
//!         .backoff_strategy(BackoffStrategy::Exponential { base: 2.0, multiplier: 1.0 })
//!         .retry_condition(RetryCondition::TransientOnly)
//!         .build()
//! )]
//! async fn network_request(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     // This will retry up to 3 times with exponential backoff
//!     // if it fails with a transient error
//!     match make_api_call().await {
//!         Ok(data) => {
//!             ctx.insert("api_response", data)?;
//!             Ok(())
//!         }
//!         Err(e) if is_transient_error(&e) => {
//!             Err(TaskError::TransientFailure(e.to_string()))
//!         }
//!         Err(e) => {
//!             Err(TaskError::PermanentFailure(e.to_string()))
//!         }
//!     }
//! }
//!
//! # async fn make_api_call() -> Result<serde_json::Value, Box<dyn std::error::Error>> { Ok(serde_json::json!({})) }
//! # fn is_transient_error(_: &Box<dyn std::error::Error>) -> bool { true }
//! ```
//!
//! ## How-To Guide: Conditional Execution
//!
//! Use trigger rules for conditional task execution based on context data:
//!
//! ```rust
//! use cloacina::*;
//!
//! #[task(id = "validate_input", dependencies = [])]
//! async fn validate_input(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     // Simulate validation
//!     let is_valid = true; // Your validation logic here
//!     ctx.insert("validation_passed", serde_json::json!(is_valid))?;
//!     Ok(())
//! }
//!
//! #[task(
//!     id = "process_if_valid",
//!     dependencies = ["validate_input"],
//!     trigger_rules = serde_json::json!({
//!         "type": "Conditional",
//!         "condition": {
//!             "field": "validation_passed",
//!             "operator": "Equals",
//!             "value": true
//!         }
//!     })
//! )]
//! async fn process_if_valid(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     // This only runs if validation_passed == true in the context
//!     println!("Processing valid data");
//!     Ok(())
//! }
//! ```
//!
//! ## How-To Guide: Working with Context Data
//!
//! The [`Context`] is your primary way to share data between tasks:
//!
//! ```rust
//! use cloacina::*;
//! use serde_json::json;
//!
//! #[task(id = "producer", dependencies = [])]
//! async fn producer_task(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     // Insert various types of data
//!     ctx.insert("user_id", json!(12345))?;
//!     ctx.insert("config", json!({"env": "production", "batch_size": 100}))?;
//!     ctx.insert("timestamp", json!(chrono::Utc::now()))?;
//!     Ok(())
//! }
//!
//! #[task(id = "consumer", dependencies = ["producer"])]
//! async fn consumer_task(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     // Read data from context
//!     let user_id: i64 = ctx.get("user_id")
//!         .and_then(|v| v.as_i64())
//!         .ok_or_else(|| TaskError::DataError("Missing user_id".to_string()))?;
//!
//!     let config = ctx.get("config")
//!         .ok_or_else(|| TaskError::DataError("Missing config".to_string()))?;
//!
//!     let batch_size = config.get("batch_size")
//!         .and_then(|v| v.as_i64())
//!         .unwrap_or(50);
//!
//!     // Use the data
//!     println!("Processing user {} with batch size {}", user_id, batch_size);
//!
//!     // Update context with results
//!     ctx.insert("processed_count", json!(batch_size))?;
//!     Ok(())
//! }
//! ```
//!
//! ## How-To Guide: Task Testing
//!
//! Test your tasks in isolation:
//!
//! ```rust
//! use cloacina::*;
//!
//! #[task(id = "math_task", dependencies = [])]
//! async fn math_task(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     let a = ctx.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
//!     let b = ctx.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
//!     ctx.insert("result", serde_json::json!(a + b))?;
//!     Ok(())
//! }
//!
//! #[cfg(test)]
//! mod tests {
//!     use super::*;
//!
//!     #[tokio::test]
//!     async fn test_math_task() {
//!         let mut ctx = Context::new();
//!         ctx.insert("a", serde_json::json!(5)).unwrap();
//!         ctx.insert("b", serde_json::json!(3)).unwrap();
//!
//!         // Test the task function directly
//!         math_task(&mut ctx).await.unwrap();
//!
//!         let result = ctx.get("result").unwrap();
//!         assert_eq!(result, &serde_json::json!(8));
//!     }
//! }
//! ```
//!
//! ## Advanced: Manual Task Implementation
//!
//! For advanced use cases, you can implement the [`Task`] trait manually instead of using the macro:
//!
//! ```rust
//! use cloacina::*;
//! use async_trait::async_trait;
//!
//! struct CustomTask {
//!     id: String,
//!     dependencies: Vec<String>,
//! }
//!
//! #[async_trait]
//! impl Task for CustomTask {
//!     async fn execute(&self, mut context: Context<serde_json::Value>) -> Result<Context<serde_json::Value>, TaskError> {
//!         // Your custom logic here
//!         context.insert("custom_processed", serde_json::json!(true))?;
//!         Ok(context)
//!     }
//!
//!     fn id(&self) -> &str {
//!         &self.id
//!     }
//!
//!     fn dependencies(&self) -> &[String] {
//!         &self.dependencies
//!     }
//!
//!     fn retry_policy(&self) -> crate::retry::RetryPolicy {
//!         // Custom retry policy
//!         crate::retry::RetryPolicy::builder()
//!             .max_attempts(5)
//!             .build()
//!     }
//! }
//! ```
//!
//! ## Task State Management
//!
//! Tasks track their execution state for monitoring and recovery:

pub mod namespace;

use crate::context::Context;
use crate::error::{CheckpointError, RegistrationError, TaskError, ValidationError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
pub use namespace::TaskNamespace;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Represents the execution state of a task throughout its lifecycle.
///
/// Tasks progress through these states during execution, providing visibility
/// into the current status and enabling proper error handling and recovery.
///
/// # State Transitions
///
/// - `Pending` → `Running`: When task execution begins
/// - `Running` → `Completed`: When task completes successfully
/// - `Running` → `Failed`: When task encounters an error
/// - `Failed` → `Running`: When task is retried
///
/// Terminal states (`Completed` and `Failed`) do not transition to other states
/// unless a retry is attempted.
///
/// # State Details
///
/// ## Pending
/// Initial state when a task is registered but not yet started. Tasks remain in this
/// state until all dependencies are satisfied and the scheduler begins execution.
///
/// ## Running
/// Active state during task execution. Includes a timestamp of when execution began.
/// Tasks can transition from this state to either `Completed` or `Failed`.
///
/// ## Completed
/// Terminal state indicating successful task completion. Includes a timestamp of
/// when the task finished. Tasks in this state have successfully executed and
/// won't be retried.
///
/// ## Failed
/// Terminal state indicating task failure. Includes both the error message and
/// timestamp of when the failure occurred. Tasks in this state may be retried
/// based on their retry policy.
///
/// # Examples
///
/// ```rust
/// use cloacina::TaskState;
/// use chrono::Utc;
///
/// // Create a pending task
/// let state = TaskState::Pending;
/// assert!(state.is_pending());
///
/// // Start task execution
/// let state = TaskState::Running { start_time: Utc::now() };
/// assert!(state.is_running());
///
/// // Complete task successfully
/// let state = TaskState::Completed { completion_time: Utc::now() };
/// assert!(state.is_completed());
///
/// // Handle task failure
/// let state = TaskState::Failed {
///     error: "Network timeout".to_string(),
///     failure_time: Utc::now()
/// };
/// assert!(state.is_failed());
///
/// // Check state transitions
/// let mut state = TaskState::Pending;
/// assert!(state.is_pending());
///
/// // Simulate task start
/// state = TaskState::Running { start_time: Utc::now() };
/// assert!(state.is_running());
///
/// // Simulate task completion
/// state = TaskState::Completed { completion_time: Utc::now() };
/// assert!(state.is_completed());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskState {
    Pending,
    Running {
        start_time: DateTime<Utc>,
    },
    Completed {
        completion_time: DateTime<Utc>,
    },
    Failed {
        error: String,
        failure_time: DateTime<Utc>,
    },
    Skipped {
        reason: String,
        skip_time: DateTime<Utc>,
    },
}

impl TaskState {
    /// Returns true if the task is in the completed state
    pub fn is_completed(&self) -> bool {
        matches!(self, TaskState::Completed { .. })
    }

    /// Returns true if the task is in the failed state
    pub fn is_failed(&self) -> bool {
        matches!(self, TaskState::Failed { .. })
    }

    /// Returns true if the task is currently running
    pub fn is_running(&self) -> bool {
        matches!(self, TaskState::Running { .. })
    }

    /// Returns true if the task is pending execution
    pub fn is_pending(&self) -> bool {
        matches!(self, TaskState::Pending)
    }

    /// Returns true if the task was skipped
    pub fn is_skipped(&self) -> bool {
        matches!(self, TaskState::Skipped { .. })
    }
}

/// Core trait that defines an executable task in a pipeline.
///
/// Tasks are the fundamental units of work in Cloacina. Most users should use the
/// `#[task]` macro instead of implementing this trait directly, as the macro provides
/// automatic registration, code fingerprinting, and convenient syntax.
///
/// # Task Execution Model
///
/// Tasks follow a simple but powerful execution model:
///
/// 1. **Input**: Receive a context containing data from previous tasks
/// 2. **Processing**: Execute the task's business logic
/// 3. **Output**: Update the context with results
/// 4. **Completion**: Return success or failure
///
/// # Error Handling
///
/// Tasks can return two types of errors:
///
/// - **Transient Failures**: Temporary issues that may succeed on retry
///   - Network timeouts
///   - Rate limiting
///   - Temporary resource unavailability
///
/// - **Permanent Failures**: Issues that won't succeed on retry
///   - Invalid input data
///   - Configuration errors
///   - Business logic violations
///
/// # Context Management
///
/// The context provides a thread-safe way to share data between tasks:
///
/// - **Reading Data**: Use `context.get()` to access data from previous tasks
/// - **Writing Data**: Use `context.insert()` to store results for downstream tasks
/// - **Type Safety**: The generic type parameter ensures data consistency
/// - **Serialization**: Data is automatically serialized for persistence
///
/// # Using the Macro (Recommended)
///
/// ```rust
/// use cloacina::*;
///
/// #[task(id = "my_task", dependencies = [])]
/// async fn my_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
///     // Your task logic here
///     Ok(())
/// }
/// ```
///
/// # Manual Implementation (Advanced)
///
/// For advanced use cases where you need full control over the task behavior:
///
/// ```rust
/// use cloacina::*;
/// use async_trait::async_trait;
///
/// struct CustomTask;
///
/// #[async_trait]
/// impl Task for CustomTask {
///     async fn execute(&self, context: Context<serde_json::Value>) -> Result<Context<serde_json::Value>, TaskError> {
///         // Custom implementation
///         Ok(context)
///     }
///
///     fn id(&self) -> &str {
///         "custom_task"
///     }
///
///     fn dependencies(&self) -> &[String] {
///         &[]
///     }
/// }
/// ```
#[async_trait]
pub trait Task: Send + Sync {
    /// Executes the task with the provided context.
    ///
    /// This is the main entry point for task execution. The method receives
    /// a context containing data from previous tasks and should return an
    /// updated context with any new or modified data.
    ///
    /// # Arguments
    ///
    /// * `context` - The execution context containing task data
    ///
    /// # Returns
    ///
    /// * `Ok(Context)` - Updated context with task results
    /// * `Err(TaskError)` - If the task execution fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # use async_trait::async_trait;
    /// # struct MyTask;
    /// # #[async_trait]
    /// # impl Task for MyTask {
    /// async fn execute(&self, mut context: Context<serde_json::Value>) -> Result<Context<serde_json::Value>, TaskError> {
    ///     // Read input data
    ///     let input = context.get("input").cloned().unwrap_or_default();
    ///
    ///     // Process data
    ///     let result = serde_json::json!({"processed": true, "input": input});
    ///
    ///     // Store result
    ///     context.insert("result", result)?;
    ///
    ///     Ok(context)
    /// }
    /// #     fn id(&self) -> &str { "my_task" }
    /// #     fn dependencies(&self) -> &[String] { &[] }
    /// # }
    /// ```
    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError>;

    /// Returns the unique identifier for this task.
    ///
    /// The task ID must be unique within a Workflow or TaskRegistry.
    /// It's used for dependency resolution and task lookup.
    fn id(&self) -> &str;

    /// Returns the list of task namespaces that this task depends on.
    ///
    /// Dependencies define the execution order - this task will only
    /// execute after all its dependencies have completed successfully.
    fn dependencies(&self) -> &[TaskNamespace];

    /// Saves a checkpoint for this task.
    ///
    /// This method is called to save intermediate state during task execution.
    /// The default implementation is a no-op, but tasks can override this
    /// to implement custom checkpointing logic.
    ///
    /// # Arguments
    ///
    /// * `context` - The current execution context
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If checkpointing succeeds
    /// * `Err(CheckpointError)` - If checkpointing fails
    fn checkpoint(&self, _context: &Context<serde_json::Value>) -> Result<(), CheckpointError> {
        // Default implementation - tasks can override for custom checkpointing
        Ok(())
    }

    /// Returns the retry policy for this task.
    ///
    /// This method defines how the task should behave when it fails, including
    /// the number of retry attempts, backoff strategy, and conditions under
    /// which retries should be attempted.
    ///
    /// The default implementation returns a sensible production-ready policy
    /// with exponential backoff and 3 retry attempts.
    fn retry_policy(&self) -> crate::retry::RetryPolicy {
        crate::retry::RetryPolicy::default()
    }

    /// Returns the trigger rules for this task.
    ///
    /// Trigger rules define the conditions under which this task should execute
    /// beyond simple dependency satisfaction. The default implementation returns
    /// an "Always" trigger rule, meaning the task executes whenever its dependencies
    /// are satisfied.
    ///
    /// # Returns
    ///
    /// A JSON value representing the trigger rules for this task.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # use async_trait::async_trait;
    /// # struct MyTask;
    /// # #[async_trait]
    /// # impl Task for MyTask {
    /// fn trigger_rules(&self) -> serde_json::Value {
    ///     serde_json::json!({
    ///         "type": "Conditional",
    ///         "condition": {
    ///             "field": "should_run",
    ///             "operator": "Equals",
    ///             "value": true
    ///         }
    ///     })
    /// }
    /// #     async fn execute(&self, context: Context<serde_json::Value>) -> Result<Context<serde_json::Value>, TaskError> { Ok(context) }
    /// #     fn id(&self) -> &str { "my_task" }
    /// #     fn dependencies(&self) -> &[String] { &[] }
    /// # }
    /// ```
    fn trigger_rules(&self) -> serde_json::Value {
        serde_json::json!({"type": "Always"})
    }

    /// Returns a code fingerprint for content-based versioning.
    ///
    /// This method should return a hash of the task's implementation code,
    /// enabling automatic detection of changes for Workflow versioning.
    ///
    /// The default implementation returns None, indicating that the task
    /// doesn't support code fingerprinting. Tasks generated by the `#[task]`
    /// macro automatically provide fingerprints.
    ///
    /// # Returns
    ///
    /// - `Some(String)` - A hex-encoded hash of the task's code content
    /// - `None` - Task doesn't support code fingerprinting
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::*;
    /// # use async_trait::async_trait;
    /// # struct MyMacroTask;
    /// # #[async_trait]
    /// # impl Task for MyMacroTask {
    /// #     async fn execute(&self, context: Context<serde_json::Value>) -> Result<Context<serde_json::Value>, TaskError> { Ok(context) }
    /// #     fn id(&self) -> &str { "my_task" }
    /// #     fn dependencies(&self) -> &[String] { &[] }
    /// // Macro-generated tasks have fingerprints automatically
    /// fn code_fingerprint(&self) -> Option<String> {
    ///     Some("a1b2c3d4e5f67890".to_string())
    /// }
    /// # }
    /// ```
    fn code_fingerprint(&self) -> Option<String> {
        None
    }
}

/// Registry for managing collections of tasks and validating their dependencies.
///
/// The TaskRegistry provides a centralized container for tasks with built-in
/// validation of dependency relationships, cycle detection, and topological sorting.
/// Most users won't interact with this directly as the `#[task]` macro and
/// `workflow!` macro handle registration automatically.
///
/// Now supports namespaced task registration for isolation and conflict resolution.
///
/// # Features
///
/// - **Namespaced Task Registration**: Tasks organized by hierarchical namespace
/// - **Dependency Validation**: Ensure all dependencies exist and detect cycles
/// - **Topological Sorting**: Get tasks in dependency-safe execution order
/// - **Namespace-aware Lookup**: Retrieve tasks by namespace or with fallback
/// - **Multi-tenant Support**: Isolate tasks by tenant, package, and workflow
///
/// # Namespace Format
///
/// Tasks are identified by: `tenant_id::package_name::workflow_id::task_id`
///
/// # Examples
///
/// ```rust
/// use cloacina::*;
///
/// let mut registry = TaskRegistry::new();
///
/// // Register namespaced tasks
/// let ns1 = TaskNamespace::embedded("customer_etl", "extract");
/// let ns2 = TaskNamespace::embedded("customer_etl", "transform");
///
/// registry.register_with_namespace(ns1, TestTask::new("extract", vec![]))?;
/// registry.register_with_namespace(ns2, TestTask::new("transform", vec!["extract"]))?;
///
/// // Look up tasks by namespace
/// let task = registry.get_task_by_namespace(&TaskNamespace::embedded("customer_etl", "extract"));
/// assert!(task.is_some());
/// ```
pub struct TaskRegistry {
    tasks: HashMap<TaskNamespace, Arc<dyn Task>>,
}

impl TaskRegistry {
    /// Create a new empty task registry
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    /// Register a task in the registry
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace for the task
    /// * `task` - The task to register
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If registration succeeds
    /// * `Err(RegistrationError)` - If the namespace is already taken
    pub fn register<T: Task + 'static>(
        &mut self,
        namespace: TaskNamespace,
        task: T,
    ) -> Result<(), RegistrationError> {
        // Validate task ID is not empty
        if namespace.task_id.is_empty() {
            return Err(RegistrationError::InvalidTaskId {
                message: "Task ID cannot be empty".to_string(),
            });
        }

        // Check for duplicate namespace
        if self.tasks.contains_key(&namespace) {
            return Err(RegistrationError::DuplicateTaskId {
                id: namespace.to_string(),
            });
        }

        self.tasks.insert(namespace, Arc::new(task));
        Ok(())
    }

    /// Register a boxed task in the registry (used internally)
    pub fn register_arc(
        &mut self,
        namespace: TaskNamespace,
        task: Arc<dyn Task>,
    ) -> Result<(), RegistrationError> {
        // Validate task ID is not empty
        if namespace.task_id.is_empty() {
            return Err(RegistrationError::InvalidTaskId {
                message: "Task ID cannot be empty".to_string(),
            });
        }

        // Check for duplicate namespace
        if self.tasks.contains_key(&namespace) {
            return Err(RegistrationError::DuplicateTaskId {
                id: namespace.to_string(),
            });
        }

        self.tasks.insert(namespace, task);
        Ok(())
    }

    /// Get a task by namespace
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace to look up
    ///
    /// # Returns
    ///
    /// * `Some(Arc<dyn Task>)` - If the task exists
    /// * `None` - If no task with that namespace is registered
    pub fn get_task(&self, namespace: &TaskNamespace) -> Option<Arc<dyn Task>> {
        self.tasks.get(namespace).cloned()
    }

    /// Get all registered task namespaces
    ///
    /// # Returns
    ///
    /// A vector of all task namespaces currently registered
    pub fn task_ids(&self) -> Vec<TaskNamespace> {
        self.tasks.keys().cloned().collect()
    }

    /// Validate all task dependencies
    ///
    /// Checks that:
    /// - All dependencies exist as registered tasks
    /// - No circular dependencies exist
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If all dependencies are valid
    /// * `Err(ValidationError)` - If validation fails
    pub fn validate_dependencies(&self) -> Result<(), ValidationError> {
        // Check for missing dependencies
        for (namespace, task) in &self.tasks {
            for dependency_namespace in task.dependencies() {
                if !self.tasks.contains_key(dependency_namespace) {
                    return Err(ValidationError::MissingDependencyOld {
                        task_id: namespace.to_string(),
                        dependency: dependency_namespace.to_string(),
                    });
                }
            }
        }

        // Check for circular dependencies using DFS
        let mut visited = HashMap::new();
        let mut rec_stack = HashMap::new();

        for namespace in self.tasks.keys() {
            if !visited.get(namespace).unwrap_or(&false) {
                if let Err(cycle) = self.check_cycles(namespace, &mut visited, &mut rec_stack) {
                    return Err(ValidationError::CircularDependency { cycle });
                }
            }
        }

        Ok(())
    }

    /// Helper method to detect circular dependencies using DFS
    fn check_cycles(
        &self,
        namespace: &TaskNamespace,
        visited: &mut HashMap<TaskNamespace, bool>,
        rec_stack: &mut HashMap<TaskNamespace, bool>,
    ) -> Result<(), String> {
        visited.insert(namespace.clone(), true);
        rec_stack.insert(namespace.clone(), true);

        if let Some(task) = self.tasks.get(namespace) {
            for dependency_namespace in task.dependencies() {
                if !visited.get(dependency_namespace).unwrap_or(&false) {
                    if let Err(cycle) = self.check_cycles(dependency_namespace, visited, rec_stack)
                    {
                        return Err(format!("{} -> {}", namespace.task_id, cycle));
                    }
                } else if *rec_stack.get(dependency_namespace).unwrap_or(&false) {
                    return Err(format!(
                        "{} -> {}",
                        namespace.task_id, dependency_namespace.task_id
                    ));
                }
            }
        }

        rec_stack.insert(namespace.clone(), false);
        Ok(())
    }

    /// Get tasks in topological order (dependencies first)
    ///
    /// Returns tasks sorted so that dependencies come before the tasks that depend on them.
    /// This is the safe execution order for the tasks.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<TaskNamespace>)` - Task namespaces in topological order
    /// * `Err(ValidationError)` - If dependencies are invalid or cycles exist
    pub fn topological_sort(&self) -> Result<Vec<TaskNamespace>, ValidationError> {
        // First validate dependencies
        self.validate_dependencies()?;

        let mut in_degree = HashMap::new();
        let mut adj_list = HashMap::new();

        // Initialize in-degree and adjacency list
        for namespace in self.tasks.keys() {
            in_degree.insert(namespace.clone(), 0);
            adj_list.insert(namespace.clone(), Vec::new());
        }

        // Build adjacency list and calculate in-degrees
        for (namespace, task) in &self.tasks {
            for dependency_namespace in task.dependencies() {
                if let Some(adj_list_entry) = adj_list.get_mut(dependency_namespace) {
                    adj_list_entry.push(namespace.clone());
                    *in_degree.get_mut(namespace).unwrap() += 1;
                }
            }
        }

        // Kahn's algorithm for topological sorting
        let mut queue = Vec::new();
        let mut result = Vec::new();

        // Add nodes with no incoming edges
        for (namespace, &degree) in &in_degree {
            if degree == 0 {
                queue.push(namespace.clone());
            }
        }

        while let Some(current) = queue.pop() {
            result.push(current.clone());

            // Process all neighbors
            for neighbor in &adj_list[&current] {
                let degree = in_degree.get_mut(neighbor).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push(neighbor.clone());
                }
            }
        }

        if result.len() != self.tasks.len() {
            return Err(ValidationError::InvalidGraph {
                message: "Graph contains cycles".to_string(),
            });
        }

        Ok(result)
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for the task constructor function stored in the global registry
type TaskConstructor = Box<dyn Fn() -> Arc<dyn Task> + Send + Sync>;

/// Type alias for the global task registry containing task constructors
type GlobalTaskRegistry = Arc<RwLock<HashMap<TaskNamespace, TaskConstructor>>>;

/// Global registry for automatically registering tasks created with the `#[task]` macro
static GLOBAL_TASK_REGISTRY: Lazy<GlobalTaskRegistry> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Register a task constructor function globally with namespace
///
/// This is used internally by the `workflow!` macro to automatically register tasks.
/// Most users won't call this directly.
pub fn register_task_constructor<F>(namespace: TaskNamespace, constructor: F)
where
    F: Fn() -> Arc<dyn Task> + Send + Sync + 'static,
{
    let mut registry = match GLOBAL_TASK_REGISTRY.write() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!("Task registry RwLock was poisoned, recovering data");
            poisoned.into_inner()
        }
    };
    registry.insert(namespace.clone(), Box::new(constructor));
    tracing::debug!(
        "Successfully registered task constructor for namespace: {}",
        namespace
    );
}

/// Get the global task registry
///
/// This provides access to the global task registry used by the macro system.
/// Most users won't need to call this directly.
pub fn global_task_registry() -> GlobalTaskRegistry {
    GLOBAL_TASK_REGISTRY.clone()
}

/// Get a task instance from the global registry by namespace
///
/// This is a convenience function for getting task instances without
/// directly accessing the registry.
pub fn get_task(namespace: &TaskNamespace) -> Option<Arc<dyn Task>> {
    let registry = match GLOBAL_TASK_REGISTRY.read() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!("Task registry RwLock was poisoned, recovering data");
            poisoned.into_inner()
        }
    };
    registry.get(namespace).map(|constructor| constructor())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_test_logging;

    // Test task implementation
    struct TestTask {
        id: String,
        dependencies: Vec<TaskNamespace>,
        fingerprint: Option<String>,
    }

    impl TestTask {
        fn new(id: &str, dependencies: Vec<TaskNamespace>) -> Self {
            Self {
                id: id.to_string(),
                dependencies,
                fingerprint: None,
            }
        }

        fn with_fingerprint(mut self, fingerprint: &str) -> Self {
            self.fingerprint = Some(fingerprint.to_string());
            self
        }
    }

    #[async_trait]
    impl Task for TestTask {
        async fn execute(
            &self,
            context: Context<serde_json::Value>,
        ) -> Result<Context<serde_json::Value>, TaskError> {
            // Simple test implementation
            Ok(context)
        }

        fn id(&self) -> &str {
            &self.id
        }

        fn dependencies(&self) -> &[TaskNamespace] {
            &self.dependencies
        }

        fn code_fingerprint(&self) -> Option<String> {
            self.fingerprint.clone()
        }
    }

    #[test]
    fn test_task_state() {
        init_test_logging();

        let pending = TaskState::Pending;
        assert!(pending.is_pending());
        assert!(!pending.is_running());
        assert!(!pending.is_completed());
        assert!(!pending.is_failed());

        let running = TaskState::Running {
            start_time: Utc::now(),
        };
        assert!(running.is_running());
        assert!(!running.is_pending());

        let completed = TaskState::Completed {
            completion_time: Utc::now(),
        };
        assert!(completed.is_completed());
        assert!(!running.is_failed());

        let failed = TaskState::Failed {
            error: "test error".to_string(),
            failure_time: Utc::now(),
        };
        assert!(failed.is_failed());
        assert!(!failed.is_completed());
    }

    #[test]
    fn test_task_registry_basic() {
        init_test_logging();

        let mut registry = TaskRegistry::new();

        let ns1 = TaskNamespace::new("public", "embedded", "test_workflow", "task1");
        let ns2 = TaskNamespace::new("public", "embedded", "test_workflow", "task2");

        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec![ns1.clone()]);

        assert!(registry.register(ns1.clone(), task1).is_ok());
        assert!(registry.register(ns2.clone(), task2).is_ok());

        assert!(registry.get_task(&ns1).is_some());
        assert!(registry.get_task(&ns2).is_some());
    }

    #[test]
    fn test_task_registry_duplicate_id() {
        init_test_logging();

        let mut registry = TaskRegistry::new();

        let ns1 = TaskNamespace::new("public", "embedded", "test_workflow", "task1");

        let task1 = TestTask::new("task1", vec![]);
        let task1_duplicate = TestTask::new("task1", vec![]);

        assert!(registry.register(ns1.clone(), task1).is_ok());
        assert!(matches!(
            registry.register(ns1, task1_duplicate),
            Err(RegistrationError::DuplicateTaskId { .. })
        ));
    }

    #[test]
    fn test_dependency_validation() {
        init_test_logging();

        let mut registry = TaskRegistry::new();

        let ns1 = TaskNamespace::new("public", "embedded", "test_workflow", "task1");
        let ns2 = TaskNamespace::new("public", "embedded", "test_workflow", "task2");
        let ns3 = TaskNamespace::new("public", "embedded", "test_workflow", "task3");
        let nonexistent_ns =
            TaskNamespace::new("public", "embedded", "test_workflow", "nonexistent");

        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec![ns1.clone()]);
        let task3 = TestTask::new("task3", vec![nonexistent_ns]);

        registry.register(ns1, task1).unwrap();
        registry.register(ns2, task2).unwrap();
        registry.register(ns3, task3).unwrap();

        // Should fail due to missing dependency
        assert!(matches!(
            registry.validate_dependencies(),
            Err(ValidationError::MissingDependencyOld { .. })
        ));
    }

    #[test]
    fn test_circular_dependency_detection() {
        init_test_logging();

        let mut registry = TaskRegistry::new();

        let ns1 = TaskNamespace::new("public", "embedded", "test_workflow", "task1");
        let ns2 = TaskNamespace::new("public", "embedded", "test_workflow", "task2");

        let task1 = TestTask::new("task1", vec![ns2.clone()]);
        let task2 = TestTask::new("task2", vec![ns1.clone()]);

        registry.register(ns1, task1).unwrap();
        registry.register(ns2, task2).unwrap();

        assert!(matches!(
            registry.validate_dependencies(),
            Err(ValidationError::CircularDependency { .. })
        ));
    }

    #[test]
    fn test_topological_sort() {
        init_test_logging();

        let mut registry = TaskRegistry::new();

        let ns1 = TaskNamespace::new("public", "embedded", "test_workflow", "task1");
        let ns2 = TaskNamespace::new("public", "embedded", "test_workflow", "task2");
        let ns3 = TaskNamespace::new("public", "embedded", "test_workflow", "task3");

        let task1 = TestTask::new("task1", vec![]);
        let task2 = TestTask::new("task2", vec![ns1.clone()]);
        let task3 = TestTask::new("task3", vec![ns1.clone(), ns2.clone()]);

        registry.register(ns1.clone(), task1).unwrap();
        registry.register(ns2.clone(), task2).unwrap();
        registry.register(ns3.clone(), task3).unwrap();

        let sorted = registry.topological_sort().unwrap();

        // task1 should come before task2 and task3
        // task2 should come before task3
        let pos1 = sorted.iter().position(|x| x.task_id == "task1").unwrap();
        let pos2 = sorted.iter().position(|x| x.task_id == "task2").unwrap();
        let pos3 = sorted.iter().position(|x| x.task_id == "task3").unwrap();

        assert!(pos1 < pos2);
        assert!(pos1 < pos3);
        assert!(pos2 < pos3);
    }

    #[test]
    fn test_code_fingerprint_none_by_default() {
        init_test_logging();

        let task = TestTask::new("test", vec![]);
        assert_eq!(task.code_fingerprint(), None);
    }

    #[test]
    fn test_code_fingerprint_when_provided() {
        init_test_logging();

        let task = TestTask::new("test", vec![]).with_fingerprint("abc123def456");
        assert_eq!(task.code_fingerprint(), Some("abc123def456".to_string()));
    }
}
