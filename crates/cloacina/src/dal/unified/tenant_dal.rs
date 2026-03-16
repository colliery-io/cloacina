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

//! DAL for tenant operations (auth system).

use super::models::{NewTenant, TenantRow};
use super::DAL;
use crate::database::schema::unified::tenants;
use crate::database::universal_types::UniversalUuid;
use diesel::prelude::*;

/// Data access layer for tenant operations.
#[derive(Clone)]
pub struct TenantDAL<'a> {
    dal: &'a DAL,
}

impl<'a> TenantDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Create a new tenant.
    pub async fn create(&self, new_tenant: NewTenant) -> Result<(), String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_tenant.clone()).await,
            self.create_sqlite(new_tenant).await
        )
    }

    /// List all active tenants.
    pub async fn list(&self) -> Result<Vec<TenantRow>, String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_postgres().await,
            self.list_sqlite().await
        )
    }

    /// Get a tenant by ID.
    pub async fn get(&self, id: UniversalUuid) -> Result<Option<TenantRow>, String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_postgres(id.clone()).await,
            self.get_sqlite(id).await
        )
    }

    /// Get a tenant by name.
    pub async fn get_by_name(&self, name: &str) -> Result<Option<TenantRow>, String> {
        let name = name.to_string();
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_name_postgres(name.clone()).await,
            self.get_by_name_sqlite(name).await
        )
    }

    /// Deactivate a tenant (set status to 'deactivated').
    pub async fn deactivate(&self, id: UniversalUuid) -> Result<(), String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.deactivate_postgres(id.clone()).await,
            self.deactivate_sqlite(id).await
        )
    }

    // --- Postgres implementations ---

    #[cfg(feature = "postgres")]
    async fn create_postgres(&self, new_tenant: NewTenant) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::insert_into(tenants::table)
                .values(&new_tenant)
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn list_postgres(&self) -> Result<Vec<TenantRow>, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            tenants::table
                .filter(tenants::status.eq("active"))
                .load::<TenantRow>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn get_postgres(&self, id: UniversalUuid) -> Result<Option<TenantRow>, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            tenants::table
                .filter(tenants::id.eq(&id))
                .first::<TenantRow>(conn)
                .optional()
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn get_by_name_postgres(&self, name: String) -> Result<Option<TenantRow>, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            tenants::table
                .filter(tenants::name.eq(&name))
                .first::<TenantRow>(conn)
                .optional()
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn deactivate_postgres(&self, id: UniversalUuid) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::update(tenants::table.filter(tenants::id.eq(&id)))
                .set(tenants::status.eq("deactivated"))
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    // --- SQLite implementations ---

    #[cfg(feature = "sqlite")]
    async fn create_sqlite(&self, new_tenant: NewTenant) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::insert_into(tenants::table)
                .values(&new_tenant)
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn list_sqlite(&self) -> Result<Vec<TenantRow>, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            tenants::table
                .filter(tenants::status.eq("active"))
                .load::<TenantRow>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn get_sqlite(&self, id: UniversalUuid) -> Result<Option<TenantRow>, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            tenants::table
                .filter(tenants::id.eq(&id))
                .first::<TenantRow>(conn)
                .optional()
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn get_by_name_sqlite(&self, name: String) -> Result<Option<TenantRow>, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            tenants::table
                .filter(tenants::name.eq(&name))
                .first::<TenantRow>(conn)
                .optional()
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn deactivate_sqlite(&self, id: UniversalUuid) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::update(tenants::table.filter(tenants::id.eq(&id)))
                .set(tenants::status.eq("deactivated"))
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
