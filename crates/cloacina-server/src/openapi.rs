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
//! `docs/public/openapi.json` (published at the docs-site root);
//! `angreal docs spec-check` fails CI when the two diverge.
//!
//! Scope: the public REST surface only. The agent-fleet routes are
//! intentionally absent (server↔agent internal protocol, not public
//! contract), and the WebSocket endpoints are documented separately in
//! `docs/src/content/docs/reference/websocket-protocol.md` (T-0644) since
//! OpenAPI cannot describe WS message flows.

use cloacina_api_types::{
    AccumulatorStatus, AgentInfo, BackfillDetail, BackfillFailedInterval, BackfillProgress,
    BackfillSummary, CompilerStatus, CreateBackfillRequest, CreateInstanceRequest,
    CreateKeyRequest, CreateTenantRequest, DeclaredSurface, DeleteInstanceResponse, ErrorBody,
    ExecuteRequest, ExecuteResponse, ExecutionDetail, ExecutionEvent, ExecutionEventsResponse,
    ExecutionSummary, ExecutionTasksResponse, FireMode, FireReactorRequest, FireReactorResponse,
    GraphStatus, GraphTopology, GraphTopologyEdge, GraphTopologyNode, InjectAccumulatorRequest,
    InjectAccumulatorResponse, InputSlot, KeyCreatedResponse, KeyInfo, KeyRevokedResponse, KeyRole,
    ListResponse, ReactorFire, ReactorFireTimeseries, ReactorStatus, RetryBackfillRequest,
    SecretDeletedResponse, SecretMetadataResponse, TaskAttemptErrorDetail, TaskExecutionDetail,
    TenantCreatedResponse, TenantListResponse, TenantRemovedResponse, TenantSummary,
    TriggerDetailResponse, TriggerExecution, TriggerPauseResponse, TriggerScheduleInfo,
    TriggerScheduleSummary, WorkflowDeletedResponse, WorkflowDetail, WorkflowInstanceSummary,
    WorkflowPauseResponse, WorkflowSourceFile, WorkflowSourceResponse, WorkflowSummary,
    WorkflowTaskNode, WorkflowUploadedResponse, WorkflowVersionActivatedResponse,
    WorkflowVersionSummary, WsTicketResponse,
};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi, ToSchema};

