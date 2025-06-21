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

//! PostgreSQL DAL for workflow registry storage operations.
//!
//! This module provides PostgreSQL-specific data access operations for workflow
//! registry binary data storage, following the established DAL patterns.

use async_trait::async_trait;
use diesel::prelude::*;
use uuid::Uuid;

use crate::database::schema::workflow_registry;
use crate::database::Database;
use crate::models::workflow_registry::{NewWorkflowRegistryEntry, WorkflowRegistryEntry};
use crate::registry::error::StorageError;
use crate::registry::traits::RegistryStorage;

/// PostgreSQL-based DAL for workflow registry storage operations.
///
/// This DAL implementation uses the `workflow_registry` table to store binary
/// workflow data alongside generated UUIDs. All operations are atomic and
/// benefit from PostgreSQL's ACID properties.
///
/// # Example
///
/// ```rust,no_run
/// use cloacina::dal::postgres_dal::PostgresWorkflowRegistryDAL;
/// use cloacina::registry::RegistryStorage;
/// use cloacina::database::Database;
///
/// # async fn example(database: Database) -> Result<(), Box<dyn std::error::Error>> {
/// let mut dal = PostgresWorkflowRegistryDAL::new(database);
///
/// // Store binary workflow data
/// let workflow_data = std::fs::read("my_workflow.so")?;
/// let id = dal.store_binary(workflow_data).await?;
///
/// // Retrieve it later
/// if let Some(data) = dal.retrieve_binary(&id).await? {
///     println!("Retrieved {} bytes", data.len());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct PostgresWorkflowRegistryDAL {
    database: Database,
}

impl PostgresWorkflowRegistryDAL {
    /// Create a new PostgreSQL workflow registry DAL.
    ///
    /// # Arguments
    ///
    /// * `database` - Database instance for PostgreSQL operations
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cloacina::dal::postgres_dal::PostgresWorkflowRegistryDAL;
    /// use cloacina::database::Database;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let database = Database::new("postgresql://user:pass@localhost", "cloacina", 5);
    /// let dal = PostgresWorkflowRegistryDAL::new(database);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    /// Get a reference to the underlying database.
    pub fn database(&self) -> &Database {
        &self.database
    }
}

#[async_trait]
impl RegistryStorage for PostgresWorkflowRegistryDAL {
    async fn store_binary(&mut self, data: Vec<u8>) -> Result<String, StorageError> {
        let conn = self
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let new_entry = NewWorkflowRegistryEntry::new(data);

        let entry: WorkflowRegistryEntry = conn
            .interact(move |conn| {
                diesel::insert_into(workflow_registry::table)
                    .values(&new_entry)
                    .get_result(conn)
            })
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
            .map_err(|e| match e {
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    info,
                ) => StorageError::Backend(format!("Constraint violation: {}", info.message())),
                _ => StorageError::Backend(format!("Database error: {}", e)),
            })?;

        Ok(entry.id.to_string())
    }

    async fn retrieve_binary(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;

        let conn = self
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let uuid_param = crate::database::universal_types::UniversalUuid::from(uuid);
        let result: Result<Option<WorkflowRegistryEntry>, _> = conn
            .interact(move |conn| {
                workflow_registry::table
                    .filter(workflow_registry::id.eq(uuid_param))
                    .first::<WorkflowRegistryEntry>(conn)
                    .optional()
            })
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
            .map_err(|e| StorageError::Backend(format!("Database error: {}", e)));

        match result {
            Ok(Some(entry)) => Ok(Some(entry.data)),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn delete_binary(&mut self, id: &str) -> Result<(), StorageError> {
        let uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;

        let conn = self
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let uuid_param = crate::database::universal_types::UniversalUuid::from(uuid);
        let _rows_affected = conn
            .interact(move |conn| {
                diesel::delete(
                    workflow_registry::table.filter(workflow_registry::id.eq(uuid_param)),
                )
                .execute(conn)
            })
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
            .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        // Idempotent - success even if no rows deleted
        Ok(())
    }
}
