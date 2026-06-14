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
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WorkflowUploadedResponse {
    /// UUID of the registered package.
    pub package_id: String,
    pub tenant_id: String,
}

/// One row in the workflow list (`GET /tenants/{tenant_id}/workflows`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WorkflowSummary {
    /// Package UUID.
    pub id: String,
    pub package_name: String,
    /// Executable workflow name (the identifier to execute by). Differs from
    /// `package_name` under the standard convention (package `demo-slow-rust`
    /// → workflow `demo_slow_workflow`). Falls back to `package_name` for
    /// packages predating workflow-name persistence. (CLOACI-T-0671)
    pub workflow_name: String,
    pub version: String,
    pub description: Option<String>,
    /// Task IDs included in this package.
    pub tasks: Vec<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
}

/// `DELETE /tenants/{tenant_id}/workflows/{name}/{version}` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WorkflowDeletedResponse {
    /// Always `"deleted"`.
    pub status: String,
    pub package_name: String,
    pub version: String,
}

/// One node in a workflow's task dependency graph — a task plus the ids of the
/// tasks it depends on. The UI renders these as a DAG. (CLOACI-T-0663)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WorkflowTaskNode {
    /// Local task id (the node id), e.g. `"validate"`.
    pub id: String,
    /// Local ids of the tasks this task depends on (its incoming edges).
    pub dependencies: Vec<String>,
    /// Optional human-readable task description.
    pub description: Option<String>,
}

/// `GET /tenants/{tenant_id}/workflows/{name}` response — summary fields
/// plus real build state (pending/building/failed/success).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WorkflowDetail {
    pub tenant_id: String,
    /// Package UUID.
    pub id: String,
    pub package_name: String,
    /// Executable workflow name (the identifier to execute by). Differs from
    /// `package_name` under the standard convention (package `demo-slow-rust`
    /// → workflow `demo_slow_workflow`). Falls back to `package_name` for
    /// packages predating workflow-name persistence. (CLOACI-T-0671)
    pub workflow_name: String,
    pub version: String,
    pub description: Option<String>,
    /// Task IDs included in this package.
    pub tasks: Vec<String>,
    /// The task dependency graph (nodes + their upstream dependencies) for
    /// rendering the full workflow DAG. Empty for packages predating
    /// task-graph persistence. (CLOACI-T-0663)
    #[serde(default)]
    pub task_graph: Vec<WorkflowTaskNode>,
    /// RFC 3339 timestamp.
    pub created_at: String,
    pub build_status: String,
    pub build_error: Option<String>,
}
