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
    pub tenant_id: Option<String>,
    pub is_admin: bool,
    /// CLOACI-T-0792: minted-key expiry (`None` for manual keys = no expiry).
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// CLOACI-T-0792: minted-key provenance (e.g. `oidc:<issuer>:<sub>` /
    /// `local:<account_id>`; `None` for manual keys).
    pub issued_via: Option<String>,
}

/// The role of the key of an execution agent.
pub const AGENT_ROLE: &str = "agent";

/// Refuse a key of role `agent` with no tenant (CLOACINA-T-1118). An agent
/// registers in the tenant of its key, and an agent with no tenant serves no
/// tenant's work. [`ApiKeyDAL::create_key`] and [`ApiKeyDAL::mint_key`], the
/// only code that writes an `api_keys` row, call it, so no route can make
/// such a key.
pub fn check_key_scope(tenant_id: Option<&str>, role: &str) -> Result<(), ValidationError> {
    if role == AGENT_ROLE && tenant_id.is_none() {
        return Err(ValidationError::ExecutionFailed {
            message: "a key of role `agent` needs a tenant".to_string(),
        });
    }
    Ok(())
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
        tenant_id: Option<&str>,
        is_admin: bool,
        role: &str,
    ) -> Result<ApiKeyInfo, ValidationError> {
        crud::create_key(self.dal, key_hash, name, tenant_id, is_admin, role, None).await
    }

    /// [`Self::create_key`] with an expiry (CLOACINA-T-1083). `None`: the key
    /// does not expire. After `expires_at` the key is refused.
    #[cfg(feature = "postgres")]
    pub async fn create_key_expiring(
        &self,
        key_hash: &str,
        name: &str,
        tenant_id: Option<&str>,
        is_admin: bool,
        role: &str,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<ApiKeyInfo, ValidationError> {
        crud::create_key(
            self.dal, key_hash, name, tenant_id, is_admin, role, expires_at,
        )
        .await
    }

    /// True when the hash is of a key that is not revoked and whose expiry
    /// has passed (CLOACINA-T-1083). Asked only after [`Self::validate_hash`]
    /// refused the hash, to answer `key_expired` in place of `unauthorized`.
    #[cfg(feature = "postgres")]
    pub async fn is_expired_hash(&self, key_hash: &str) -> Result<bool, ValidationError> {
        crud::is_expired_hash(self.dal, key_hash).await
    }

    /// CLOACI-T-0792: mint a short-TTL, provenance-tagged key (never god-mode).
    #[cfg(feature = "postgres")]
    #[allow(clippy::too_many_arguments)]
    pub async fn mint_key(
        &self,
        key_hash: &str,
        name: &str,
        tenant_id: Option<&str>,
        role: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
        issued_via: &str,
    ) -> Result<ApiKeyInfo, ValidationError> {
        crud::mint_key(
            self.dal, key_hash, name, tenant_id, role, expires_at, issued_via,
        )
        .await
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

    /// CLOACI-T-0784: list keys scoped to a single tenant (tenant-admin view).
    #[cfg(feature = "postgres")]
    pub async fn list_keys_for_tenant(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<ApiKeyInfo>, ValidationError> {
        crud::list_keys_for_tenant(self.dal, tenant_id).await
    }

    /// CLOACI-T-0784: fetch one key's info by id (for revoke ownership checks).
    #[cfg(feature = "postgres")]
    pub async fn get_key(&self, id: uuid::Uuid) -> Result<Option<ApiKeyInfo>, ValidationError> {
        crud::get_key(self.dal, id).await
    }

    /// CLOACI-T-0581: bulk-revoke every still-active key bound to the
    /// tenant. Returns the number of rows updated.
    #[cfg(feature = "postgres")]
    pub async fn revoke_keys_for_tenant(&self, tenant_id: &str) -> Result<usize, ValidationError> {
        crud::revoke_keys_for_tenant(self.dal, tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1118_an_agent_key_needs_a_tenant() {
        assert!(check_key_scope(None, AGENT_ROLE).is_err());
        assert!(check_key_scope(Some("acme"), AGENT_ROLE).is_ok());
        for role in ["read", "write", "admin"] {
            assert!(check_key_scope(None, role).is_ok(), "{role}");
            assert!(check_key_scope(Some("acme"), role).is_ok(), "{role}");
        }
    }
}
