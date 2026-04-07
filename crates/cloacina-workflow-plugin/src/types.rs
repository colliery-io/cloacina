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
// Computation graph plugin interface types (cross FFI boundary)
// ============================================================================

/// Metadata for a computation graph package, returned by `get_graph_metadata()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPackageMetadata {
    /// Name of the computation graph
    pub graph_name: String,
    /// Cargo package name
    pub package_name: String,
    /// Reaction mode: "when_any" or "when_all"
    pub reaction_mode: String,
    /// Input strategy: "latest" or "sequential"
    #[serde(default = "default_input_strategy")]
    pub input_strategy: String,
    /// Accumulator declarations
    pub accumulators: Vec<AccumulatorDeclarationEntry>,
}

fn default_input_strategy() -> String {
    "latest".to_string()
}

/// Declaration of an accumulator within a computation graph package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccumulatorDeclarationEntry {
    /// Accumulator name (used as source name and WebSocket endpoint)
    pub name: String,
    /// Accumulator type: "passthrough", "stream", "polling", "batch"
    pub accumulator_type: String,
    /// Type-specific configuration (e.g., topic, interval, flush_interval)
    #[serde(default)]
    pub config: std::collections::HashMap<String, String>,
}

/// Request to execute a computation graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphExecutionRequest {
    /// Cache entries: source name → JSON-serialized boundary value
    pub cache: std::collections::HashMap<String, String>,
}

/// Result of a computation graph execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphExecutionResult {
    /// Whether the graph completed successfully
    pub success: bool,
    /// JSON-serialized terminal node outputs (on success)
    pub terminal_outputs_json: Option<Vec<String>>,
    /// Error message (on failure)
    pub error: Option<String>,
}

// ============================================================================
// Package manifest metadata (for package.toml [metadata] section)
// ============================================================================

/// Host-defined metadata schema for cloacina packages.
///
/// This struct defines what fields are required/optional in the `[metadata]`
/// section of a package's `package.toml`. Validated at load time
/// via `PackageManifest<CloacinaMetadata>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloacinaMetadata {
    /// What this package contains: ["workflow"], ["computation_graph"], or both.
    /// Defaults to ["workflow"] for backward compatibility with existing packages.
    #[serde(default = "default_package_type")]
    pub package_type: Vec<String>,
    /// Name of the workflow (required if package_type includes "workflow")
    #[serde(default)]
    pub workflow_name: Option<String>,
    /// Name of the computation graph (required if package_type includes "computation_graph")
    #[serde(default)]
    pub graph_name: Option<String>,
    /// Package language: "rust" or "python"
    pub language: String,
    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,
    /// Author information
    #[serde(default)]
    pub author: Option<String>,
    /// Minimum Python version (Python packages only, e.g., ">=3.11")
    #[serde(default)]
    pub requires_python: Option<String>,
    /// Python entry module (Python packages only, e.g., "workflow.tasks")
    #[serde(default)]
    pub entry_module: Option<String>,
    /// Trigger definitions for this package
    #[serde(default)]
    pub triggers: Vec<TriggerDefinition>,
    /// Reaction mode for computation graphs: "when_any" or "when_all"
    #[serde(default)]
    pub reaction_mode: Option<String>,
    /// Input strategy for computation graphs: "latest" or "sequential"
    #[serde(default)]
    pub input_strategy: Option<String>,
    /// Accumulator configuration overrides (from package.toml, merged with FFI defaults)
    #[serde(default)]
    pub accumulators: Vec<AccumulatorConfig>,
}

/// Accumulator configuration from package.toml metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccumulatorConfig {
    /// Accumulator name (must match a name in the graph's react declaration)
    pub name: String,
    /// Accumulator type override: "passthrough", "stream", "batch"
    #[serde(default = "default_accumulator_type")]
    pub accumulator_type: String,
    /// Type-specific config (topic, group, flush_interval, etc.)
    #[serde(default)]
    pub config: std::collections::HashMap<String, String>,
}

fn default_accumulator_type() -> String {
    "passthrough".to_string()
}

fn default_package_type() -> Vec<String> {
    vec!["workflow".to_string()]
}

impl CloacinaMetadata {
    /// Check if this package contains a workflow.
    pub fn has_workflow(&self) -> bool {
        self.package_type.iter().any(|t| t == "workflow")
    }

    /// Check if this package contains a computation graph.
    pub fn has_computation_graph(&self) -> bool {
        self.package_type.iter().any(|t| t == "computation_graph")
    }

    /// Get the workflow name, falling back for backward compatibility.
    /// Old packages had `workflow_name` as required — now it's optional
    /// but we still need it for workflow packages.
    pub fn effective_workflow_name(&self) -> Option<&str> {
        self.workflow_name.as_deref()
    }
}

