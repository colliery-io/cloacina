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

//! Tenant secrets CRUD endpoints (CLOACI-I-0133 / T-0862).
//!
//! Tenant-scoped, metadata-only reads (REQ-002 / NFR-001): create/rotate accept
//! a `{field: value}` map in the request body, but **no read endpoint ever
//! returns a plaintext or ciphertext value** — list/get return names +
//! timestamps only. The at-rest encryption + per-tenant DEK live in
//! [`cloacina::security::SecretStore`]; these handlers are the HTTP surface over
//! it, keyed by the tenant's derived `org_id` ([`crate::secrets::tenant_org_id`]).
//!
//! All routes are `TenantParam + Admin` in `build_authz_table`, so the caller is
//! already confined to `{tenant_id}` and holds admin before a handler runs.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use tracing::warn;

use cloacina::security::{SecretError, SecretMetadata, SecretStore, SecretStoreResolver};
use cloacina_api_types::{
    CreateSecretRequest, ListResponse, RotateSecretRequest, SecretDeletedResponse,
    SecretMetadataResponse,
};

use crate::routes::error::ApiError;
use crate::AppState;

/// Project the engine's [`SecretMetadata`] into the wire type — names +
/// timestamps only, never a value.
fn to_metadata_response(m: SecretMetadata) -> SecretMetadataResponse {
    SecretMetadataResponse {
        id: m.id.to_string(),
        name: m.name,
        field_names: m.field_names,
        created_at: m.created_at.to_rfc3339(),
        updated_at: m.updated_at.to_rfc3339(),
    }
}

/// Read + parse the server KEK, or a 503 the caller can surface as
/// "secrets not configured on this deployment".
fn server_kek() -> Result<Vec<u8>, ApiError> {
    SecretStoreResolver::kek_from_env().map_err(|e| {
        warn!("secrets: server KEK unavailable: {e}");
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "secrets_not_configured",
            "secrets are not configured on this server (CLOACINA_SECRET_KEK unset or invalid)",
        )
    })
}

/// Build a tenant-scoped [`SecretStore`] over the tenant's own schema.
async fn tenant_store(state: &AppState, tenant_id: &str) -> Result<SecretStore, ApiError> {
    let db = state
        .tenant_databases
        .resolve(tenant_id, &state.database)
        .await
        .map_err(|e| {
            warn!("secrets: could not open tenant '{tenant_id}' database: {e}");
            ApiError::internal("failed to open tenant database")
        })?;
    Ok(SecretStore::new(cloacina::dal::DAL::new(db)))
}

/// Map a [`SecretError`] onto the canonical HTTP envelopes. Messages here are
/// non-plaintext by construction (the store never puts a value in an error).
fn map_secret_error(e: SecretError) -> ApiError {
    match e {
        SecretError::NotFound(name) => {
            ApiError::not_found("secret_not_found", format!("secret not found: {name}"))
        }
        SecretError::DuplicateName(name) => ApiError::new(
            StatusCode::CONFLICT,
            "secret_exists",
            format!("a secret named '{name}' already exists in this tenant"),
        ),
        other => {
            warn!("secrets: store error: {other}");
            ApiError::internal("secret store error")
        }
    }
}

