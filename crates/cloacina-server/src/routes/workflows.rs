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
    Extension, Json,
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use tracing::{info, warn};

use cloacina::dal::UnifiedRegistryStorage;
use cloacina::registry::traits::WorkflowRegistry;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use cloacina::security::audit;
use cloacina_api_types::{
    TenantListResponse, WorkflowDeletedResponse, WorkflowDetail, WorkflowPauseResponse,
    WorkflowSourceFile, WorkflowSourceResponse, WorkflowSummary, WorkflowTaskNode,
    WorkflowUploadedResponse, WorkflowVersionActivatedResponse, WorkflowVersionSummary,
};

use crate::AppState;
use crate::routes::audit::{self as audit_record, AuditEvent};
use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;

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
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // CLOACINA-T-1048: refuse an upload over the limit with its size and the
    // limit, before the body is read.
    let limit = crate::upload_limit::max_package_bytes();
    let declared = headers
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    if let Some(size) = declared
        && size > limit.saturating_add(crate::upload_limit::MULTIPART_OVERHEAD_BYTES)
    {
        return package_too_large(Some(size), limit).into_response();
    }

    // Extract file from multipart
    let package_data = match extract_file_field(&mut multipart).await {
        Ok(data) => data,
        Err(FileFieldError::TooLarge) => {
            return package_too_large(declared, limit).into_response();
        }
        Err(FileFieldError::Invalid(msg)) => {
            return ApiError::bad_request("invalid_request", msg).into_response();
        }
    };
    if package_data.len() as u64 > limit {
        return package_too_large(Some(package_data.len() as u64), limit).into_response();
    }

    if package_data.is_empty() {
        return ApiError::bad_request("invalid_request", "empty package file").into_response();
    }

    // Signature verification at upload time (T-0557; CLOACINA-A-0018).
    //
    // With `require_signatures` on, the signer of the upload must be a live
    // trusted key of the tenant in the path, or of the server-wide org if one
    // is set (`--verification-org-id`). The tenant's org is asked first. The
    // signature is looked up by package hash in `package_signatures`
    // (`SignatureSource::Database`).
    if state.security_config.require_signatures {
        let tenant_org = cloacina::security::tenant_org_id(&tenant_id);
        let mut orgs = vec![tenant_org];
        if let Some(server_org) = state.security_config.verification_org_id
            && server_org != tenant_org
        {
            orgs.push(server_org);
        }
        let dal = cloacina::dal::DAL::new(state.database.clone());
        let package_signer = cloacina::security::DbPackageSigner::new(dal.clone());
        let key_manager = cloacina::security::DbKeyManager::new(dal);
        // CLOACI-I-0103 / T-0568: route both success and failure through the
        // structured audit log so deployments with a centralised log pipeline
        // get a single, parseable record per upload. The human-readable
        // info!/warn! lines stay because they carry message-level context
        // operators tail in real time.
        let audit_path = format!("upload:tenant={}", tenant_id);
        match cloacina::security::verify_package_bytes_for_orgs(
            &package_data,
            &orgs,
            cloacina::security::SignatureSource::Database,
            &package_signer,
            &key_manager,
        )
        .await
        {
            // `trusted_by` is the org whose key list held the signer.
            Ok((result, trusted_by)) => {
                info!(
                    "Package signature verified: hash={} signer={} org={}",
                    result.package_hash, result.signer_fingerprint, trusted_by
                );
                audit::log_package_load_success(
                    trusted_by,
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
                audit::log_package_load_failure(tenant_org, &audit_path, &e.to_string(), code);
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
            return ApiError::internal_logged("tenant database error", &e).into_response();
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

    let (content_hash, size_bytes) = (
        audit::package_content_hash(&package_data),
        package_data.len(),
    );
    match registry.register_workflow_package(package_data).await {
        Ok(package_id) => {
            audit_record::note(
                Some(&tenant_id),
                AuditEvent::ok(
                    audit_record::actions::PACKAGE_UPLOAD,
                    package_id.to_string(),
                )
                .detail(
                    audit_record::upload_detail(&registry, package_id, content_hash, size_bytes)
                        .await,
                ),
            );
            info!(
                "Uploaded workflow package for tenant '{}': {}",
                tenant_id, package_id
            );
            // CLOACINA-T-1031: the tenant's runner loads the package and
            // registers its schedules. Make sure it is live, so the publish
            // needs no other call. Best effort: the runner pass retries.
            if let Err(e) = crate::tenant_runner_cache::ensure_runner(&state, &tenant_id).await {
                warn!(
                    tenant = %tenant_id,
                    error = %e,
                    "tenant runner not started at upload; the runner pass retries"
                );
            }
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
            upload_error(&e).into_response()
        }
    }
}

/// The answer to a failed registration. A Python package with dependencies
/// that no valid lock file pins has a code of its own (CLOACINA-T-1097).
fn upload_error(e: &cloacina::registry::error::RegistryError) -> ApiError {
    match e {
        cloacina::registry::error::RegistryError::PythonDependenciesNotLocked { reason } => {
            ApiError::bad_request("python_dependencies_not_locked", reason.clone())
        }
        _ => ApiError::bad_request("upload_failed", format!("{}", e)),
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
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    let tenant_db: cloacina::database::Database = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal_logged("tenant database error", &e).into_response();
        }
    };
    let storage = UnifiedRegistryStorage::new(tenant_db.clone());
    let registry = match WorkflowRegistryImpl::new(storage, tenant_db) {
        Ok(r) => r,
        Err(e) => return ApiError::internal_logged("list workflows failed", &e).into_response(),
    };

    match registry.list_workflows().await {
        Ok(workflows) => {
            let ids = workflows.iter().map(|w| w.id.to_string()).collect();
            let uploaders = audit_record::package_uploaders(&state, &tenant_id, ids).await;
            let items: Vec<WorkflowSummary> = workflows
                .into_iter()
                .map(|w| WorkflowSummary {
                    uploaded_by: uploaders.get(&w.id.to_string()).cloned(),
                    id: w.id.to_string(),
                    package_name: w.package_name,
                    workflow_name: w.workflow_name,
                    version: w.version,
                    description: w.description,
                    tasks: w.tasks,
                    created_at: w.created_at.to_rfc3339(),
                    paused: w.paused,
                })
                .collect();
            // CLOACI-T-0594 / API-03: unified `{items, total}` envelope.
            Json(TenantListResponse::new(tenant_id, items)).into_response()
        }
        Err(e) => {
            warn!("Failed to list workflows for tenant '{}': {}", tenant_id, e);
            ApiError::internal_logged("list workflows failed", &e).into_response()
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
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let tenant_db: cloacina::database::Database = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal_logged("tenant database error", &e).into_response();
        }
    };
    let storage = UnifiedRegistryStorage::new(tenant_db.clone());
    let registry = match WorkflowRegistryImpl::new(storage, tenant_db) {
        Ok(r) => r,
        Err(e) => return ApiError::internal_logged("get workflow failed", &e).into_response(),
    };

    // Use inspect_package_by_id when `name` parses as a UUID so operators can
    // drill into pending / building / failed rows. Falls back to the list-scan
    // by package_name for the human-friendly lookup path.
    if let Ok(pkg_id) = uuid::Uuid::parse_str(&name) {
        match registry.inspect_package_by_id(pkg_id).await {
            Ok(Some(ins)) => {
                return Json(WorkflowDetail {
                    uploaded_by: audit_record::package_uploader(
                        &state,
                        &tenant_id,
                        ins.metadata.id.to_string(),
                    )
                    .await,
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
                            doc_what: n.doc_what,
                            doc_why: n.doc_why,
                        })
                        .collect(),
                    created_at: ins.metadata.created_at.to_rfc3339(),
                    build_status: ins.build_status,
                    build_error: ins.build_error,
                    paused: ins.metadata.paused,
                    declared_params: ins.metadata.declared_params.clone(),
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
            Err(e) => return ApiError::internal_logged("get workflow failed", &e).into_response(),
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
                            uploaded_by: audit_record::package_uploader(
                                &state,
                                &tenant_id,
                                ins.metadata.id.to_string(),
                            )
                            .await,
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
                                    doc_what: n.doc_what,
                                    doc_why: n.doc_why,
                                })
                                .collect(),
                            created_at: ins.metadata.created_at.to_rfc3339(),
                            build_status: ins.build_status,
                            build_error: ins.build_error,
                            paused: ins.metadata.paused,
                            declared_params: ins.metadata.declared_params.clone(),
                        })
                        .into_response(),
                        Ok(None) => ApiError::not_found(
                            "workflow_not_found",
                            format!("workflow '{}' not found", name),
                        )
                        .into_response(),
                        Err(e) => {
                            ApiError::internal_logged("get workflow failed", &e).into_response()
                        }
                    }
                }
                None => ApiError::not_found(
                    "workflow_not_found",
                    format!("workflow '{}' not found", name),
                )
                .into_response(),
            }
        }
        Err(e) => ApiError::internal_logged("get workflow failed", &e).into_response(),
    }
}

