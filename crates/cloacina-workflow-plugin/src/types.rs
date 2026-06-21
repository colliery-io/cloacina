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
    /// Serialized trigger-rules JSON for conditional execution (CLOACI-T-0721).
    /// Carries the task's `trigger_rules()` across the cdylib FFI boundary so
    /// packaged workflows honour conditional execution / skips — without it the
    /// host's `DynamicLibraryTask` defaulted every task to `Always`. Defaults to
    /// `{"type":"Always"}` for packages built before this field existed.
    #[serde(default = "default_trigger_rules")]
    pub trigger_rules: String,
}

/// Default trigger-rules JSON (`Always`) for back-compat deserialization of
/// packages built before `trigger_rules` was carried in the metadata.
fn default_trigger_rules() -> String {
    "{\"type\":\"Always\"}".to_string()
}

/// One injectable surface's declared input interface (CLOACI-I-0128). Carried
/// over a dedicated FFI entrypoint (`get_input_interface`), separate from the
/// `PackageTasksMetadata` wire struct so the per-task metadata ABI is untouched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputInterfaceEntry {
    /// `"workflow"`, `"accumulator"`, or `"reactor"`.
    pub surface_kind: String,
    /// The surface's name (workflow name, accumulator/source name, reactor name).
    pub surface_name: String,
    /// JSON array of `cloacina_api_types::InputSlot`, serialized. Kept as a
    /// string so the fidius wire stays simple; the host parses it.
    pub slots_json: String,
}

/// Descriptor returned by the optional `get_input_interface()` plugin method
/// (method index 9, since interface version 3) — the declared input interfaces
/// for every injectable surface in the package (CLOACI-I-0128).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputInterfaceDescriptor {
    pub entries: Vec<InputInterfaceEntry>,
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
    /// Names of triggers this workflow subscribes to. Sourced from the
    /// `#[workflow(triggers = [...])]` macro arg. The reconciler binds
    /// each named trigger → this workflow at load time. (T-A)
    #[serde(default)]
    pub triggers: Vec<String>,
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
    /// Name of the reactor this graph is bound to. `Some(name)` opts into
    /// shared-reactor binding — multiple graph packages naming the same
    /// reactor share a single reactor instance in the runtime (T-0544
    /// fan-out). `None` (today's bundled-form default) gets a per-graph
    /// synthesized reactor name with 1:1 lifecycle. `#[serde(default)]`
    /// keeps this backward compatible with packages built before T-0544 M5.
    #[serde(default)]
    pub trigger_reactor: Option<String>,
    /// Serialized node/edge topology of the graph, as JSON
    /// `{"nodes":[{"id","inputs":[..]}],"edges":[{"from","to","label":null|"Variant"}]}`.
    /// Emitted by the `#[computation_graph]` macro (Rust) or the Python graph
    /// builder, so the API/UI can render the CG DAG. `None`/empty for packages
    /// predating topology emission. (CLOACI-T-0673)
    #[serde(default)]
    pub graph_data_json: Option<String>,
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

/// Metadata for a single reactor declared by this package, returned by
/// `get_reactor_metadata()`. Mirrors `GraphPackageMetadata` shape: the
/// reactor publishes accumulators and a reaction mode that downstream
/// computation graphs (in this or other packages) can subscribe to by name.
/// (T-A — I-0102)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactorPackageMetadata {
    /// Reactor name (used as the reactor's identity in the runtime registry
    /// and as the binding target for `trigger = reactor("name")` graph refs).
    pub name: String,
    /// Cargo package name (sourcing context for diagnostics).
    pub package_name: String,
    /// Reaction mode: "when_any" or "when_all".
    pub reaction_mode: String,
    /// Accumulator declarations.
    pub accumulators: Vec<AccumulatorDeclarationEntry>,
}

/// Metadata entry for a single trigger-less computation graph declared
/// by this package, returned by `get_triggerless_graph_metadata()`.
/// `terminal_node_names` mirrors the field on
/// `TriggerlessGraphRegistration` so the host's
/// `register_triggerless_graph` registration carries the same
/// ordering — workflow tasks invoke the graph and write each terminal
/// output into context under the corresponding name. (T-0553 follow-up
/// — Trigger-less CG FFI bridge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerlessGraphMetadataEntry {
    /// Graph name (the `#[computation_graph]` mod name).
    pub name: String,
    /// Cargo package name (sourcing context for diagnostics).
    pub package_name: String,
    /// Terminal node names in declaration order. The host adapter uses
    /// this when registering the graph into the scoped Runtime so
    /// `#[task(invokes = computation_graph(...))]` writes the right
    /// keys back into context after invocation.
    pub terminal_node_names: Vec<String>,
}

