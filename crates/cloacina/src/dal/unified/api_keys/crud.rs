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

//! Postgres CRUD operations for api_keys table.

use chrono::Utc;
use diesel::prelude::*;
use uuid::Uuid;

use super::ApiKeyInfo;
use crate::dal::unified::DAL;
use crate::database::schema::postgres::api_keys;
use crate::error::ValidationError;

/// Diesel model for reading api_keys rows.
#[derive(Queryable, Debug)]
#[diesel(table_name = api_keys)]
struct ApiKeyRow {
    pub id: Uuid,
    #[allow(dead_code)]
    pub key_hash: String,
    pub name: String,
    pub permissions: String,
    pub created_at: chrono::NaiveDateTime,
    pub revoked_at: Option<chrono::NaiveDateTime>,
    pub tenant_id: Option<String>,
    pub is_admin: bool,
    pub expires_at: Option<chrono::NaiveDateTime>,
    pub issued_via: Option<String>,
}

/// Diesel model for inserting api_keys rows.
#[derive(Insertable)]
#[diesel(table_name = api_keys)]
struct NewApiKey {
    pub id: Uuid,
    pub key_hash: String,
    pub name: String,
    pub permissions: String,
    pub tenant_id: Option<String>,
    pub is_admin: bool,
}

fn to_info(row: ApiKeyRow) -> ApiKeyInfo {
    ApiKeyInfo {
        id: row.id,
        name: row.name,
        permissions: row.permissions,
        created_at: row.created_at.and_utc(),
        revoked: row.revoked_at.is_some(),
        tenant_id: row.tenant_id,
        is_admin: row.is_admin,
        expires_at: row.expires_at.map(|t| t.and_utc()),
        issued_via: row.issued_via,
    }
}

pub async fn create_key(
    dal: &DAL,
    key_hash: &str,
    name: &str,
    tenant_id: Option<&str>,
    is_admin: bool,
    role: &str,
) -> Result<ApiKeyInfo, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let id = Uuid::new_v4();
    let new_key = NewApiKey {
        id,
        key_hash: key_hash.to_string(),
        name: name.to_string(),
        permissions: role.to_string(),
        tenant_id: tenant_id.map(|s| s.to_string()),
        is_admin,
    };

    let row: ApiKeyRow = conn
        .interact(move |conn| {
            diesel::insert_into(api_keys::table)
                .values(&new_key)
                .get_result(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(to_info(row))
}

/// Insertable for a minted (short-TTL, provenance-tagged) key (CLOACI-T-0792).
#[derive(Insertable)]
#[diesel(table_name = api_keys)]
struct MintApiKey {
    pub id: Uuid,
    pub key_hash: String,
    pub name: String,
    pub permissions: String,
    pub tenant_id: Option<String>,
    pub is_admin: bool,
    pub expires_at: Option<chrono::NaiveDateTime>,
    pub issued_via: Option<String>,
}

/// CLOACI-T-0792: mint a short-TTL, provenance-tagged key (OIDC / local login).
/// Like `create_key` but sets `expires_at` + `issued_via`; `is_admin` is always
/// false — minted keys are never god-mode. The minted row is an ordinary
/// `api_keys` row that flows through `validate_hash` + the bearer path unchanged
/// (and is rejected once `expires_at` passes).
#[allow(clippy::too_many_arguments)]
pub async fn mint_key(
    dal: &DAL,
    key_hash: &str,
    name: &str,
    tenant_id: Option<&str>,
    role: &str,
    expires_at: chrono::DateTime<Utc>,
    issued_via: &str,
) -> Result<ApiKeyInfo, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let new_key = MintApiKey {
        id: Uuid::new_v4(),
        key_hash: key_hash.to_string(),
        name: name.to_string(),
        permissions: role.to_string(),
        tenant_id: tenant_id.map(|s| s.to_string()),
        is_admin: false,
        expires_at: Some(expires_at.naive_utc()),
        issued_via: Some(issued_via.to_string()),
    };

    let row: ApiKeyRow = conn
        .interact(move |conn| {
            diesel::insert_into(api_keys::table)
                .values(&new_key)
                .get_result(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(to_info(row))
}

pub async fn validate_hash(
    dal: &DAL,
    key_hash: &str,
) -> Result<Option<ApiKeyInfo>, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let hash = key_hash.to_string();
    // CLOACI-T-0792: reject expired minted keys (NULL expiry = never expires).
    let now = Utc::now().naive_utc();
    let result: Option<ApiKeyRow> = conn
        .interact(move |conn| {
            api_keys::table
                .filter(api_keys::key_hash.eq(&hash))
                .filter(api_keys::revoked_at.is_null())
                .filter(
                    api_keys::expires_at
                        .is_null()
                        .or(api_keys::expires_at.gt(now)),
                )
                .first(conn)
                .optional()
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(result.map(to_info))
}

pub async fn has_any_keys(dal: &DAL) -> Result<bool, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let result: Option<ApiKeyRow> = conn
        .interact(move |conn| {
            api_keys::table
                .filter(api_keys::revoked_at.is_null())
                .first(conn)
                .optional()
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(result.is_some())
}

pub async fn list_keys(dal: &DAL) -> Result<Vec<ApiKeyInfo>, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let results: Vec<ApiKeyRow> = conn
        .interact(move |conn| {
            api_keys::table
                .order(api_keys::created_at.desc())
                .load(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(results.into_iter().map(to_info).collect())
}

/// CLOACI-T-0784: list keys scoped to a single tenant (tenant-admin view).
/// Includes revoked rows (like `list_keys`); the caller renders the `revoked`
/// flag.
pub async fn list_keys_for_tenant(
    dal: &DAL,
    tenant_id: &str,
) -> Result<Vec<ApiKeyInfo>, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let tenant_owned = tenant_id.to_string();
    let results: Vec<ApiKeyRow> = conn
        .interact(move |conn| {
            api_keys::table
                .filter(api_keys::tenant_id.eq(Some(tenant_owned)))
                .order(api_keys::created_at.desc())
                .load(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(results.into_iter().map(to_info).collect())
}

/// CLOACI-T-0784: fetch a single key's info by id (used by the tenant-scoped
/// revoke path to verify the target key belongs to the caller's tenant before
/// revoking). Returns `None` if no such key exists.
pub async fn get_key(dal: &DAL, id: Uuid) -> Result<Option<ApiKeyInfo>, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let result: Option<ApiKeyRow> = conn
        .interact(move |conn| api_keys::table.find(id).first(conn).optional())
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(result.map(to_info))
}

/// CLOACI-T-0581: bulk-revoke every still-active key bound to `tenant_id`.
/// Returns the number of rows updated. Used by tenant teardown to close
/// out the auth surface before the schema is dropped.
pub async fn revoke_keys_for_tenant(dal: &DAL, tenant_id: &str) -> Result<usize, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let now = Utc::now().naive_utc();
    let tenant_owned = tenant_id.to_string();
    let rows: usize = conn
        .interact(move |conn| {
            diesel::update(
                api_keys::table
                    .filter(api_keys::tenant_id.eq(Some(tenant_owned)))
                    .filter(api_keys::revoked_at.is_null()),
            )
            .set(api_keys::revoked_at.eq(Some(now)))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(rows)
}

pub async fn revoke_key(dal: &DAL, id: Uuid) -> Result<bool, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let now = Utc::now().naive_utc();
    let rows: usize = conn
        .interact(move |conn| {
            diesel::update(
                api_keys::table
                    .find(id)
                    .filter(api_keys::revoked_at.is_null()),
            )
            .set(api_keys::revoked_at.eq(Some(now)))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(rows > 0)
}
