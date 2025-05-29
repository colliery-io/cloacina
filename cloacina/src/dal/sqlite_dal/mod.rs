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

use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sqlite::SqliteConnection;

pub mod context;
pub mod pipeline_execution;
pub mod recovery_event;
pub mod task_execution;
pub mod task_execution_metadata;
use context::ContextDAL;
use pipeline_execution::PipelineExecutionDAL;
use recovery_event::RecoveryEventDAL;
use task_execution::TaskExecutionDAL;
use task_execution_metadata::TaskExecutionMetadataDAL;

/// The main Data Access Layer struct for SQLite.
#[derive(Clone)]
pub struct DAL {
    /// A connection pool for SQLite database connections.
    pub pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl DAL {
    /// Creates a new DAL instance with the provided connection pool.
    pub fn new(pool: Pool<ConnectionManager<SqliteConnection>>) -> Self {
        DAL { pool }
    }

    /// Executes a closure within a database transaction.
    pub fn transaction<T, F>(&self, f: F) -> Result<T, crate::error::ValidationError>
    where
        F: FnOnce(
            &mut PooledConnection<ConnectionManager<SqliteConnection>>,
        ) -> Result<T, crate::error::ValidationError>,
    {
        use diesel::connection::Connection;

        let mut conn = self.pool.get()?;
        conn.transaction(f)
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
}
