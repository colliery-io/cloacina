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

//! SQLite DAL for workflow registry storage operations.
//!
//! This module provides SQLite-specific data access operations for workflow
//! registry binary data storage, following the established DAL patterns.

use async_trait::async_trait;
use diesel::prelude::*;
use uuid::Uuid;

use super::models::{NewSqliteWorkflowRegistryEntry, SqliteWorkflowRegistryEntry, uuid_to_blob, current_timestamp_string};
use crate::database::schema::sqlite::workflow_registry;
use crate::database::Database;
use crate::models::workflow_packages::StorageType;
use crate::registry::error::StorageError;
use crate::registry::traits::RegistryStorage;

/// SQLite-based DAL for workflow registry storage operations.
///
/// This DAL implementation handles binary workflow data storage in SQLite
/// using the workflow_registry table for pure binary storage.
#[derive(Debug, Clone)]
pub struct SqliteRegistryStorage {
    database: Database,
}

impl SqliteRegistryStorage {
    /// Create a new SQLite registry storage.
    ///
    /// # Arguments
    ///
    /// * `database` - Database instance for SQLite operations
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    /// Get a reference to the underlying database.
    pub fn database(&self) -> &Database {
        &self.database
    }
}

#[async_trait]
impl RegistryStorage for SqliteRegistryStorage {
    async fn store_binary(&mut self, data: Vec<u8>) -> Result<String, StorageError> {
        let conn = self
            .database
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to get connection: {}", e)))?;

        // Generate UUID client-side
        let id = Uuid::new_v4();
        let id_blob = uuid_to_blob(&id);
        let now = current_timestamp_string();

        let new_entry = NewSqliteWorkflowRegistryEntry {
            id: id_blob,
            created_at: now,
            data,
        };

        let _entry: SqliteWorkflowRegistryEntry = conn
            .interact(move |conn| {
                diesel::insert_into(workflow_registry::table)
                    .values(&new_entry)
                    .get_result(conn)
            })
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
            .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        Ok(id.to_string())
    }

    async fn retrieve_binary(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;

        let conn = self
            .database
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to get connection: {}", e)))?;

        let entry_id_blob = uuid_to_blob(&uuid);

        let result: Result<Option<SqliteWorkflowRegistryEntry>, diesel::result::Error> = conn
            .interact(move |conn| {
                workflow_registry::table
                    .filter(workflow_registry::id.eq(&entry_id_blob))
                    .first::<SqliteWorkflowRegistryEntry>(conn)
                    .optional()
            })
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        match result {
            Ok(Some(entry)) => Ok(Some(entry.data)),
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Backend(format!("Database error: {}", e))),
        }
    }

    async fn delete_binary(&mut self, id: &str) -> Result<(), StorageError> {
        let uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;

        let conn = self
            .database
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to get connection: {}", e)))?;

        let entry_id_blob = uuid_to_blob(&uuid);

        let _rows_affected: usize = conn
            .interact(move |conn| {
                diesel::delete(workflow_registry::table.filter(workflow_registry::id.eq(&entry_id_blob)))
                    .execute(conn)
            })
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
            .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        // Idempotent - success even if no rows deleted
        Ok(())
    }

    fn storage_type(&self) -> StorageType {
        StorageType::Database
    }
}
