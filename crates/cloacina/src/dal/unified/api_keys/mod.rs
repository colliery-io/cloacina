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

//! API key DAL — Postgres only.
//!
//! The server uses API keys for authentication. Keys are stored as SHA-256
//! hashes — plaintext is never persisted. This module provides CRUD operations
//! for the `api_keys` table.

#[cfg(feature = "postgres")]
mod crud;

use super::DAL;
use crate::error::ValidationError;

/// Information about an API key (never includes the hash).
#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub id: uuid::Uuid,
    pub name: String,
    pub permissions: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub revoked: bool,
}

/// DAL for API key operations. Postgres only.
#[derive(Clone)]
pub struct ApiKeyDAL<'a> {
    dal: &'a DAL,
}

impl<'a> ApiKeyDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Create a new API key record.
    #[cfg(feature = "postgres")]
    pub async fn create_key(
        &self,
        key_hash: &str,
        name: &str,
    ) -> Result<ApiKeyInfo, ValidationError> {
        crud::create_key(self.dal, key_hash, name).await
    }

    /// Validate a key hash — returns key info if found and not revoked.
    #[cfg(feature = "postgres")]
    pub async fn validate_hash(
        &self,
        key_hash: &str,
    ) -> Result<Option<ApiKeyInfo>, ValidationError> {
        crud::validate_hash(self.dal, key_hash).await
    }

    /// Check if any non-revoked API keys exist.
    #[cfg(feature = "postgres")]
    pub async fn has_any_keys(&self) -> Result<bool, ValidationError> {
        crud::has_any_keys(self.dal).await
    }

    /// List all API keys (no hashes).
    #[cfg(feature = "postgres")]
    pub async fn list_keys(&self) -> Result<Vec<ApiKeyInfo>, ValidationError> {
        crud::list_keys(self.dal).await
    }

    /// Soft-revoke a key. Returns true if found and revoked.
    #[cfg(feature = "postgres")]
    pub async fn revoke_key(&self, id: uuid::Uuid) -> Result<bool, ValidationError> {
        crud::revoke_key(self.dal, id).await
    }
}
