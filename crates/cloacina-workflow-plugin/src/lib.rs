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

//! Cloacina plugin interface for the fidius plugin system.
//!
//! This crate defines the plugin contract between cloacina workflow packages
//! (compiled as cdylib plugins) and the cloacina host engine. Both sides
//! depend on this crate — it is the single source of truth for the FFI ABI.
//!
//! # For plugin authors
//!
//! Use `#[workflow]` with `features = ["packaged"]` — the macro generates
//! the `#[plugin_impl(CloacinaPlugin)]` code automatically. You don't need
//! to implement this trait directly.
//!
//! # For host integration
//!
//! Use `fidius-host` to load plugins and call methods through `PluginHandle`.
//! Validate loaded plugins against `CloacinaPlugin_INTERFACE_HASH` to detect
//! ABI drift at load time.

pub mod types;

// Re-export the interface types for convenience
pub use types::{
    AccumulatorDeclarationEntry, CloacinaMetadata, GraphExecutionRequest, GraphExecutionResult,
    GraphPackageMetadata, PackageTasksMetadata, TaskExecutionRequest, TaskExecutionResult,
    TaskMetadataEntry, TriggerDefinition,
};

// Re-export fidius crates so generated code can reference them
pub use fidius;
pub use fidius_core;

// Re-export fidius modules needed by generated code when crate = "cloacina_workflow_plugin"
pub use fidius_core::descriptor;
pub use fidius_core::inventory;
pub use fidius_core::registry;
pub use fidius_core::status;
pub use fidius_core::wire;

// Re-export fidius types that plugin authors need
pub use fidius::plugin_impl;
pub use fidius_core::error::PluginError;
pub use fidius_core::package::{PackageHeader, PackageManifest};

// Re-export the plugin registry macro
pub use fidius::fidius_plugin_registry;

/// The plugin interface for cloacina workflow packages.
///
/// Every packaged workflow implements this trait (via `#[plugin_impl]` generated
/// by the `#[workflow]` macro). The host calls these methods through a fidius
/// `PluginHandle` — never directly.
///
/// ## Methods
///
/// - `get_task_metadata` — Returns metadata about all tasks in the workflow
///   (IDs, dependencies, descriptions). Called once at registration time.
///
/// - `execute_task` — Runs a specific task by name with a JSON-serialized
///   context. Returns the updated context or an error.
#[fidius::plugin_interface(version = 1, buffer = PluginAllocated)]
pub trait CloacinaPlugin: Send + Sync {
    /// Returns metadata about all tasks in this workflow package.
    /// Method index 0.
    fn get_task_metadata(&self) -> Result<PackageTasksMetadata, PluginError>;

    /// Executes a task by name with the given context.
    /// Method index 1.
    fn execute_task(
        &self,
        request: TaskExecutionRequest,
    ) -> Result<TaskExecutionResult, PluginError>;

    /// Returns metadata about the computation graph in this package.
    /// Method index 2. Only called when package_type includes "computation_graph".
    /// Workflow-only plugins should return an error.
    fn get_graph_metadata(&self) -> Result<GraphPackageMetadata, PluginError>;

    /// Executes the computation graph with the given cache state.
    /// Method index 3. Only called when package_type includes "computation_graph".
    /// Workflow-only plugins should return an error.
    fn execute_graph(
        &self,
        request: GraphExecutionRequest,
    ) -> Result<GraphExecutionResult, PluginError>;
}
