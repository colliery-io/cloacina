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
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_entry = NewUnifiedWorkflowRegistryEntry {
            id,
            created_at: now,
            data: UniversalBinary::from(data),
        };

        let dal = crate::dal::unified::DAL::new(self.database.clone());
        crate::interact_on_backend!(dal, |conn| {
            diesel::insert_into(workflow_registry::table)
                .values(&new_entry)
                .execute(conn)
        })
        .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        Ok(id.0.to_string())
    }

    async fn retrieve_binary(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let registry_uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;
        let registry_id = UniversalUuid(registry_uuid);

        let dal = crate::dal::unified::DAL::new(self.database.clone());
        let entry: Option<UnifiedWorkflowRegistryEntry> =
            crate::interact_on_backend!(dal, |conn| {
                workflow_registry::table
                    .filter(workflow_registry::id.eq(registry_id))
                    .first::<UnifiedWorkflowRegistryEntry>(conn)
                    .optional()
            })
            .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        Ok(entry.map(|e| e.data.into_inner()))
    }

    async fn delete_binary(&mut self, id: &str) -> Result<(), StorageError> {
        let registry_uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;
        let registry_id = UniversalUuid(registry_uuid);

        let dal = crate::dal::unified::DAL::new(self.database.clone());
        crate::interact_on_backend!(dal, |conn| {
            diesel::delete(workflow_registry::table.filter(workflow_registry::id.eq(registry_id)))
                .execute(conn)
        })
        .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        // Idempotent - success even if no rows deleted
        Ok(())
    }

    fn storage_type(&self) -> StorageType {
        StorageType::Database
    }
}
