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

//! # Data Access Layer (DAL)
//!
//! This module provides the data access layer for Cloacina, offering a clean
//! interface between the application logic and the database. The DAL abstracts
//! database operations and manages connection pooling.
//!
//! ## Overview
//!
//! The DAL follows a modular approach where each entity type has its own
//! specialized data access module:
//!
//! - [`context`]: Context persistence and retrieval operations
//! - [`pipeline_execution`]: Pipeline execution tracking and management
//! - [`recovery_event`]: Recovery event logging and tracking
//! - [`task_execution`]: Task execution state and history management
//! - [`task_execution_metadata`]: Additional metadata for task executions
//!
//! ## Architecture
//!
//! ```mermaid
//! graph TB
//!     A["Application Logic"] --> B["DAL"]
//!     B --> C["ContextDAL"]
//!     B --> D["PipelineExecutionDAL"]
//!     B --> E["RecoveryEventDAL"]
//!     B --> F["TaskExecutionDAL"]
//!     B --> G["TaskExecutionMetadataDAL"]
//!     C --> H["Database Models"]
//!     D --> H
//!     E --> H
//!     F --> H
//!     G --> H
//!     H --> I["PostgreSQL"]
//!
//!     B -.-> J["Connection Pool"]
//!     J -.-> I
//! ```
//!
//! ## Usage
//!
//! ```rust
//! use cloacina::{Database, dal::DAL};
//! use diesel::result::Error;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize database and DAL
//! let database = Database::new("postgresql://localhost/cloacina")?;
//! let dal = DAL::new(database.pool());
//!
//! // Use specific DAL modules
//! let contexts = dal.context().list()?;
//! let pipeline_executions = dal.pipeline_execution().list()?;
//! let task_executions = dal.task_execution().list()?;
//! let recovery_events = dal.recovery_event().list()?;
//! let task_metadata = dal.task_execution_metadata().list()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Error Handling
//!
//! All DAL operations return `Result` types that wrap either the successful result
//! or a database error. The error types are typically `diesel::result::Error` for
//! database-specific errors.
//!
//! ## Connection Management
//!
//! The DAL uses a connection pool to manage database connections efficiently.
//! Connections are automatically acquired from the pool when needed and returned
//! when operations complete. The pool size and other connection parameters can be
//! configured when creating the database instance.

use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

pub mod context;
pub mod cron_execution;
pub mod cron_schedule;
pub mod pipeline_execution;
pub mod recovery_event;
pub mod task_execution;
pub mod task_execution_metadata;
use context::ContextDAL;
use cron_execution::CronExecutionDAL;
use cron_schedule::CronScheduleDAL;
use pipeline_execution::PipelineExecutionDAL;
use recovery_event::RecoveryEventDAL;
use task_execution::TaskExecutionDAL;
use task_execution_metadata::TaskExecutionMetadataDAL;

// Re-export for public API
pub use cron_execution::CronExecutionStats;

/// The main Data Access Layer struct.
///
/// This struct serves as the central point for database operations,
/// managing a connection pool and providing access to specific DAL
/// implementations for different entities.
///
/// # Examples
///
/// ```rust
/// use cloacina::dal::DAL;
/// use diesel::r2d2::{Pool, ConnectionManager};
/// use diesel::pg::PgConnection;
///
/// # fn example(pool: Pool<ConnectionManager<PgConnection>>) {
/// let dal = DAL::new(pool);
///
/// // Access context operations
/// let context_dal = dal.context();
/// # }
/// ```
#[derive(Clone)]
pub struct DAL {
    /// A connection pool for PostgreSQL database connections.
    pub pool: Pool<ConnectionManager<PgConnection>>,
}

impl DAL {
    /// Creates a new DAL instance with the provided connection pool.
    ///
    /// # Arguments
    ///
    /// * `pool` - A connection pool for PostgreSQL database connections
    ///
    /// # Returns
    ///
    /// A new DAL instance ready for database operations.
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        DAL { pool }
    }

    /// Executes a closure within a database transaction.
    ///
    /// This method provides transaction support for grouping multiple database
    /// operations into a single atomic unit. If any operation fails, the entire
    /// transaction is rolled back.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that receives a connection and performs database operations
    ///
    /// # Returns
    ///
    /// The result of the closure execution, or an error if the transaction fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::dal::DAL;
    /// # use diesel::r2d2::{Pool, ConnectionManager};
    /// # use diesel::pg::PgConnection;
    /// # fn example(dal: &DAL) -> Result<(), crate::error::ValidationError> {
    /// dal.transaction(|conn| {
    ///     // Multiple operations in a single transaction
    ///     // If any fail, all are rolled back
    ///     Ok(())
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn transaction<T, F>(&self, f: F) -> Result<T, crate::error::ValidationError>
    where
        F: FnOnce(
            &mut PooledConnection<ConnectionManager<PgConnection>>,
        ) -> Result<T, crate::error::ValidationError>,
    {
        use diesel::connection::Connection;

        let mut conn = self.pool.get()?;
        conn.transaction(f)
    }

    /// Returns a ContextDAL instance for context-related database operations.
    ///
    /// # Returns
    ///
    /// A ContextDAL instance that provides methods for context persistence,
    /// retrieval, and management.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cloacina::dal::DAL;
    /// # use diesel::r2d2::{Pool, ConnectionManager};
    /// # use diesel::pg::PgConnection;
    /// # fn example(dal: &DAL) -> Result<(), Box<dyn std::error::Error>> {
    /// let context_dal = dal.context();
    /// let contexts = context_dal.list()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn context(&self) -> ContextDAL {
        ContextDAL { dal: self }
    }

    pub fn pipeline_execution(&self) -> PipelineExecutionDAL {
        PipelineExecutionDAL { dal: self }
    }

    pub fn task_execution(&self) -> TaskExecutionDAL {
        TaskExecutionDAL { dal: self }
    }

    pub fn task_execution_metadata(&self) -> TaskExecutionMetadataDAL {
        TaskExecutionMetadataDAL { dal: self }
    }

    pub fn recovery_event(&self) -> RecoveryEventDAL {
        RecoveryEventDAL { dal: self }
    }

    pub fn cron_schedule(&self) -> CronScheduleDAL {
        CronScheduleDAL { dal: self }
    }

    /// Returns a CronExecutionDAL for cron execution audit operations
    pub fn cron_execution(&self) -> CronExecutionDAL {
        CronExecutionDAL { dal: self }
    }
}