/// GET /tenants/:tenant_id/workflows/:name/source — read-only source view.
///
/// Surfaces the original source retained in the package's `.cloacina` archive
/// (CLOACI-T-0750). `name` may be a package name or a package UUID, matching
/// `get_workflow`. Source is independent of build state, so it is available
/// even while a package is building or after a failed build.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/source",
    tag = "workflows",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Package name, or package UUID"),
    ),
    responses(
        (status = 200, description = "Workflow source files", body = WorkflowSourceResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Workflow not found", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_workflow_source(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let tenant_db: cloacina::database::Database = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal_logged("tenant database error", &e).into_response();
        }
    };
    let storage = UnifiedRegistryStorage::new(tenant_db.clone());
    let registry = match WorkflowRegistryImpl::new(storage, tenant_db) {
        Ok(r) => r,
        Err(e) => {
            return ApiError::internal_logged("get workflow source failed", &e).into_response();
        }
    };

    // Resolve `name` to a package UUID. Mirrors get_workflow: a UUID path is
    // used directly; otherwise scan the workflow list by package name.
    let package_id = if let Ok(pkg_id) = uuid::Uuid::parse_str(&name) {
        pkg_id
    } else {
        match registry.list_workflows().await {
            Ok(workflows) => match workflows.into_iter().find(|w| w.package_name == name) {
                Some(w) => w.id,
                None => {
                    return ApiError::not_found(
                        "workflow_not_found",
                        format!("workflow '{}' not found", name),
                    )
                    .into_response();
                }
            },
            Err(e) => {
                return ApiError::internal_logged("get workflow source failed", &e).into_response();
            }
        }
    };

    match registry.get_workflow_source(package_id).await {
        Ok(Some((metadata, files))) => Json(WorkflowSourceResponse {
            tenant_id,
            id: metadata.id.to_string(),
            package_name: metadata.package_name,
            workflow_name: metadata.workflow_name,
            version: metadata.version,
            files: files
                .into_iter()
                .map(|f| WorkflowSourceFile {
                    language: detect_source_language(&f.path),
                    path: f.path,
                    contents: f.contents,
                })
                .collect(),
        })
        .into_response(),
        Ok(None) => ApiError::not_found(
            "workflow_not_found",
            format!("workflow '{}' not found", name),
        )
        .into_response(),
        Err(e) => {
            warn!(
                "Failed to read source for workflow '{}' (tenant '{}'): {}",
                name, tenant_id, e
            );
            ApiError::internal_logged("get workflow source failed", &e).into_response()
        }
    }
}

