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

//! Executor trait definitions
//!
//! This module contains the trait abstractions for task execution,
//! enabling pluggable executor implementations.

use crate::error::ExecutorError;
use async_trait::async_trait;

/// Core trait for task execution engines
///
/// Implementors of this trait provide different execution strategies
/// for running tasks that have been marked as ready by the scheduler.
///
/// The executor is responsible for:
/// - Polling the database for ready tasks
/// - Claiming and executing tasks
/// - Updating task status and context
/// - Managing concurrency and resource limits
///
/// # Implementation Requirements
///
/// Implementors must ensure:
/// - Thread safety (Send + Sync)
/// - Proper error handling and recovery
/// - Database state consistency
/// - Graceful handling of shutdown signals
///
/// # Examples
///
/// ```rust
/// use cloacina::executor::traits::TaskExecutorTrait;
/// use async_trait::async_trait;
///
/// struct MyExecutor;
///
/// #[async_trait]
/// impl TaskExecutorTrait for MyExecutor {
///     async fn run(&self) -> Result<(), ExecutorError> {
///         // Implementation here
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait TaskExecutorTrait: Send + Sync {
    /// Run the executor's main polling and execution loop
    ///
    /// This method should:
    /// - Poll the database for ready tasks
    /// - Execute tasks according to the implementation strategy
    /// - Handle errors and update task states
    /// - Continue until shutdown is requested
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Normal shutdown
    /// * `Err(ExecutorError)` - Fatal error requiring shutdown
    async fn run(&self) -> Result<(), ExecutorError>;
}
