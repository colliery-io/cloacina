/*
 *  Copyright 2026 Colliery Software
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

//! The full metadata document of a built package (CLOACINA-T-1074).
//!
//! A package's metadata is in its code: a Rust library answers FFI calls, a
//! Python package registers tasks and triggers when it is imported. Reading it
//! runs the package's code. On a server that code must not run in the server
//! process (CLOACINA-A-0019), so a child process of the tenant's compiler
//! reads it once and writes a [`PackageDescription`]. The compiler stores the
//! document in `workflow_packages.metadata_document`.
//!
//! The document holds every fact the load path reads from the code today:
//! tasks with their dependencies, trigger rules, retry policy, time limit and
//! placement; triggers and cron schedules; reactors, the graph, trigger-less
//! graphs and constructor nodes; declared params, secrets and surfaces. It
//! reuses the wire types of `cloacina-workflow-plugin`, so a Rust and a Python
//! package give a document of one shape.

use serde::{Deserialize, Serialize};

use cloacina_workflow::RetryPolicy;
use cloacina_workflow_plugin::types::{AccumulatorConfig, AccumulatorDeclarationEntry};
use cloacina_workflow_plugin::{
    ConstructorPackageMetadata, GraphPackageMetadata, PackageTasksMetadata, ReactorPackageMetadata,
    TriggerPackageMetadata, TriggerlessGraphMetadataEntry,
};

use crate::Runtime;
pub use crate::registry::in_process::{InProcessReason, ProviderDescription};
use crate::registry::loader::package_loader::{PackageMetadata, TaskDocs, TaskMetadata};
use crate::task::TaskNamespace;

/// The format version this build writes into
/// [`PackageDescription::format_version`]. Raise it when a field is added
/// that a reader needs: the compiler then describes each stored package again.
///
/// - 1: CLOACINA-T-1074.
/// - 2 (CLOACINA-T-1078): the bundled providers with their runtime, the list
///   of what would run in process, Python constructor declarations, and a
///   Python graph's accumulators with their authored types.
pub const PACKAGE_DESCRIPTION_VERSION: u32 = 2;

/// The full metadata of one built package. See the module docs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDescription {
    /// The format of this document: [`PACKAGE_DESCRIPTION_VERSION`] of the
    /// compiler that wrote it.
    pub format_version: u32,
    /// `"rust"` or `"python"`.
    pub language: String,
    /// The version of the compiler that wrote the document.
    pub described_by: String,
    /// The package name, as the package's code reports it.
    pub package_name: String,
    /// The workflow the package registers. Empty for a package with no
    /// workflow (a graph, reactor or trigger package).
    #[serde(default)]
    pub workflow_name: String,
    /// The package description, from the code.
    #[serde(default)]
    pub description: Option<String>,
    /// The package author, from the code.
    #[serde(default)]
    pub author: Option<String>,
    /// The workflow fingerprint, when the code gives one.
    #[serde(default)]
    pub fingerprint: Option<String>,
    /// The architecture of the process that described the package.
    #[serde(default)]
    pub architecture: String,
    /// Every task of the workflow.
    #[serde(default)]
    pub tasks: Vec<TaskDescription>,
    /// The workflow DAG as nodes and edges, for display.
    #[serde(default)]
    pub graph_data: Option<serde_json::Value>,
    /// The triggers the workflow subscribes to, by name.
    #[serde(default)]
    pub workflow_triggers: Vec<String>,
    /// Every trigger the package declares: cron schedules (those with a
    /// `cron_expression`) and poll triggers.
    #[serde(default)]
    pub triggers: Vec<TriggerPackageMetadata>,
    /// Every reactor the package declares, with its accumulators.
    #[serde(default)]
    pub reactors: Vec<ReactorPackageMetadata>,
    /// The reactor-bound computation graph of the package.
    #[serde(default)]
    pub graph: Option<GraphPackageMetadata>,
    /// The trigger-less computation graphs of the package.
    #[serde(default)]
    pub triggerless_graphs: Vec<TriggerlessGraphMetadataEntry>,
    /// The constructor nodes the package declares. A Python package's
    /// (format 2) are also in `tasks`, as its import registers them.
    #[serde(default)]
    pub constructors: Vec<ConstructorPackageMetadata>,
    /// The providers the package bundles, with their runtime (format 2).
    #[serde(default)]
    pub providers: Vec<ProviderDescription>,
    /// What of the package would run tenant code in the process that loads
    /// it, as the compiler found it (format 2). See
    /// [`crate::registry::in_process`].
    #[serde(default)]
    pub in_process: Vec<InProcessReason>,
    /// The declared workflow params. A declared secret is a slot with
    /// `encrypted = true`.
    #[serde(default)]
    pub declared_params: Vec<cloacina_api_types::InputSlot>,
    /// The names of the declared secrets: the `encrypted` slots of
    /// `declared_params`.
    #[serde(default)]
    pub declared_secrets: Vec<String>,
    /// The declared input interfaces of graphs, reactors and accumulators.
    #[serde(default)]
    pub declared_surfaces: Vec<cloacina_api_types::DeclaredSurface>,
    /// What only a Python package has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python: Option<PythonDescription>,
}

/// One task of a package, with everything the scheduler and the fleet read
/// from a loaded task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskDescription {
    /// Position of the task in the package.
    pub index: u32,
    /// Local task id.
    pub id: String,
    /// The namespaced id with `{tenant}` not filled in.
    pub namespaced_id_template: String,
    /// Local ids of the tasks this task depends on.
    pub dependencies: Vec<String>,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Where the task is in the source.
    #[serde(default)]
    pub source_location: String,
    /// The task's trigger rules (conditional execution).
    pub trigger_rules: serde_json::Value,
    /// Whether the task takes a `TaskHandle`.
    #[serde(default)]
    pub requires_handle: bool,
    /// The task's retry policy.
    pub retry_policy: RetryPolicy,
    /// The task's own time limit in seconds. `None`: the executor's limit.
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    /// The agent pool the task runs in. `None`: the pool `default`.
    #[serde(default)]
    pub pool: Option<String>,
    /// The task's CPU request, as the author wrote it.
    #[serde(default)]
    pub cpu: Option<String>,
    /// The task's memory request, as the author wrote it.
    #[serde(default)]
    pub memory: Option<String>,
    /// The inline mark of the task.
    #[serde(default)]
    pub inline: bool,
    /// The task's own priority, 0 to 10 (CLOACINA-T-1045). `None`: the
    /// priority of its workflow.
    #[serde(default)]
    pub priority: Option<u8>,
    /// The priority of the task's workflow (CLOACINA-T-1045). `None`: the
    /// default priority.
    #[serde(default)]
    pub workflow_priority: Option<u8>,
    /// The fingerprint of the task's code, when the task gives one.
    #[serde(default)]
    pub code_fingerprint: Option<String>,
    /// "What" from the author's doc comment or docstring.
    #[serde(default)]
    pub doc_what: Option<String>,
    /// "Why" from the author's doc comment or docstring.
    #[serde(default)]
    pub doc_why: Option<String>,
}

impl TaskDescription {
    /// The task's placement as one value.
    pub fn placement(&self) -> cloacina_workflow::TaskPlacement {
        cloacina_workflow::TaskPlacement {
            pool: self.pool.clone(),
            cpu: self.cpu.clone(),
            memory: self.memory.clone(),
            inline: self.inline,
            priority: self.priority,
            workflow_priority: self.workflow_priority,
        }
    }
}

/// What only a Python package has.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PythonDescription {
    /// The module that was imported.
    #[serde(default)]
    pub entry_module: Option<String>,
    /// The `requires_python` of the manifest.
    #[serde(default)]
    pub requires_python: Option<String>,
    /// The version of the interpreter that imported the package.
    #[serde(default)]
    pub python_version: Option<String>,
}

/// The architecture name the registry stores for a library.
fn host_architecture() -> String {
    if cfg!(target_arch = "x86_64") {
        "x86_64".to_string()
    } else if cfg!(target_arch = "aarch64") {
        "aarch64".to_string()
    } else {
        std::env::consts::ARCH.to_string()
    }
}

/// The workflow DAG as nodes and edges, from the task list.
pub fn graph_data_from_tasks(tasks: &[TaskDescription]) -> serde_json::Value {
    let nodes: Vec<serde_json::Value> = tasks
        .iter()
        .map(|task| {
            serde_json::json!({
                "id": task.id,
                "label": task.id,
                "description": task.description,
                "node_type": "task"
            })
        })
        .collect();
    let edges: Vec<serde_json::Value> = tasks
        .iter()
        .flat_map(|task| {
            task.dependencies.iter().map(move |dependency| {
                serde_json::json!({
                    "source": dependency,
                    "target": task.id,
                    "edge_type": "dependency"
                })
            })
        })
        .collect();
    serde_json::json!({
        "nodes": nodes,
        "edges": edges,
        "metadata": {
            "task_count": tasks.len(),
            "generated_from": "task_dependencies"
        }
    })
}

/// The trigger rules of a packaged Rust task, from the JSON text of its FFI
/// entry. Text that does not parse reads as `Always`, as the task registrar
/// reads it.
fn parse_trigger_rules(json: &str) -> serde_json::Value {
    serde_json::from_str(json).unwrap_or_else(|_| serde_json::json!({ "type": "Always" }))
}

/// The retry policy of a packaged Rust task, from the JSON text of its FFI
/// entry. Empty text, or text that does not parse, reads as the default
/// policy, as the task registrar reads it.
fn parse_retry_policy(json: &str) -> RetryPolicy {
    if json.trim().is_empty() {
        return RetryPolicy::default();
    }
    serde_json::from_str(json).unwrap_or_default()
}

/// What the FFI calls of a Rust library returned, as the describe step
/// collected them.
pub struct RustLibraryMetadata {
    /// `get_task_metadata`.
    pub tasks: PackageTasksMetadata,
    /// The workflow slots of `get_input_interface`.
    pub declared_params: Vec<cloacina_api_types::InputSlot>,
    /// The other surfaces of `get_input_interface`.
    pub declared_surfaces: Vec<cloacina_api_types::DeclaredSurface>,
    /// `get_graph_metadata`.
    pub graph: Option<GraphPackageMetadata>,
    /// `get_reactor_metadata`.
    pub reactors: Vec<ReactorPackageMetadata>,
    /// `get_trigger_metadata`.
    pub triggers: Vec<TriggerPackageMetadata>,
    /// `get_triggerless_graph_metadata`.
    pub triggerless_graphs: Vec<TriggerlessGraphMetadataEntry>,
    /// `get_constructor_metadata`.
    pub constructors: Vec<ConstructorPackageMetadata>,
}

impl PackageDescription {
    /// The document of a Rust package, from what its library answered.
    pub fn from_rust_library(meta: RustLibraryMetadata) -> Self {
        let tasks: Vec<TaskDescription> = meta
            .tasks
            .tasks
            .into_iter()
            .map(|t| TaskDescription {
                index: t.index,
                trigger_rules: parse_trigger_rules(&t.trigger_rules),
                retry_policy: parse_retry_policy(&t.retry_policy),
                id: t.id,
                namespaced_id_template: t.namespaced_id_template,
                dependencies: t.dependencies,
                description: t.description,
                source_location: t.source_location,
                requires_handle: t.requires_handle,
                timeout_seconds: t.timeout_seconds,
                pool: t.pool,
                cpu: t.cpu,
                memory: t.memory,
                inline: t.inline,
                priority: t.priority,
                workflow_priority: t.workflow_priority,
                code_fingerprint: None,
                doc_what: None,
                doc_why: None,
            })
            .collect();

        // The library's own graph JSON when it parses, else one made from
        // the task list: what `PackageLoader` stores today.
        let from_library = meta
            .tasks
            .graph_data_json
            .as_deref()
            .filter(|json| !json.trim().is_empty())
            .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok());
        let graph_data = match from_library {
            Some(value) => Some(value),
            None if !tasks.is_empty() => Some(graph_data_from_tasks(&tasks)),
            None => None,
        };

        let mut doc = Self {
            format_version: PACKAGE_DESCRIPTION_VERSION,
            language: "rust".to_string(),
            described_by: env!("CARGO_PKG_VERSION").to_string(),
            package_name: meta.tasks.package_name,
            workflow_name: meta.tasks.workflow_name,
            description: meta.tasks.package_description,
            author: meta.tasks.package_author,
            fingerprint: meta.tasks.workflow_fingerprint,
            architecture: host_architecture(),
            tasks,
            graph_data,
            workflow_triggers: meta.tasks.triggers,
            triggers: meta.triggers,
            reactors: meta.reactors,
            graph: meta.graph,
            triggerless_graphs: meta.triggerless_graphs,
            constructors: meta.constructors,
            providers: Vec::new(),
            in_process: Vec::new(),
            declared_params: meta.declared_params,
            declared_secrets: Vec::new(),
            declared_surfaces: meta.declared_surfaces,
            python: None,
        };
        doc.refresh_declared_secrets();
        doc
    }

    /// The document of a Python package, read from the `Runtime` its import
    /// registered into.
    ///
    /// `runtime` must hold nothing but this package: the caller imports into
    /// an empty one. `task_namespaces` are the tasks the import registered, in
    /// registration order. `workflow_name` is empty for a graph package.
    pub fn from_python_runtime(
        runtime: &Runtime,
        package_name: &str,
        workflow_name: &str,
        task_namespaces: &[TaskNamespace],
        manifest: &cloacina_workflow_plugin::CloacinaMetadata,
        python_version: Option<String>,
    ) -> Self {
        // The import reports its tasks in no fixed order; the document lists
        // them by id.
        let mut ordered: Vec<&TaskNamespace> = task_namespaces.iter().collect();
        ordered.sort_by(|a, b| a.task_id.cmp(&b.task_id));
        let tasks: Vec<TaskDescription> = ordered
            .into_iter()
            .filter_map(|ns| {
                let task = runtime.get_task(ns)?;
                let placement = task.placement();
                Some(TaskDescription {
                    index: 0,
                    id: ns.task_id.clone(),
                    namespaced_id_template: format!(
                        "{{tenant}}::{}::{}::{}",
                        package_name, workflow_name, ns.task_id
                    ),
                    dependencies: task
                        .dependencies()
                        .iter()
                        .map(|dep| dep.task_id.clone())
                        .collect(),
                    description: String::new(),
                    source_location: String::new(),
                    trigger_rules: task.trigger_rules(),
                    requires_handle: task.requires_handle(),
                    retry_policy: task.retry_policy(),
                    timeout_seconds: task.timeout().map(|d| d.as_secs()),
                    pool: placement.pool,
                    cpu: placement.cpu,
                    memory: placement.memory,
                    inline: placement.inline,
                    priority: placement.priority,
                    workflow_priority: placement.workflow_priority,
                    code_fingerprint: task.code_fingerprint(),
                    doc_what: None,
                    doc_why: None,
                })
            })
            .enumerate()
            .map(|(index, mut task)| {
                task.index = index as u32;
                task
            })
            .collect();

        let none = std::collections::HashSet::new();
        let mut view = python_load_view(
            runtime,
            package_name,
            &none,
            &none,
            &none,
            &manifest.accumulators,
        );
        // The runtime lists names in no fixed order; the document has one.
        view.triggers.sort_by(|a, b| a.name.cmp(&b.name));
        view.reactors.sort_by(|a, b| a.name.cmp(&b.name));

        let mut triggerless_graphs: Vec<TriggerlessGraphMetadataEntry> = runtime
            .triggerless_graph_names()
            .into_iter()
            .filter_map(|name| {
                let reg = runtime.get_triggerless_graph(&name)?;
                Some(TriggerlessGraphMetadataEntry {
                    name,
                    package_name: package_name.to_string(),
                    terminal_node_names: reg.terminal_node_names.clone(),
                })
            })
            .collect();
        triggerless_graphs.sort_by(|a, b| a.name.cmp(&b.name));

        let graph_data = if tasks.is_empty() {
            None
        } else {
            Some(graph_data_from_tasks(&tasks))
        };

        let mut doc = Self {
            format_version: PACKAGE_DESCRIPTION_VERSION,
            language: "python".to_string(),
            described_by: env!("CARGO_PKG_VERSION").to_string(),
            package_name: package_name.to_string(),
            workflow_name: workflow_name.to_string(),
            description: manifest.description.clone(),
            author: manifest.author.clone(),
            fingerprint: None,
            architecture: host_architecture(),
            tasks,
            graph_data,
            // A Python workflow names no triggers: each trigger names the
            // workflow it fires (`workflow_name` of the trigger).
            workflow_triggers: Vec::new(),
            triggers: view.triggers,
            reactors: view.reactors,
            graph: view.graph,
            triggerless_graphs,
            constructors: Vec::new(),
            providers: Vec::new(),
            in_process: Vec::new(),
            declared_params: Vec::new(),
            declared_secrets: Vec::new(),
            declared_surfaces: Vec::new(),
            python: Some(PythonDescription {
                entry_module: manifest.entry_module.clone(),
                requires_python: manifest.requires_python.clone(),
                python_version,
            }),
        };
        doc.refresh_declared_secrets();
        doc
    }

    /// Add what the compiler parses from the package source, which the code
    /// does not report: the docs of each task, and for a Python package the
    /// declared params, secrets and surfaces (a Rust library reports its own
    /// through the input-interface call).
    pub fn overlay_source_parse(
        &mut self,
        task_docs: &std::collections::HashMap<String, TaskDocs>,
        declared_params: &[cloacina_api_types::InputSlot],
        declared_surfaces: &[cloacina_api_types::DeclaredSurface],
    ) {
        for task in &mut self.tasks {
            if let Some(docs) = task_docs.get(&task.id) {
                task.doc_what = docs.what.clone();
                task.doc_why = docs.why.clone();
            }
        }
        if self.language == "python" {
            self.declared_params = declared_params.to_vec();
            self.declared_surfaces = declared_surfaces.to_vec();
        }
        self.refresh_declared_secrets();
    }

    fn refresh_declared_secrets(&mut self) {
        self.declared_secrets = self
            .declared_params
            .iter()
            .filter(|slot| slot.encrypted)
            .map(|slot| slot.name.clone())
            .collect();
    }

    /// Write the fields the registry row's `metadata` JSON has always held
    /// (workflow name, task list, graph data, subscriptions, declared params
    /// and surfaces) from this document into `base`. The identity fields of
    /// `base` (package name, version, description, author) are kept.
    ///
    /// This is what the compiler used to read by opening the library in its
    /// own process.
    pub fn merge_into_legacy(&self, base: &mut PackageMetadata) {
        base.workflow_name = self.workflow_name.clone();
        base.tasks = self
            .tasks
            .iter()
            .map(|t| TaskMetadata {
                index: t.index,
                local_id: t.id.clone(),
                namespaced_id_template: t.namespaced_id_template.clone(),
                dependencies: t.dependencies.clone(),
                description: t.description.clone(),
                source_location: t.source_location.clone(),
                doc_what: t.doc_what.clone(),
                doc_why: t.doc_why.clone(),
            })
            .collect();
        base.graph_data = self.graph_data.clone();
        base.architecture = self.architecture.clone();
        base.symbols = vec!["fidius_get_registry".to_string()];
        base.workflow_triggers = self.workflow_triggers.clone();
        base.declared_params = self.declared_params.clone();
        base.declared_surfaces = self.declared_surfaces.clone();
    }
}

/// The triggers, reactors and graph a Python import registered, in the wire
/// types a Rust library reports them in.
pub struct PythonLoadView {
    /// Every trigger the import registered.
    pub triggers: Vec<TriggerPackageMetadata>,
    /// Every reactor the import registered.
    pub reactors: Vec<ReactorPackageMetadata>,
    /// The computation graph the import registered.
    pub graph: Option<GraphPackageMetadata>,
}

/// Read what a Python import registered into `runtime`.
///
/// The three `pre_*` sets are the names the runtime held before the import:
/// what is not in them is this package's. `accumulator_overrides` are the
/// `[[metadata.accumulators]]` entries of the package manifest.
///
/// The reconciler calls this after it imported a package into the server's
/// runtime; the compiler's describe child calls it after it imported the
/// package into an empty one. One function, so both read the same.
pub fn python_load_view(
    runtime: &Runtime,
    package_name: &str,
    pre_reactor_names: &std::collections::HashSet<String>,
    pre_trigger_names: &std::collections::HashSet<String>,
    pre_graph_names: &std::collections::HashSet<String>,
    accumulator_overrides: &[AccumulatorConfig],
) -> PythonLoadView {
    // Reactors: diff vs. pre-snapshot, then walk each runtime
    // ReactorRegistration into wire-format with manifest accumulator
    // overrides folded in.
    let mut reactors: Vec<ReactorPackageMetadata> = Vec::new();
    for name in runtime.reactor_names() {
        if pre_reactor_names.contains(&name) {
            continue;
        }
        let Some(reg) = runtime.get_reactor(&name) else {
            continue;
        };
        let accumulators: Vec<AccumulatorDeclarationEntry> = reg
            .accumulator_names
            .iter()
            .map(|acc_name| {
                // CLOACI-T-0839 precedence: manifest override (deployment
                // wins) → authored spec carried on the registration (e.g.
                // Python's `@cloaca.state_accumulator`) → passthrough.
                // Before the authored-spec fallback existed, packaged
                // Python state/stream accumulators silently degraded to
                // passthrough here.
                let (accumulator_type, config) = accumulator_overrides
                    .iter()
                    .find(|cfg| &cfg.name == acc_name)
                    .map(|cfg| (cfg.accumulator_type.clone(), cfg.config.clone()))
                    .or_else(|| {
                        reg.accumulator_specs
                            .iter()
                            .find(|spec| &spec.name == acc_name)
                            .map(|spec| (spec.accumulator_type.clone(), spec.config.clone()))
                    })
                    .unwrap_or_else(|| ("passthrough".to_string(), Default::default()));
                AccumulatorDeclarationEntry {
                    name: acc_name.clone(),
                    accumulator_type,
                    config,
                }
            })
            .collect();
        let reaction_mode = match reg.reaction_mode {
            cloacina_computation_graph::ReactionMode::WhenAll => "when_all".to_string(),
            _ => "when_any".to_string(),
        };
        reactors.push(ReactorPackageMetadata {
            name: reg.name,
            package_name: package_name.to_string(),
            reaction_mode,
            accumulators,
        });
    }

    // Triggers: walk each runtime trigger and emit wire-format with
    // cron_expression carried through.
    let mut triggers: Vec<TriggerPackageMetadata> = Vec::new();
    for name in runtime.trigger_names() {
        if pre_trigger_names.contains(&name) {
            continue;
        }
        let Some(impl_) = runtime.get_trigger(&name) else {
            continue;
        };
        triggers.push(TriggerPackageMetadata {
            name: impl_.name().to_string(),
            workflow_name: impl_.workflow_name().to_string(),
            package_name: package_name.to_string(),
            poll_interval: format!("{}s", impl_.poll_interval().as_secs()),
            cron_expression: impl_.cron_expression(),
            allow_concurrent: impl_.allow_concurrent(),
            timezone: impl_.cron_timezone(),
            catchup_policy: impl_.cron_catchup_policy(),
            max_active_runs: impl_.cron_max_active_runs(),
        });
    }

    // Computation graph: Python packages declare at most one CG per
    // package today. Pull the first non-pre-snapshot graph name and
    // shape it into wire-format. trigger_reactor / accumulators come
    // from the runtime registration directly.
    let graph: Option<GraphPackageMetadata> = runtime
        .computation_graph_names()
        .into_iter()
        .find(|n| !pre_graph_names.contains(n))
        .and_then(|graph_name| {
            let reg = runtime.get_computation_graph(&graph_name)?;
            // CLOACINA-T-1078: the authored kinds of the graph's reactor, so
            // the document names a polling or stream accumulator of a Python
            // graph as the reactor entry does.
            let authored: Vec<cloacina_computation_graph::AccumulatorSpec> = reg
                .trigger_reactor
                .as_deref()
                .and_then(|name| runtime.get_reactor(name))
                .map(|reactor| reactor.accumulator_specs)
                .unwrap_or_default();
            let accumulators: Vec<AccumulatorDeclarationEntry> = reg
                .accumulator_names
                .iter()
                .map(|acc_name| {
                    // Precedence as for reactors: manifest entry, then the
                    // authored kind, then passthrough.
                    let (accumulator_type, config) = accumulator_overrides
                        .iter()
                        .find(|cfg| &cfg.name == acc_name)
                        .map(|cfg| (cfg.accumulator_type.clone(), cfg.config.clone()))
                        .or_else(|| {
                            authored
                                .iter()
                                .find(|spec| &spec.name == acc_name)
                                .map(|spec| (spec.accumulator_type.clone(), spec.config.clone()))
                        })
                        .unwrap_or_else(|| ("passthrough".to_string(), Default::default()));
                    AccumulatorDeclarationEntry {
                        name: acc_name.clone(),
                        accumulator_type,
                        config,
                    }
                })
                .collect();
            Some(GraphPackageMetadata {
                graph_name,
                package_name: package_name.to_string(),
                reaction_mode: reg.reaction_mode.clone(),
                // Wire-format default only (CLOACI-T-0918): the Python
                // CG's REAL strategy travels on the declaration built by
                // `load_cg_package`, not this view; and the Rust path's
                // manifest override happens in
                // `step_load_reactor_bound_cgs`.
                input_strategy: "latest".to_string(),
                accumulators,
                trigger_reactor: reg.trigger_reactor.clone(),
                // Python CG topology threading is a follow-up — the runtime
                // registration carries no node/edge graph yet. (CLOACI-T-0673)
                graph_data_json: None,
            })
        });

    PythonLoadView {
        triggers,
        reactors,
        graph,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina_workflow_plugin::types::TaskMetadataEntry;

    fn entry(id: &str, deps: &[&str]) -> TaskMetadataEntry {
        TaskMetadataEntry {
            index: 0,
            id: id.to_string(),
            namespaced_id_template: format!("{{tenant}}::pkg::wf::{id}"),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            description: format!("{id} task"),
            source_location: "src/lib.rs".to_string(),
            trigger_rules: "{\"type\":\"Always\"}".to_string(),
            requires_handle: false,
            retry_policy: String::new(),
            timeout_seconds: None,
            pool: None,
            cpu: None,
            memory: None,
            inline: false,
            priority: None,
            workflow_priority: None,
        }
    }

    fn rust_meta(tasks: Vec<TaskMetadataEntry>) -> RustLibraryMetadata {
        RustLibraryMetadata {
            tasks: PackageTasksMetadata {
                workflow_name: "wf".to_string(),
                package_name: "pkg".to_string(),
                package_description: Some("a package".to_string()),
                package_author: None,
                workflow_fingerprint: Some("fp".to_string()),
                graph_data_json: None,
                tasks,
                triggers: vec!["nightly".to_string()],
            },
            declared_params: vec![
                cloacina_api_types::InputSlot::required("region", serde_json::json!({})),
                cloacina_api_types::InputSlot::secret("warehouse"),
            ],
            declared_surfaces: Vec::new(),
            graph: None,
            reactors: Vec::new(),
            triggers: Vec::new(),
            triggerless_graphs: Vec::new(),
            constructors: Vec::new(),
        }
    }

    /// The task fields the registrar reads from the FFI entry are in the
    /// document, parsed the way the registrar parses them.
    #[test]
    fn rust_document_holds_each_task_field() {
        let mut first = entry("extract", &[]);
        first.retry_policy = "{not json".to_string();
        let mut second = entry("load", &["extract"]);
        second.index = 1;
        second.trigger_rules = "{\"type\":\"All\",\"conditions\":[]}".to_string();
        second.requires_handle = true;
        second.retry_policy =
            serde_json::to_string(&RetryPolicy::builder().max_attempts(0).build()).unwrap();
        second.timeout_seconds = Some(600);
        second.pool = Some("large".to_string());
        second.cpu = Some("4".to_string());
        second.memory = Some("32Gi".to_string());
        second.inline = true;

        let doc = PackageDescription::from_rust_library(rust_meta(vec![first, second]));

        assert_eq!(doc.format_version, PACKAGE_DESCRIPTION_VERSION);
        assert_eq!(doc.language, "rust");
        assert_eq!(doc.workflow_name, "wf");
        assert_eq!(doc.fingerprint.as_deref(), Some("fp"));
        assert_eq!(doc.workflow_triggers, vec!["nightly"]);
        assert_eq!(doc.tasks.len(), 2);

        let extract = &doc.tasks[0];
        assert_eq!(extract.trigger_rules, serde_json::json!({"type": "Always"}));
        assert_eq!(extract.retry_policy, RetryPolicy::default());
        assert!(extract.placement().is_default());

        let load = &doc.tasks[1];
        assert_eq!(load.dependencies, vec!["extract"]);
        assert_eq!(load.trigger_rules["type"], "All");
        assert!(load.requires_handle);
        assert_eq!(load.retry_policy.max_attempts, 0);
        assert_eq!(load.timeout_seconds, Some(600));
        assert_eq!(load.placement().pool_name(), "large");
        assert_eq!(load.placement().cpu_millis(), Some(4000));
        assert_eq!(load.placement().memory_bytes(), Some(32 << 30));
        assert!(load.inline);

        // The graph is made from the tasks when the library gives none.
        let graph = doc.graph_data.expect("graph data");
        assert_eq!(graph["metadata"]["task_count"], 2);
        assert_eq!(graph["edges"][0]["source"], "extract");
        assert_eq!(graph["edges"][0]["target"], "load");
    }

    /// A declared secret is named in `declared_secrets`.
    #[test]
    fn declared_secrets_are_the_encrypted_slots() {
        let doc = PackageDescription::from_rust_library(rust_meta(vec![entry("a", &[])]));
        assert_eq!(doc.declared_secrets, vec!["warehouse"]);
        assert_eq!(doc.declared_params.len(), 2);
    }

    /// The source parse adds docs for both languages, and params only for
    /// Python.
    #[test]
    fn overlay_adds_docs_and_keeps_a_rust_librarys_own_params() {
        let mut doc = PackageDescription::from_rust_library(rust_meta(vec![entry("a", &[])]));
        let docs = std::collections::HashMap::from([(
            "a".to_string(),
            TaskDocs {
                what: Some("does a".to_string()),
                why: Some("because".to_string()),
            },
        )]);
        let parsed = vec![cloacina_api_types::InputSlot::secret("other")];
        doc.overlay_source_parse(&docs, &parsed, &[]);
        assert_eq!(doc.tasks[0].doc_what.as_deref(), Some("does a"));
        assert_eq!(doc.tasks[0].doc_why.as_deref(), Some("because"));
        assert_eq!(doc.declared_secrets, vec!["warehouse"]);

        doc.language = "python".to_string();
        doc.overlay_source_parse(&docs, &parsed, &[]);
        assert_eq!(doc.declared_secrets, vec!["other"]);
    }

    /// The document reads back from its JSON, and a document with fields this
    /// build does not know still reads.
    #[test]
    fn document_round_trips_and_tolerates_unknown_fields() {
        let doc = PackageDescription::from_rust_library(rust_meta(vec![entry("a", &[])]));
        let mut value = serde_json::to_value(&doc).unwrap();
        let again: PackageDescription = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(&again).unwrap(), value);

        value["a_field_of_a_later_version"] = serde_json::json!(true);
        value["format_version"] = serde_json::json!(PACKAGE_DESCRIPTION_VERSION + 1);
        let later: PackageDescription = serde_json::from_value(value).unwrap();
        assert_eq!(later.format_version, PACKAGE_DESCRIPTION_VERSION + 1);
    }

    /// The old `metadata` JSON gets the fields it always held.
    #[test]
    fn legacy_metadata_is_filled_from_the_document() {
        let mut doc = PackageDescription::from_rust_library(rust_meta(vec![entry("a", &[]), {
            let mut b = entry("b", &["a"]);
            b.index = 1;
            b
        }]));
        let docs = std::collections::HashMap::from([(
            "b".to_string(),
            TaskDocs {
                what: Some("does b".to_string()),
                why: None,
            },
        )]);
        doc.overlay_source_parse(&docs, &[], &[]);

        let mut base = PackageMetadata {
            package_name: "from-manifest".to_string(),
            workflow_name: String::new(),
            version: "2.0.0".to_string(),
            description: Some("manifest text".to_string()),
            author: None,
            tasks: Vec::new(),
            graph_data: None,
            architecture: String::new(),
            symbols: Vec::new(),
            workflow_triggers: Vec::new(),
            declared_params: Vec::new(),
            declared_surfaces: Vec::new(),
            task_docs: Default::default(),
            python_platforms: None,
        };
        doc.merge_into_legacy(&mut base);

        assert_eq!(base.package_name, "from-manifest");
        assert_eq!(base.version, "2.0.0");
        assert_eq!(base.description.as_deref(), Some("manifest text"));
        assert_eq!(base.workflow_name, "wf");
        assert_eq!(base.tasks.len(), 2);
        assert_eq!(base.tasks[1].local_id, "b");
        assert_eq!(base.tasks[1].dependencies, vec!["a"]);
        assert_eq!(base.tasks[1].doc_what.as_deref(), Some("does b"));
        assert_eq!(base.workflow_triggers, vec!["nightly"]);
        assert_eq!(base.declared_params.len(), 2);
        assert!(base.graph_data.is_some());
    }
}
