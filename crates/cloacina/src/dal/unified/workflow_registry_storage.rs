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

//! Unified workflow registry storage with runtime backend selection
//!
//! This module provides binary storage operations that work with both
//! PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.

use async_trait::async_trait;
use diesel::prelude::*;
use uuid::Uuid;

use super::models::{NewUnifiedWorkflowRegistryEntry, UnifiedWorkflowRegistryEntry};
use crate::database::schema::unified::workflow_registry;
use crate::database::universal_types::{UniversalBinary, UniversalTimestamp, UniversalUuid};
use crate::database::Database;
use crate::models::workflow_packages::StorageType;
use crate::registry::error::StorageError;
use crate::registry::traits::RegistryStorage;

/// Unified registry storage that works with both PostgreSQL and SQLite.
#[derive(Debug, Clone)]
pub struct UnifiedRegistryStorage {
    database: Database,
}

impl UnifiedRegistryStorage {
    /// Creates a new UnifiedRegistryStorage instance.
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    /// Returns a reference to the underlying database.
    pub fn database(&self) -> &Database {
        &self.database
    }
}

#[async_trait]
impl RegistryStorage for UnifiedRegistryStorage {
    async fn store_binary(&mut self, data: Vec<u8>) -> Result<String, StorageError> {
        crate::dispatch_backend!(
            self.database.backend(),
            self.store_binary_postgres(data).await,
            self.store_binary_sqlite(data).await
        )
    }

    async fn retrieve_binary(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        crate::dispatch_backend!(
            self.database.backend(),
            self.retrieve_binary_postgres(id).await,
            self.retrieve_binary_sqlite(id).await
        )
    }

    async fn delete_binary(&mut self, id: &str) -> Result<(), StorageError> {
        crate::dispatch_backend!(
            self.database.backend(),
            self.delete_binary_postgres(id).await,
            self.delete_binary_sqlite(id).await
        )
    }

    fn storage_type(&self) -> StorageType {
        StorageType::Database
    }
}

impl UnifiedRegistryStorage {
    #[cfg(feature = "postgres")]
    async fn store_binary_postgres(&self, data: Vec<u8>) -> Result<String, StorageError> {
        let conn = self.database.get_postgres_connection().await.map_err(|e| {
            StorageError::Backend(format!("Failed to get database connection: {}", e))
        })?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_entry = NewUnifiedWorkflowRegistryEntry {
            id,
            created_at: now,
            data: UniversalBinary::from(data),
        };

        conn.interact(move |conn| {
            diesel::insert_into(workflow_registry::table)
                .values(&new_entry)
                .execute(conn)
        })
        .await
        .map_err(|e| StorageError::Backend(format!("Database interaction error: {}", e)))?
        .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        Ok(id.0.to_string())
    }

    #[cfg(feature = "sqlite")]
    async fn store_binary_sqlite(&self, data: Vec<u8>) -> Result<String, StorageError> {
        let conn = self
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to get connection: {}", e)))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_entry = NewUnifiedWorkflowRegistryEntry {
            id,
            created_at: now,
            data: UniversalBinary::from(data),
        };

        conn.interact(move |conn| {
            diesel::insert_into(workflow_registry::table)
                .values(&new_entry)
                .execute(conn)
        })
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?
        .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        Ok(id.0.to_string())
    }

    #[cfg(feature = "postgres")]
    async fn retrieve_binary_postgres(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let registry_uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;

        let conn = self.database.get_postgres_connection().await.map_err(|e| {
            StorageError::Backend(format!("Failed to get database connection: {}", e))
        })?;

        let registry_id = UniversalUuid(registry_uuid);
        let entry: Option<UnifiedWorkflowRegistryEntry> = conn
            .interact(move |conn| {
                workflow_registry::table
                    .filter(workflow_registry::id.eq(registry_id))
                    .first::<UnifiedWorkflowRegistryEntry>(conn)
                    .optional()
            })
            .await
            .map_err(|e| StorageError::Backend(format!("Database interaction error: {}", e)))?
            .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        Ok(entry.map(|e| e.data.into_inner()))
    }

    #[cfg(feature = "sqlite")]
    async fn retrieve_binary_sqlite(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;

        let conn = self
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to get connection: {}", e)))?;

        let registry_id = UniversalUuid(uuid);
        let result: Result<Option<UnifiedWorkflowRegistryEntry>, diesel::result::Error> = conn
            .interact(move |conn| {
                workflow_registry::table
                    .filter(workflow_registry::id.eq(registry_id))
                    .first::<UnifiedWorkflowRegistryEntry>(conn)
                    .optional()
            })
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        match result {
            Ok(Some(entry)) => Ok(Some(entry.data.into_inner())),
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Backend(format!("Database error: {}", e))),
        }
    }

    #[cfg(feature = "postgres")]
    async fn delete_binary_postgres(&self, id: &str) -> Result<(), StorageError> {
        let registry_uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;

        let conn = self.database.get_postgres_connection().await.map_err(|e| {
            StorageError::Backend(format!("Failed to get database connection: {}", e))
        })?;

        let registry_id = UniversalUuid(registry_uuid);
        conn.interact(move |conn| {
            diesel::delete(workflow_registry::table.filter(workflow_registry::id.eq(registry_id)))
                .execute(conn)
        })
        .await
        .map_err(|e| StorageError::Backend(format!("Database interaction error: {}", e)))?
        .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn delete_binary_sqlite(&self, id: &str) -> Result<(), StorageError> {
        let uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;

        let conn = self
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to get connection: {}", e)))?;

        let registry_id = UniversalUuid(uuid);
        conn.interact(move |conn| {
            diesel::delete(workflow_registry::table.filter(workflow_registry::id.eq(registry_id)))
                .execute(conn)
        })
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?
        .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        // Idempotent - success even if no rows deleted
        Ok(())
    }
}
