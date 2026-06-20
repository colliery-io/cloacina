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
    /// Whether this workflow is paused (CLOACI-T-0749). Paused workflows refuse
    /// new executions until resumed.
    #[serde(default)]
    pub paused: bool,
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
    /// CLOACI-T-0752 "what" — short summary parsed from the task's
    /// doc-comment/docstring at build time. `None` when undocumented.
    #[serde(default)]
    pub doc_what: Option<String>,
    /// CLOACI-T-0752 "why" — rationale parsed from the doc-comment/docstring.
    /// `None` when undocumented.
    #[serde(default)]
    pub doc_why: Option<String>,
}

/// One source file from a workflow package's retained `.cloacina` archive,
/// surfaced read-only for display (CLOACI-T-0750).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WorkflowSourceFile {
    /// Path relative to the package source root, using forward slashes
    /// (e.g. `"src/lib.rs"`, `"package.toml"`).
    pub path: String,
    /// Best-effort language id derived from the file extension (`"rust"`,
    /// `"python"`, `"toml"`, …), for syntax highlighting. `None` when unknown.
    pub language: Option<String>,
    /// UTF-8 file contents.
    pub contents: String,
}

/// `GET /tenants/{tenant_id}/workflows/{name}/source` response — the original
/// source retained in the package's `.cloacina` archive, surfaced read-only
/// (CLOACI-T-0750). The source is independent of build state, so it is
/// available even for packages that are still building or failed to build.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WorkflowSourceResponse {
    pub tenant_id: String,
    /// Package UUID.
    pub id: String,
    pub package_name: String,
    /// Executable workflow name (see `WorkflowSummary::workflow_name`).
    pub workflow_name: String,
    pub version: String,
    /// Source files in the package, sorted by path. Binary and oversized files
    /// are omitted.
    pub files: Vec<WorkflowSourceFile>,
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
    /// Whether this workflow is paused (CLOACI-T-0749). Paused workflows refuse
    /// new executions until resumed.
    #[serde(default)]
    pub paused: bool,
    /// CLOACI-I-0128: declared input params (named, JSON-Schema-typed slots) the
    /// workflow accepts at execute time. Empty when undeclared. Lets the UI
    /// render a typed execute form and the server validate context.
    #[serde(default)]
    pub declared_params: Vec<crate::InputSlot>,
}

/// `POST /tenants/{tenant_id}/workflows/{name}/pause` and `/resume` response
/// (CLOACI-T-0749).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WorkflowPauseResponse {
    pub tenant_id: String,
    /// Package UUID of the affected workflow.
    pub id: String,
    /// The name the workflow was addressed by (workflow or package name).
    pub name: String,
    /// `"paused"` or `"resumed"`.
    pub status: String,
    /// Current paused state after the operation.
    pub paused: bool,
}
