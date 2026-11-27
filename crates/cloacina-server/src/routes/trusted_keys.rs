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

//! Trusted keys of package signing, per tenant (CLOACINA-A-0018,
//! CLOACINA-T-1061).
//!
//! A tenant's trusted keys are the rows of `trusted_keys` in the admin schema
//! whose `org_id` is [`cloacina::security::tenant_org_id`] of the tenant. With
//! signatures required, an upload to the tenant passes when its signer is a
//! live key of that org, or of the server-wide org if one is set
//! (`routes/workflows.rs`).
//!
//! All three routes need `trusted_keys.manage` in the tenant of the path: a
//! tenant admin for its own tenant, a platform admin for any tenant.

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use tracing::warn;

use cloacina::security::{DbKeyManager, KeyError, KeyManager, TrustedKeyInfo, tenant_org_id};
use cloacina_api_types::{
    AddTrustedKeyRequest, ListResponse, TrustedKeyRevokedResponse, TrustedKeySummary,
};

use crate::AppState;
use crate::routes::audit::{self as audit_record, AuditEvent};
use crate::routes::auth::AuthenticatedKey;
use crate::routes::error::ApiError;

/// The longest name the `key_name` column holds.
const MAX_NAME_LEN: usize = 255;

fn key_manager(state: &AppState) -> DbKeyManager {
    DbKeyManager::new(cloacina::dal::DAL::new(state.database.clone()))
}

fn to_summary(key: TrustedKeyInfo) -> TrustedKeySummary {
    TrustedKeySummary {
        id: key.id.to_string(),
        fingerprint: key.fingerprint,
        name: key.key_name,
        trusted_at: key.trusted_at.to_rfc3339(),
        revoked: key.revoked_at.is_some(),
        revoked_at: key.revoked_at.map(|t| t.to_rfc3339()),
    }
}

/// The 32 raw bytes of the key of a request: from the PEM or from the hex
/// form, exactly one of which must be there.
fn public_key_of(body: &AddTrustedKeyRequest) -> Result<Vec<u8>, ApiError> {
    let invalid = |message: String| ApiError::bad_request("invalid_public_key", message);
    match (body.public_key_pem.as_deref(), body.public_key.as_deref()) {
        (Some(pem), None) => {
            DbKeyManager::decode_public_key_pem(pem).map_err(|e| invalid(e.to_string()))
        }
        (None, Some(hex_key)) => {
            let bytes = hex::decode(hex_key.trim())
                .map_err(|_| invalid("public_key is not hexadecimal".to_string()))?;
            if bytes.len() != 32 {
                return Err(invalid(format!(
                    "public_key must be 32 bytes (64 hexadecimal characters), got {} bytes",
                    bytes.len()
                )));
            }
            Ok(bytes)
        }
        _ => Err(ApiError::bad_request(
            "invalid_request",
            "give exactly one of public_key_pem and public_key",
        )),
    }
}

fn exists(revoked: bool) -> ApiError {
    let message = if revoked {
        "this tenant revoked this key; a revoked key cannot be added again"
    } else {
        "this tenant already trusts this key"
    };
    ApiError::conflict("trusted_key_exists", message)
}