/// Best-effort language id from a file extension, for client-side syntax
/// highlighting. `None` when the extension is unknown.
fn detect_source_language(path: &str) -> Option<String> {
    let ext = std::path::Path::new(path).extension()?.to_str()?;
    let lang = match ext {
        "rs" => "rust",
        "py" => "python",
        "pyi" => "python",
        "toml" => "toml",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "md" => "markdown",
        "txt" => "text",
        "cfg" | "ini" => "ini",
        "sh" => "shell",
        _ => return None,
    };
    Some(lang.to_string())
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
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name, version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    let tenant_db: cloacina::database::Database = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal_logged("tenant database error", &e).into_response();
        }
    };
    let storage = UnifiedRegistryStorage::new(tenant_db.clone());
    let mut registry = match WorkflowRegistryImpl::new(storage, tenant_db) {
        Ok(r) => r,
        Err(e) => return ApiError::internal_logged("delete workflow failed", &e).into_response(),
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
            audit_record::note(
                Some(&tenant_id),
                AuditEvent::ok(audit_record::actions::PACKAGE_DELETE, name.as_str())
                    .detail(serde_json::json!({ "version": version })),
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

/// POST /tenants/:tenant_id/workflows/:name/pause — pause a workflow (CLOACI-T-0749).
///
/// Blocks new executions of the workflow (manual and triggered) until resumed.
/// In-flight executions are unaffected. `name` may be the workflow name or
/// package name.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/pause",
    tag = "workflows",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Workflow or package name"),
    ),
    responses(
        (status = 200, description = "Workflow paused", body = WorkflowPauseResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or role denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Workflow not found", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn pause_workflow(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    set_workflow_paused(state, _auth, tenant_id, name, true).await
}

/// POST /tenants/:tenant_id/workflows/:name/resume — resume a paused workflow
/// (CLOACI-T-0749). New executions are accepted again.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/resume",
    tag = "workflows",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Workflow or package name"),
    ),
    responses(
        (status = 200, description = "Workflow resumed", body = WorkflowPauseResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or role denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Workflow not found", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn resume_workflow(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    set_workflow_paused(state, _auth, tenant_id, name, false).await
}

/// Shared pause/resume implementation for workflows (CLOACI-T-0749).
async fn set_workflow_paused(
    state: AppState,
    _auth: AuthenticatedKey,
    tenant_id: String,
    name: String,
    pause: bool,
) -> axum::response::Response {
    let tenant_db: cloacina::database::Database = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal_logged("tenant database error", &e).into_response();
        }
    };
    let storage = UnifiedRegistryStorage::new(tenant_db.clone());
    let registry = match WorkflowRegistryImpl::new(storage, tenant_db) {
        Ok(r) => r,
        Err(e) => {
            return ApiError::internal_logged("set workflow paused failed", &e).into_response();
        }
    };

    match registry.set_workflow_paused(&name, pause).await {
        Ok(Some(id)) => {
            info!(
                "{} workflow '{}' for tenant '{}'",
                if pause { "Paused" } else { "Resumed" },
                name,
                tenant_id
            );
            let action = if pause {
                audit_record::actions::WORKFLOW_PAUSE
            } else {
                audit_record::actions::WORKFLOW_RESUME
            };
            audit_record::note(
                Some(&tenant_id),
                AuditEvent::ok(action, name.as_str())
                    .detail(serde_json::json!({ "package_id": id.to_string() })),
            );
            Json(WorkflowPauseResponse {
                tenant_id,
                id: id.to_string(),
                name,
                status: if pause { "paused" } else { "resumed" }.to_string(),
                paused: pause,
            })
            .into_response()
        }
        Ok(None) => ApiError::not_found(
            "workflow_not_found",
            format!("workflow '{}' not found", name),
        )
        .into_response(),
        Err(e) => {
            warn!(
                "Failed to {} workflow '{}' for tenant '{}': {}",
                if pause { "pause" } else { "resume" },
                name,
                tenant_id,
                e
            );
            ApiError::internal_logged("set workflow paused failed", &e).into_response()
        }
    }
}

