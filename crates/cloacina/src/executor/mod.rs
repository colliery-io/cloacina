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

//! # Task Executor
//!
//! The Task Executor provides task execution via the dispatcher architecture.
//! The scheduler pushes task-ready events to executors, which handle execution
//! with automatic context loading and state management.
//!
//! ## Key Features
//!
//! - **Push-Based Execution**: Scheduler dispatches tasks directly to executors
//! - **Atomic Task Claiming**: Thread-safe task claiming with database locking
//! - **Lazy Context Loading**: Automatic dependency context loading at execution time
//! - **Simple Conflict Resolution**: Latest task wins for key conflicts
//! - **Concurrency Management**: Configurable limits with semaphore-based control
//! - **Timeout Handling**: Per-task execution timeout with cancellation
//!
//! ## Components
//!
//! - `ThreadTaskExecutor`: Default executor implementing the `TaskExecutor` trait
//! - `PipelineExecutor`: Handles pipeline execution results and status tracking
//!
//! ## Configuration
//!
//! The executor can be configured using `ExecutorConfig`:
//! - Concurrency limits (`max_concurrent_tasks`)
//! - Execution timeouts (`task_timeout`)
//!
//! ## Thread Safety
//!
//! All components are thread-safe and can be used in concurrent environments.

pub mod pipeline_executor;
pub mod thread_task_executor;
pub mod types;

pub use pipeline_executor::{
    PipelineError, PipelineExecution, PipelineExecutor, PipelineResult, PipelineStatus, TaskResult,
};
pub use thread_task_executor::ThreadTaskExecutor;
pub use types::{ClaimedTask, DependencyLoader, ExecutorConfig};
