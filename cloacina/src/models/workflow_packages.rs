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
//! This module defines domain structures for workflow package metadata.
//! These are API-level types; backend-specific models handle database storage.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};

/// Domain model for workflow package metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPackage {
    pub id: UniversalUuid,
    pub registry_id: UniversalUuid,
    pub package_name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub metadata: String,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

/// Model for creating new workflow package metadata entries (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkflowPackage {
    pub registry_id: UniversalUuid,
    pub package_name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub metadata: String,
}

impl NewWorkflowPackage {
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
