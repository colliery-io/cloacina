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

use crate::database::{AnyPool, BackendType, Database};

// Sub-modules for each entity type
#[cfg(feature = "postgres")]
pub mod api_keys;
pub mod checkpoint;
pub mod context;
pub mod execution_event;
pub mod models;
pub mod pipeline_execution;
pub mod recovery_event;
pub mod schedule;
pub mod schedule_execution;
pub mod task_execution;
pub mod task_execution_metadata;
pub mod task_outbox;
pub mod workflow_packages;
pub mod workflow_registry;
pub mod workflow_registry_storage;

// Re-export DAL components
#[cfg(feature = "postgres")]
pub use api_keys::{ApiKeyDAL, ApiKeyInfo};
pub use checkpoint::CheckpointDAL;
pub use context::ContextDAL;
pub use execution_event::ExecutionEventDAL;
pub use pipeline_execution::WorkflowExecutionDAL;
pub use recovery_event::RecoveryEventDAL;
pub use schedule::ScheduleDAL;
pub use schedule_execution::{ScheduleExecutionDAL, ScheduleExecutionStats};
pub use task_execution::{ClaimResult, RetryStats, TaskExecutionDAL};
pub use task_execution_metadata::TaskExecutionMetadataDAL;
pub use task_outbox::TaskOutboxDAL;
pub use workflow_packages::WorkflowPackagesDAL;
pub use workflow_registry::WorkflowRegistryDAL;
pub use workflow_registry_storage::UnifiedRegistryStorage;

/// Helper macro for dispatching operations based on backend type.
///
/// This macro simplifies writing code that needs to execute different
/// implementations based on the database backend.
///
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

    /// Returns an API key DAL (Postgres only).
    #[cfg(feature = "postgres")]
    pub fn api_keys(&self) -> ApiKeyDAL<'_> {
        ApiKeyDAL::new(self)
    }

    /// Returns a checkpoint DAL for computation graph state persistence.
    pub fn checkpoint(&self) -> CheckpointDAL<'_> {
        CheckpointDAL::new(self)
    }

    /// Returns a context DAL for context operations.
    pub fn context(&self) -> ContextDAL<'_> {
        ContextDAL::new(self)
    }

    /// Returns a workflow execution DAL for workflow execution operations.
    pub fn workflow_execution(&self) -> WorkflowExecutionDAL<'_> {
        WorkflowExecutionDAL::new(self)
    }

    /// Returns a task execution DAL for task operations.
    pub fn task_execution(&self) -> TaskExecutionDAL<'_> {
        TaskExecutionDAL::new(self)
    }

    /// Returns a task execution metadata DAL for metadata operations.
    pub fn task_execution_metadata(&self) -> TaskExecutionMetadataDAL<'_> {
        TaskExecutionMetadataDAL::new(self)
    }

    /// Returns a task outbox DAL for work distribution operations.
    pub fn task_outbox(&self) -> TaskOutboxDAL<'_> {
        TaskOutboxDAL::new(self)
    }

    /// Returns a recovery event DAL for recovery operations.
    pub fn recovery_event(&self) -> RecoveryEventDAL<'_> {
        RecoveryEventDAL::new(self)
    }

    /// Returns an execution event DAL for execution event operations.
    pub fn execution_event(&self) -> ExecutionEventDAL<'_> {
        ExecutionEventDAL::new(self)
    }

    /// Returns a unified schedule DAL for schedule operations.
    pub fn schedule(&self) -> ScheduleDAL<'_> {
        ScheduleDAL::new(self)
    }

    /// Returns a unified schedule execution DAL for schedule execution operations.
    pub fn schedule_execution(&self) -> ScheduleExecutionDAL<'_> {
        ScheduleExecutionDAL::new(self)
    }

    /// Returns a workflow packages DAL for package operations.
    pub fn workflow_packages(&self) -> WorkflowPackagesDAL<'_> {
        WorkflowPackagesDAL::new(self)
    }

    /// Creates a workflow registry implementation with the given storage backend.
    ///
    /// # Arguments
    ///
    /// * `storage` - A storage backend implementing `RegistryStorage`
    ///
    /// # Panics
    ///
    /// Panics if the workflow registry cannot be created.
    /// Use [`try_workflow_registry`](Self::try_workflow_registry) for fallible construction.
    pub fn workflow_registry<S: crate::registry::traits::RegistryStorage + 'static>(
        &self,
        storage: S,
    ) -> crate::registry::workflow_registry::WorkflowRegistryImpl<S> {
        self.try_workflow_registry(storage)
            .expect("Failed to create workflow registry")
    }

    /// Creates a workflow registry implementation with the given storage backend.
    ///
    /// This is the fallible version of [`workflow_registry`](Self::workflow_registry).
    ///
    /// # Arguments
    ///
    /// * `storage` - A storage backend implementing `RegistryStorage`
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow registry cannot be initialized.
    pub fn try_workflow_registry<S: crate::registry::traits::RegistryStorage + 'static>(
        &self,
        storage: S,
    ) -> Result<
        crate::registry::workflow_registry::WorkflowRegistryImpl<S>,
        crate::registry::error::RegistryError,
    > {
        crate::registry::workflow_registry::WorkflowRegistryImpl::new(
            storage,
            self.database.clone(),
        )
    }
}
