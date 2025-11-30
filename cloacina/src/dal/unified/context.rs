/*
 *  Copyright 2025 Colliery Software
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

//! Unified Context DAL with runtime backend selection
//!
//! This module provides CRUD operations for Context entities that work with
//! both PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.

use super::DAL;
use crate::context::Context;
use crate::database::universal_types::UniversalUuid;
use crate::database::BackendType;
use crate::error::ContextError;
use diesel::prelude::*;
use tracing::warn;

/// Data access layer for context operations with runtime backend selection.
#[derive(Clone)]
pub struct ContextDAL<'a> {
    dal: &'a DAL,
}

impl<'a> ContextDAL<'a> {
    /// Creates a new ContextDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Create a new context in the database.
    ///
    /// This method serializes the provided context data to JSON and stores it in the database.
    /// Empty contexts (containing only whitespace or empty objects) are skipped.
    ///
    /// # Arguments
    ///
    /// * `context` - The context to be stored in the database
    ///
    /// # Returns
    ///
    /// * `Result<Option<UniversalUuid>, ContextError>` - The UUID of the created context if successful,
    ///   or None if the context was empty and skipped
    pub async fn create<T>(
        &self,
        context: &Context<T>,
    ) -> Result<Option<UniversalUuid>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
    {
        // Serialize the context data
        let value = context.to_json()?;

        // Skip insertion if context is empty or whitespace-only
        let trimmed_value = value
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>();
        if trimmed_value == "{}" {
            warn!("Skipping insertion of empty context");
            return Ok(None);
        }

        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => self.create_postgres(value).await,
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => self.create_sqlite(value).await,
        }
    }

    #[cfg(feature = "postgres")]
    async fn create_postgres(&self, value: String) -> Result<Option<UniversalUuid>, ContextError> {
        use crate::dal::postgres_dal::models::{NewPgDbContext, PgDbContext};
        use crate::database::schema::postgres::contexts;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;

        let new_context = NewPgDbContext { value };

        let db_context: PgDbContext = conn
            .interact(move |conn| {
                diesel::insert_into(contexts::table)
                    .values(&new_context)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;

        Ok(Some(UniversalUuid(db_context.id)))
    }

    #[cfg(feature = "sqlite")]
    async fn create_sqlite(&self, value: String) -> Result<Option<UniversalUuid>, ContextError> {
        use crate::dal::sqlite_dal::models::{
            current_timestamp_string, uuid_to_blob, NewSqliteDbContext,
        };
        use crate::database::schema::sqlite::contexts;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;

        // For SQLite, generate UUID and timestamps client-side
        let id = UniversalUuid::new_v4();
        let now = current_timestamp_string();

        let new_context = NewSqliteDbContext {
            id: uuid_to_blob(&id.0),
            value,
            created_at: now.clone(),
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(contexts::table)
                .values(&new_context)
                .execute(conn)
        })
        .await
        .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;

        Ok(Some(id))
    }

    /// Read a context from the database.
    ///
    /// Retrieves a context by its UUID and deserializes it into the specified type.
    pub async fn read<T>(&self, id: UniversalUuid) -> Result<Context<T>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
    {
        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => self.read_postgres(id).await,
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => self.read_sqlite(id).await,
        }
    }

    #[cfg(feature = "postgres")]
    async fn read_postgres<T>(&self, id: UniversalUuid) -> Result<Context<T>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
    {
        use crate::dal::postgres_dal::models::PgDbContext;
        use crate::database::schema::postgres::contexts;
        use uuid::Uuid;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;

        let uuid_id: Uuid = id.into();
        let db_context: PgDbContext = conn
            .interact(move |conn| contexts::table.find(uuid_id).first(conn))
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;

        Context::<T>::from_json(db_context.value)
    }

    #[cfg(feature = "sqlite")]
    async fn read_sqlite<T>(&self, id: UniversalUuid) -> Result<Context<T>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
    {
        use crate::dal::sqlite_dal::models::{uuid_to_blob, SqliteDbContext};
        use crate::database::schema::sqlite::contexts;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let db_context: SqliteDbContext = conn
            .interact(move |conn| contexts::table.find(id_blob).first(conn))
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;

        Context::<T>::from_json(db_context.value)
    }

    /// Update an existing context in the database.
    pub async fn update<T>(
        &self,
        id: UniversalUuid,
        context: &Context<T>,
    ) -> Result<(), ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
    {
        let value = context.to_json()?;

        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => self.update_postgres(id, value).await,
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => self.update_sqlite(id, value).await,
        }
    }

    #[cfg(feature = "postgres")]
    async fn update_postgres(&self, id: UniversalUuid, value: String) -> Result<(), ContextError> {
        use crate::database::schema::postgres::contexts;
        use uuid::Uuid;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;

        let uuid_id: Uuid = id.into();
        conn.interact(move |conn| {
            diesel::update(contexts::table.find(uuid_id))
                .set(contexts::value.eq(value))
                .execute(conn)
        })
        .await
        .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn update_sqlite(&self, id: UniversalUuid, value: String) -> Result<(), ContextError> {
        use crate::dal::sqlite_dal::models::{current_timestamp_string, uuid_to_blob};
        use crate::database::schema::sqlite::contexts;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let now = current_timestamp_string();
        conn.interact(move |conn| {
            diesel::update(contexts::table.find(id_blob))
                .set((contexts::value.eq(value), contexts::updated_at.eq(now)))
                .execute(conn)
        })
        .await
        .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Delete a context from the database.
    pub async fn delete(&self, id: UniversalUuid) -> Result<(), ContextError> {
        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => self.delete_postgres(id).await,
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => self.delete_sqlite(id).await,
        }
    }

    #[cfg(feature = "postgres")]
    async fn delete_postgres(&self, id: UniversalUuid) -> Result<(), ContextError> {
        use crate::database::schema::postgres::contexts;
        use uuid::Uuid;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;

        let uuid_id: Uuid = id.into();
        conn.interact(move |conn| diesel::delete(contexts::table.find(uuid_id)).execute(conn))
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn delete_sqlite(&self, id: UniversalUuid) -> Result<(), ContextError> {
        use crate::dal::sqlite_dal::models::uuid_to_blob;
        use crate::database::schema::sqlite::contexts;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        conn.interact(move |conn| diesel::delete(contexts::table.find(id_blob)).execute(conn))
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// List contexts with pagination.
    ///
    /// Retrieves a paginated list of contexts from the database, ordered by creation date
    /// in descending order.
    pub async fn list<T>(&self, limit: i64, offset: i64) -> Result<Vec<Context<T>>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
    {
        match self.dal.backend() {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => self.list_postgres(limit, offset).await,
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => self.list_sqlite(limit, offset).await,
        }
    }

    #[cfg(feature = "postgres")]
    async fn list_postgres<T>(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Context<T>>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
    {
        use crate::dal::postgres_dal::models::PgDbContext;
        use crate::database::schema::postgres::contexts;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;

        let db_contexts: Vec<PgDbContext> = conn
            .interact(move |conn| {
                contexts::table
                    .limit(limit)
                    .offset(offset)
                    .order(contexts::created_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;

        let mut contexts = Vec::new();
        for db_context in db_contexts {
            let context = Context::<T>::from_json(db_context.value)?;
            contexts.push(context);
        }

        Ok(contexts)
    }

    #[cfg(feature = "sqlite")]
    async fn list_sqlite<T>(&self, limit: i64, offset: i64) -> Result<Vec<Context<T>>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
    {
        use crate::dal::sqlite_dal::models::SqliteDbContext;
        use crate::database::schema::sqlite::contexts;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;

        let db_contexts: Vec<SqliteDbContext> = conn
            .interact(move |conn| {
                contexts::table
                    .limit(limit)
                    .offset(offset)
                    .order(contexts::created_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;

        let mut contexts = Vec::new();
        for db_context in db_contexts {
            let context = Context::<T>::from_json(db_context.value)?;
            contexts.push(context);
        }

        Ok(contexts)
    }
}
