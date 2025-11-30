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

//! Data Access Layer (DAL) implementation for managing Context entities in the database.
//!
//! This module provides CRUD operations for Context entities, handling serialization
//! and deserialization of context data to/from JSON format in the database.

use super::models::{NewPgDbContext, PgDbContext};
use super::DAL;
use crate::context::Context;
use crate::database::schema::postgres::contexts;
use crate::database::universal_types::UniversalUuid;
use crate::error::ContextError;
use diesel::prelude::*;
use tracing::warn;
use uuid::Uuid;

/// The Data Access Layer implementation for Context entities.
pub struct ContextDAL<'a> {
    /// Reference to the parent DAL instance
    pub dal: &'a DAL,
}

impl<'a> ContextDAL<'a> {
    /// Create a new context in the database.
    pub async fn create<T>(
        &self,
        context: &Context<T>,
    ) -> Result<Option<UniversalUuid>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug,
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

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;

        // Create new database record using backend-specific model
        let new_context = NewPgDbContext { value };

        // Insert and get the result
        let pg_context: PgDbContext = conn
            .interact(move |conn| {
                diesel::insert_into(contexts::table)
                    .values(&new_context)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;

        Ok(Some(UniversalUuid(pg_context.id)))
    }

    /// Read a context from the database.
    pub async fn read<T>(&self, id: UniversalUuid) -> Result<Context<T>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug,
    {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;

        // Get the database record
        let uuid_id: Uuid = id.0;
        let pg_context: PgDbContext = conn
            .interact(move |conn| contexts::table.find(uuid_id).first(conn))
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;

        // Deserialize into application context
        Context::<T>::from_json(pg_context.value)
    }

    /// Update an existing context in the database.
    pub async fn update<T>(
        &self,
        id: UniversalUuid,
        context: &Context<T>,
    ) -> Result<(), ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug,
    {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;

        // Serialize the context data
        let value = context.to_json()?;

        // Update the database record
        let uuid_id: Uuid = id.0;
        conn.interact(move |conn| {
            diesel::update(contexts::table.find(uuid_id))
                .set(contexts::value.eq(value))
                .execute(conn)
        })
        .await
        .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Delete a context from the database.
    pub async fn delete(&self, id: UniversalUuid) -> Result<(), ContextError> {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;
        let uuid_id: Uuid = id.0;
        conn.interact(move |conn| diesel::delete(contexts::table.find(uuid_id)).execute(conn))
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;
        Ok(())
    }

    /// List contexts with pagination.
    pub async fn list<T>(&self, limit: i64, offset: i64) -> Result<Vec<Context<T>>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug,
    {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))?;

        // Get the database records with pagination
        let pg_contexts: Vec<PgDbContext> = conn
            .interact(move |conn| {
                contexts::table
                    .limit(limit)
                    .offset(offset)
                    .order(contexts::created_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ContextError::ConnectionPool(e.to_string()))??;

        // Convert to application contexts
        let mut contexts = Vec::new();
        for pg_context in pg_contexts {
            let context = Context::<T>::from_json(pg_context.value)?;
            contexts.push(context);
        }

        Ok(contexts)
    }
}
