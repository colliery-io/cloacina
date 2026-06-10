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

//! API key management types.

use serde::{Deserialize, Serialize};

/// Allowed roles for API keys.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KeyRole {
    #[default]
    Admin,
    Write,
    Read,
}

impl KeyRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            KeyRole::Admin => "admin",
            KeyRole::Write => "write",
            KeyRole::Read => "read",
        }
    }
}

/// Request body for `POST /auth/keys` and `POST /tenants/{tenant_id}/keys`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
    #[serde(default)]
    pub role: KeyRole,
}

/// `201 Created` body for a new API key. The plaintext `key` is returned
/// exactly once — it cannot be retrieved again.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyCreatedResponse {
    /// Key UUID.
    pub id: String,
    pub name: String,
    /// One-time plaintext API key.
    pub key: String,
    /// Role string: `read` | `write` | `admin`.
    pub permissions: String,
    /// Tenant scope; `null` for global keys.
    pub tenant_id: Option<String>,
    /// God-mode flag — never granted via the create endpoints.
    pub is_admin: bool,
    /// RFC 3339 timestamp.
    pub created_at: String,
}

/// One row in the key list (`GET /auth/keys`). No hashes or plaintext.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    /// Key UUID.
    pub id: String,
    pub name: String,
    /// Role string: `read` | `write` | `admin`.
    pub permissions: String,
    /// Tenant scope; `null` for global keys.
    pub tenant_id: Option<String>,
    pub is_admin: bool,
    /// RFC 3339 timestamp.
    pub created_at: String,
    pub revoked: bool,
}

/// `DELETE /auth/keys/{key_id}` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRevokedResponse {
    /// Always `"revoked"`.
    pub status: String,
    /// UUID of the revoked key.
    pub id: String,
}

/// `POST /auth/ws-ticket` response — a single-use, short-lived ticket for
/// WebSocket upgrade auth (avoids long-lived API keys in URLs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsTicketResponse {
    pub ticket: String,
    pub expires_in_seconds: u64,
}
