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

//! Workflow metadata and versioning.
//!
//! This module contains the `WorkflowMetadata` struct for managing
//! workflow versioning, timestamps, and organizational tags.

use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Metadata information for a Workflow.
///
/// Contains versioning, creation timestamps, and arbitrary tags for
/// organizing and managing workflow instances.
///
/// # Fields
///
/// * `created_at`: DateTime<Utc> - When the workflow was created
/// * `version`: String - Content-based version hash
/// * `description`: Option<String> - Optional human-readable description
/// * `tags`: HashMap<String, String> - Arbitrary key-value tags for organization
///
/// # Implementation Details
///
/// The version field is automatically calculated based on:
/// - Workflow topology (task IDs and dependencies)
/// - Task definitions (code fingerprints)
/// - Workflow configuration (name, description, tags)
///
/// # Examples
///
/// ```rust
/// use cloacina::WorkflowMetadata;
/// use std::collections::HashMap;
///
/// let mut metadata = WorkflowMetadata::default();
/// metadata.version = "a1b2c3d4".to_string();
/// metadata.description = Some("Production ETL pipeline".to_string());
/// metadata.tags.insert("team".to_string(), "data-engineering".to_string());
/// ```
#[derive(Debug, Clone)]
pub struct WorkflowMetadata {
    /// When the workflow was created
    pub created_at: DateTime<Utc>,
    /// Content-based version hash
    pub version: String,
    /// Optional human-readable description
    pub description: Option<String>,
    /// Arbitrary key-value tags for organization
    pub tags: HashMap<String, String>,
}

impl Default for WorkflowMetadata {
    fn default() -> Self {
        Self {
            created_at: Utc::now(),
            version: String::new(), // Will be auto-calculated
            description: None,
            tags: HashMap::new(),
        }
    }
}
