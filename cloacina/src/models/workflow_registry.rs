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

//! Database models for workflow registry storage.
//!
//! This module defines the Diesel models for the workflow_registry table,
//! providing serialization and deserialization for binary workflow data.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};

/// Database model for a workflow registry entry.
///
/// This represents a stored binary workflow package with its metadata.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::workflow_registry)]
pub struct WorkflowRegistryEntry {
    /// Unique identifier for this registry entry
    pub id: UniversalUuid,

    /// When this entry was created
    pub created_at: UniversalTimestamp,

    /// Binary data of the workflow package
    pub data: Vec<u8>,
}

/// Model for creating new workflow registry entries.
///
/// This is used when inserting new binary workflow data into the registry.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::database::schema::workflow_registry)]
pub struct NewWorkflowRegistryEntry {
    /// Binary data of the workflow package
    pub data: Vec<u8>,
}

impl NewWorkflowRegistryEntry {
    /// Create a new workflow registry entry model.
    ///
    /// # Arguments
    ///
    /// * `data` - Binary data of the workflow package
    ///
    /// # Returns
    ///
    /// A new `NewWorkflowRegistryEntry` ready for insertion
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}
