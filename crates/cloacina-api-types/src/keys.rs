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

/// The role of an API key: a named set of permissions, fixed in the server.
///
/// - `read`: list and inspect.
/// - `write`: `read`, plus upload, run and operate.
/// - `admin`: `write`, plus keys, accounts, secrets, the audit record and the
///   fleet size of its tenant.
/// - `agent`: act as an execution agent, and nothing else.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum KeyRole {
    Admin,
    Write,
    Read,
    Agent,
}

impl KeyRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            KeyRole::Admin => "admin",
            KeyRole::Write => "write",
            KeyRole::Read => "read",
            KeyRole::Agent => "agent",
        }
    }
}

/// Request body for `POST /auth/keys` and `POST /tenants/{tenant_id}/keys`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateKeyRequest {
    pub name: String,
    /// Required. A request with no role is refused with 400: the server does
    /// not choose a role for the caller.
    pub role: KeyRole,
    /// Optional. When the key expires, as an RFC 3339 timestamp in the
    /// future (for example `2027-01-31T00:00:00Z`). After it the key is
    /// refused with 401 `key_expired`. Not set: the key does not expire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// `201 Created` body for a new API key. The plaintext `key` is returned
/// exactly once — it cannot be retrieved again.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct KeyCreatedResponse {
    /// Key UUID.
    pub id: String,
    pub name: String,
    /// One-time plaintext API key.
    pub key: String,
    /// Role string: `read` | `write` | `admin` | `agent`.
    pub permissions: String,
    /// Tenant scope; `null` for global keys.
    pub tenant_id: Option<String>,
    /// God-mode flag — never granted via the create endpoints.
    pub is_admin: bool,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// When the key expires (RFC 3339); `null`: it does not expire.
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// One row in the key list (`GET /auth/keys`). No hashes or plaintext.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct KeyInfo {
    /// Key UUID.
    pub id: String,
    pub name: String,
    /// Role string: `read` | `write` | `admin` | `agent`.
    pub permissions: String,
    /// Tenant scope; `null` for global keys.
    pub tenant_id: Option<String>,
    pub is_admin: bool,
    /// RFC 3339 timestamp.
    pub created_at: String,
    pub revoked: bool,
    /// When the key expires (RFC 3339); `null`: it does not expire. The
    /// keys the Kubernetes actuator mints for agents expire.
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// `DELETE /auth/keys/{key_id}` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct KeyRevokedResponse {
    /// Always `"revoked"`.
    pub status: String,
    /// UUID of the revoked key.
    pub id: String,
}

/// `POST /auth/ws-ticket` response — a single-use, short-lived ticket for
/// WebSocket upgrade auth (avoids long-lived API keys in URLs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WsTicketResponse {
    pub ticket: String,
    pub expires_in_seconds: u64,
}
