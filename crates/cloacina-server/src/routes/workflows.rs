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

//! Workflow package API — upload/list/get/delete .cloacina packages per tenant.

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use tracing::{info, warn};

use cloacina::dal::UnifiedRegistryStorage;
use cloacina::registry::traits::WorkflowRegistry;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use cloacina::security::audit;
use cloacina_api_types::{
    TenantListResponse, WorkflowDeletedResponse, WorkflowDetail, WorkflowSummary, WorkflowTaskNode,
    WorkflowUploadedResponse,
};

use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;
use crate::AppState;

/// POST /tenants/:tenant_id/workflows — multipart upload of .cloacina source package.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/workflows",
    tag = "workflows",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    request_body(
        content = crate::openapi::PackageUploadForm,
        content_type = "multipart/form-data",
        description = "The first file field is taken as the .cloacina package"
    ),
    responses(
        (status = 201, description = "Package registered", body = WorkflowUploadedResponse),
        (status = 400, description = "Invalid or empty package", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant/role denied or signature verification failed", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn upload_workflow(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }
    if !auth.can_write() {
        return AuthenticatedKey::insufficient_role_response().into_response();
    }

    // Extract file from multipart
    let package_data = match extract_file_field(&mut multipart).await {
        Ok(data) => data,
        Err(msg) => return ApiError::bad_request("invalid_request", msg).into_response(),
    };

    if package_data.is_empty() {
        return ApiError::bad_request("invalid_request", "empty package file").into_response();
    }

    // T-0557 Bug 2: signature verification at upload time.
    //
    // When `require_signatures` is enabled AND a `verification_org_id`
    // is configured, run real verification against the trusted-key list
    // for that org. The signature is looked up by package hash from the
    // `package_signatures` table (`SignatureSource::Database`); the
    // signing flow (`cloacinactl pack`/`publish` + the future T-0514
    // sidecar) is responsible for inserting the row before upload.
    //
    // When `require_signatures` is enabled but no org is configured
    // we fail-safe with a clearer error than the old "TODO" stub —
    // the operator knows verification is enabled but unwired, not
    // that signing is required for the upload itself.
    if state.security_config.require_signatures {
        let Some(org_id) = state.security_config.verification_org_id else {
            warn!(
                "Package upload rejected: require_signatures=true but no \
                 verification_org_id configured. Set SecurityConfig::verification_org_id \
                 before enabling signature requirements."
            );
            return ApiError::forbidden(
                "signature_verification_unconfigured",
                "signature verification is required but server is not configured \
                 with a verification_org_id; contact the server operator",
            )
            .into_response();
        };
        let dal = cloacina::dal::DAL::new(state.database.clone());
        let package_signer = cloacina::security::DbPackageSigner::new(dal.clone());
        let key_manager = cloacina::security::DbKeyManager::new(dal);
        // CLOACI-I-0103 / T-0568: route both success and failure through the
        // structured audit log so deployments with a centralised log pipeline
        // get a single, parseable record per upload. The human-readable
        // info!/warn! lines stay because they carry message-level context
        // operators tail in real time.
        let audit_path = format!("upload:tenant={}", tenant_id);
        match cloacina::security::verify_package_bytes(
            &package_data,
            org_id,
            cloacina::security::SignatureSource::Database,
            &package_signer,
            &key_manager,
        )
        .await
        {
            Ok(result) => {
                info!(
                    "Package signature verified: hash={} signer={}",
                    result.package_hash, result.signer_fingerprint
                );
                audit::log_package_load_success(
                    org_id,
                    &audit_path,
                    &result.package_hash,
                    Some(&result.signer_fingerprint),
                    /* signature_verified */ true,
                );
            }
            Err(e) => {
                warn!("Package signature verification failed: {}", e);
                let (code, msg) = match &e {
                    cloacina::security::VerificationError::TamperedPackage { .. } => (
                        "package_tampered",
                        "package contents do not match the signed hash".to_string(),
                    ),
                    cloacina::security::VerificationError::UntrustedSigner { fingerprint } => (
                        "untrusted_signer",
                        format!("package signed by untrusted key: {}", fingerprint),
                    ),
                    cloacina::security::VerificationError::InvalidSignature => (
                        "invalid_signature",
                        "cryptographic signature verification failed".to_string(),
                    ),
                    cloacina::security::VerificationError::SignatureNotFound { .. } => (
                        "signature_not_found",
                        "no signature row found for this package; sign before uploading"
                            .to_string(),
                    ),
                    _ => ("signature_verification_error", format!("{}", e)),
                };
                audit::log_package_load_failure(org_id, &audit_path, &e.to_string(), code);
                return ApiError::forbidden(code, msg).into_response();
            }
        }
    }

    // Register via WorkflowRegistry
    let tenant_db: cloacina::database::Database = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal(format!("tenant database error: {}", e)).into_response()
        }
    };
    let storage = UnifiedRegistryStorage::new(tenant_db.clone());
    let mut registry = match WorkflowRegistryImpl::new(storage, tenant_db) {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to create registry: {}", e);
            return ApiError::internal("internal registry error").into_response();
        }
    };

    match registry.register_workflow_package(package_data).await {
        Ok(package_id) => {
            info!(
                "Uploaded workflow package for tenant '{}': {}",
                tenant_id, package_id
            );
            (
                StatusCode::CREATED,
                Json(WorkflowUploadedResponse {
                    package_id: package_id.to_string(),
                    tenant_id,
                }),
            )
                .into_response()
        }
        Err(e) => {
            warn!(
                "Failed to register workflow for tenant '{}': {}",
                tenant_id, e
            );
            ApiError::bad_request("upload_failed", format!("{}", e)).into_response()
        }
    }
}

