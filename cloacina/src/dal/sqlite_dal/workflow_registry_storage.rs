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
use chrono::Utc;
use uuid::Uuid;

use crate::database::schema::workflow_registry;
use crate::models::workflow_registry::{
    NewWorkflowRegistryEntry, NewWorkflowRegistryEntryWithId, WorkflowRegistryEntry,
};
use crate::registry::error::StorageError;
use crate::registry::traits::RegistryStorage;
use crate::Database;

/// SQLite-based DAL for workflow registry storage operations.
///
/// This DAL implementation stores binary workflow data directly in the SQLite database
/// using the `workflow_registry` table, enabling foreign key relationships with
/// the `workflow_packages` metadata table.
///
/// # Features
///
/// - **ACID Compliance**: Full transaction support via SQLite
/// - **Foreign Key Support**: Proper referential integrity with metadata
/// - **Efficient Storage**: Binary data stored as BLOB
/// - **Easy Backup**: All data in a single database file
/// - **Simple Deployment**: No external dependencies
///
/// # Examples
///
/// ```rust,no_run
/// use cloacina::dal::sqlite_dal::SqliteWorkflowRegistryDAL;
/// use cloacina::registry::RegistryStorage;
/// use cloacina::Database;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let database = Database::new("sqlite:///var/lib/cloacina/registry.db", "", 5);
/// let mut dal = SqliteWorkflowRegistryDAL::new(database);
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
pub struct SqliteWorkflowRegistryDAL {
    database: Database,
}

impl SqliteWorkflowRegistryDAL {
    /// Create a new SQLite workflow registry DAL.
    ///
    /// # Arguments
    ///
    /// * `database` - Database instance for SQLite operations
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cloacina::dal::sqlite_dal::SqliteWorkflowRegistryDAL;
    /// use cloacina::database::Database;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let database = Database::new("sqlite:///path/to/registry.db", "", 5);
    /// let dal = SqliteWorkflowRegistryDAL::new(database);
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
impl RegistryStorage for SqliteWorkflowRegistryDAL {
    async fn store_binary(&mut self, data: Vec<u8>) -> Result<String, StorageError> {
        let conn = self
            .database
            .pool()
            .get()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let entry_id = crate::database::universal_types::UniversalUuid::new_v4();
        let created_at = crate::database::universal_types::UniversalTimestamp(chrono::Utc::now());

        let new_entry = NewWorkflowRegistryEntryWithId {
            id: entry_id,
            created_at,
            data,
        };

        let entry: WorkflowRegistryEntry = conn
            .interact(move |conn| {
                use diesel::prelude::*;
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
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                    info,
                ) => StorageError::Backend(format!("Foreign key violation: {}", info.message())),
                _ => StorageError::Backend(format!("Database error: {}", e)),
            })?;

        Ok(entry_id.to_string())
    }

