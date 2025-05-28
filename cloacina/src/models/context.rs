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

//! Context management module for storing and retrieving context data.
//!
//! This module provides structures for working with context data in the database.
//! Contexts are used to store serialized JSON data that can be associated with various
//! parts of the application. The data is stored as a string in the database and can be
//! deserialized into appropriate structures when needed.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a context record in the database.
///
/// This structure maps to the `contexts` table in the database and provides
/// functionality for querying and serializing context data.
///
/// # Fields
/// * `id` - Unique identifier for the context
/// * `value` - Serialized JSON string containing the context data
/// * `created_at` - Timestamp when the context was created
/// * `updated_at` - Timestamp when the context was last updated
#[derive(Debug, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::contexts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbContext {
    pub id: Uuid,
    pub value: String, // Serialized JSON of the context data
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Structure for creating new context records in the database.
///
/// This structure is used when inserting new contexts into the database.
/// It only requires the `value` field as other fields are automatically
/// populated by the database.
///
/// # Fields
/// * `value` - Serialized JSON string containing the context data
#[derive(Debug, Insertable)]
#[diesel(table_name = crate::database::schema::contexts)]
pub struct NewDbContext {
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_db_context_creation() {
        let now = Utc::now().naive_utc();
        let context = DbContext {
            id: Uuid::new_v4(),
            value: "{\"test\":42}".to_string(),
            created_at: now,
            updated_at: now,
        };

        assert_eq!(context.value, "{\"test\":42}");
        assert_eq!(context.created_at, now);
        assert_eq!(context.updated_at, now);
    }

    #[test]
    fn test_new_db_context_creation() {
        let new_context = NewDbContext {
            value: "{\"test\":42}".to_string(),
        };

        assert_eq!(new_context.value, "{\"test\":42}");
    }
}
