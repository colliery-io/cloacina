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

pub use inventory_entries::{
    ComputationGraphEntry, ReactorEntry, TaskEntry, TriggerEntry, TriggerlessGraph,
    TriggerlessGraphEntry, TriggerlessGraphFn, TriggerlessGraphRegistration,
    WorkflowDescriptorEntry,
};

// Re-export the interface types for convenience
pub use types::{
    AccumulatorDeclarationEntry, CloacinaMetadata, GraphExecutionRequest, GraphExecutionResult,
    GraphPackageMetadata, PackageTasksMetadata, ReactorPackageMetadata, TaskExecutionRequest,
    TaskExecutionResult, TaskMetadataEntry, TriggerDefinition, TriggerInvokeRequest,
    TriggerInvokeResult, TriggerPackageMetadata,
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
            use $crate::CloacinaPlugin as _;
            use $crate::__fidius_CloacinaPlugin;

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
                ) -> ::core::result::Result<$crate::PackageTasksMetadata, $crate::PluginError>
                {
                    let mut tasks: ::std::vec::Vec<$crate::TaskMetadataEntry> =
                        ::std::vec::Vec::new();
                    let mut workflow_name: ::std::string::String = ::std::string::String::new();
                    for (idx, entry) in $crate::inventory::iter::<$crate::TaskEntry>
                        .into_iter()
                        .enumerate()
                    {
                        let ns = (entry.namespace)();
                        if workflow_name.is_empty() {
                            workflow_name = ns.workflow_id.clone();
                        }
                        let task = (entry.constructor)();
                        let dependencies: ::std::vec::Vec<::std::string::String> =
                            cloacina_workflow::Task::dependencies(&*task)
                                .iter()
                                .map(|n| n.task_id.clone())
                                .collect();
                        tasks.push($crate::TaskMetadataEntry {
                            index: idx as u32,
                            id: cloacina_workflow::Task::id(&*task).to_string(),
                            namespaced_id_template: format!(
                                "{}::{}::{}::{}",
                                ns.tenant_id, ns.package_name, ns.workflow_id, ns.task_id,
                            ),
                            dependencies,
                            description: format!("Task: {}", cloacina_workflow::Task::id(&*task)),
                            source_location: format!("{}/lib.rs", env!("CARGO_PKG_NAME")),
                        });
                    }
                    // Look up WorkflowDescriptorEntry for description / author /
                    // fingerprint / graph_data / triggers. Multiple workflows in
                    // the same cdylib aren't supported by the shell — the first
                    // descriptor wins; T-D fixtures verify single-workflow
                    // assumption.
                    let descriptor = $crate::inventory::iter::<$crate::WorkflowDescriptorEntry>
                        .into_iter()
                        .next();
                    let (description, author, fingerprint, graph_data_json, triggers) =
                        match descriptor {
                            Some(d) => (
                                if d.description.is_empty() {
                                    None
                                } else {
                                    Some(d.description.to_string())
                                },
                                if d.author.is_empty() {
                                    None
                                } else {
                                    Some(d.author.to_string())
                                },
                                if d.fingerprint.is_empty() {
                                    None
                                } else {
                                    Some(d.fingerprint.to_string())
                                },
                                if d.graph_data_json.is_empty() {
                                    None
                                } else {
                                    Some(d.graph_data_json.to_string())
                                },
                                (d.triggers)(),
                            ),
                            None => (None, None, None, None, ::std::vec::Vec::new()),
                        };
                    Ok($crate::PackageTasksMetadata {
                        workflow_name,
                        package_name: env!("CARGO_PKG_NAME").to_string(),
                        package_description: description,
                        package_author: author,
                        workflow_fingerprint: fingerprint,
                        graph_data_json,
                        tasks,
                        triggers,
                    })
                }

                fn execute_task(
                    &self,
                    request: $crate::TaskExecutionRequest,
                ) -> ::core::result::Result<$crate::TaskExecutionResult, $crate::PluginError>
                {
                    use $crate::CloacinaPlugin as _;
                    static CDYLIB_RUNTIME: ::std::sync::OnceLock<
                        cloacina_workflow::__private::tokio::runtime::Runtime,
                    > = ::std::sync::OnceLock::new();
                    let rt = CDYLIB_RUNTIME.get_or_init(|| {
                        cloacina_workflow::__private::tokio::runtime::Builder::new_multi_thread()
                            .enable_all()
                            .worker_threads(2)
                            .thread_name("package-shell-cdylib-worker")
                            .build()
                            .expect("Failed to create cdylib tokio runtime")
                    });

                    // Resolve the named task by walking inventory.
                    let task_arc_opt = $crate::inventory::iter::<$crate::TaskEntry>
                        .into_iter()
                        .map(|entry| (entry.constructor)())
                        .find(|t| cloacina_workflow::Task::id(&**t) == request.task_name);

                    let task = match task_arc_opt {
                        Some(t) => t,
                        None => {
                            return Ok($crate::TaskExecutionResult {
                                success: false,
                                context_json: None,
                                error: Some(format!("Unknown task: {}", request.task_name)),
                            });
                        }
                    };

                    let context: cloacina_workflow::Context<::serde_json::Value> =
                        match cloacina_workflow::Context::from_json(request.context_json) {
                            Ok(c) => c,
                            Err(e) => {
                                return Err($crate::PluginError {
                                    code: "CONTEXT_ERROR".to_string(),
                                    message: format!("Failed to parse context: {}", e),
                                    details: None,
                                });
                            }
                        };

                    let result = rt.block_on(async move {
                        cloacina_workflow::Task::execute(&*task, context).await
                    });

                    match result {
                        Ok(updated) => {
                            let ctx_json = updated.to_json().map_err(|e| $crate::PluginError {
                                code: "SERIALIZATION_ERROR".to_string(),
                                message: format!("Failed to serialize context: {}", e),
                                details: None,
                            })?;
                            Ok($crate::TaskExecutionResult {
                                success: true,
                                context_json: Some(ctx_json),
                                error: None,
                            })
                        }
                        Err(e) => Ok($crate::TaskExecutionResult {
                            success: false,
                            context_json: None,
                            error: Some(format!("Task '{}' failed: {:?}", request.task_name, e)),
                        }),
                    }
                }

                fn get_graph_metadata(
                    &self,
                ) -> ::core::result::Result<$crate::GraphPackageMetadata, $crate::PluginError>
                {
                    let entries: ::std::vec::Vec<&$crate::ComputationGraphEntry> =
                        $crate::inventory::iter::<$crate::ComputationGraphEntry>
                            .into_iter()
                            .collect();
                    if entries.is_empty() {
                        return Err($crate::PluginError {
                            code: "NOT_SUPPORTED".to_string(),
                            message: "Package declares no computation graph".to_string(),
                            details: None,
                        });
                    }
                    if entries.len() > 1 {
                        return Err($crate::PluginError {
                            code: "MULTIPLE_GRAPHS".to_string(),
                            message: format!(
                                "Package declares {} computation graphs; the unified shell \
                                 supports at most one CG per cdylib",
                                entries.len()
                            ),
                            details: None,
                        });
                    }
                    let reg = (entries[0].constructor)();
                    let accumulators: ::std::vec::Vec<$crate::AccumulatorDeclarationEntry> = reg
                        .accumulator_names
                        .iter()
                        .map(|name| $crate::AccumulatorDeclarationEntry {
                            name: name.clone(),
                            accumulator_type: "passthrough".to_string(),
                            config: ::std::collections::HashMap::new(),
                        })
                        .collect();
                    Ok($crate::GraphPackageMetadata {
                        graph_name: entries[0].name.to_string(),
                        package_name: env!("CARGO_PKG_NAME").to_string(),
                        reaction_mode: reg.reaction_mode.clone(),
                        input_strategy: "latest".to_string(),
                        accumulators,
                        trigger_reactor: reg.trigger_reactor.clone(),
                    })
                }

                fn execute_graph(
                    &self,
                    request: $crate::GraphExecutionRequest,
                ) -> ::core::result::Result<$crate::GraphExecutionResult, $crate::PluginError>
                {
                    static CDYLIB_RUNTIME: ::std::sync::OnceLock<
                        cloacina_workflow::__private::tokio::runtime::Runtime,
                    > = ::std::sync::OnceLock::new();
                    let rt = CDYLIB_RUNTIME.get_or_init(|| {
                        cloacina_workflow::__private::tokio::runtime::Builder::new_multi_thread()
                            .enable_all()
                            .worker_threads(2)
                            .thread_name("package-shell-cg-worker")
                            .build()
                            .expect("Failed to create cdylib tokio runtime for computation graph")
                    });

                    let entry_opt = $crate::inventory::iter::<$crate::ComputationGraphEntry>
                        .into_iter()
                        .next();
                    let entry = match entry_opt {
                        Some(e) => e,
                        None => {
                            return Err($crate::PluginError {
                                code: "NOT_SUPPORTED".to_string(),
                                message: "Package declares no computation graph".to_string(),
                                details: None,
                            });
                        }
                    };
                    let reg = (entry.constructor)();

                    let mut cache = cloacina_computation_graph::InputCache::new();
                    for (source_name, json_str) in &request.cache {
                        let value: ::serde_json::Value =
                            ::serde_json::from_str(json_str).map_err(|e| $crate::PluginError {
                                code: "DESERIALIZATION_ERROR".to_string(),
                                message: format!(
                                    "Failed to parse cache entry '{}': {}",
                                    source_name, e
                                ),
                                details: None,
                            })?;
                        let bytes = cloacina_computation_graph::serialize(&value).map_err(|e| {
                            $crate::PluginError {
                                code: "SERIALIZATION_ERROR".to_string(),
                                message: format!(
                                    "Failed to serialize cache entry '{}': {}",
                                    source_name, e
                                ),
                                details: None,
                            }
                        })?;
                        cache.update(
                            cloacina_computation_graph::SourceName::new(source_name),
                            bytes,
                        );
                    }

                    let result = rt.block_on(async { (reg.graph_fn)(cache).await });

                    match result {
                        cloacina_computation_graph::GraphResult::Completed { outputs } => {
                            let terminal_json: ::std::vec::Vec<::std::string::String> = outputs
                                .iter()
                                .filter_map(|o| {
                                    o.downcast_ref::<::serde_json::Value>()
                                        .map(|v| ::serde_json::to_string(v).unwrap_or_default())
                                })
                                .collect();
                            Ok($crate::GraphExecutionResult {
                                success: true,
                                terminal_outputs_json: if terminal_json.is_empty() {
                                    None
                                } else {
                                    Some(terminal_json)
                                },
                                error: None,
                            })
                        }
                        cloacina_computation_graph::GraphResult::Error(e) => {
                            Ok($crate::GraphExecutionResult {
                                success: false,
                                terminal_outputs_json: None,
                                error: Some(format!("{}", e)),
                            })
                        }
                    }
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
                        let accumulators: ::std::vec::Vec<$crate::AccumulatorDeclarationEntry> =
                            reg.accumulator_names
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
                    let mut out: ::std::vec::Vec<$crate::TriggerPackageMetadata> =
                        ::std::vec::Vec::new();
                    for entry in $crate::inventory::iter::<$crate::TriggerEntry> {
                        let trigger = (entry.constructor)();
                        let poll_interval = format!(
                            "{}ms",
                            cloacina_workflow::Trigger::poll_interval(&*trigger).as_millis()
                        );
                        out.push($crate::TriggerPackageMetadata {
                            name: entry.name.to_string(),
                            package_name: env!("CARGO_PKG_NAME").to_string(),
                            poll_interval,
                            cron_expression: cloacina_workflow::Trigger::cron_expression(&*trigger),
                            allow_concurrent: cloacina_workflow::Trigger::allow_concurrent(
                                &*trigger,
                            ),
                        });
                    }
                    Ok(out)
                }

                fn invoke_trigger_poll(
                    &self,
                    request: $crate::TriggerInvokeRequest,
                ) -> ::core::result::Result<$crate::TriggerInvokeResult, $crate::PluginError>
                {
                    static CDYLIB_TRIGGER_RUNTIME: ::std::sync::OnceLock<
                        cloacina_workflow::__private::tokio::runtime::Runtime,
                    > = ::std::sync::OnceLock::new();
                    let rt = CDYLIB_TRIGGER_RUNTIME.get_or_init(|| {
                        cloacina_workflow::__private::tokio::runtime::Builder::new_multi_thread()
                            .enable_all()
                            .worker_threads(2)
                            .thread_name("package-shell-trigger-worker")
                            .build()
                            .expect("Failed to create cdylib trigger tokio runtime")
                    });

                    let trigger_arc_opt = $crate::inventory::iter::<$crate::TriggerEntry>
                        .into_iter()
                        .find(|entry| entry.name == request.trigger_name)
                        .map(|entry| (entry.constructor)());

                    let trigger = match trigger_arc_opt {
                        Some(t) => t,
                        None => {
                            return Ok($crate::TriggerInvokeResult {
                                fire: false,
                                context_json: None,
                                error: Some(format!("Unknown trigger: {}", request.trigger_name)),
                            });
                        }
                    };

                    let poll_result = rt
                        .block_on(async move { cloacina_workflow::Trigger::poll(&*trigger).await });

                    match poll_result {
                        Ok(cloacina_workflow::TriggerResult::Skip) => {
                            Ok($crate::TriggerInvokeResult {
                                fire: false,
                                context_json: None,
                                error: None,
                            })
                        }
                        Ok(cloacina_workflow::TriggerResult::Fire(None)) => {
                            Ok($crate::TriggerInvokeResult {
                                fire: true,
                                context_json: None,
                                error: None,
                            })
                        }
                        Ok(cloacina_workflow::TriggerResult::Fire(Some(ctx))) => {
                            match ctx.to_json() {
                                Ok(ctx_json) => Ok($crate::TriggerInvokeResult {
                                    fire: true,
                                    context_json: Some(ctx_json),
                                    error: None,
                                }),
                                Err(e) => Err($crate::PluginError {
                                    code: "SERIALIZATION_ERROR".to_string(),
                                    message: format!("Failed to serialize trigger context: {}", e),
                                    details: None,
                                }),
                            }
                        }
                        Err(e) => Ok($crate::TriggerInvokeResult {
                            fire: false,
                            context_json: None,
                            error: Some(format!(
                                "Trigger '{}' poll failed: {:?}",
                                request.trigger_name, e
                            )),
                        }),
                    }
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

    /// Polls a named trigger across the FFI boundary and returns a wire-
    /// format `TriggerInvokeResult` describing whether to fire the
    /// associated workflow. Method index 6. Optional (since version 2):
    /// the host's `FfiTriggerImpl` adapter calls this on every scheduled
    /// poll for triggers that came from a packaged cdylib (where
    /// `inventory` doesn't span linker boundaries, so the host can't
    /// build a host-side `Arc<dyn Trigger>` directly). Plugins built
    /// from the unified `cloacina::package!()` shell walk
    /// `inventory::iter::<TriggerEntry>` for the matching name, call
    /// the constructor, and dispatch `Trigger::poll()` through the
    /// shared cdylib tokio runtime.
    #[optional(since = 2)]
    fn invoke_trigger_poll(
        &self,
        request: TriggerInvokeRequest,
    ) -> Result<TriggerInvokeResult, PluginError>;
}
