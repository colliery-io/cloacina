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

//! # Database Models
//!
//! This module contains Diesel model definitions for database entities.
//! Models define the structure of database tables and provide type-safe
//! access to database records.
//!
//! ## Models
//!
//! - [`context`]: Models for execution context storage
//! - [`cron_execution`]: Models for tracking cron schedule execution handoffs and audit trail
//! - [`cron_schedule`]: Models for time-based workflow scheduling with cron expressions
//! - [`pipeline_execution`]: Models for tracking pipeline execution state and metadata
//! - [`recovery_event`]: Models for recording system recovery events and state transitions
//! - [`task_execution`]: Models for managing individual task execution records
//! - [`task_execution_metadata`]: Models for storing task execution metadata and context references
//! - [`workflow_registry`]: Models for binary workflow package storage
//!
//! ## Usage
//!
//! Models are typically used with the Data Access Layer (DAL) to perform
//! database operations:
//!
//! ```rust,ignore
//! use cloacina::models::context::NewDbContext;
//! use cloacina::dal::DAL;
//!
//! # fn example(dal: &DAL) -> Result<(), Box<dyn std::error::Error>> {
//! let new_context = NewDbContext {
//!     value: r#"{"key": "value"}"#.to_string(),
//! };
//!
//! let inserted = dal.context().create(new_context)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Module Structure
//!
//! Each module contains:
//! - Struct definitions for database tables
//! - New-type structs for creating new records
//! - Type-safe query builders
//! - Serialization/deserialization implementations
//!
//! ## Best Practices
//!
//! - Always use the provided new-type structs for creating records
//! - Utilize the DAL for database operations rather than direct Diesel queries
//! - Handle database errors appropriately using the Result type
//! - Keep model definitions in sync with database schema migrations

// #[cfg(feature = "auth")]
// pub mod auth_audit_log;
// #[cfg(feature = "auth")]
// pub mod auth_tokens;
pub mod context;
pub mod cron_execution;
pub mod cron_schedule;
pub mod pipeline_execution;
pub mod recovery_event;
pub mod task_execution;
pub mod task_execution_metadata;
pub mod trigger_execution;
pub mod trigger_schedule;
pub mod workflow_packages;
pub mod workflow_registry;
