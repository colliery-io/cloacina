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

//! Audit record DTOs (CLOACINA-T-1024).

use serde::{Deserialize, Serialize};

/// Query string of `GET /v1/tenants/{tenant_id}/audit` and `GET /v1/audit`.
/// Every field narrows the result.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema, utoipa::IntoParams))]
#[cfg_attr(feature = "openapi", into_params(parameter_in = Query))]
pub struct ListAuditQuery {
    /// Only rows of this tenant. Read by `GET /v1/audit` only: the
    /// tenant route takes the tenant from the path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    /// Only rows with this action, for example `package.upload`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// Only rows whose key id, key name or identity equals this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    /// Only rows at or after this RFC 3339 time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
    /// Only rows before this RFC 3339 time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    /// Page size, 1 to 200. Default 50.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// `next_cursor` of the page before: only older rows are returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<i64>,
}

/// One audit row: who did what, when, in which tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AuditEventInfo {
    /// Row id. Larger is newer.
    pub id: i64,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// The tenant the action applied to; `null` for a platform action.
    pub tenant_id: Option<String>,
    /// UUID of the API key that made the request.
    pub actor_key_id: String,
    /// Name of that key.
    pub actor_key_name: String,
    /// Who the key was minted for at login (`oidc:<issuer>:<sub>` or
    /// `local:<account_id>`); `null` for a manually created key.
    pub actor_identity: Option<String>,
    /// A fixed string, for example `package.upload`.
    pub action: String,
    /// What the action applied to, for example `package` or `workflow`.
    pub target_kind: String,
    /// Name or id of the target; `null` when the action has none.
    pub target: Option<String>,
    /// `ok` or `denied`.
    pub outcome: String,
    /// A small JSON object with facts of the action (a version, an
    /// execution id). Never a secret value or a request body.
    #[cfg_attr(feature = "openapi", schema(value_type = Object))]
    pub detail: serde_json::Value,
}

/// A page of audit rows, newest first.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AuditListResponse {
    pub items: Vec<AuditEventInfo>,
    /// Number of rows in this page.
    pub total: usize,
    /// Pass as `cursor` to read the next (older) page; `null` when this page
    /// was not full.
    pub next_cursor: Option<i64>,
}
