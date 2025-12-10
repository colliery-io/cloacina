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

//! # Dispatcher Layer for Executor Decoupling
//!
//! The dispatcher module provides a clean abstraction between the scheduler and executor,
//! enabling pluggable executor backends without tight coupling through database polling.
//!
//! ## Architecture
//!
//! ```text
//! Scheduler (mark_ready) --> Dispatcher --> Executor(s)
//!                                |
//!                                v
//!                           Routing Logic
//! ```
//!
//! The dispatcher receives `TaskReadyEvent`s from the scheduler and routes them to
//! the appropriate executor based on configurable routing rules.
//!
//! ## Key Components
//!
//! - [`TaskReadyEvent`]: Event emitted when a task becomes ready for execution
//! - [`Dispatcher`]: Trait for routing events to executors
//! - [`TaskExecutor`]: Trait for executor backends that receive and execute tasks
//! - [`DefaultDispatcher`]: Standard implementation with glob-based routing
//!
//! ## Usage
//!
//! ```rust,ignore
//! use cloacina::dispatcher::{DefaultDispatcher, RoutingConfig, TaskReadyEvent};
//! use cloacina::dispatcher::TaskExecutor;
//!
//! // Create dispatcher with routing configuration
//! let config = RoutingConfig::default();
//! let mut dispatcher = DefaultDispatcher::new(dal, config);
//!
//! // Register executor backends
//! dispatcher.register_executor("default", Arc::new(thread_executor));
//!
//! // Dispatch ready tasks
//! dispatcher.dispatch(event).await?;
//! ```

pub mod default;
pub mod router;
pub mod traits;
pub mod types;

pub use default::DefaultDispatcher;
pub use router::Router;
pub use traits::{Dispatcher, TaskExecutor};
pub use types::{
    DispatchError, ExecutionResult, ExecutionStatus, ExecutorMetrics, RoutingConfig, RoutingRule,
    TaskReadyEvent,
};
