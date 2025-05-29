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
//!
//! # Examples
//!
//! ```rust
//! use uuid::Uuid;
//!
//! // Create a new context
//! let context = Context::new(data);
//! let context_id = context_dal.create(&context)?;
//!
//! // Read a context
//! let context = context_dal.read::<MyDataType>(context_id.unwrap())?;
//!
//! // Update a context
//! context_dal.update(context_id.unwrap(), &updated_context)?;
//!
//! // Delete a context
//! context_dal.delete(context_id.unwrap())?;
//!
//! // List contexts with pagination
//! let contexts = context_dal.list::<MyDataType>(10, 0)?;
//! ```

use super::DAL;
use crate::context::Context;
use crate::database::schema::contexts;
use crate::database::universal_types::UniversalUuid;
use crate::error::ContextError;
use crate::models::context::{DbContext, NewDbContext};
use diesel::prelude::*;
use tracing::warn;

/// The Data Access Layer implementation for Context entities.
///
/// This struct provides methods for creating, reading, updating, and deleting
/// context data in the database. It handles the serialization and deserialization
/// of context data to/from JSON format.
///
/// # Type Parameters
///
/// * `'a` - The lifetime of the reference to the parent DAL instance
pub struct ContextDAL<'a> {
    /// Reference to the parent DAL instance
    pub dal: &'a DAL,
}

impl<'a> ContextDAL<'a> {
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
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of data stored in the context, must implement Serialize, Deserialize, and Debug
    pub fn create<T>(&self, context: &Context<T>) -> Result<Option<UniversalUuid>, ContextError>
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

        let conn = &mut self.dal.pool.get()?;

        // For SQLite, we need to manually generate the UUID
        let id = UniversalUuid::new_v4();

        // Create new database record
        let new_context = NewDbContext { value };

        // Insert with explicit ID for SQLite
        diesel::insert_into(contexts::table)
            .values((contexts::id.eq(&id), &new_context))
            .execute(conn)?;

        Ok(Some(id))
    }

    /// Read a context from the database.
    ///
    /// Retrieves a context by its UUID and deserializes it into the specified type.
    ///
    /// # Arguments
    ///
    /// * `id` - The UUID of the context to retrieve
    ///
    /// # Returns
    ///
    /// * `Result<Context<T>, ContextError>` - The deserialized context if successful
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of data stored in the context, must implement Serialize, Deserialize, and Debug
    pub fn read<T>(&self, id: UniversalUuid) -> Result<Context<T>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug,
    {
        let conn = &mut self.dal.pool.get()?;

        // Get the database record
        let db_context: DbContext = contexts::table.find(id).first(conn)?;

        // Deserialize into application context
        Context::<T>::from_json(db_context.value)
    }

    /// Update an existing context in the database.
    ///
    /// Updates the JSON data for a context with the specified UUID.
    ///
    /// # Arguments
    ///
    /// * `id` - The UUID of the context to update
    /// * `context` - The new context data to store
    ///
    /// # Returns
    ///
    /// * `Result<(), ContextError>` - Success or error
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of data stored in the context, must implement Serialize, Deserialize, and Debug
    pub fn update<T>(&self, id: UniversalUuid, context: &Context<T>) -> Result<(), ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug,
    {
        let conn = &mut self.dal.pool.get()?;

        // Serialize the context data
        let value = context.to_json()?;

        // Update the database record
        diesel::update(contexts::table.find(id))
            .set(contexts::value.eq(value))
            .execute(conn)?;

        Ok(())
    }

    /// Delete a context from the database.
    ///
    /// Removes a context with the specified UUID from the database.
    ///
    /// # Arguments
    ///
    /// * `id` - The UUID of the context to delete
    ///
    /// # Returns
    ///
    /// * `Result<(), ContextError>` - Success or error
    pub fn delete(&self, id: UniversalUuid) -> Result<(), ContextError> {
        let conn = &mut self.dal.pool.get()?;
        diesel::delete(contexts::table.find(id)).execute(conn)?;
        Ok(())
    }

    /// List contexts with pagination.
    ///
    /// Retrieves a paginated list of contexts from the database, ordered by creation date
    /// in descending order.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of contexts to retrieve
    /// * `offset` - Number of contexts to skip
    ///
    /// # Returns
    ///
    /// * `Result<Vec<Context<T>>, ContextError>` - Vector of deserialized contexts if successful
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of data stored in the context, must implement Serialize, Deserialize, and Debug
    pub fn list<T>(&self, limit: i64, offset: i64) -> Result<Vec<Context<T>>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug,
    {
        let conn = &mut self.dal.pool.get()?;

        // Get the database records with pagination
        let db_contexts: Vec<DbContext> = contexts::table
            .limit(limit)
            .offset(offset)
            .order(contexts::created_at.desc())
            .load(conn)?;

        // Convert to application contexts
        let mut contexts = Vec::new();
        for db_context in db_contexts {
            let context = Context::<T>::from_json(db_context.value)?;
            contexts.push(context);
        }

        Ok(contexts)
    }
}