/// GET /tenants/:tenant_id/workflows/:name/versions — every stored version of
/// a package (CLOACINA-T-1018).
///
/// Newest upload first. Includes the active version, versions that wait for a
/// build or whose build failed, and superseded versions. `name` is the package
/// name.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/versions",
    tag = "workflows",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Package name"),
    ),
    responses(
        (status = 200, description = "Versions of the package, newest upload first", body = TenantListResponse<WorkflowVersionSummary>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "No version of this package exists", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_workflow_versions(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let tenant_db: cloacina::database::Database = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal_logged("tenant database error", &e).into_response();
        }
    };
    let storage = UnifiedRegistryStorage::new(tenant_db.clone());
    let registry = match WorkflowRegistryImpl::new(storage, tenant_db) {
        Ok(r) => r,
        Err(e) => {
            return ApiError::internal_logged("list workflow versions failed", &e).into_response();
        }
    };

    match registry.list_package_versions(&name).await {
        Ok(versions) if versions.is_empty() => ApiError::not_found(
            "workflow_not_found",
            format!("package '{}' has no versions", name),
        )
        .into_response(),
        Ok(versions) => {
            let items: Vec<WorkflowVersionSummary> = versions
                .into_iter()
                .map(|v| WorkflowVersionSummary {
                    id: v.id.to_string(),
                    package_name: v.package_name,
                    version: v.version,
                    active: v.active,
                    superseded: v.superseded,
                    build_status: v.build_status,
                    build_error: v.build_error,
                    content_hash: v.content_hash,
                    created_at: v.created_at.to_rfc3339(),
                    compiled_at: v.compiled_at.map(|t| t.to_rfc3339()),
                })
                .collect();
            Json(TenantListResponse::new(tenant_id, items)).into_response()
        }
        Err(e) => {
            warn!(
                "Failed to list versions of '{}' for tenant '{}': {}",
                name, tenant_id, e
            );
            ApiError::internal_logged("list workflow versions failed", &e).into_response()
        }
    }
}

