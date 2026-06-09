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

//! # Dispatcher Layer for Executor Decoupling
//!
//! The dispatcher module provides a clean abstraction between the scheduler and executor,
//! enabling pluggable executor backends without tight coupling through database polling.
//!
//! ## Architecture
//!
//! ```text
//! Scheduler (mark_ready) --> Dispatcher --> Executor
//! ```
//!
//! The dispatcher receives `TaskReadyEvent`s from the scheduler and sends every
//! task to a single server-configured executor (CLOACI-T-0640). Selecting which
//! node/compute a task lands on is an executor-internal concern, not something
//! the scheduler/dispatcher decides.
//!
//! ## Key Components
//!
//! - [`TaskReadyEvent`]: Event emitted when a task becomes ready for execution
//! - [`Dispatcher`]: Trait for dispatching events to the configured executor
//! - [`TaskExecutor`]: Trait for executor backends that receive and execute tasks
//! - [`DefaultDispatcher`]: Standard single-executor implementation
//!
//! ## Usage
//!
//! ```rust,ignore
//! use cloacina::dispatcher::{DefaultDispatcher, TaskReadyEvent};
//! use cloacina::dispatcher::TaskExecutor;
//!
//! // Dispatch every task to the "default" (thread) executor.
//! let mut dispatcher = DefaultDispatcher::new(dal, "default");
//!
//! // Register executor backends
//! dispatcher.register_executor("default", Arc::new(thread_executor));
//!
//! // Dispatch ready tasks
//! dispatcher.dispatch(event).await?;
//! ```

pub mod default;
pub mod traits;
pub mod types;

pub use default::DefaultDispatcher;
pub use traits::{Dispatcher, TaskExecutor};
pub use types::{DispatchError, ExecutionResult, ExecutionStatus, ExecutorMetrics, TaskReadyEvent};
