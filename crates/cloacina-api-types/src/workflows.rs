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
    /// The highest priority the runs of this package may get, as its
    /// publisher may set now (CLOACINA-T-1123). A workflow or task that
    /// declares a higher priority (a smaller number) in its code runs at
    /// this one. Absent in an answer from an older server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority_limit: Option<u8>,
    /// Set when the limit is below the highest priority: one sentence that
    /// says the limit and why. `cloacinactl` prints it as a warning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority_notice: Option<String>,
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
    /// Who uploaded the package (CLOACINA-T-1024): the login identity, or
    /// the name of the API key. Read from the audit record; `null` for a
    /// package uploaded before the audit record existed.
    #[serde(default)]
    pub uploaded_by: Option<String>,
    /// The plugin interface version of the package's Rust build
    /// (CLOACINA-T-1085). Absent for a Python package and for a build whose
    /// interface is not recorded yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_interface_version: Option<u32>,
    /// Set when the package needs a new publish: it was built for another
    /// plugin interface than this release loads, and its rebuild from kept
    /// source did not change that (CLOACINA-T-1085). Absent when it needs
    /// nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs_new_build: Option<NeedsNewBuild>,
}

/// Why a package needs a new publish (CLOACINA-T-1085).
///
/// After a minor release that changes the plugin interface, a Rust package
/// built for the old interface does not run on the new agents. The compiler
/// rebuilds it once from its kept source; when the package pins the old
/// minor (in Cargo.toml or Cargo.lock), the rebuild has the old interface
/// again. The team changes the `cloacina` pin to the new minor and publishes
/// again. The package stays in service until then: agents of the old
/// interface still run it during a roll.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NeedsNewBuild {
    /// The reason and the step to take.
    pub reason: String,
    /// The plugin interface version the package was built for. Absent when
    /// it is not known: a library of another fidius ABI does not open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built_for: Option<u32>,
    /// The plugin interface version this release loads.
    pub current: u32,
}

/// One package that needs a new publish, in the list across tenants
/// (`GET /v1/packages/needs-new-build`, platform admin, CLOACINA-T-1085).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PackageNeedingNewBuild {
    pub tenant_id: String,
    /// Package UUID.
    pub id: String,
    pub package_name: String,
    pub version: String,
    /// The plugin interface version the package was built for. Absent when
    /// it is not known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built_for: Option<u32>,
    /// The plugin interface version this release loads.
    pub current: u32,
    /// The reason and the step to take.
    pub reason: String,
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
    /// Why the compiler could not write the metadata document of this built
    /// package (CLOACINA-T-1074). Set for a package that was built before the
    /// document existed and cannot be described from its stored build.
    /// Absent when the package is described, or has not been tried yet. A
    /// package whose describe fails during a build has `build_status`
    /// `failed` and the reason in `build_error`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub describe_error: Option<String>,
    /// Why this built package is not loaded (CLOACINA-T-1075). A server that
    /// loads packages from their metadata only does not load a package that
    /// has no metadata document: it waits for the tenant's compiler to
    /// describe it, or the compiler could not (`describe_error`). Absent when
    /// the package has a document, is not built, or the server runs with
    /// single trust. A package the server refuses has `build_status`
    /// `failed` and the reason in `build_error`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_loaded_reason: Option<String>,
    /// Whether this workflow is paused (CLOACI-T-0749). Paused workflows refuse
    /// new executions until resumed.
    #[serde(default)]
    pub paused: bool,
    /// CLOACI-I-0128: declared input params (named, JSON-Schema-typed slots) the
    /// workflow accepts at execute time. Empty when undeclared. Lets the UI
    /// render a typed execute form and the server validate context.
    #[serde(default)]
    pub declared_params: Vec<crate::InputSlot>,
    /// Who uploaded the package (CLOACINA-T-1024): the login identity, or
    /// the name of the API key. Read from the audit record; `null` for a
    /// package uploaded before the audit record existed.
    #[serde(default)]
    pub uploaded_by: Option<String>,
    /// As in [`WorkflowSummary::plugin_interface_version`] (CLOACINA-T-1085).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_interface_version: Option<u32>,
    /// As in [`WorkflowSummary::needs_new_build`] (CLOACINA-T-1085).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs_new_build: Option<NeedsNewBuild>,
    /// The workflow priority its code declares (`asked`, 5 when it declares
    /// none) and the one its runs get now (`given`): the asked one held to
    /// the limit of the publisher at this time (CLOACINA-T-1123). Absent for
    /// a package that is not built yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<crate::priority::PriorityGrant>,
    /// Set when `given` is lower than `asked`: what was asked, what is
    /// given, and why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority_notice: Option<String>,
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

/// One stored version of a package, in
/// `GET /tenants/{tenant_id}/workflows/{name}/versions` (CLOACINA-T-1018).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WorkflowVersionSummary {
    /// Package UUID of this version.
    pub id: String,
    pub package_name: String,
    pub version: String,
    /// `true` for the version in service: built and not superseded. At most
    /// one version of a package is active.
    pub active: bool,
    /// `true` once another version replaced this one, or a newer upload
    /// replaced it before it was built.
    pub superseded: bool,
    /// `pending`, `building`, `success` or `failed`.
    pub build_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_error: Option<String>,
    /// Why the compiler could not write the metadata document of this built
    /// version (CLOACINA-T-1074). Absent when the version is described, or
    /// has not been tried yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub describe_error: Option<String>,
    /// SHA-256 of the uploaded archive.
    pub content_hash: String,
    /// Upload time (RFC 3339).
    pub created_at: String,
    /// Time the build succeeded (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compiled_at: Option<String>,
}

/// `POST /tenants/{tenant_id}/workflows/{name}/versions/{version}/activate`
/// response (CLOACINA-T-1018).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WorkflowVersionActivatedResponse {
    pub tenant_id: String,
    /// Package UUID of the version that is now active.
    pub id: String,
    pub package_name: String,
    pub version: String,
    /// Always `"active"`.
    pub status: String,
}