/// Request to invoke a trigger-less computation graph from the host
/// across the FFI boundary. The host's `FfiTriggerlessGraph` adapter
/// sends one of these per workflow-task invocation. (T-0553 follow-up
/// — Trigger-less CG FFI bridge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerlessGraphInvokeRequest {
    /// Inventory-registered graph name to invoke.
    pub graph_name: String,
    /// Serialized `Context<serde_json::Value>` carrying inputs.
    pub context_json: String,
}

/// Result of a cross-FFI trigger-less graph invocation. Mirrors
/// `GraphResult::Completed { outputs }` / `GraphResult::Error(...)` in
/// a wire-format-friendly shape: terminal outputs ride as a serialized
/// `Vec<serde_json::Value>` ordered to match the metadata's
/// `terminal_node_names`. The host's `FfiTriggerlessGraph` reconstructs
/// `GraphResult` from this. (T-0553 follow-up — Trigger-less CG FFI bridge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerlessGraphInvokeResult {
    /// `true` if the cdylib's graph_fn returned `Completed { .. }`,
    /// `false` if it returned `Error(_)`. When `error` is set, this
    /// field is meaningless.
    pub success: bool,
    /// Serialized `Vec<serde_json::Value>` of terminal outputs, indexed
    /// by `terminal_node_names`. `None` for graphs that don't return
    /// any terminal output values (the typical packaged shape).
    pub terminal_outputs_json: Option<String>,
    /// When the cdylib's graph_fn returned `Error(_)` or the named
    /// graph wasn't found in inventory, this carries a description.
    pub error: Option<String>,
}

/// Request to invoke a trigger's `poll()` from the host across the FFI
/// boundary. Used by the reconciler to drive trigger polling without
/// relying on `inventory` crossing the cdylib linker boundary (which
/// fails when fixtures/example crates are independently-compiled
/// workspaces). The host's `FfiTriggerImpl` adapter sends one of these
/// per scheduled poll. (T-0553 follow-up — Trigger FFI bridge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerInvokeRequest {
    /// Inventory-registered trigger name to poll.
    pub trigger_name: String,
}

/// Result of a cross-FFI trigger poll. Mirrors `cloacina_workflow::TriggerResult`
/// but in a wire-format-friendly shape: the `Context` becomes a JSON
/// string so it can travel through the bincode boundary. The host's
/// `FfiTriggerImpl::poll` reconstructs `TriggerResult` from this. (T-0553
/// follow-up — Trigger FFI bridge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerInvokeResult {
    /// `true` if the trigger returned `Fire(_)`, `false` if it returned
    /// `Skip`. When `error` is set, this field is meaningless.
    pub fire: bool,
    /// Serialized `Context<serde_json::Value>` for the `Fire(Some(ctx))`
    /// case. `None` for `Fire(None)` and `Skip`.
    pub context_json: Option<String>,
    /// When the cdylib's `poll()` returned `Err(_)` or could not find the
    /// requested trigger by name, this carries a description. The host
    /// converts it to `TriggerError::PollError` so the polling supervisor
    /// can log + back off.
    pub error: Option<String>,
}

