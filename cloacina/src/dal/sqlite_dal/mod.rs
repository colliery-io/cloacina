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

use deadpool_diesel::sqlite::Pool;
use diesel::sqlite::SqliteConnection;

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

/// The main Data Access Layer struct for SQLite.
#[derive(Clone)]
pub struct DAL {
    /// A connection pool for SQLite database connections.
    pub pool: Pool,
}

impl DAL {
    /// Creates a new DAL instance with the provided connection pool.
    pub fn new(pool: Pool) -> Self {
        DAL { pool }
    }

    /// Executes a closure within a database transaction.
    pub async fn transaction<T, F>(&self, f: F) -> Result<T, crate::error::ValidationError>
    where
        F: FnOnce(&mut SqliteConnection) -> Result<T, crate::error::ValidationError> + Send + 'static,
        T: Send + 'static,
    {
        use diesel::connection::Connection;
        let conn = self.pool.get().await?;
        conn.interact(move |conn| conn.transaction(f)).await.map_err(|e| crate::error::ValidationError::ConnectionPool(e.to_string()))?
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
}
