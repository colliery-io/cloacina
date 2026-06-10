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

//! Workflow registry types.

use serde::{Deserialize, Serialize};

/// `201 Created` body for a workflow package upload
/// (`POST /tenants/{tenant_id}/workflows`, multipart).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowUploadedResponse {
    /// UUID of the registered package.
    pub package_id: String,
    pub tenant_id: String,
}

/// One row in the workflow list (`GET /tenants/{tenant_id}/workflows`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSummary {
    /// Package UUID.
    pub id: String,
    pub package_name: String,
    pub version: String,
    pub description: Option<String>,
    /// Task IDs included in this package.
    pub tasks: Vec<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
}

/// `DELETE /tenants/{tenant_id}/workflows/{name}/{version}` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDeletedResponse {
    /// Always `"deleted"`.
    pub status: String,
    pub package_name: String,
    pub version: String,
}

/// `GET /tenants/{tenant_id}/workflows/{name}` response — summary fields
/// plus real build state (pending/building/failed/success).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDetail {
    pub tenant_id: String,
    /// Package UUID.
    pub id: String,
    pub package_name: String,
    pub version: String,
    pub description: Option<String>,
    /// Task IDs included in this package.
    pub tasks: Vec<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
    pub build_status: String,
    pub build_error: Option<String>,
}