/// Metadata for a single trigger declared by this package, returned by
/// `get_trigger_metadata()`. The reconciler routes cron-shaped triggers
/// (`cron_expression.is_some()`) to the cron scheduler and custom-poll
/// triggers to the runtime trigger registry. (T-A — I-0102)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerPackageMetadata {
    /// Trigger name.
    pub name: String,
    /// Target workflow this trigger fires — the `#[trigger(on = "...")]`
    /// binding. For cron triggers the reconciler registers the schedule against
    /// THIS workflow, not `name` (which is just the trigger's identity).
    /// `#[serde(default)]` keeps older package metadata (pre-CLOACI-T-0669,
    /// without this field) deserializable. Empty falls back to `name`.
    #[serde(default)]
    pub workflow_name: String,
    /// Cargo package name (sourcing context for diagnostics).
    pub package_name: String,
    /// Polling interval as a humantime-parseable string (e.g., "5s", "1m").
    /// Ignored when `cron_expression.is_some()`.
    pub poll_interval: String,
    /// Cron expression (e.g., "*/10 * * * *"). When present, the reconciler
    /// routes this trigger to the cron scheduler.
    #[serde(default)]
    pub cron_expression: Option<String>,
    /// Whether concurrent executions are allowed.
    #[serde(default)]
    pub allow_concurrent: bool,
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
#[serde(deny_unknown_fields)]
pub struct CloacinaMetadata {
    /// Name of the workflow. Optional — Rust packages source this from
    /// `#[workflow(name = "...")]`; Python packages still set it here.
    #[serde(default)]
    pub workflow_name: Option<String>,
    /// Name of the computation graph. Used by Python CG packages and as
    /// the signal that distinguishes CG from workflow packages on the
    /// Python load path. (T-E: replaces the old `package_type` field.)
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

impl CloacinaMetadata {
    /// Check if this package contains a workflow.
    /// (T-E: post-removal of `package_type`, "is workflow" is "not CG-only".)
    pub fn has_workflow(&self) -> bool {
        self.graph_name.is_none() || self.workflow_name.is_some()
    }

    /// Check if this package contains a computation graph.
    /// (T-E: post-removal of `package_type`, presence of `graph_name`
    /// signals a CG package on the Python load path. Rust packages get
    /// the actual signal from FFI metadata extraction.)
    pub fn has_computation_graph(&self) -> bool {
        self.graph_name.is_some()
    }

    /// Get the workflow name as a `&str`. Used by the cloacina-python
    /// loader to derive the workflow registry key from the manifest.
    pub fn effective_workflow_name(&self) -> Option<&str> {
        self.workflow_name.as_deref()
    }
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
            trigger_rules: "{\"type\":\"Always\"}".to_string(),
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
                trigger_rules: "{\"type\":\"Always\"}".to_string(),
            }],
            triggers: Vec::new(),
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
    fn test_cloacina_metadata_workflow_classification() {
        // T-E / I-0102: post-removal of `package_type`, has_workflow() /
        // has_computation_graph() are derived from the presence of
        // workflow_name / graph_name. A package with only workflow_name is
        // a workflow; one with only graph_name is a CG; one with both is
        // both.
        let toml_str = r#"
            workflow_name = "legacy_workflow"
            language = "rust"
        "#;
        let metadata: CloacinaMetadata = toml::from_str(toml_str).unwrap();
        assert!(metadata.has_workflow());
        assert!(!metadata.has_computation_graph());
    }

    #[test]
    fn test_cloacina_metadata_computation_graph_from_toml() {
        let toml_str = r#"
            graph_name = "market_maker"
            language = "rust"
            reaction_mode = "when_any"
            input_strategy = "latest"
        "#;

        let metadata: CloacinaMetadata = toml::from_str(toml_str).unwrap();
        assert!(metadata.has_computation_graph());
        assert_eq!(metadata.graph_name.as_deref(), Some("market_maker"));
        assert_eq!(metadata.reaction_mode.as_deref(), Some("when_any"));
        assert!(metadata.workflow_name.is_none());
    }

    #[test]
    fn test_cloacina_metadata_legacy_package_type_rejected() {
        // T-E / I-0102: deny_unknown_fields makes legacy `package_type` a
        // hard error at the deserializer; the reconciler rewraps with a
        // friendly migration message.
        let toml_str = r#"
            package_type = ["computation_graph"]
            workflow_name = "x"
            language = "rust"
        "#;
        let err = toml::from_str::<CloacinaMetadata>(toml_str).unwrap_err();
        assert!(
            err.to_string().contains("package_type"),
            "expected error to name `package_type`, got: {}",
            err
        );
    }

    #[test]
    fn test_cloacina_metadata_legacy_triggers_rejected() {
        let toml_str = r#"
            workflow_name = "x"
            language = "rust"

            [[triggers]]
            name = "t"
            workflow = "x"
            poll_interval = "5s"
            allow_concurrent = false
        "#;
        let err = toml::from_str::<CloacinaMetadata>(toml_str).unwrap_err();
        assert!(
            err.to_string().contains("triggers"),
            "expected error to name `triggers`, got: {}",
            err
        );
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
            trigger_reactor: None,
            graph_data_json: None,
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
