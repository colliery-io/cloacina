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

//! OpenAPI document assembly (CLOACI-I-0113 / T-0643).
//!
//! Every REST handler carries a `#[utoipa::path]` annotation; this module
//! collects them into one document, served at `/openapi.json` and emitted
//! by the `emit-openapi` subcommand. The committed copy lives at
//! `docs/static/openapi.json` (published at the docs-site root);
//! `angreal docs spec-check` fails CI when the two diverge.
//!
//! Scope: the public REST surface only. The agent-fleet routes are
//! intentionally absent (server↔agent internal protocol, not public
//! contract), and the WebSocket endpoints are documented separately in
//! `docs/content/platform/reference/websocket-protocol.md` (T-0644) since
//! OpenAPI cannot describe WS message flows.

use cloacina_api_types::{
    AccumulatorStatus, AgentInfo, CompilerStatus, CreateKeyRequest, CreateTenantRequest,
    DeclaredSurface, ErrorBody, ExecuteRequest, ExecuteResponse, ExecutionDetail, ExecutionEvent,
    ExecutionEventsResponse, ExecutionSummary, ExecutionTasksResponse, FireMode,
    FireReactorRequest, FireReactorResponse, GraphStatus, GraphTopology, GraphTopologyEdge,
    GraphTopologyNode, InjectAccumulatorRequest, InjectAccumulatorResponse, InputSlot,
    KeyCreatedResponse, KeyInfo, KeyRevokedResponse, KeyRole, ListResponse, ReactorFire,
    ReactorFireTimeseries, ReactorStatus, TaskExecutionDetail, TenantCreatedResponse,
    TenantListResponse, TenantRemovedResponse, TenantSummary, TriggerDetailResponse,
    TriggerExecution, TriggerPauseResponse, TriggerScheduleInfo, TriggerScheduleSummary,
    WorkflowDeletedResponse, WorkflowDetail, WorkflowPauseResponse, WorkflowSourceFile,
    WorkflowSourceResponse, WorkflowSummary, WorkflowTaskNode, WorkflowUploadedResponse,
    WsTicketResponse,
};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi, ToSchema};

/// Multipart form for workflow package upload. Spec-only type: the handler
/// accepts the first file field regardless of name; `file` is the
/// conventional field name.
#[derive(ToSchema)]
#[allow(dead_code)]
pub struct PackageUploadForm {
    /// The `.cloacina` package archive.
    #[schema(format = Binary, content_media_type = "application/octet-stream")]
    pub file: String,
}

