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

use serde::{Deserialize, Serialize};

/// Configuration for building a package
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuildPackageRequest {
    /// Path to the Rust project directory
    pub project_path: String,
    /// Output path for the .cloacina package
    pub output_path: String,
    /// Target triple for cross-compilation (optional)
    pub target: Option<String>,
    /// Build profile (debug or release)
    pub profile: String,
    /// Additional cargo build flags
    pub cargo_flags: Vec<String>,
}

/// Response from building a package
#[derive(Serialize, Deserialize, Debug)]
pub struct BuildPackageResponse {
    /// Whether the build was successful
    pub success: bool,
    /// Build output/log messages
    pub output: String,
    /// Error message if build failed
    pub error: Option<String>,
    /// Path to the generated package file
    pub package_path: Option<String>,
}

/// Request to inspect a package
#[derive(Serialize, Deserialize, Debug)]
pub struct InspectPackageRequest {
    /// Path to the .cloacina package file
    pub package_path: String,
    /// Output format (human or json)
    pub format: String,
}

/// Response from inspecting a package
#[derive(Serialize, Deserialize, Debug)]
pub struct InspectPackageResponse {
    /// Whether inspection was successful
    pub success: bool,
    /// Package manifest/information
    pub manifest: Option<serde_json::Value>,
    /// Human-readable package information
    pub info: Option<String>,
    /// Error message if inspection failed
    pub error: Option<String>,
}

/// Request to debug a package
#[derive(Serialize, Deserialize, Debug)]
pub struct DebugPackageRequest {
    /// Path to the .cloacina package file
    pub package_path: String,
    /// Task identifier (name or index)
    pub task_identifier: Option<String>,
    /// JSON context for task execution
    pub context: Option<String>,
    /// Environment variables
    pub env_vars: Option<Vec<String>>,
}

/// Response from debugging a package
#[derive(Serialize, Deserialize, Debug)]
pub struct DebugPackageResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// List of available tasks (if listing)
    pub tasks: Option<Vec<TaskInfo>>,
    /// Task execution output (if executing)
    pub output: Option<String>,
    /// Error message if operation failed
    pub error: Option<String>,
}

/// Information about a task in a package
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskInfo {
    /// Task index
    pub index: usize,
    /// Task identifier/name
    pub id: String,
    /// Task description
    pub description: String,
    /// Task dependencies
    pub dependencies: Vec<String>,
    /// Source location in code
    pub source_location: String,
}

/// Request to visualize a package
#[derive(Serialize, Deserialize, Debug)]
pub struct VisualizePackageRequest {
    /// Path to the .cloacina package file
    pub package_path: String,
    /// Layout style (horizontal, compact, etc.)
    pub layout: String,
    /// Whether to show detailed information
    pub details: bool,
    /// Output format (ascii, json for web rendering)
    pub format: String,
}

/// Response from visualizing a package
#[derive(Serialize, Deserialize, Debug)]
pub struct VisualizePackageResponse {
    /// Whether visualization was successful
    pub success: bool,
    /// Visualization data (ASCII or JSON for D3.js)
    pub visualization: Option<String>,
    /// Graph data for web rendering
    pub graph_data: Option<GraphData>,
    /// Error message if visualization failed
    pub error: Option<String>,
}

/// Graph data for D3.js visualization
#[derive(Serialize, Deserialize, Debug)]
pub struct GraphData {
    /// List of nodes (tasks)
    pub nodes: Vec<GraphNode>,
    /// List of edges (dependencies)
    pub edges: Vec<GraphEdge>,
}

/// A node in the dependency graph
#[derive(Serialize, Deserialize, Debug)]
pub struct GraphNode {
    /// Node ID (task name)
    pub id: String,
    /// Display label
    pub label: String,
    /// Node description
    pub description: String,
    /// Node type/category
    pub node_type: String,
    /// Visual properties
    pub x: Option<f64>,
    pub y: Option<f64>,
}

/// An edge in the dependency graph
#[derive(Serialize, Deserialize, Debug)]
pub struct GraphEdge {
    /// Source node ID
    pub source: String,
    /// Target node ID
    pub target: String,
    /// Edge type
    pub edge_type: String,
}