/// `POST /v1/tenants/{tenant_id}/trusted-keys` — add a public key to the
/// tenant's trusted keys.
#[utoipa::path(
    post,
    path = "/v1/tenants/{tenant_id}/trusted-keys",
    tag = "trusted-keys",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    request_body = AddTrustedKeyRequest,
    responses(
        (status = 201, description = "Key added", body = TrustedKeySummary),
        (status = 400, description = "No key, two keys, or a key that is not Ed25519", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
        (status = 409, description = "The tenant already has this key, live or revoked", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn add_trusted_key(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
    Json(body): Json<AddTrustedKeyRequest>,
) -> impl IntoResponse {
    let public_key = match public_key_of(&body) {
        Ok(k) => k,
        Err(e) => return e.into_response(),
    };
    let name = body
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty());
    if name.is_some_and(|n| n.len() > MAX_NAME_LEN) {
        return ApiError::bad_request(
            "invalid_request",
            format!("name is longer than {MAX_NAME_LEN} bytes"),
        )
        .into_response();
    }

    let org_id = tenant_org_id(&tenant_id);
    let km = key_manager(&state);

    // The table holds one row for a key in an org, revoked or not.
    let fingerprint = cloacina::crypto::compute_key_fingerprint(&public_key);
    match km.list_org_trusted_keys(org_id).await {
        Ok(keys) => {
            if let Some(known) = keys.iter().find(|k| k.fingerprint == fingerprint) {
                return exists(known.revoked_at.is_some()).into_response();
            }
        }
        Err(e) => {
            warn!("trusted keys: list for tenant '{tenant_id}' failed: {e}");
            return ApiError::internal("failed to add trusted key").into_response();
        }
    }

    match km.trust_public_key(org_id, &public_key, name).await {
        Ok(key) => {
            audit_record::note(
                Some(&tenant_id),
                AuditEvent::ok(audit_record::actions::TRUSTED_KEY_ADD, key.id.to_string()).detail(
                    serde_json::json!({ "fingerprint": key.fingerprint, "name": key.key_name }),
                ),
            );
            (StatusCode::CREATED, Json(to_summary(key))).into_response()
        }
        // Two adds of one key at the same time: the second meets the unique
        // constraint.
        Err(KeyError::Database(e)) if e.to_lowercase().contains("unique") => {
            exists(false).into_response()
        }
        Err(KeyError::InvalidFormat(e)) => {
            ApiError::bad_request("invalid_public_key", e).into_response()
        }
        Err(e) => {
            warn!("trusted keys: add for tenant '{tenant_id}' failed: {e}");
            ApiError::internal("failed to add trusted key").into_response()
        }
    }
}

/// `GET /v1/tenants/{tenant_id}/trusted-keys` — the tenant's trusted keys,
/// the revoked ones included and marked.
#[utoipa::path(
    get,
    path = "/v1/tenants/{tenant_id}/trusted-keys",
    tag = "trusted-keys",
    params(("tenant_id" = String, Path, description = "Tenant identifier")),
    responses(
        (status = 200, description = "The tenant's trusted keys, oldest first", body = ListResponse<TrustedKeySummary>),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn list_trusted_keys(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    match key_manager(&state)
        .list_org_trusted_keys(tenant_org_id(&tenant_id))
        .await
    {
        Ok(keys) => {
            let items: Vec<TrustedKeySummary> = keys.into_iter().map(to_summary).collect();
            Json(ListResponse::new(items)).into_response()
        }
        Err(e) => {
            warn!("trusted keys: list for tenant '{tenant_id}' failed: {e}");
            ApiError::internal("failed to list trusted keys").into_response()
        }
    }
}

/// `DELETE /v1/tenants/{tenant_id}/trusted-keys/{key_id}` — revoke a trusted
/// key of the tenant. The key of another tenant is not found, and is not
/// revoked.
#[utoipa::path(
    delete,
    path = "/v1/tenants/{tenant_id}/trusted-keys/{key_id}",
    tag = "trusted-keys",
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("key_id" = String, Path, description = "Trusted key UUID"),
    ),
    responses(
        (status = 200, description = "Key revoked", body = TrustedKeyRevokedResponse),
        (status = 400, description = "Invalid key ID format", body = cloacina_api_types::ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = cloacina_api_types::ErrorBody),
        (status = 403, description = "Tenant access or admin role denied", body = cloacina_api_types::ErrorBody),
        (status = 404, description = "No such trusted key in this tenant", body = cloacina_api_types::ErrorBody),
    ),
    security(("api_key" = []))
)]
pub async fn revoke_trusted_key(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path((tenant_id, key_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let id = match uuid::Uuid::parse_str(&key_id) {
        Ok(id) => cloacina::UniversalUuid::from(id),
        Err(_) => {
            return ApiError::bad_request("invalid_key_id", "invalid key ID format")
                .into_response();
        }
    };

    // The org is part of the revoke (CLOACINA-T-1037): the key of another
    // tenant matches no row.
    match key_manager(&state)
        .revoke_trusted_key(tenant_org_id(&tenant_id), id)
        .await
    {
        Ok(()) => {
            audit_record::note(
                Some(&tenant_id),
                AuditEvent::ok(audit_record::actions::TRUSTED_KEY_REVOKE, key_id.as_str()),
            );
            Json(TrustedKeyRevokedResponse {
                status: "revoked".to_string(),
                id: key_id,
            })
            .into_response()
        }
        Err(KeyError::NotFound(_)) => ApiError::not_found(
            "trusted_key_not_found",
            "trusted key not found in this tenant",
        )
        .into_response(),
        Err(e) => {
            warn!("trusted keys: revoke '{key_id}' for tenant '{tenant_id}' failed: {e}");
            ApiError::internal("failed to revoke trusted key").into_response()
        }
    }
}
