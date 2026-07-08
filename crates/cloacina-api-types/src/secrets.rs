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

//! Tenant secrets API types (CLOACI-I-0133 / T-0862).
//!
//! **Metadata-only reads (REQ-002 / NFR-001).** A secret's plaintext field
//! *values* NEVER cross this wire on a read. The only place values appear is the
//! request body of create/rotate ([`CreateSecretRequest`] / [`RotateSecretRequest`]);
//! every response ([`SecretMetadataResponse`]) carries names + timestamps only.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Request body for `POST /v1/tenants/{tenant_id}/secrets` — create a secret.
///
/// `fields` is the `{field_name: value}` map. The values are write-only: they
/// are encrypted at rest and never returned by any read endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateSecretRequest {
    /// The secret's name (unique within the tenant).
    pub name: String,
    /// The `{field: value}` map. Values are write-only.
    pub fields: BTreeMap<String, String>,
}

/// Request body for `PUT|POST /v1/tenants/{tenant_id}/secrets/{name}` — rotate.
///
/// Replaces the secret's field map in place (D-8/OQ-5: in-place, no versioning).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RotateSecretRequest {
    /// The new `{field: value}` map. Values are write-only.
    pub fields: BTreeMap<String, String>,
}

/// Metadata view of a secret — the ONLY shape a read returns. Carries names +
/// timestamps; **never** a plaintext or ciphertext value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SecretMetadataResponse {
    /// Secret UUID.
    pub id: String,
    /// The secret's name.
    pub name: String,
    /// The declared field names (no values).
    pub field_names: Vec<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp.
    pub updated_at: String,
}

/// `DELETE /v1/tenants/{tenant_id}/secrets/{name}` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SecretDeletedResponse {
    /// Always `"deleted"`.
    pub status: String,
    /// The deleted secret's name.
    pub name: String,
}
