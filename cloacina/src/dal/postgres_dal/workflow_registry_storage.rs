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

//! PostgreSQL binary storage backend.
//!
//! Pure binary storage - no table management.

use async_trait::async_trait;
use diesel::prelude::*;
use uuid::Uuid;

use crate::database::schema::workflow_registry;
use crate::database::Database;
use crate::models::workflow_registry::{NewWorkflowRegistryEntry, WorkflowRegistryEntry};
use crate::registry::error::StorageError;
use crate::registry::traits::RegistryStorage;

/// PostgreSQL binary storage backend.
#[derive(Debug, Clone)]
pub struct PostgresRegistryStorage {
    database: Database,
}

impl PostgresRegistryStorage {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub fn database(&self) -> &Database {
        &self.database
    }
}

#[async_trait]
impl RegistryStorage for PostgresRegistryStorage {
    async fn store_binary(&mut self, data: Vec<u8>) -> Result<String, StorageError> {
        let conn = self
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| {
                StorageError::Backend(format!("Failed to get database connection: {}", e))
            })?;

        let new_entry = NewWorkflowRegistryEntry::new(data);

        let entry: WorkflowRegistryEntry = conn
            .interact(move |conn| {
                diesel::insert_into(workflow_registry::table)
                    .values(&new_entry)
                    .get_result(conn)
            })
            .await
            .map_err(|e| StorageError::Backend(format!("Database interaction error: {}", e)))?
            .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        Ok(entry.id.to_string())
    }

    async fn retrieve_binary(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let registry_uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;
        let registry_universal_uuid =
            crate::database::universal_types::UniversalUuid::from(registry_uuid);

        let conn = self
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| {
                StorageError::Backend(format!("Failed to get database connection: {}", e))
            })?;

        let entry: Option<WorkflowRegistryEntry> = conn
            .interact(move |conn| {
                workflow_registry::table
                    .filter(workflow_registry::id.eq(&registry_universal_uuid))
                    .first::<WorkflowRegistryEntry>(conn)
                    .optional()
            })
            .await
            .map_err(|e| StorageError::Backend(format!("Database interaction error: {}", e)))?
            .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        Ok(entry.map(|e| e.data))
    }

    async fn delete_binary(&mut self, id: &str) -> Result<(), StorageError> {
        let registry_uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;
        let registry_universal_uuid =
            crate::database::universal_types::UniversalUuid::from(registry_uuid);

        let conn = self
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| {
                StorageError::Backend(format!("Failed to get database connection: {}", e))
            })?;

        conn.interact(move |conn| {
            diesel::delete(
                workflow_registry::table.filter(workflow_registry::id.eq(&registry_universal_uuid)),
            )
            .execute(conn)
        })
        .await
        .map_err(|e| StorageError::Backend(format!("Database interaction error: {}", e)))?
        .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        Ok(())
    }
}
