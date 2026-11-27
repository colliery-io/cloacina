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

//! Trusted keys of package signing, per tenant (CLOACINA-A-0018).
//!
//! A trusted key is an Ed25519 public key. An upload to a tenant passes the
//! signature check when its signer is a live trusted key of the tenant.

use serde::{Deserialize, Serialize};

/// Request body for `POST /v1/tenants/{tenant_id}/trusted-keys`.
///
/// Give the key in exactly one of the two forms.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AddTrustedKeyRequest {
    /// A name for the key, to tell the keys of the list apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The public key as PEM (`-----BEGIN PUBLIC KEY-----`, Ed25519
    /// SubjectPublicKeyInfo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key_pem: Option<String>,
    /// The raw 32-byte Ed25519 public key as 64 hexadecimal characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
}

/// One trusted key of a tenant. The body of the add, and one row of the list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TrustedKeySummary {
    /// Key UUID: the id a revoke takes.
    pub id: String,
    /// SHA-256 of the public key, as hexadecimal. A signature names its
    /// signer by this value.
    pub fingerprint: String,
    /// The name given at the add, if any.
    pub name: Option<String>,
    /// RFC 3339 timestamp.
    pub trusted_at: String,
    /// True when the key was revoked: no upload passes with it.
    pub revoked: bool,
    /// RFC 3339 timestamp of the revoke; `null` for a live key.
    pub revoked_at: Option<String>,
}

/// `DELETE /v1/tenants/{tenant_id}/trusted-keys/{key_id}` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TrustedKeyRevokedResponse {
    /// Always `"revoked"`.
    pub status: String,
    /// The id of the revoked key.
    pub id: String,
}