    async fn retrieve_binary(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let conn = self
            .database
            .pool()
            .get()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;
        let entry_id = crate::database::universal_types::UniversalUuid::from(uuid);

        let result: Result<Option<WorkflowRegistryEntry>, diesel::result::Error> = conn
            .interact(move |conn| {
                use diesel::prelude::*;
                workflow_registry::table
                    .filter(workflow_registry::id.eq(&entry_id))
                    .first::<WorkflowRegistryEntry>(conn)
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
        let conn = self
            .database
            .pool()
            .get()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;
        let entry_id = crate::database::universal_types::UniversalUuid::from(uuid);

        let rows_affected: usize = conn
            .interact(move |conn| {
                use diesel::prelude::*;
                diesel::delete(workflow_registry::table.filter(workflow_registry::id.eq(&entry_id)))
                    .execute(conn)
            })
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
            .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        if rows_affected == 0 {
            // Silently succeed if the entry doesn't exist (idempotent operation)
            tracing::debug!(
                "Binary data with ID '{}' was already deleted or never existed",
                id
            );
        } else {
            tracing::debug!("Successfully deleted binary data with ID '{}'", id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    async fn create_test_database() -> Database {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path().to_string_lossy();
        let db_url = format!("sqlite://{}?mode=rwc", db_path);

        let database = Database::new(&db_url, "", 5);

        // Run migrations
        let conn = database.pool().get().await.unwrap();
        conn.interact(move |conn| crate::database::run_migrations(conn))
            .await
            .unwrap()
            .unwrap();

        // Keep the temp file alive
        std::mem::forget(temp_file);

        database
    }

    #[tokio::test]
    async fn test_store_and_retrieve_binary() {
        let database = create_test_database().await;
        let mut dal = SqliteWorkflowRegistryDAL::new(database);

        let test_data = b"test workflow binary data".to_vec();

        // Store binary data
        let id = dal.store_binary(test_data.clone()).await.unwrap();
        assert!(!id.is_empty(), "ID should not be empty");

        // Retrieve binary data
        let retrieved = dal.retrieve_binary(&id).await.unwrap();
        assert!(retrieved.is_some(), "Data should be found");
        assert_eq!(
            retrieved.unwrap(),
            test_data,
            "Retrieved data should match original"
        );
    }

    #[tokio::test]
    async fn test_retrieve_nonexistent_binary() {
        let database = create_test_database().await;
        let dal = SqliteWorkflowRegistryDAL::new(database);

        let fake_id = crate::UniversalUuid::new_v4().to_string();
        let result = dal.retrieve_binary(&fake_id).await.unwrap();
        assert!(result.is_none(), "Should return None for nonexistent ID");
    }

    #[tokio::test]
    async fn test_delete_binary() {
        let database = create_test_database().await;
        let mut dal = SqliteWorkflowRegistryDAL::new(database);

        let test_data = b"test data for deletion".to_vec();

        // Store binary data
        let id = dal.store_binary(test_data).await.unwrap();

        // Verify it exists
        let retrieved = dal.retrieve_binary(&id).await.unwrap();
        assert!(retrieved.is_some(), "Data should exist before deletion");

        // Delete it
        dal.delete_binary(&id).await.unwrap();

        // Verify it's gone
        let retrieved_after = dal.retrieve_binary(&id).await.unwrap();
        assert!(
            retrieved_after.is_none(),
            "Data should be gone after deletion"
        );
    }

    #[tokio::test]
    async fn test_delete_nonexistent_binary() {
        let database = create_test_database().await;
        let mut dal = SqliteWorkflowRegistryDAL::new(database);

        let fake_id = crate::UniversalUuid::new_v4().to_string();

        // Should succeed silently (idempotent)
        dal.delete_binary(&fake_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_invalid_id_format() {
        let database = create_test_database().await;
        let dal = SqliteWorkflowRegistryDAL::new(database);

        let invalid_id = "not-a-valid-uuid";
        let result = dal.retrieve_binary(invalid_id).await;

        assert!(result.is_err(), "Should fail with invalid UUID");
        match result.unwrap_err() {
            StorageError::InvalidId { id } => {
                assert_eq!(id, invalid_id);
            }
            other => panic!("Expected InvalidId error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_large_binary_data() {
        let database = create_test_database().await;
        let mut dal = SqliteWorkflowRegistryDAL::new(database);

        // Create 1MB of test data
        let large_data = vec![0xAB; 1024 * 1024];

        let id = dal.store_binary(large_data.clone()).await.unwrap();
        let retrieved = dal.retrieve_binary(&id).await.unwrap();

        assert!(
            retrieved.is_some(),
            "Large data should be stored and retrieved"
        );
        assert_eq!(
            retrieved.unwrap(),
            large_data,
            "Large data should match exactly"
        );
    }

    #[tokio::test]
    async fn test_multiple_store_operations() {
        let database = create_test_database().await;
        let mut dal = SqliteWorkflowRegistryDAL::new(database);

        let mut stored_ids = Vec::new();

        // Store multiple pieces of data
        for i in 0..5 {
            let test_data = format!("test data {}", i).into_bytes();
            let id = dal.store_binary(test_data).await.unwrap();
            stored_ids.push(id);
        }

        // Verify all IDs are unique
        let mut unique_ids = stored_ids.clone();
        unique_ids.sort();
        unique_ids.dedup();
        assert_eq!(
            unique_ids.len(),
            stored_ids.len(),
            "All IDs should be unique"
        );

        // Verify all data can be retrieved
        for (i, id) in stored_ids.iter().enumerate() {
            let retrieved = dal.retrieve_binary(id).await.unwrap();
            assert!(retrieved.is_some(), "Data {} should exist", i);
            let expected = format!("test data {}", i).into_bytes();
            assert_eq!(retrieved.unwrap(), expected, "Data {} should match", i);
        }
    }

    #[tokio::test]
    async fn test_storage_backend_interface() {
        let database = create_test_database().await;
        let mut dal = SqliteWorkflowRegistryDAL::new(database);

        // Test that it properly implements the RegistryStorage trait
        let test_data = b"interface test data".to_vec();

        let id = RegistryStorage::store_binary(&mut dal, test_data.clone())
            .await
            .unwrap();
        let retrieved = RegistryStorage::retrieve_binary(&dal, &id).await.unwrap();
        assert_eq!(retrieved.unwrap(), test_data);

        RegistryStorage::delete_binary(&mut dal, &id).await.unwrap();
        let deleted_check = RegistryStorage::retrieve_binary(&dal, &id).await.unwrap();
        assert!(deleted_check.is_none());
    }
}
