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

//! Tenant management types.

use serde::{Deserialize, Serialize};

/// Request body for `POST /tenants`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTenantRequest {
    /// Tenant name — doubles as schema name + database username.
    /// Must be alphanumeric + underscore.
    pub name: String,
    /// Optional operator-facing description.
    #[serde(default)]
    pub description: Option<String>,
    /// Optional password (auto-generated if absent).
    #[serde(default)]
    pub password: Option<String>,
}

/// `201 Created` body for a new tenant. Password and connection string are
/// intentionally excluded to prevent credential leakage (SEC-08).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantCreatedResponse {
    pub name: String,
    pub username: String,
    pub description: Option<String>,
}

/// `DELETE /tenants/{schema_name}` response — orchestrated teardown report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantRemovedResponse {
    /// Always `"removed"`.
    pub status: String,
    pub schema_name: String,
    /// Number of still-active API keys revoked during teardown.
    pub revoked_keys: usize,
    pub runner_evicted: bool,
    pub db_cache_evicted: bool,
}

/// One row in the tenant list (`GET /tenants`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantSummary {
    pub name: String,
}
