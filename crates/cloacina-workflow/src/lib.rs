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

//! # Cloacina Workflow - Minimal Types for Workflow Authoring
//!
//! This crate provides the minimal set of types needed to compile Cloacina workflows
//! without pulling in heavy runtime dependencies like database drivers.
//!
//! ## Purpose
//!
//! Workflow authors who only need to compile workflows can depend on this lightweight
//! crate instead of the full `cloacina` crate. This provides:
//!
//! - Faster compile times
//! - Smaller binary sizes
//! - Easier cross-compilation (no native database drivers)
//! - Clear separation between "authoring workflows" and "running workflows"
//!
//! ## Types Provided
//!
//! - [`Context`] - Data container for sharing values between tasks
//! - [`Task`] - Trait defining executable tasks
//! - [`TaskState`] - Task execution state enum
//! - [`TaskNamespace`] - Hierarchical task identification
//! - [`TaskError`], [`ContextError`], [`CheckpointError`] - Error types
//! - [`RetryPolicy`], [`BackoffStrategy`], [`RetryCondition`] - Retry configuration
//!
//! ## Usage
//!
//! ```rust
//! use cloacina_workflow::{Context, TaskError};
//!
//! // Create a context
//! let mut ctx = Context::<serde_json::Value>::new();
//! ctx.insert("key", serde_json::json!("value")).unwrap();
//!
//! // Access data
//! let value = ctx.get("key").unwrap();
//! ```
//!
//! ## With the Task Macro
//!
//! The macros are included by default, so you only need one import:
//!
//! ```rust,ignore
//! use cloacina_workflow::{task, packaged_workflow, Context, TaskError};
//!
//! #[task(id = "my_task", dependencies = [])]
//! async fn my_task(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     ctx.insert("result", serde_json::json!("done"))?;
//!     Ok(())
//! }
//! ```

pub mod context;
pub mod error;
pub mod namespace;
pub mod retry;
pub mod task;

// Re-export primary types at crate root for convenience
pub use context::Context;
pub use error::{CheckpointError, ContextError, TaskError};
pub use namespace::{parse_namespace, TaskNamespace};
pub use retry::{BackoffStrategy, RetryCondition, RetryPolicy, RetryPolicyBuilder};
pub use task::{Task, TaskState};

// Re-export macros when the feature is enabled
#[cfg(feature = "macros")]
pub use cloacina_macros::{packaged_workflow, task, workflow};
