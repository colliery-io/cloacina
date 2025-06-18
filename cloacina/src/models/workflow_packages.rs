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

//! Database models for workflow package metadata.
//!
//! This module defines the Diesel models for the workflow_packages table,
//! providing rich metadata storage with foreign key relationships to the
//! workflow registry binary storage.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};

/// Database model for workflow package metadata.
///
/// This represents the rich metadata for a packaged workflow, with a foreign
/// key relationship to the binary storage in workflow_registry.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::workflow_packages)]
#[cfg_attr(feature = "postgres", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "sqlite", diesel(check_for_backend(diesel::sqlite::Sqlite)))]
pub struct WorkflowPackage {
    /// Unique identifier for this package metadata
    pub id: UniversalUuid,

    /// Foreign key to workflow_registry entry containing binary data
    pub registry_id: UniversalUuid,

    /// Human-readable package name
    pub package_name: String,

    /// Package version (semver recommended)
    pub version: String,

    /// Optional package description
    pub description: Option<String>,

    /// Optional package author
    pub author: Option<String>,

    /// Package metadata as JSON string (includes tasks, schedules, etc.)
    pub metadata: String,

    /// When this package was first registered
    pub created_at: UniversalTimestamp,

    /// When this package metadata was last updated
    pub updated_at: UniversalTimestamp,
}

/// Model for creating new workflow package metadata entries.
///
/// This is used when registering new workflow packages in the registry.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::database::schema::workflow_packages)]
#[cfg_attr(feature = "postgres", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "sqlite", diesel(check_for_backend(diesel::sqlite::Sqlite)))]
pub struct NewWorkflowPackage {
    /// Foreign key to workflow_registry entry
    pub registry_id: UniversalUuid,

    /// Package name
    pub package_name: String,

    /// Package version
    pub version: String,

    /// Optional description
    pub description: Option<String>,

    /// Optional author
    pub author: Option<String>,

    /// Package metadata as JSON string (includes tasks, schedules, etc.)
    pub metadata: String,
}

impl NewWorkflowPackage {
    /// Create a new workflow package metadata model.
    ///
    /// # Arguments
    ///
    /// * `registry_id` - UUID of the corresponding workflow_registry entry
    /// * `package_name` - Name of the package
    /// * `version` - Version string
    /// * `description` - Optional description
    /// * `author` - Optional author
    /// * `metadata` - Package metadata as JSON string
    ///
    /// # Returns
    ///
    /// A new `NewWorkflowPackage` ready for insertion
    pub fn new(
        registry_id: UniversalUuid,
        package_name: String,
        version: String,
        description: Option<String>,
        author: Option<String>,
        metadata: String,
    ) -> Self {
        Self {
            registry_id,
            package_name,
            version,
            description,
            author,
            metadata,
        }
    }
}
