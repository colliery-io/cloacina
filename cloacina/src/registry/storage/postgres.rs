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

//! PostgreSQL storage backend for workflow registry.
//!
//! This implementation stores binary workflow data directly in the PostgreSQL
//! database using BYTEA columns. It provides ACID guarantees and leverages
//! database-level integrity constraints.

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::registry::error::StorageError;
use crate::registry::traits::RegistryStorage;

/// PostgreSQL-based storage backend for workflow registry.
///
/// This storage backend uses the `workflow_registry` table to store binary
/// workflow data alongside generated UUIDs. All operations are atomic and
/// benefit from PostgreSQL's ACID properties.
///
/// # Example
///
/// ```rust,no_run
/// use cloacina::registry::storage::PostgresRegistryStorage;
/// use cloacina::registry::RegistryStorage;
/// use sqlx::PgPool;
///
/// # async fn example(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
/// let mut storage = PostgresRegistryStorage::new(pool);
///
/// // Store binary workflow data
/// let workflow_data = std::fs::read("my_workflow.so")?;
/// let id = storage.store_binary(workflow_data).await?;
///
/// // Retrieve it later
/// if let Some(data) = storage.retrieve_binary(&id).await? {
///     println!("Retrieved {} bytes", data.len());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct PostgresRegistryStorage {
    pool: PgPool,
}

impl PostgresRegistryStorage {
    /// Create a new PostgreSQL registry storage backend.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool for PostgreSQL
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cloacina::registry::storage::PostgresRegistryStorage;
    /// use sqlx::PgPool;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let database_url = "postgresql://user:pass@localhost/cloacina";
    /// let pool = PgPool::connect(database_url).await?;
    /// let storage = PostgresRegistryStorage::new(pool);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a reference to the underlying database pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl RegistryStorage for PostgresRegistryStorage {
    async fn store_binary(&mut self, data: Vec<u8>) -> Result<String, StorageError> {
        let id = Uuid::new_v4();

        let result = sqlx::query!(
            r#"
            INSERT INTO workflow_registry (id, data, created_at)
            VALUES ($1, $2, CURRENT_TIMESTAMP)
            "#,
            id,
            data
        )
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(id.to_string()),
            Err(sqlx::Error::Database(db_err)) if db_err.constraint().is_some() => Err(
                StorageError::Backend(format!("Constraint violation: {}", db_err.message())),
            ),
            Err(e) => Err(StorageError::Postgres(e)),
        }
    }

    async fn retrieve_binary(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;

        let result = sqlx::query!(
            r#"
            SELECT data
            FROM workflow_registry
            WHERE id = $1
            "#,
            uuid
        )
        .fetch_optional(&self.pool)
        .await;

        match result {
            Ok(Some(row)) => Ok(Some(row.data)),
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Postgres(e)),
        }
    }

    async fn delete_binary(&mut self, id: &str) -> Result<(), StorageError> {
        let uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;

        let result = sqlx::query!(
            r#"
            DELETE FROM workflow_registry
            WHERE id = $1
            "#,
            uuid
        )
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()), // Idempotent - success even if no rows deleted
            Err(e) => Err(StorageError::Postgres(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    // Helper to create test storage (requires running PostgreSQL)
    async fn create_test_storage() -> Result<PostgresRegistryStorage, sqlx::Error> {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://postgres:password@localhost/cloacina_test".to_string()
        });
        let pool = PgPool::connect(&database_url).await?;
        Ok(PostgresRegistryStorage::new(pool))
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL database
    async fn test_store_and_retrieve() {
        let mut storage = create_test_storage().await.unwrap();

        let test_data = b"test workflow binary data".to_vec();
        let id = storage.store_binary(test_data.clone()).await.unwrap();

        let retrieved = storage.retrieve_binary(&id).await.unwrap();
        assert_eq!(retrieved, Some(test_data));
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL database
    async fn test_retrieve_nonexistent() {
        let storage = create_test_storage().await.unwrap();
        let fake_id = Uuid::new_v4().to_string();

        let result = storage.retrieve_binary(&fake_id).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL database
    async fn test_delete_binary() {
        let mut storage = create_test_storage().await.unwrap();

        let test_data = b"test data for deletion".to_vec();
        let id = storage.store_binary(test_data).await.unwrap();

        // Verify it exists
        let retrieved = storage.retrieve_binary(&id).await.unwrap();
        assert!(retrieved.is_some());

        // Delete it
        storage.delete_binary(&id).await.unwrap();

        // Verify it's gone
        let retrieved = storage.retrieve_binary(&id).await.unwrap();
        assert_eq!(retrieved, None);

        // Verify idempotent deletion
        storage.delete_binary(&id).await.unwrap();
    }

    #[tokio::test]
    async fn test_invalid_uuid() {
        let storage = create_test_storage().await.unwrap();

        let result = storage.retrieve_binary("not-a-uuid").await;
        assert!(matches!(result, Err(StorageError::InvalidId { .. })));
    }
}