/// GET /tenants/:tenant_id/workflows — list registered workflows.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/workflows",
    tag = "workflows",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    responses(
        (status = 200, description = "Registered workflows", body = TenantListResponse<WorkflowSummary>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_workflows(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }

    let tenant_db: cloacina::database::Database = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal(format!("tenant database error: {}", e)).into_response()
        }
    };
    let storage = UnifiedRegistryStorage::new(tenant_db.clone());
    let registry = match WorkflowRegistryImpl::new(storage, tenant_db) {
        Ok(r) => r,
        Err(e) => return ApiError::internal(format!("{}", e)).into_response(),
    };

    match registry.list_workflows().await {
        Ok(workflows) => {
            let items: Vec<WorkflowSummary> = workflows
                .into_iter()
                .map(|w| WorkflowSummary {
                    id: w.id.to_string(),
                    package_name: w.package_name,
                    workflow_name: w.workflow_name,
                    version: w.version,
                    description: w.description,
                    tasks: w.tasks,
                    created_at: w.created_at.to_rfc3339(),
                })
                .collect();
            // CLOACI-T-0594 / API-03: unified `{items, total}` envelope.
            Json(TenantListResponse::new(tenant_id, items)).into_response()
        }
        Err(e) => {
            warn!("Failed to list workflows for tenant '{}': {}", tenant_id, e);
            ApiError::internal(format!("{}", e)).into_response()
        }
    }
}