/// A trigger definition within a workflow package manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDefinition {
    /// Trigger name
    pub name: String,
    /// Workflow to fire when trigger activates
    pub workflow: String,
    /// Poll interval for custom poll triggers (e.g., "5s", "1m")
    pub poll_interval: String,
    /// Cron expression (e.g., "*/10 * * * *"). If present, this is a cron trigger.
    #[serde(default)]
    pub cron_expression: Option<String>,
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
    fn test_cloacina_metadata_rust_from_toml() {
        let toml_str = r#"
            workflow_name = "analytics_pipeline"
            language = "rust"
            description = "Data analytics workflow"
            author = "Analytics Team"

            [[triggers]]
            name = "file_watcher"
            workflow = "analytics_pipeline"
            poll_interval = "5s"
            allow_concurrent = false
        "#;

        let metadata: CloacinaMetadata = toml::from_str(toml_str).unwrap();
        assert_eq!(
            metadata.workflow_name.as_deref(),
            Some("analytics_pipeline")
        );
        assert_eq!(metadata.language, "rust");
        assert_eq!(
            metadata.description.as_deref(),
            Some("Data analytics workflow")
        );
        assert!(metadata.requires_python.is_none());
        assert!(metadata.entry_module.is_none());
        assert_eq!(metadata.triggers.len(), 1);
        assert_eq!(metadata.triggers[0].name, "file_watcher");
        assert!(!metadata.triggers[0].allow_concurrent);
    }

    #[test]
    fn test_cloacina_metadata_python_from_toml() {
        let toml_str = r#"
            workflow_name = "etl_pipeline"
            language = "python"
            description = "Python ETL workflow"
            requires_python = ">=3.11"
            entry_module = "workflow.tasks"
        "#;

        let metadata: CloacinaMetadata = toml::from_str(toml_str).unwrap();
        assert_eq!(metadata.workflow_name.as_deref(), Some("etl_pipeline"));
        assert_eq!(metadata.language, "python");
        assert_eq!(metadata.requires_python.as_deref(), Some(">=3.11"));
        assert_eq!(metadata.entry_module.as_deref(), Some("workflow.tasks"));
        assert!(metadata.triggers.is_empty());
    }

    #[test]
    fn test_cloacina_metadata_minimal_rust() {
        let toml_str = r#"
            workflow_name = "simple_workflow"
            language = "rust"
        "#;

        let metadata: CloacinaMetadata = toml::from_str(toml_str).unwrap();
        assert_eq!(metadata.workflow_name.as_deref(), Some("simple_workflow"));
        assert_eq!(metadata.language, "rust");
        assert!(metadata.description.is_none());
        assert!(metadata.triggers.is_empty());
    }

    #[test]
    fn test_cloacina_metadata_missing_language_fails() {
        let toml_str = r#"
            workflow_name = "no_language"
        "#;

        let result = toml::from_str::<CloacinaMetadata>(toml_str);
        assert!(result.is_err(), "Missing language field should fail");
    }

    #[test]
    fn test_cloacina_metadata_defaults_to_workflow_package_type() {
        let toml_str = r#"
            workflow_name = "legacy_workflow"
            language = "rust"
        "#;

        let metadata: CloacinaMetadata = toml::from_str(toml_str).unwrap();
        assert_eq!(metadata.package_type, vec!["workflow"]);
        assert!(metadata.has_workflow());
        assert!(!metadata.has_computation_graph());
    }

    #[test]
    fn test_cloacina_metadata_computation_graph_from_toml() {
        let toml_str = r#"
            package_type = ["computation_graph"]
            graph_name = "market_maker"
            language = "rust"
            reaction_mode = "when_any"
            input_strategy = "latest"
        "#;

        let metadata: CloacinaMetadata = toml::from_str(toml_str).unwrap();
        assert_eq!(metadata.package_type, vec!["computation_graph"]);
        assert!(!metadata.has_workflow());
        assert!(metadata.has_computation_graph());
        assert_eq!(metadata.graph_name.as_deref(), Some("market_maker"));
        assert_eq!(metadata.reaction_mode.as_deref(), Some("when_any"));
        assert!(metadata.workflow_name.is_none());
    }

    #[test]
    fn test_cloacina_metadata_both_types() {
        let toml_str = r#"
            package_type = ["workflow", "computation_graph"]
            workflow_name = "analytics"
            graph_name = "market_maker"
            language = "rust"
        "#;

        let metadata: CloacinaMetadata = toml::from_str(toml_str).unwrap();
        assert!(metadata.has_workflow());
        assert!(metadata.has_computation_graph());
    }

    #[test]
    fn test_graph_package_metadata_round_trip() {
        let metadata = GraphPackageMetadata {
            graph_name: "market_maker".to_string(),
            package_name: "mm-pkg".to_string(),
            reaction_mode: "when_any".to_string(),
            input_strategy: "latest".to_string(),
            accumulators: vec![
                AccumulatorDeclarationEntry {
                    name: "orderbook".to_string(),
                    accumulator_type: "stream".to_string(),
                    config: [("topic".to_string(), "market.orderbook".to_string())]
                        .into_iter()
                        .collect(),
                },
                AccumulatorDeclarationEntry {
                    name: "pricing".to_string(),
                    accumulator_type: "passthrough".to_string(),
                    config: std::collections::HashMap::new(),
                },
            ],
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let roundtrip: GraphPackageMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.graph_name, "market_maker");
        assert_eq!(roundtrip.accumulators.len(), 2);
        assert_eq!(roundtrip.accumulators[0].accumulator_type, "stream");
        assert_eq!(
            roundtrip.accumulators[0].config.get("topic").unwrap(),
            "market.orderbook"
        );
    }

    #[test]
    fn test_graph_execution_request_round_trip() {
        let request = GraphExecutionRequest {
            cache: [("alpha".to_string(), r#"{"value": 42.0}"#.to_string())]
                .into_iter()
                .collect(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let roundtrip: GraphExecutionRequest = serde_json::from_str(&json).unwrap();
        assert!(roundtrip.cache.contains_key("alpha"));
    }

    #[test]
    fn test_graph_execution_result_round_trip() {
        let result = GraphExecutionResult {
            success: true,
            terminal_outputs_json: Some(vec![r#"{"published": true}"#.to_string()]),
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let roundtrip: GraphExecutionResult = serde_json::from_str(&json).unwrap();
        assert!(roundtrip.success);
        assert_eq!(roundtrip.terminal_outputs_json.unwrap().len(), 1);
    }
}
