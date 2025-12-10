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

//! Trait definitions for dispatcher and executor abstractions.
//!
//! This module defines the core traits that enable pluggable executor backends.
//! Implementors can create custom executors (Kubernetes, serverless, message queues)
//! that integrate seamlessly with the scheduler.

use async_trait::async_trait;
use std::sync::Arc;

use super::types::{DispatchError, ExecutionResult, ExecutorMetrics, TaskReadyEvent};

/// Dispatcher routes task-ready events to appropriate executors.
///
/// The dispatcher acts as a routing layer between the scheduler and executors,
/// enabling flexible task routing based on configuration rules.
///
/// # Implementation Requirements
///
/// Implementors must ensure:
/// - Thread safety (Send + Sync)
/// - Proper executor registration and lookup
/// - Routing based on configuration
///
/// # Example
///
/// ```rust,ignore
/// use cloacina::dispatcher::{Dispatcher, TaskReadyEvent, TaskExecutor};
///
/// struct MyDispatcher {
///     executors: HashMap<String, Arc<dyn TaskExecutor>>,
/// }
///
/// #[async_trait]
/// impl Dispatcher for MyDispatcher {
///     async fn dispatch(&self, event: TaskReadyEvent) -> Result<(), DispatchError> {
///         let executor = self.resolve_executor(&event.task_name);
///         executor.execute(event).await?;
///         Ok(())
///     }
///     // ...
/// }
/// ```
#[async_trait]
pub trait Dispatcher: Send + Sync {
    /// Dispatch a ready task to the appropriate executor.
    ///
    /// This method routes the task to an executor based on routing rules
    /// and initiates execution. The actual execution may be synchronous
    /// or asynchronous depending on the executor implementation.
    ///
    /// # Arguments
    ///
    /// * `event` - The task ready event containing all execution context
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Task was successfully dispatched
    /// * `Err(DispatchError)` - Dispatch failed (executor not found, no capacity, etc.)
    async fn dispatch(&self, event: TaskReadyEvent) -> Result<(), DispatchError>;

    /// Register an executor backend.
    ///
    /// Executors are registered with a string key that is used by routing rules
    /// to direct tasks to the appropriate backend.
    ///
    /// # Arguments
    ///
    /// * `key` - Unique identifier for this executor
    /// * `executor` - The executor implementation
    fn register_executor(&self, key: &str, executor: Arc<dyn TaskExecutor>);

    /// Check if the dispatcher has any executor with available capacity.
    ///
    /// This can be used by the scheduler to throttle task marking when
    /// all executors are at capacity.
    fn has_capacity(&self) -> bool;

    /// Get the executor key that would handle a given task.
    ///
    /// Useful for debugging and monitoring which executor will handle a task.
    fn resolve_executor_key(&self, task_name: &str) -> String;
}

/// Executor receives task-ready events and executes them.
///
/// This trait represents a task execution backend. Implementations can vary
/// from local thread pools to remote execution on Kubernetes, serverless
/// platforms, or message queues.
///
/// # Implementation Requirements
///
/// Implementors must ensure:
/// - Thread safety (Send + Sync)
/// - Proper resource management (concurrency limits)
/// - Accurate capacity reporting
/// - Clean execution isolation
///
/// # Example
///
/// ```rust,ignore
/// use cloacina::dispatcher::{TaskExecutor, TaskReadyEvent, ExecutionResult};
///
/// struct KubernetesExecutor {
///     client: kube::Client,
///     namespace: String,
/// }
///
/// #[async_trait]
/// impl TaskExecutor for KubernetesExecutor {
///     async fn execute(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError> {
///         // Create a Kubernetes Job for the task
///         let job = self.create_job(&event).await?;
///         let result = self.wait_for_completion(job).await?;
///         Ok(result)
///     }
///     // ...
/// }
/// ```
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Execute a task from a ready event.
    ///
    /// The executor is responsible for:
    /// - Running the task according to its paradigm
    /// - Managing execution timeouts
    /// - Capturing output context
    /// - Reporting completion status
    ///
    /// # Arguments
    ///
    /// * `event` - The task ready event with execution context
    ///
    /// # Returns
    ///
    /// * `Ok(ExecutionResult)` - Task completed (success or failure captured in result)
    /// * `Err(DispatchError)` - Execution infrastructure failed
    async fn execute(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError>;

    /// Check if this executor can accept more work.
    ///
    /// Used by the dispatcher to route work only to executors with available capacity.
    fn has_capacity(&self) -> bool;

    /// Get executor metrics for monitoring.
    ///
    /// Returns current statistics about the executor's state and performance.
    fn metrics(&self) -> ExecutorMetrics;

    /// Get a human-readable name for this executor type.
    ///
    /// Used for logging and debugging purposes.
    fn name(&self) -> &str;
}