/// GET /tenants/:tenant_id/workflows/:name — get workflow details.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/workflows/{name}",
    tag = "workflows",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Package name, or package UUID to inspect pending/building/failed rows"),
    ),
    responses(
        (status = 200, description = "Workflow detail incl. build state", body = WorkflowDetail),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Workflow not found", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_workflow(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }

    let tenant_db: cloacina::database::Database = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal(format!("tenant database error: {}", e)).into_response()
        }
    };
    let storage = UnifiedRegistryStorage::new(tenant_db.clone());
    let registry = match WorkflowRegistryImpl::new(storage, tenant_db) {
        Ok(r) => r,
        Err(e) => return ApiError::internal(format!("{}", e)).into_response(),
    };

    // Use inspect_package_by_id when `name` parses as a UUID so operators can
    // drill into pending / building / failed rows. Falls back to the list-scan
    // by package_name for the human-friendly lookup path.
    if let Ok(pkg_id) = uuid::Uuid::parse_str(&name) {
        match registry.inspect_package_by_id(pkg_id).await {
            Ok(Some(ins)) => {
                return Json(WorkflowDetail {
                    tenant_id,
                    id: ins.metadata.id.to_string(),
                    package_name: ins.metadata.package_name,
                    workflow_name: ins.metadata.workflow_name,
                    version: ins.metadata.version,
                    description: ins.metadata.description,
                    tasks: ins.metadata.tasks,
                    task_graph: ins
                        .metadata
                        .task_graph
                        .into_iter()
                        .map(|n| WorkflowTaskNode {
                            id: n.id,
                            dependencies: n.dependencies,
                            description: n.description,
                        })
                        .collect(),
                    created_at: ins.metadata.created_at.to_rfc3339(),
                    build_status: ins.build_status,
                    build_error: ins.build_error,
                })
                .into_response();
            }
            Ok(None) => {
                return ApiError::not_found(
                    "workflow_not_found",
                    format!("workflow '{}' not found", name),
                )
                .into_response();
            }
            Err(e) => return ApiError::internal(format!("{}", e)).into_response(),
        }
    }

    match registry.list_workflows().await {
        Ok(workflows) => {
            let found = workflows.into_iter().find(|w| w.package_name == name);
            match found {
                Some(w) => {
                    // T-0557 Bug 4: previously hard-coded
                    // build_status: "success" here. Route the
                    // name-lookup path through the same inspector the
                    // UUID-lookup path uses so the response reflects
                    // real build state (pending/building/failed in
                    // addition to success).
                    match registry.inspect_package_by_id(w.id).await {
                        Ok(Some(ins)) => Json(WorkflowDetail {
                            tenant_id,
                            id: ins.metadata.id.to_string(),
                            package_name: ins.metadata.package_name,
                            workflow_name: ins.metadata.workflow_name,
                            version: ins.metadata.version,
                            description: ins.metadata.description,
                            tasks: ins.metadata.tasks,
                            task_graph: ins
                                .metadata
                                .task_graph
                                .into_iter()
                                .map(|n| WorkflowTaskNode {
                                    id: n.id,
                                    dependencies: n.dependencies,
                                    description: n.description,
                                })
                                .collect(),
                            created_at: ins.metadata.created_at.to_rfc3339(),
                            build_status: ins.build_status,
                            build_error: ins.build_error,
                        })
                        .into_response(),
                        Ok(None) => ApiError::not_found(
                            "workflow_not_found",
                            format!("workflow '{}' not found", name),
                        )
                        .into_response(),
                        Err(e) => ApiError::internal(format!("{}", e)).into_response(),
                    }
                }
                None => ApiError::not_found(
                    "workflow_not_found",
                    format!("workflow '{}' not found", name),
                )
                .into_response(),
            }
        }
        Err(e) => ApiError::internal(format!("{}", e)).into_response(),
    }
}

/// DELETE /tenants/:tenant_id/workflows/:name/:version — unregister workflow.
#[utoipa::path(
    delete,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/{version}",
    tag = "workflows",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Package name"),
        ("version" = String, Path, description = "Package version"),
    ),
    responses(
        (status = 200, description = "Workflow unregistered. Idempotent: also returned when no matching package existed", body = WorkflowDeletedResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or role denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Registry lookup failed", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn delete_workflow(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name, version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    if !auth.can_access_tenant(&tenant_id) {
        return AuthenticatedKey::forbidden_response().into_response();
    }
    if !auth.can_write() {
        return AuthenticatedKey::insufficient_role_response().into_response();
    }

    let tenant_db: cloacina::database::Database = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal(format!("tenant database error: {}", e)).into_response()
        }
    };
    let storage = UnifiedRegistryStorage::new(tenant_db.clone());
    let mut registry = match WorkflowRegistryImpl::new(storage, tenant_db) {
        Ok(r) => r,
        Err(e) => return ApiError::internal(format!("{}", e)).into_response(),
    };

    match registry
        .unregister_workflow_package_by_name(&name, &version)
        .await
    {
        Ok(()) => {
            info!(
                "Deleted workflow '{}' v{} for tenant '{}'",
                name, version, tenant_id
            );
            Json(WorkflowDeletedResponse {
                status: "deleted".to_string(),
                package_name: name,
                version,
            })
            .into_response()
        }
        Err(e) => {
            warn!(
                "Failed to delete workflow '{}' v{} for tenant '{}': {}",
                name, version, tenant_id, e
            );
            ApiError::not_found("workflow_not_found", format!("{}", e)).into_response()
        }
    }
}

/// Extract the first file field from a multipart request.
async fn extract_file_field(multipart: &mut Multipart) -> Result<Vec<u8>, String> {
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            let data = field
                .bytes()
                .await
                .map_err(|e| format!("failed to read file: {}", e))?;
            return Ok(data.to_vec());
        }
    }
    Err("no 'file' field in multipart request".to_string())
}
