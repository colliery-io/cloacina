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

//! SQLite Data Access Layer
//!
//! This module provides the SQLite-specific implementation of the data access layer.
//! It uses universal wrapper types to handle SQLite-specific storage requirements.

use crate::database::Database;
use deadpool_diesel::sqlite::Pool;
use diesel::sqlite::SqliteConnection;

pub mod context;
pub mod cron_execution;
pub mod models;
pub mod cron_schedule;
pub mod pipeline_execution;
pub mod recovery_event;
pub mod task_execution;
pub mod task_execution_metadata;
pub mod workflow_packages;
pub mod workflow_registry;
pub mod workflow_registry_storage;
use context::ContextDAL;
use cron_execution::CronExecutionDAL;
use cron_schedule::CronScheduleDAL;
use pipeline_execution::PipelineExecutionDAL;
use recovery_event::RecoveryEventDAL;
use task_execution::TaskExecutionDAL;
use task_execution_metadata::TaskExecutionMetadataDAL;
use workflow_packages::WorkflowPackagesDAL;
use workflow_registry::WorkflowRegistryDAL;
// Re-export with specific name to avoid ambiguity
pub use workflow_registry_storage::SqliteRegistryStorage;

// Re-export for public API
pub use cron_execution::CronExecutionStats;

/// The main Data Access Layer struct for SQLite.
#[derive(Clone)]
pub struct DAL {
    /// A connection pool for SQLite database connections.
    pub pool: Pool,
    /// The Database instance for consistency with PostgreSQL
    pub database: Database,
}

impl DAL {
    /// Creates a new DAL instance with the provided database.
    ///
    /// # Panics
    ///
    /// Panics if the database is not configured for SQLite backend.
    pub fn new(database: Database) -> Self {
        let pool = database.pool().expect_sqlite().clone();
        DAL { pool, database }
    }

    /// Executes a closure within a database transaction.
    pub async fn transaction<T, F>(&self, f: F) -> Result<T, crate::error::ValidationError>
    where
        F: FnOnce(&mut SqliteConnection) -> Result<T, crate::error::ValidationError>
            + Send
            + 'static,
        T: Send + 'static,
    {
        use diesel::connection::Connection;
        let conn = self.pool.get().await?;
        conn.interact(move |conn| conn.transaction(f))
            .await
            .map_err(|e| crate::error::ValidationError::ConnectionPool(e.to_string()))?
    }

    /// Returns a ContextDAL instance for context-related database operations.
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

    /// Returns a WorkflowPackagesDAL for workflow package metadata operations
    pub fn workflow_packages(&self) -> WorkflowPackagesDAL {
        WorkflowPackagesDAL { dal: self }
    }

    /// Returns a WorkflowRegistryDAL for complete workflow package management
    ///
    /// # Arguments
    ///
    /// * `storage` - Storage backend for binary package data
    pub fn workflow_registry(
        &self,
        storage: std::sync::Arc<dyn crate::registry::traits::RegistryStorage + Send + Sync>,
    ) -> WorkflowRegistryDAL {
        WorkflowRegistryDAL::new(self.clone(), storage)
    }
}
