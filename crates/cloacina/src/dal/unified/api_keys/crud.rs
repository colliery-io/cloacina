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
}

/// Diesel model for inserting api_keys rows.
#[derive(Insertable)]
#[diesel(table_name = api_keys)]
struct NewApiKey {
    pub id: Uuid,
    pub key_hash: String,
    pub name: String,
    pub permissions: String,
}

fn to_info(row: ApiKeyRow) -> ApiKeyInfo {
    ApiKeyInfo {
        id: row.id,
        name: row.name,
        permissions: row.permissions,
        created_at: row.created_at.and_utc(),
        revoked: row.revoked_at.is_some(),
    }
}

pub async fn create_key(
    dal: &DAL,
    key_hash: &str,
    name: &str,
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
        permissions: "admin".to_string(),
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
    let result: Option<ApiKeyRow> = conn
        .interact(move |conn| {
            api_keys::table
                .filter(api_keys::key_hash.eq(&hash))
                .filter(api_keys::revoked_at.is_null())
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
