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

//! Unified Data Access Layer with runtime backend selection
//!
//! This module provides a unified DAL implementation that works with both
//! PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.
//!
//! # Architecture
//!
//! The unified DAL uses Diesel's `MultiConnection` feature to support
//! runtime backend selection. Each DAL operation dispatches to the
//! appropriate backend-specific implementation based on the connection type.
//!
//! # Example
//!
//! ```rust,ignore
//! use cloacina::dal::unified::DAL;
//! use cloacina::database::Database;
//!
//! // Create database with runtime backend detection
//! let db = Database::new("postgres://localhost/mydb", "mydb", 10);
//! let dal = DAL::new(db);
//!
//! // Operations automatically use the correct backend
//! let contexts = dal.context().list().await?;
//! ```

use crate::database::{AnyConnection, AnyPool, BackendType, Database};

// Sub-modules for each entity type
pub mod context;
pub mod cron_execution;
pub mod cron_schedule;
pub mod pipeline_execution;
pub mod recovery_event;
pub mod task_execution;
pub mod task_execution_metadata;
pub mod workflow_packages;
pub mod workflow_registry;
pub mod workflow_registry_storage;

// Re-export DAL components
pub use context::ContextDAL;
pub use cron_execution::CronExecutionDAL;
pub use cron_schedule::CronScheduleDAL;
pub use pipeline_execution::PipelineExecutionDAL;
pub use recovery_event::RecoveryEventDAL;
pub use task_execution::TaskExecutionDAL;
pub use task_execution_metadata::TaskExecutionMetadataDAL;
pub use workflow_packages::WorkflowPackagesDAL;
pub use workflow_registry::WorkflowRegistryDAL;
pub use workflow_registry_storage::UnifiedRegistryStorage;

/// Helper macro for dispatching operations based on backend type.
///
/// This macro simplifies writing code that needs to execute different
/// implementations based on the database backend.
///
/// # Example
///
/// ```rust,ignore
/// backend_dispatch!(self.database.backend(), {
///     // PostgreSQL implementation
///     postgres_specific_operation()
/// }, {
///     // SQLite implementation
///     sqlite_specific_operation()
/// })
/// ```
#[macro_export]
macro_rules! backend_dispatch {
    ($backend:expr, $pg_block:block, $sqlite_block:block) => {
        match $backend {
            #[cfg(feature = "postgres")]
            $crate::database::BackendType::Postgres => $pg_block,
            #[cfg(feature = "sqlite")]
            $crate::database::BackendType::Sqlite => $sqlite_block,
        }
    };
}

/// Helper macro for matching on AnyConnection variants.
///
/// This macro simplifies pattern matching on connection types when
/// executing backend-specific queries.
///
/// # Example
///
/// ```rust,ignore
/// connection_match!(conn, pg_conn => {
///     // Use pg_conn for PostgreSQL operations
///     diesel::select(1).get_result::<i32>(pg_conn)
/// }, sqlite_conn => {
///     // Use sqlite_conn for SQLite operations
///     diesel::select(1).get_result::<i32>(sqlite_conn)
/// })
/// ```
#[macro_export]
macro_rules! connection_match {
    ($conn:expr, $pg_var:ident => $pg_block:block, $sqlite_var:ident => $sqlite_block:block) => {
        match $conn {
            #[cfg(feature = "postgres")]
            $crate::database::AnyConnection::Postgres($pg_var) => $pg_block,
            #[cfg(feature = "sqlite")]
            $crate::database::AnyConnection::Sqlite($sqlite_var) => $sqlite_block,
        }
    };
}

/// The unified Data Access Layer struct.
///
/// This struct provides access to all database operations through a single
/// interface that works with both PostgreSQL and SQLite backends.
///
/// # Thread Safety
///
/// The `DAL` struct is `Clone` and can be safely shared between threads.
/// Each clone references the same underlying database connection pool.
#[derive(Clone, Debug)]
pub struct DAL {
    /// The database instance with connection pool
    pub database: Database,
}

impl DAL {
    /// Creates a new unified DAL instance.
    ///
    /// # Arguments
    ///
    /// * `database` - A Database instance configured for either PostgreSQL or SQLite
    ///
    /// # Returns
    ///
    /// A new DAL instance ready for database operations.
    pub fn new(database: Database) -> Self {
        DAL { database }
    }

    /// Returns the backend type for this DAL instance.
    pub fn backend(&self) -> BackendType {
        self.database.backend()
    }

    /// Returns a reference to the underlying database.
    pub fn database(&self) -> &Database {
        &self.database
    }

    /// Returns the connection pool.
    pub fn pool(&self) -> AnyPool {
        self.database.pool()
    }

    /// Returns a context DAL for context operations.
    pub fn context(&self) -> ContextDAL {
        ContextDAL::new(self)
    }

    /// Returns a pipeline execution DAL for pipeline operations.
    pub fn pipeline_execution(&self) -> PipelineExecutionDAL {
        PipelineExecutionDAL::new(self)
    }

    /// Returns a task execution DAL for task operations.
    pub fn task_execution(&self) -> TaskExecutionDAL {
        TaskExecutionDAL::new(self)
    }

    /// Returns a task execution metadata DAL for metadata operations.
    pub fn task_execution_metadata(&self) -> TaskExecutionMetadataDAL {
        TaskExecutionMetadataDAL::new(self)
    }

    /// Returns a recovery event DAL for recovery operations.
    pub fn recovery_event(&self) -> RecoveryEventDAL {
        RecoveryEventDAL::new(self)
    }

    /// Returns a cron schedule DAL for schedule operations.
    pub fn cron_schedule(&self) -> CronScheduleDAL {
        CronScheduleDAL::new(self)
    }

    /// Returns a cron execution DAL for cron execution operations.
    pub fn cron_execution(&self) -> CronExecutionDAL {
        CronExecutionDAL::new(self)
    }

    /// Returns a workflow packages DAL for package operations.
    pub fn workflow_packages(&self) -> WorkflowPackagesDAL {
        WorkflowPackagesDAL::new(self)
    }
}