/// Multipart form for workflow package upload. Spec-only type: the handler
/// reads the fields `file` and `signature` by name.
#[derive(ToSchema)]
#[allow(dead_code)]
pub struct PackageUploadForm {
    /// The `.cloacina` package archive.
    #[schema(format = Binary, content_media_type = "application/octet-stream")]
    pub file: String,
    /// The detached signature of the archive: the JSON text of the `.sig`
    /// file that `cloacinactl package pack --sign` writes (`version`,
    /// `algorithm`, `package_hash`, `key_fingerprint`, `signature`,
    /// `signed_at`). Ed25519 over the SHA-256 of the archive. Needed when the
    /// server requires signed packages; checked whenever it is sent.
    /// Left out of an upload with no signature: the field has no null value.
    #[schema(required = false)]
    pub signature: String,
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
        crate::routes::keys::list_tenant_keys,
        crate::routes::keys::revoke_tenant_key,
        crate::routes::keys::update_tenant_key,
        crate::routes::local_auth::local_login,
        crate::routes::local_auth::create_account,
        crate::routes::local_auth::list_accounts,
        crate::routes::local_auth::disable_account,
        crate::routes::local_auth::reset_password,
        crate::routes::secrets::create_secret,
        crate::routes::secrets::rotate_secret,
        crate::routes::secrets::list_secrets,
        crate::routes::secrets::get_secret,
        crate::routes::secrets::delete_secret,
        crate::routes::trusted_keys::add_trusted_key,
        crate::routes::trusted_keys::list_trusted_keys,
        crate::routes::trusted_keys::revoke_trusted_key,
        crate::routes::session::refresh,
        crate::routes::session::logout,
        crate::routes::session::whoami,
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
        crate::routes::workflows::list_workflow_versions,
        crate::routes::workflows::activate_workflow_version,
        crate::routes::workflows::list_packages_needing_new_build,
        crate::routes::instances::create_instance,
        crate::routes::instances::list_instances,
        crate::routes::instances::get_instance,
        crate::routes::instances::delete_instance,
        crate::routes::backfills::create_backfill,
        crate::routes::backfills::list_backfills,
        crate::routes::backfills::get_backfill,
        crate::routes::backfills::cancel_backfill,
        crate::routes::backfills::retry_backfill,
        crate::routes::triggers::list_triggers,
        crate::routes::triggers::get_trigger,
        crate::routes::triggers::pause_trigger,
        crate::routes::triggers::resume_trigger,
        crate::routes::triggers::fire_trigger,
        crate::routes::triggers::get_trigger_interface,
        crate::routes::executions::execute_workflow,
        crate::routes::executions::list_executions,
        crate::routes::executions::get_execution,
        crate::routes::executions::get_execution_events,
        crate::routes::executions::get_execution_tasks,
        crate::routes::agent::list_agents,
        crate::routes::pools::list_pools,
        crate::routes::pools::get_pool,
        crate::routes::pools::put_pool,
        crate::routes::pools::delete_pool,
        crate::routes::audit::list_tenant_audit,
        crate::routes::audit::list_audit,
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
        cloacina_api_types::UpdateKeyRequest,
        KeyCreatedResponse,
        crate::routes::local_auth::LocalLoginRequest,
        crate::routes::local_auth::LocalLoginResponse,
        crate::routes::local_auth::CreateAccountRequest,
        crate::routes::local_auth::ResetPasswordRequest,
        crate::routes::local_auth::AccountInfo,
        crate::routes::local_auth::AccountActionResponse,
        cloacina_api_types::CreateSecretRequest,
        cloacina_api_types::RotateSecretRequest,
        SecretMetadataResponse,
        SecretDeletedResponse,
        ListResponse<SecretMetadataResponse>,
        cloacina_api_types::AddTrustedKeyRequest,
        cloacina_api_types::TrustedKeySummary,
        cloacina_api_types::TrustedKeyRevokedResponse,
        ListResponse<cloacina_api_types::TrustedKeySummary>,
        crate::routes::session::LogoutResponse,
        crate::routes::session::WhoamiResponse,
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
        WorkflowVersionSummary,
        WorkflowVersionActivatedResponse,
        TenantListResponse<WorkflowVersionSummary>,
        cloacina_api_types::NeedsNewBuild,
        cloacina_api_types::PackageNeedingNewBuild,
        ListResponse<cloacina_api_types::PackageNeedingNewBuild>,
        TriggerScheduleSummary,
        TriggerScheduleInfo,
        TriggerExecution,
        TriggerDetailResponse,
        TriggerPauseResponse,
        TenantListResponse<TriggerScheduleSummary>,
        CreateInstanceRequest,
        WorkflowInstanceSummary,
        DeleteInstanceResponse,
        TenantListResponse<WorkflowInstanceSummary>,
        CreateBackfillRequest,
        RetryBackfillRequest,
        BackfillSummary,
        BackfillProgress,
        BackfillFailedInterval,
        BackfillDetail,
        TenantListResponse<BackfillSummary>,
        ExecuteRequest,
        ExecuteResponse,
        ExecutionSummary,
        ExecutionDetail,
        ExecutionEvent,
        ExecutionEventsResponse,
        TaskExecutionDetail,
        TaskAttemptErrorDetail,
        ExecutionTasksResponse,
        TenantListResponse<ExecutionSummary>,
        AgentInfo,
        ListResponse<AgentInfo>,
        cloacina_api_types::AgentPoolSettings,
        cloacina_api_types::AgentEgressRule,
        cloacina_api_types::AgentEgressPort,
        cloacina_api_types::AgentPoolSettingsInfo,
        cloacina_api_types::AgentEgressRuleInfo,
        cloacina_api_types::AgentEgressPortInfo,
        cloacina_api_types::AgentPoolInfo,
        ListResponse<cloacina_api_types::AgentPoolInfo>,
        cloacina_api_types::AuditEventInfo,
        cloacina_api_types::AuditListResponse,
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
        (name = "trusted-keys", description = "Trusted keys of package signing, per tenant (admin)"),
        (name = "tenants", description = "Tenant lifecycle (admin)"),
        (name = "workflows", description = "Workflow package registry"),
        (name = "triggers", description = "Cron + trigger schedules (read-only)"),
        (name = "executions", description = "Workflow execution + event log"),
        (name = "fleet", description = "Execution-agent fleet roster (admin)"),
        (name = "audit", description = "Audit record: who did what, when, in which tenant (admin)"),
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