/// POST /tenants/:tenant_id/workflows/:name/versions/:version/activate — make
/// an earlier built version the active one (CLOACINA-T-1018).
///
/// No rebuild: the stored build of that version is put back in service and the
/// version that was active is superseded. A version that waits for a build is
/// abandoned. Executions that already run stay on the version they started
/// with. Idempotent for the version that is already active.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/workflows/{name}/versions/{version}/activate",
    tag = "workflows",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Package name"),
        ("version" = String, Path, description = "Version to make active"),
    ),
    responses(
        (status = 200, description = "The version is active", body = WorkflowVersionActivatedResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or role denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "No such version of the package", body = cloacina_api_types::ErrorBody),
        (status = 409, description = "The version has no successful build", body = cloacina_api_types::ErrorBody),
        (status = 500, description = "Internal error", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn activate_workflow_version(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, name, version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    let tenant_db: cloacina::database::Database = match state
        .tenant_databases
        .resolve(&tenant_id, &state.database)
        .await
    {
        Ok(db) => db,
        Err(e) => {
            return ApiError::internal_logged("tenant database error", &e).into_response();
        }
    };
    let storage = UnifiedRegistryStorage::new(tenant_db.clone());
    let registry = match WorkflowRegistryImpl::new(storage, tenant_db) {
        Ok(r) => r,
        Err(e) => {
            return ApiError::internal_logged("activate workflow version failed", &e)
                .into_response();
        }
    };

    use cloacina::registry::error::RegistryError;
    match registry.activate_package_version(&name, &version).await {
        Ok(id) => {
            info!(
                "Activated version {} of package '{}' for tenant '{}'",
                version, name, tenant_id
            );
            let detail = serde_json::json!({ "version": version, "package_id": id.to_string() });
            let event = AuditEvent::ok(audit_record::actions::PACKAGE_ACTIVATE, name.as_str());
            audit_record::note(Some(&tenant_id), event.detail(detail));
            Json(WorkflowVersionActivatedResponse {
                tenant_id,
                id: id.to_string(),
                package_name: name,
                version,
                status: "active".to_string(),
            })
            .into_response()
        }
        Err(e @ RegistryError::PackageNotFound { .. }) => {
            ApiError::not_found("workflow_version_not_found", format!("{}", e)).into_response()
        }
        Err(RegistryError::ValidationError { reason }) => {
            ApiError::conflict("workflow_version_not_built", reason).into_response()
        }
        Err(e) => {
            warn!(
                "Failed to activate version {} of '{}' for tenant '{}': {}",
                version, name, tenant_id, e
            );
            ApiError::internal_logged("activate workflow version failed", &e).into_response()
        }
    }
}

/// 413 for an upload over the server's package size limit.
fn package_too_large(size: Option<u64>, limit: u64) -> ApiError {
    ApiError::new(
        StatusCode::PAYLOAD_TOO_LARGE,
        "package_too_large",
        crate::upload_limit::too_large_message(size, limit),
    )
}

/// Why the file of an upload could not be read.
enum FileFieldError {
    /// The body is larger than the server's limit.
    TooLarge,
    /// The request is not a multipart upload with a `file` field.
    Invalid(String),
}

/// Extract the first file field from a multipart request.
async fn extract_file_field(multipart: &mut Multipart) -> Result<Vec<u8>, FileFieldError> {
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) if e.status() == StatusCode::PAYLOAD_TOO_LARGE => {
                return Err(FileFieldError::TooLarge);
            }
            Err(_) => break,
        };
        if field.name() == Some("file") {
            return match field.bytes().await {
                Ok(data) => Ok(data.to_vec()),
                Err(e) if e.status() == StatusCode::PAYLOAD_TOO_LARGE => {
                    Err(FileFieldError::TooLarge)
                }
                Err(e) => Err(FileFieldError::Invalid(format!(
                    "failed to read file: {}",
                    e
                ))),
            };
        }
    }
    Err(FileFieldError::Invalid(
        "no 'file' field in multipart request".to_string(),
    ))
}

#[cfg(test)]
mod upload_error_tests {
    use super::upload_error;
    use axum::http::StatusCode;
    use cloacina::registry::error::RegistryError;

    /// CLOACINA-T-1097: a Python package with dependencies and no valid lock
    /// file has its own code, and its message is the check's text.
    #[test]
    fn python_dependencies_not_locked_has_its_own_code() {
        let reason = "package `demo` declares dependencies in pyproject.toml (six) and has no \
                      requirements.lock and no vendor/ folder."
            .to_string();
        let e = upload_error(&RegistryError::PythonDependenciesNotLocked {
            reason: reason.clone(),
        });
        assert_eq!(e.status, StatusCode::BAD_REQUEST);
        assert_eq!(e.code, "python_dependencies_not_locked");
        assert_eq!(e.message, reason);
    }

    #[test]
    fn other_registration_errors_stay_upload_failed() {
        let e = upload_error(&RegistryError::ValidationError {
            reason: "bad manifest".to_string(),
        });
        assert_eq!(e.status, StatusCode::BAD_REQUEST);
        assert_eq!(e.code, "upload_failed");
    }
}
