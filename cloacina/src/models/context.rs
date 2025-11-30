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
//! This module provides domain structures for working with context data.
//! These are API-level types used in business logic; backend-specific
//! models handle actual database interaction.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};

/// Represents a context record (domain type).
///
/// This is used at the API boundary and in business logic.
/// Backend-specific models (PgDbContext, SqliteDbContext) handle database storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbContext {
    pub id: UniversalUuid,
    pub value: String,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

/// Structure for creating new context records (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDbContext {
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::universal_types::current_timestamp;

    #[test]
    fn test_db_context_creation() {
        let now = current_timestamp();
        let context = DbContext {
            id: UniversalUuid::new_v4(),
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