/// Adds the bearer API-key security scheme referenced by every
/// authenticated path as `api_key`.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "api_key",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .description(Some(
                        "Cloacina API key, sent as `Authorization: Bearer <key>`. \
                         Tenant scope is carried by the key itself; tenant-scoped \
                         routes additionally name the tenant in the path.",
                    ))
                    .build(),
            ),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "cloacina-server API",
        description = "REST API for the Cloacina workflow-orchestration server. \
                       WebSocket protocols (execution events, accumulator ingest, \
                       reactor commands, substrate delivery) are specified in the \
                       WebSocket Protocol reference of the documentation site.",
        license(name = "Apache-2.0", url = "https://www.apache.org/licenses/LICENSE-2.0"),
    ),
    paths(
        crate::health,
        crate::ready,
        crate::routes::keys::create_key,
        crate::routes::keys::list_keys,
        crate::routes::keys::revoke_key,
        crate::routes::keys::create_ws_ticket,
        crate::routes::keys::create_tenant_key,
        crate::routes::tenants::create_tenant,
        crate::routes::tenants::list_tenants,
        crate::routes::tenants::remove_tenant,
        crate::routes::workflows::upload_workflow,
        crate::routes::workflows::list_workflows,
        crate::routes::workflows::get_workflow,
        crate::routes::workflows::get_workflow_source,
        crate::routes::workflows::pause_workflow,
        crate::routes::workflows::resume_workflow,
        crate::routes::workflows::delete_workflow,
        crate::routes::triggers::list_triggers,
        crate::routes::triggers::get_trigger,
        crate::routes::triggers::pause_trigger,
        crate::routes::triggers::resume_trigger,
        crate::routes::triggers::fire_trigger,
        crate::routes::executions::execute_workflow,
        crate::routes::executions::list_executions,
        crate::routes::executions::get_execution,
        crate::routes::executions::get_execution_events,
        crate::routes::executions::get_execution_tasks,
        crate::routes::agent::list_agents,
        crate::routes::compiler::compiler_status,
        crate::routes::health_graphs::list_accumulators,
        crate::routes::health_graphs::list_reactors,
        crate::routes::health_graphs::fire_reactor,
        crate::routes::health_graphs::list_reactor_fires,
        crate::routes::health_graphs::reactor_fire_timeseries,
        crate::routes::health_graphs::inject_accumulator,
        crate::routes::health_graphs::get_reactor_interface,
        crate::routes::health_graphs::get_accumulator_interface,
        crate::routes::health_graphs::list_graphs,
        crate::routes::health_graphs::get_graph,
    ),
    components(schemas(
        ErrorBody,
        KeyRole,
        CreateKeyRequest,
        KeyCreatedResponse,
        KeyInfo,
        KeyRevokedResponse,
        WsTicketResponse,
        ListResponse<KeyInfo>,
        CreateTenantRequest,
        TenantCreatedResponse,
        TenantRemovedResponse,
        TenantSummary,
        ListResponse<TenantSummary>,
        PackageUploadForm,
        WorkflowUploadedResponse,
        WorkflowSummary,
        WorkflowDetail,
        WorkflowTaskNode,
        WorkflowSourceResponse,
        WorkflowSourceFile,
        WorkflowPauseResponse,
        InputSlot,
        DeclaredSurface,
        WorkflowDeletedResponse,
        TenantListResponse<WorkflowSummary>,
        TriggerScheduleSummary,
        TriggerScheduleInfo,
        TriggerExecution,
        TriggerDetailResponse,
        TriggerPauseResponse,
        TenantListResponse<TriggerScheduleSummary>,
        ExecuteRequest,
        ExecuteResponse,
        ExecutionSummary,
        ExecutionDetail,
        ExecutionEvent,
        ExecutionEventsResponse,
        TaskExecutionDetail,
        ExecutionTasksResponse,
        TenantListResponse<ExecutionSummary>,
        AgentInfo,
        ListResponse<AgentInfo>,
        CompilerStatus,
        AccumulatorStatus,
        GraphStatus,
        GraphTopology,
        GraphTopologyNode,
        GraphTopologyEdge,
        ReactorStatus,
        ReactorFire,
        ReactorFireTimeseries,
        FireMode,
        FireReactorRequest,
        FireReactorResponse,
        InjectAccumulatorRequest,
        InjectAccumulatorResponse,
        ListResponse<AccumulatorStatus>,
        ListResponse<ReactorFire>,
        ListResponse<GraphStatus>,
        ListResponse<ReactorStatus>,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "operational", description = "Liveness/readiness (no auth)"),
        (name = "keys", description = "API key management"),
        (name = "tenants", description = "Tenant lifecycle (admin)"),
        (name = "workflows", description = "Workflow package registry"),
        (name = "triggers", description = "Cron + trigger schedules (read-only)"),
        (name = "executions", description = "Workflow execution + event log"),
        (name = "fleet", description = "Execution-agent fleet roster (admin)"),
        (name = "compiler", description = "Compiler / build-pipeline status (admin)"),
        (name = "graph-health", description = "Computation-graph health"),
    )
)]
pub struct ApiDoc;

/// Pretty-printed OpenAPI document. The single source for both the
/// runtime `/openapi.json` route and the `emit-openapi` subcommand, so
/// the served and committed specs cannot disagree with each other.
pub fn openapi_json() -> String {
    ApiDoc::openapi()
        .to_pretty_json()
        .expect("OpenAPI document serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_document_builds_and_serializes() {
        let json = openapi_json();
        assert!(json.contains("\"openapi\""));
        assert!(json.contains("/v1/tenants/{tenant_id}/executions"));
        assert!(json.contains("api_key"));
    }

    #[test]
    fn spec_version_matches_crate_version() {
        // Lockstep policy (REQ-008): the spec version is the workspace
        // version utoipa picks up from CARGO_PKG_VERSION.
        let doc = ApiDoc::openapi();
        assert_eq!(doc.info.version, env!("CARGO_PKG_VERSION"));
    }
}
