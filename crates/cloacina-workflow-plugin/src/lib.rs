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

pub mod inventory_entries;
pub mod types;

pub use inventory_entries::ReactorEntry;

// Re-export the interface types for convenience
pub use types::{
    AccumulatorDeclarationEntry, CloacinaMetadata, GraphExecutionRequest, GraphExecutionResult,
    GraphPackageMetadata, PackageTasksMetadata, ReactorPackageMetadata, TaskExecutionRequest,
    TaskExecutionResult, TaskMetadataEntry, TriggerDefinition, TriggerPackageMetadata,
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

/// Unified plugin shell macro for I-0102.
///
/// Single-line invocation at the crate root of any packaged Rust cdylib:
///
/// ```rust,ignore
/// cloacina::package!();
/// ```
///
/// Emits, gated on `#[cfg(feature = "packaged")]`:
///
/// 1. A `pub mod _ffi` containing a `CloacinaPackagePlugin` unit struct.
/// 2. A `#[plugin_impl]` block exporting the version-2 `CloacinaPlugin`
///    trait. Six methods: `get_task_metadata`, `execute_task`,
///    `get_graph_metadata`, `execute_graph`, `get_reactor_metadata`,
///    `get_trigger_metadata`. The reactor body walks
///    `inventory::iter::<ReactorEntry>` and projects each entry to a
///    `ReactorPackageMetadata` value. The other bodies are stubs at this
///    iteration of T-A — they ship empty/NotImplemented and will be
///    fleshed out as the remaining inventory entry types relocate to leaf
///    crates reachable from packaged cdylibs.
/// 3. A `mod __cloacina_package_marker { pub struct Once; }` guard. A
///    second invocation in the same crate fails to compile (duplicate
///    module name).
/// 4. A `fidius_plugin_registry!()` invocation to export the plugin.
///
/// **Coexistence:** in T-A the per-macro `_ffi` emission from
/// `#[computation_graph]` and `#[workflow]` is unchanged. A crate that
/// adds `cloacina::package!();` AND has `#[computation_graph]` /
/// `#[workflow]` would emit two `fidius_plugin_registry!()` calls →
/// linker conflict. T-C strips the per-macro emission so the shell
/// becomes the only path.
#[macro_export]
macro_rules! package {
    () => {
        #[cfg(feature = "packaged")]
        pub mod _ffi {
            use $crate::__fidius_CloacinaPlugin;
            use $crate::CloacinaPlugin as _;

            // Single-emission guard: a duplicate `cloacina::package!()` in
            // the same crate produces "the name `__cloacina_package_marker`
            // is defined multiple times" at compile time.
            mod __cloacina_package_marker {
                pub struct Once;
            }

            pub struct CloacinaPackagePlugin;

            #[$crate::plugin_impl(CloacinaPlugin, crate = "cloacina_workflow_plugin")]
            impl $crate::CloacinaPlugin for CloacinaPackagePlugin {
                fn get_task_metadata(
                    &self,
                ) -> ::core::result::Result<
                    $crate::PackageTasksMetadata,
                    $crate::PluginError,
                > {
                    // Stub at this iteration of T-A. Will walk
                    // inventory::iter::<TaskEntry> once TaskEntry
                    // relocates to a cdylib-reachable crate.
                    Ok($crate::PackageTasksMetadata {
                        workflow_name: ::std::string::String::new(),
                        package_name: env!("CARGO_PKG_NAME").to_string(),
                        package_description: None,
                        package_author: None,
                        workflow_fingerprint: None,
                        graph_data_json: None,
                        tasks: ::std::vec::Vec::new(),
                        triggers: ::std::vec::Vec::new(),
                    })
                }

                fn execute_task(
                    &self,
                    request: $crate::TaskExecutionRequest,
                ) -> ::core::result::Result<
                    $crate::TaskExecutionResult,
                    $crate::PluginError,
                > {
                    Ok($crate::TaskExecutionResult {
                        success: false,
                        context_json: None,
                        error: Some(format!(
                            "task '{}' is not registered: cloacina::package!() shell does not yet \
                             walk TaskEntry inventory (T-A pending follow-up)",
                            request.task_name,
                        )),
                    })
                }

                fn get_graph_metadata(
                    &self,
                ) -> ::core::result::Result<
                    $crate::GraphPackageMetadata,
                    $crate::PluginError,
                > {
                    Err($crate::PluginError {
                        code: "NOT_SUPPORTED".to_string(),
                        message: "cloacina::package!() shell does not yet walk \
                                  ComputationGraphEntry inventory (T-A pending follow-up)"
                            .to_string(),
                        details: None,
                    })
                }

                fn execute_graph(
                    &self,
                    _request: $crate::GraphExecutionRequest,
                ) -> ::core::result::Result<
                    $crate::GraphExecutionResult,
                    $crate::PluginError,
                > {
                    Err($crate::PluginError {
                        code: "NOT_SUPPORTED".to_string(),
                        message: "cloacina::package!() shell does not yet dispatch \
                                  to ComputationGraphEntry inventory (T-A pending follow-up)"
                            .to_string(),
                        details: None,
                    })
                }

                fn get_reactor_metadata(
                    &self,
                ) -> ::core::result::Result<
                    ::std::vec::Vec<$crate::ReactorPackageMetadata>,
                    $crate::PluginError,
                > {
                    let mut out: ::std::vec::Vec<$crate::ReactorPackageMetadata> =
                        ::std::vec::Vec::new();
                    for entry in $crate::inventory::iter::<$crate::ReactorEntry> {
                        let reg = (entry.constructor)();
                        let accumulators: ::std::vec::Vec<
                            $crate::AccumulatorDeclarationEntry,
                        > = reg
                            .accumulator_names
                            .iter()
                            .map(|name| $crate::AccumulatorDeclarationEntry {
                                name: name.clone(),
                                accumulator_type: "passthrough".to_string(),
                                config: ::std::collections::HashMap::new(),
                            })
                            .collect();
                        out.push($crate::ReactorPackageMetadata {
                            name: reg.name,
                            package_name: env!("CARGO_PKG_NAME").to_string(),
                            reaction_mode: reg.reaction_mode.as_str().to_string(),
                            accumulators,
                        });
                    }
                    Ok(out)
                }

                fn get_trigger_metadata(
                    &self,
                ) -> ::core::result::Result<
                    ::std::vec::Vec<$crate::TriggerPackageMetadata>,
                    $crate::PluginError,
                > {
                    // Stub at this iteration of T-A. Will walk
                    // inventory::iter::<TriggerEntry> once TriggerEntry
                    // relocates to a cdylib-reachable crate.
                    Ok(::std::vec::Vec::new())
                }
            }

            $crate::fidius_plugin_registry!();
        }
    };
}

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
#[fidius::plugin_interface(version = 2, buffer = PluginAllocated)]
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
    /// Method index 2. Only called when the package declares a CG.
    /// Packages without a graph return an error or NotImplemented.
    fn get_graph_metadata(&self) -> Result<GraphPackageMetadata, PluginError>;

    /// Executes the computation graph with the given cache state.
    /// Method index 3. Only called when the package declares a CG.
    /// Packages without a graph return an error.
    fn execute_graph(
        &self,
        request: GraphExecutionRequest,
    ) -> Result<GraphExecutionResult, PluginError>;

    /// Returns metadata about all reactors declared by this package.
    /// Method index 4. Optional (since version 2): plugins built against
    /// version-1 hosts return `CallError::NotImplemented`, which the
    /// reconciler treats as "package declares no reactors". Packages built
    /// from the unified `cloacina::package!()` shell walk their local
    /// `inventory::iter::<ReactorEntry>` and project each entry into a
    /// `ReactorPackageMetadata` value. (T-A — I-0102)
    #[optional(since = 2)]
    fn get_reactor_metadata(&self) -> Result<Vec<ReactorPackageMetadata>, PluginError>;

    /// Returns metadata about all triggers declared by this package.
    /// Method index 5. Optional (since version 2): same NotImplemented
    /// fallback as `get_reactor_metadata`. The unified `cloacina::package!()`
    /// shell walks `inventory::iter::<TriggerEntry>`, calls each entry's
    /// constructor, and queries `poll_interval()` / `cron_expression()` /
    /// `allow_concurrent()` on the resulting `Arc<dyn Trigger>`. The
    /// reconciler routes cron-shaped entries (cron_expression present) to
    /// the cron scheduler and the rest to the runtime trigger registry.
    /// (T-A — I-0102)
    #[optional(since = 2)]
    fn get_trigger_metadata(&self) -> Result<Vec<TriggerPackageMetadata>, PluginError>;
}
