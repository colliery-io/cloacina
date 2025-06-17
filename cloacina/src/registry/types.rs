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

//! Data types for the workflow registry system.
//!
//! This module defines the core data structures used throughout the registry,
//! including workflow metadata, package information, and identifiers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a workflow package.
///
/// This ID is used to reference a specific registered workflow package
/// in both the metadata and binary storage systems.
pub type WorkflowPackageId = Uuid;

/// Metadata for a registered workflow package.
///
/// This structure contains all the descriptive information about a workflow
/// package, stored in the `workflow_packages` table. It includes both
/// user-provided metadata and system-generated information.
///
/// # Examples
///
/// ```rust
/// use cloacina::registry::WorkflowMetadata;
/// use uuid::Uuid;
/// use chrono::Utc;
///
/// let metadata = WorkflowMetadata {
///     id: Uuid::new_v4(),
///     registry_id: Uuid::new_v4(),
///     package_name: "analytics_pipeline".to_string(),
///     version: "1.0.0".to_string(),
///     description: Some("Customer analytics workflow".to_string()),
///     author: Some("Data Team".to_string()),
///     tasks: vec!["extract_data".to_string(), "transform_data".to_string()],
///     schedules: vec!["daily_analytics".to_string()],
///     created_at: Utc::now(),
///     updated_at: Utc::now(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowMetadata {
    /// Unique identifier for this workflow package
    pub id: WorkflowPackageId,

    /// Foreign key to the workflow_registry table
    pub registry_id: Uuid,

    /// Name of the workflow package (e.g., "analytics_pipeline")
    pub package_name: String,

    /// Semantic version of the package (e.g., "1.0.0")
    pub version: String,

    /// Optional human-readable description
    pub description: Option<String>,

    /// Optional author information
    pub author: Option<String>,

    /// List of task IDs included in this package
    pub tasks: Vec<String>,

    /// List of schedule names defined in this package
    pub schedules: Vec<String>,

    /// When this package was registered
    pub created_at: DateTime<Utc>,

    /// When this package metadata was last updated
    pub updated_at: DateTime<Utc>,
}

/// Package metadata extracted from a .cloacina file.
///
/// This structure represents the metadata embedded in the packaged workflow
/// file itself, typically extracted during the packaging process by cloacina-ctl.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Package name from the workflow macro
    pub package: String,

    /// Version from Cargo.toml
    pub version: String,

    /// Optional description
    pub description: Option<String>,

    /// Optional author from Cargo.toml
    pub author: Option<String>,

    /// Build metadata
    pub build_info: BuildInfo,

    /// Task information
    pub tasks: Vec<TaskInfo>,

    /// Schedule information
    pub schedules: Vec<ScheduleInfo>,
}

/// Build information embedded in the package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfo {
    /// Rust compiler version used
    pub rustc_version: String,

    /// Cloacina version used
    pub cloacina_version: String,

    /// Build timestamp
    pub build_timestamp: DateTime<Utc>,

    /// Target architecture
    pub target: String,
}

/// Basic task information from package metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    /// Task identifier
    pub id: String,

    /// Task dependencies
    pub dependencies: Vec<String>,

    /// Optional task description
    pub description: Option<String>,
}

/// Schedule information from package metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleInfo {
    /// Schedule name
    pub name: String,

    /// Cron expression
    pub cron: String,

    /// Workflow to execute
    pub workflow: String,
}

/// A workflow package ready for registration.
///
/// This structure combines the extracted metadata with the raw binary
/// data of the compiled workflow .so file.
#[derive(Debug)]
pub struct WorkflowPackage {
    /// Metadata extracted from the package
    pub metadata: PackageMetadata,

    /// Raw binary data of the .so file
    pub package_data: Vec<u8>,
}

impl WorkflowPackage {
    /// Create a new workflow package from metadata and binary data.
    pub fn new(metadata: PackageMetadata, package_data: Vec<u8>) -> Self {
        Self {
            metadata,
            package_data,
        }
    }

    /// Load a workflow package from a .cloacina file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the .cloacina package file
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowPackage)` - Successfully loaded package
    /// * `Err(std::io::Error)` - If file operations fail
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use cloacina::registry::WorkflowPackage;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let package = WorkflowPackage::from_file("analytics.cloacina")?;
    /// println!("Loaded package: {} v{}",
    ///     package.metadata.package,
    ///     package.metadata.version
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_file(_path: impl AsRef<std::path::Path>) -> Result<Self, std::io::Error> {
        // This will be implemented to use cloacina-ctl's extraction logic
        todo!("Implement using cloacina-ctl archive extraction")
    }
}

/// A loaded workflow with both metadata and binary data.
///
/// This structure is returned when retrieving a workflow from the registry,
/// containing all the information needed to execute the workflow.
#[derive(Debug)]
pub struct LoadedWorkflow {
    /// Full metadata from the database
    pub metadata: WorkflowMetadata,

    /// Binary data from registry storage
    pub package_data: Vec<u8>,
}

impl LoadedWorkflow {
    /// Create a new loaded workflow.
    pub fn new(metadata: WorkflowMetadata, package_data: Vec<u8>) -> Self {
        Self {
            metadata,
            package_data,
        }
    }
}
