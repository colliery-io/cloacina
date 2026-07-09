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

//! Unified Context DAL with runtime backend selection
//!
//! This module provides CRUD operations for Context entities that work with
//! both PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.

use super::DAL;
use crate::context::Context;
use crate::database::universal_types::UniversalUuid;
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
        use super::models::NewUnifiedDbContext;
        use crate::database::schema::unified::contexts;
        use crate::database::universal_types::UniversalTimestamp;

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

        // Generate ID and timestamps
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_context = NewUnifiedDbContext {
            id,
            value,
            created_at: now,
            updated_at: now,
        };

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(contexts::table)
                .values(&new_context)
                .execute(conn)
        })?;

        Ok(Some(id))
    }

    /// Read a context from the database.
    ///
    /// Retrieves a context by its UUID and deserializes it into the specified type.
    pub async fn read<T>(&self, id: UniversalUuid) -> Result<Context<T>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
    {
        use super::models::UnifiedDbContext;
        use crate::database::schema::unified::contexts;

        let db_context: UnifiedDbContext =
            crate::interact_on_backend!(self.dal, |conn| contexts::table.find(id).first(conn))?;

        Ok(Context::<T>::from_json(db_context.value)?)
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
        use crate::database::schema::unified::contexts;
        use crate::database::universal_types::UniversalTimestamp;

        let value = context.to_json()?;
        let now = UniversalTimestamp::now();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(contexts::table.find(id))
                .set((contexts::value.eq(value), contexts::updated_at.eq(now)))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Delete a context from the database.
    pub async fn delete(&self, id: UniversalUuid) -> Result<(), ContextError> {
        use crate::database::schema::unified::contexts;

        crate::interact_on_backend!(self.dal, |conn| diesel::delete(contexts::table.find(id))
            .execute(conn))?;

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
        use super::models::UnifiedDbContext;
        use crate::database::schema::unified::contexts;

        let db_contexts: Vec<UnifiedDbContext> = crate::interact_on_backend!(self.dal, |conn| {
            contexts::table
                .limit(limit)
                .offset(offset)
                .order(contexts::created_at.desc())
                .load(conn)
        })?;

        let mut results = Vec::new();
        for db_context in db_contexts {
            let context = Context::<T>::from_json(db_context.value)?;
            results.push(context);
        }

        Ok(results)
    }
}
