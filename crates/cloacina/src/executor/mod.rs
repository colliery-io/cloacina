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
//! The Task Executor provides automated parallel execution with lazy loading context merging.
//! It polls the database for ready tasks and executes them using cloacina's existing Task trait
//! and Context system with transparent dependency loading.
//!
//! ## Key Features
//!
//! - **Atomic Task Claiming**: Thread-safe task claiming with database locking
//! - **Lazy Context Loading**: Automatic dependency context loading just-in-time
//! - **Simple Conflict Resolution**: Latest task wins for key conflicts
//! - **Concurrency Management**: Configurable limits with semaphore-based control
//! - **Timeout Handling**: Per-task execution timeout with cancellation
//!
//! ## Components
//!
//! The executor module consists of several key components:
//!
//! - `ThreadTaskExecutor`: Thread-based executor for individual task execution
//! - `PipelineEngine`: Manages pipeline-based task execution workflows
//! - `PipelineExecutor`: Handles the execution of task pipelines with dependency management
//!
//! ## Usage
//!
//! ```rust
//! use cloacina::executor::{ThreadTaskExecutor, ExecutorConfig, TaskExecutorTrait};
//! use cloacina::{Database, TaskRegistry};
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let database = Database::new("postgres://localhost/cloacina", "cloacina", 5);
//! let task_registry = Arc::new(TaskRegistry::new());
//! let config = ExecutorConfig::default();
//!
//! let executor = ThreadTaskExecutor::new(database, task_registry, config);
//! executor.run().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Configuration
//!
//! The executor can be configured using `ExecutorConfig` which allows customization of:
//! - Concurrency limits
//! - Task polling intervals
//! - Execution timeouts
//! - Error handling strategies
//!
//! ## Error Handling
//!
//! The executor provides comprehensive error handling through:
//! - `PipelineError` for pipeline-specific errors
//! - `TaskResult` for individual task execution results
//! - `PipelineStatus` for tracking pipeline execution state
//!
//! ## Dependencies
//!
//! The executor requires:
//! - A PostgreSQL database connection
//! - A configured `TaskRegistry` for task definitions
//! - Proper task implementations conforming to the `Task` trait
//!
//! ## Thread Safety
//!
//! All components are designed to be thread-safe and can be safely used in concurrent environments.
//! The executor uses internal synchronization mechanisms to ensure safe concurrent execution.

pub mod pipeline_engine;
pub mod pipeline_executor;
pub mod thread_task_executor;
pub mod traits;
pub mod types;

pub use pipeline_engine::PipelineEngine;
pub use pipeline_executor::{
    PipelineError, PipelineExecution, PipelineExecutor, PipelineResult, PipelineStatus, TaskResult,
};
pub use thread_task_executor::ThreadTaskExecutor;
pub use traits::TaskExecutorTrait;
pub use types::{ClaimedTask, DependencyLoader, EngineMode, ExecutionScope, ExecutorConfig};
