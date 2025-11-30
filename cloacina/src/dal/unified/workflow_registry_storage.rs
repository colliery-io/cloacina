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

//! Workflow registry storage for unified backend support

use crate::database::Database;

/// Unified registry storage that works with both PostgreSQL and SQLite.
#[derive(Clone)]
pub struct UnifiedRegistryStorage {
    database: Database,
}

impl UnifiedRegistryStorage {
    /// Creates a new UnifiedRegistryStorage instance.
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    /// Returns a reference to the underlying database.
    pub fn database(&self) -> &Database {
        &self.database
    }

    // TODO: Implement RegistryStorage trait with backend dispatch
}
