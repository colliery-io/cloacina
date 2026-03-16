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

//! DAL for API key operations (auth system).

use super::models::{ApiKeyRow, NewApiKey, NewWorkflowPattern, WorkflowPatternRow};
use super::DAL;
use crate::database::schema::unified::{api_key_workflow_patterns, api_keys};
use crate::database::universal_types::UniversalUuid;
use diesel::prelude::*;

/// Data access layer for API key operations.
#[derive(Clone)]
pub struct ApiKeyDAL<'a> {
    dal: &'a DAL,
}

impl<'a> ApiKeyDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Create a new API key.
    pub async fn create(&self, new_key: NewApiKey) -> Result<(), String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_key.clone()).await,
            self.create_sqlite(new_key).await
        )
    }

    /// Create workflow patterns in batch.
    pub async fn create_patterns(&self, patterns: Vec<NewWorkflowPattern>) -> Result<(), String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_patterns_postgres(patterns.clone()).await,
            self.create_patterns_sqlite(patterns).await
        )
    }

    /// Load API keys by prefix, along with their workflow patterns.
    pub async fn load_by_prefix(
        &self,
        prefix: &str,
    ) -> Result<Vec<(ApiKeyRow, Vec<WorkflowPatternRow>)>, String> {
        let prefix = prefix.to_string();
        crate::dispatch_backend!(
            self.dal.backend(),
            self.load_by_prefix_postgres(prefix.clone()).await,
            self.load_by_prefix_sqlite(prefix).await
        )
    }

    /// List API keys for a specific tenant.
    pub async fn list_by_tenant(&self, tenant_id: UniversalUuid) -> Result<Vec<ApiKeyRow>, String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_by_tenant_postgres(tenant_id.clone()).await,
            self.list_by_tenant_sqlite(tenant_id).await
        )
    }

    /// List all API keys (across all tenants).
    pub async fn list_all(&self) -> Result<Vec<ApiKeyRow>, String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_all_postgres().await,
            self.list_all_sqlite().await
        )
    }

    /// Revoke an API key by setting revoked_at to now.
    pub async fn revoke(&self, key_id: UniversalUuid) -> Result<(), String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.revoke_postgres(key_id.clone()).await,
            self.revoke_sqlite(key_id).await
        )
    }

    // --- Postgres implementations ---

    #[cfg(feature = "postgres")]
    async fn create_postgres(&self, new_key: NewApiKey) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::insert_into(api_keys::table)
                .values(&new_key)
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn create_patterns_postgres(
        &self,
        patterns: Vec<NewWorkflowPattern>,
    ) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::insert_into(api_key_workflow_patterns::table)
                .values(&patterns)
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn load_by_prefix_postgres(
        &self,
        prefix: String,
    ) -> Result<Vec<(ApiKeyRow, Vec<WorkflowPatternRow>)>, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            let keys: Vec<ApiKeyRow> = api_keys::table
                .filter(api_keys::key_prefix.eq(&prefix))
                .load::<ApiKeyRow>(conn)
                .map_err(|e| e.to_string())?;

            let key_ids: Vec<_> = keys.iter().map(|k| k.id.clone()).collect();

            let patterns: Vec<WorkflowPatternRow> = api_key_workflow_patterns::table
                .filter(api_key_workflow_patterns::api_key_id.eq_any(&key_ids))
                .load::<WorkflowPatternRow>(conn)
                .map_err(|e| e.to_string())?;

            let result = keys
                .into_iter()
                .map(|key| {
                    let key_patterns: Vec<WorkflowPatternRow> = patterns
                        .iter()
                        .filter(|p| p.api_key_id == key.id)
                        .cloned()
                        .collect();
                    (key, key_patterns)
                })
                .collect();

            Ok(result)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn list_by_tenant_postgres(
        &self,
        tenant_id: UniversalUuid,
    ) -> Result<Vec<ApiKeyRow>, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            api_keys::table
                .filter(api_keys::tenant_id.eq(&tenant_id))
                .load::<ApiKeyRow>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn list_all_postgres(&self) -> Result<Vec<ApiKeyRow>, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            api_keys::table
                .order(api_keys::created_at.desc())
                .load::<ApiKeyRow>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn revoke_postgres(&self, key_id: UniversalUuid) -> Result<(), String> {
        use crate::database::universal_types::UniversalTimestamp;

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            let now = UniversalTimestamp::now();
            diesel::update(api_keys::table.filter(api_keys::id.eq(&key_id)))
                .set(api_keys::revoked_at.eq(now))
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    // --- SQLite implementations ---

    #[cfg(feature = "sqlite")]
    async fn create_sqlite(&self, new_key: NewApiKey) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::insert_into(api_keys::table)
                .values(&new_key)
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn create_patterns_sqlite(
        &self,
        patterns: Vec<NewWorkflowPattern>,
    ) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            for pattern in &patterns {
                diesel::insert_into(api_key_workflow_patterns::table)
                    .values(pattern)
                    .execute(conn)
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn load_by_prefix_sqlite(
        &self,
        prefix: String,
    ) -> Result<Vec<(ApiKeyRow, Vec<WorkflowPatternRow>)>, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            let keys: Vec<ApiKeyRow> = api_keys::table
                .filter(api_keys::key_prefix.eq(&prefix))
                .load::<ApiKeyRow>(conn)
                .map_err(|e| e.to_string())?;

            let result = keys
                .into_iter()
                .map(|key| {
                    let patterns: Vec<WorkflowPatternRow> = api_key_workflow_patterns::table
                        .filter(api_key_workflow_patterns::api_key_id.eq(&key.id))
                        .load::<WorkflowPatternRow>(conn)
                        .map_err(|e| e.to_string())?;
                    Ok((key, patterns))
                })
                .collect::<Result<Vec<_>, String>>()?;

            Ok(result)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn list_by_tenant_sqlite(
        &self,
        tenant_id: UniversalUuid,
    ) -> Result<Vec<ApiKeyRow>, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            api_keys::table
                .filter(api_keys::tenant_id.eq(&tenant_id))
                .load::<ApiKeyRow>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn list_all_sqlite(&self) -> Result<Vec<ApiKeyRow>, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            api_keys::table
                .order(api_keys::created_at.desc())
                .load::<ApiKeyRow>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn revoke_sqlite(&self, key_id: UniversalUuid) -> Result<(), String> {
        use crate::database::universal_types::UniversalTimestamp;

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            let now = UniversalTimestamp::now();
            diesel::update(api_keys::table.filter(api_keys::id.eq(&key_id)))
                .set(api_keys::revoked_at.eq(now))
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
