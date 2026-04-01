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

//! Shared types that cross the FFI boundary via fidius wire format.
//!
//! These types are serialized/deserialized by fidius automatically —
//! no manual `#[repr(C)]` structs or `CStr` handling needed.

use serde::{Deserialize, Serialize};

// ============================================================================
// Plugin interface types (cross FFI boundary)
// ============================================================================

/// Metadata for a single task within a workflow package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadataEntry {
    /// Task index within the workflow
    pub index: u32,
    /// Local task identifier (e.g., "extract_data")
    pub id: String,
    /// Template for namespaced ID (e.g., "{tenant}::{pkg}::workflow::task")
    pub namespaced_id_template: String,
    /// Task dependency IDs (local names)
    pub dependencies: Vec<String>,
    /// Human-readable description
    pub description: String,
    /// Source file location
    pub source_location: String,
}

/// Complete metadata for a workflow package, returned by `get_task_metadata()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageTasksMetadata {
    /// Name of the workflow (from `#[workflow(name = "...")]`)
    pub workflow_name: String,
    /// Cargo package name (from `CARGO_PKG_NAME`)
    pub package_name: String,
    /// Package description
    pub package_description: Option<String>,
    /// Package author
    pub package_author: Option<String>,
    /// Deterministic fingerprint for ABI/content tracking
    pub workflow_fingerprint: Option<String>,
    /// JSON-encoded workflow graph data
    pub graph_data_json: Option<String>,
    /// All tasks in this workflow
    pub tasks: Vec<TaskMetadataEntry>,
}

/// Request to execute a task within a workflow package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionRequest {
    /// Name of the task to execute (local ID)
    pub task_name: String,
    /// JSON-serialized execution context
    pub context_json: String,
}

/// Result of a task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionResult {
    /// Whether the task completed successfully
    pub success: bool,
    /// Updated JSON-serialized context (on success)
    pub context_json: Option<String>,
    /// Error message (on failure)
    pub error: Option<String>,
}

// ============================================================================
// Package manifest metadata (for package.toml [metadata] section)
// ============================================================================

/// Host-defined metadata schema for cloacina workflow packages.
///
/// This struct defines what fields are required/optional in the `[metadata]`
/// section of a workflow package's `package.toml`. Validated at load time
/// via `PackageManifest<CloacinaMetadata>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloacinaMetadata {
    /// Name of the workflow (must match `#[workflow(name = "...")]`)
    pub workflow_name: String,
    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,
    /// Author information
    #[serde(default)]
    pub author: Option<String>,
    /// Trigger definitions for this package
    #[serde(default)]
    pub triggers: Vec<TriggerDefinition>,
}

/// A trigger definition within a workflow package manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDefinition {
    /// Trigger name
    pub name: String,
    /// Workflow to fire when trigger activates
    pub workflow: String,
    /// Poll interval (e.g., "5s", "1m")
    pub poll_interval: String,
    /// Whether concurrent executions are allowed
    #[serde(default)]
    pub allow_concurrent: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_metadata_serde_round_trip() {
        let entry = TaskMetadataEntry {
            index: 0,
            id: "extract_data".to_string(),
            namespaced_id_template: "{tenant}::{pkg}::pipeline::extract_data".to_string(),
            dependencies: vec![],
            description: "Extract data from sources".to_string(),
            source_location: "src/lib.rs".to_string(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let roundtrip: TaskMetadataEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.id, "extract_data");
        assert_eq!(roundtrip.index, 0);
    }

    #[test]
    fn test_package_tasks_metadata_serde_round_trip() {
        let metadata = PackageTasksMetadata {
            workflow_name: "analytics_pipeline".to_string(),
            package_name: "analytics-pkg".to_string(),
            package_description: Some("Analytics workflow".to_string()),
            package_author: Some("Team".to_string()),
            workflow_fingerprint: Some("sha256:abc123".to_string()),
            graph_data_json: None,
            tasks: vec![TaskMetadataEntry {
                index: 0,
                id: "step_one".to_string(),
                namespaced_id_template: "{tenant}::{pkg}::analytics::step_one".to_string(),
                dependencies: vec![],
                description: "First step".to_string(),
                source_location: "src/lib.rs".to_string(),
            }],
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let roundtrip: PackageTasksMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.workflow_name, "analytics_pipeline");
        assert_eq!(roundtrip.tasks.len(), 1);
    }

    #[test]
    fn test_task_execution_request_round_trip() {
        let request = TaskExecutionRequest {
            task_name: "extract_data".to_string(),
            context_json: r#"{"key": "value"}"#.to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let roundtrip: TaskExecutionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.task_name, "extract_data");
    }

    #[test]
    fn test_task_execution_result_success() {
        let result = TaskExecutionResult {
            success: true,
            context_json: Some(r#"{"updated": true}"#.to_string()),
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let roundtrip: TaskExecutionResult = serde_json::from_str(&json).unwrap();
        assert!(roundtrip.success);
        assert!(roundtrip.context_json.is_some());
        assert!(roundtrip.error.is_none());
    }

    #[test]
    fn test_task_execution_result_failure() {
        let result = TaskExecutionResult {
            success: false,
            context_json: None,
            error: Some("Task panicked".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        let roundtrip: TaskExecutionResult = serde_json::from_str(&json).unwrap();
        assert!(!roundtrip.success);
        assert!(roundtrip.error.is_some());
    }

    #[test]
    fn test_cloacina_metadata_from_toml() {
        let toml_str = r#"
            workflow_name = "analytics_pipeline"
            description = "Data analytics workflow"
            author = "Analytics Team"

            [[triggers]]
            name = "file_watcher"
            workflow = "analytics_pipeline"
            poll_interval = "5s"
            allow_concurrent = false
        "#;

        let metadata: CloacinaMetadata = toml::from_str(toml_str).unwrap();
        assert_eq!(metadata.workflow_name, "analytics_pipeline");
        assert_eq!(
            metadata.description.as_deref(),
            Some("Data analytics workflow")
        );
        assert_eq!(metadata.triggers.len(), 1);
        assert_eq!(metadata.triggers[0].name, "file_watcher");
        assert!(!metadata.triggers[0].allow_concurrent);
    }

    #[test]
    fn test_cloacina_metadata_minimal() {
        let toml_str = r#"
            workflow_name = "simple_workflow"
        "#;

        let metadata: CloacinaMetadata = toml::from_str(toml_str).unwrap();
        assert_eq!(metadata.workflow_name, "simple_workflow");
        assert!(metadata.description.is_none());
        assert!(metadata.triggers.is_empty());
    }
}