/// `POST /v1/tenants/{tenant_id}/secrets` — create a secret from a field map.
/// Returns metadata only.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/secrets",
    tag = "secrets",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    request_body = CreateSecretRequest,
    responses(
        (status = 201, description = "Secret created — metadata only (no values)", body = SecretMetadataResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
        (status = 409, description = "A secret of that name already exists", body = cloacina_api_types::ErrorBody),
        (status = 503, description = "Secrets not configured on this server", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn create_secret(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Json(body): Json<CreateSecretRequest>,
) -> impl IntoResponse {
    let kek = match server_kek() {
        Ok(k) => k,
        Err(e) => return e.into_response(),
    };
    let store = match tenant_store(&state, &tenant_id).await {
        Ok(s) => s,
        Err(e) => return e.into_response(),
    };
    let org_id = crate::secrets::tenant_org_id(&tenant_id);

    match store
        .create_secret(org_id, &body.name, &body.fields, &kek)
        .await
    {
        Ok(meta) => (StatusCode::CREATED, Json(to_metadata_response(meta))).into_response(),
        Err(e) => map_secret_error(e).into_response(),
    }
}

/// `PUT /v1/tenants/{tenant_id}/secrets/{name}` — rotate a secret's values in
/// place (D-8/OQ-5). Returns metadata only; the next fire sees the new value.
#[utoipa::path(
    put,
    path = "/v1/tenants/{tenant_id}/secrets/{name}",
    tag = "secrets",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Secret name"),
    ),
    request_body = RotateSecretRequest,
    responses(
        (status = 200, description = "Secret rotated — metadata only (no values)", body = SecretMetadataResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Secret not found in this tenant", body = cloacina_api_types::ErrorBody),
        (status = 503, description = "Secrets not configured on this server", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn rotate_secret(
    State(state): State<AppState>,
    Path((tenant_id, name)): Path<(String, String)>,
    Json(body): Json<RotateSecretRequest>,
) -> impl IntoResponse {
    let kek = match server_kek() {
        Ok(k) => k,
        Err(e) => return e.into_response(),
    };
    let store = match tenant_store(&state, &tenant_id).await {
        Ok(s) => s,
        Err(e) => return e.into_response(),
    };
    let org_id = crate::secrets::tenant_org_id(&tenant_id);

    match store.rotate_secret(org_id, &name, &body.fields, &kek).await {
        Ok(meta) => Json(to_metadata_response(meta)).into_response(),
        Err(e) => map_secret_error(e).into_response(),
    }
}

/// `GET /v1/tenants/{tenant_id}/secrets` — list secret metadata. **No values.**
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/secrets",
    tag = "secrets",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    responses(
        (status = 200, description = "Secret metadata (names + timestamps only)", body = ListResponse<SecretMetadataResponse>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
        (status = 503, description = "Secrets not configured on this server", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_secrets(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    // list needs no KEK (metadata is stored in plaintext columns), but we still
    // require secrets to be configured so behavior is consistent with the rest.
    if let Err(e) = server_kek() {
        return e.into_response();
    }
    let store = match tenant_store(&state, &tenant_id).await {
        Ok(s) => s,
        Err(e) => return e.into_response(),
    };
    let org_id = crate::secrets::tenant_org_id(&tenant_id);

    match store.list_secrets_metadata(org_id).await {
        Ok(list) => {
            let items: Vec<SecretMetadataResponse> =
                list.into_iter().map(to_metadata_response).collect();
            Json(ListResponse::new(items)).into_response()
        }
        Err(e) => map_secret_error(e).into_response(),
    }
}

/// `GET /v1/tenants/{tenant_id}/secrets/{name}` — one secret's metadata. **No values.**
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/secrets/{name}",
    tag = "secrets",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Secret name"),
    ),
    responses(
        (status = 200, description = "Secret metadata (names + timestamps only)", body = SecretMetadataResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Secret not found in this tenant", body = cloacina_api_types::ErrorBody),
        (status = 503, description = "Secrets not configured on this server", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn get_secret(
    State(state): State<AppState>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Err(e) = server_kek() {
        return e.into_response();
    }
    let store = match tenant_store(&state, &tenant_id).await {
        Ok(s) => s,
        Err(e) => return e.into_response(),
    };
    let org_id = crate::secrets::tenant_org_id(&tenant_id);

    match store.get_secret_metadata(org_id, &name).await {
        Ok(meta) => Json(to_metadata_response(meta)).into_response(),
        Err(e) => map_secret_error(e).into_response(),
    }
}

/// `DELETE /v1/tenants/{tenant_id}/secrets/{name}` — delete a secret.
#[utoipa::path(
    delete,
    path = "/v1/tenants/{tenant_id}/secrets/{name}",
    tag = "secrets",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("name" = String, Path, description = "Secret name"),
    ),
    responses(
        (status = 200, description = "Secret deleted", body = SecretDeletedResponse),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "Secret not found in this tenant", body = cloacina_api_types::ErrorBody),
        (status = 503, description = "Secrets not configured on this server", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn delete_secret(
    State(state): State<AppState>,
    Path((tenant_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Err(e) = server_kek() {
        return e.into_response();
    }
    let store = match tenant_store(&state, &tenant_id).await {
        Ok(s) => s,
        Err(e) => return e.into_response(),
    };
    let org_id = crate::secrets::tenant_org_id(&tenant_id);

    match store.delete_secret(org_id, &name).await {
        Ok(()) => Json(SecretDeletedResponse {
            status: "deleted".to_string(),
            name,
        })
        .into_response(),
        Err(e) => map_secret_error(e).into_response(),
    }
}
