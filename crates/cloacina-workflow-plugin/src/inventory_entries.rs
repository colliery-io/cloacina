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

//! Inventory entry types reachable from packaged cdylibs.
//!
//! These types live in `cloacina-workflow-plugin` (a leaf crate that
//! packaged cdylibs depend on) so the unified `cloacina::package!()` shell
//! macro can walk `inventory::iter::<T>` at FFI call time without dragging
//! in the full `cloacina` engine.
//!
//! `cloacina` (the host engine) re-exports these types from
//! `cloacina_workflow_plugin::inventory` so existing engine code paths
//! continue to compile unchanged.
//!
//! T-C (CLOACI-T-0549) relocated `TaskEntry` and `ComputationGraphEntry`
//! here so the shell's `get_task_metadata` / `execute_task` /
//! `get_graph_metadata` / `execute_graph` bodies can walk these inventories
//! from packaged cdylibs. `TriggerEntry` / `TriggerlessGraphEntry` /
//! `WorkflowEntry` / `StreamBackendEntry` remain in
//! `cloacina/src/inventory_entries.rs` until their constructor return
//! types likewise relocate.

use cloacina_computation_graph::{ComputationGraphRegistration, GraphResult, ReactorRegistration};
use cloacina_workflow::{Context, Task, TaskNamespace, Trigger};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// ----------------------------------------------------------------------------
// Trigger-less computation graph types (T-0552 — relocated from
// cloacina/src/computation_graph/triggerless.rs so packaged cdylibs can
// reach them).
// ----------------------------------------------------------------------------

/// The compiled function emitted for a trigger-less computation graph.
///
/// Takes the workflow context the graph was invoked with; returns a
/// `GraphResult` whose terminal outputs are pre-serialized to
/// `serde_json::Value`.
pub type TriggerlessGraphFn = Arc<
    dyn Fn(Context<::serde_json::Value>) -> Pin<Box<dyn Future<Output = GraphResult> + Send>>
        + Send
        + Sync,
>;

/// Runtime-side description of a trigger-less computation graph.
pub struct TriggerlessGraphRegistration {
    /// Graph name (the macro's `mod` name).
    pub name: String,
    /// Compiled graph function.
    pub graph_fn: TriggerlessGraphFn,
    /// Names of every terminal node in declaration order. Workflow-task
    /// invocation writes each terminal output into the post-invocation
    /// context under the matching name.
    pub terminal_node_names: Vec<String>,
}

/// Compile-time link from a `Graph` handle to its trigger-less invocation
/// surface.
///
/// The `#[computation_graph]` macro emits an `impl TriggerlessGraph for
/// __CGHandle_<mod>` only when the graph is trigger-less.
pub trait TriggerlessGraph: cloacina_computation_graph::Graph {
    /// Construct a fresh `TriggerlessGraphFn` wrapping the compiled fn.
    fn compiled_fn() -> TriggerlessGraphFn;
    /// Names of every terminal node in declaration order.
    fn terminal_node_names() -> &'static [&'static str];
}

/// Reactor entry emitted by the `#[reactor]` attribute macro. The `package!()`
/// shell walks `inventory::iter::<ReactorEntry>` at FFI call time to produce
/// `Vec<ReactorPackageMetadata>` for `get_reactor_metadata`.
pub struct ReactorEntry {
    pub name: &'static str,
    pub constructor: fn() -> ReactorRegistration,
}
inventory::collect!(ReactorEntry);

/// Constructor-node entry emitted by `constructor!(...)` inside a `#[workflow]`,
/// in PACKAGED mode (CLOACI-T-0832). A packaged cdylib cannot link the WASM
/// constructor loader, so instead of building the node it DECLARES it: the
/// `package!()` shell walks `inventory::iter::<ConstructorEntry>` to project each
/// into a [`crate::ConstructorPackageMetadata`] returned by
/// `get_constructor_metadata()`, and the server (which links wasmtime) resolves
/// each via `load_constructor_node(.., GrantSpec::from_pairs(grants))` and injects
/// the resulting task node into the rebuilt workflow DAG.
///
/// `config` / `grants` / `dependencies` are deferred (`fn() -> ..`) because they
/// carry owned `String`/`Value` data that can't be `const`. `grants` is the raw
/// `(kind, patterns)` shape the cross-surface lowering already produces.
pub struct ConstructorEntry {
    /// The workflow this constructor node belongs to.
    pub workflow: &'static str,
    /// The DAG node id (what dependents reference).
    pub id: &'static str,
    /// Provider package reference, `"name[@version]"` (the `from = ` field).
    pub from: &'static str,
    /// The constructor's `constructor.json` name inside the provider.
    pub constructor: &'static str,
    /// Author config as `(name, value)` pairs in written order; bound by name.
    pub config: fn() -> Vec<(String, serde_json::Value)>,
    /// Tenant capability grants as raw `(kind, patterns)` pairs (CLOACI-T-0834).
    pub grants: fn() -> Vec<(String, Vec<String>)>,
    /// Upstream DAG node ids this constructor depends on.
    pub dependencies: fn() -> Vec<String>,
}
inventory::collect!(ConstructorEntry);

/// Task entry emitted by `#[task]`. The `package!()` shell walks
/// `inventory::iter::<TaskEntry>` to build the per-task metadata in
/// `get_task_metadata` and to dispatch task execution by name in
/// `execute_task`.
pub struct TaskEntry {
    /// Deferred construction of the task namespace (cannot be const because
    /// `TaskNamespace` contains `String`).
    pub namespace: fn() -> TaskNamespace,
    /// Task constructor — instantiates a fresh task object on each call.
    pub constructor: fn() -> Arc<dyn Task>,
}
inventory::collect!(TaskEntry);

/// Workflow descriptor entry emitted by `#[workflow]`. Provides the
/// metadata fields the unified `cloacina::package!()` shell can't infer
/// from `TaskEntry` alone (description, author, fingerprint, graph_data,
/// triggers). Walked by the shell's `get_task_metadata` body to populate
/// `PackageTasksMetadata`. (T-C / I-0102)
pub struct WorkflowDescriptorEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub author: &'static str,
    pub fingerprint: &'static str,
    pub graph_data_json: &'static str,
    /// Trigger names this workflow subscribes to (from
    /// `#[workflow(triggers = ["..."])]`).
    pub triggers: fn() -> ::std::vec::Vec<::std::string::String>,
    /// CLOACI-I-0128: declared input params as a JSON array of
    /// `cloacina_api_types::InputSlot` (serialized). Generated at runtime by the
    /// `#[workflow(params(...))]` macro via `cloacina_workflow::schema_for`.
    /// Returns `"[]"` for workflows that declare no params.
    pub params: fn() -> ::std::string::String,
}
inventory::collect!(WorkflowDescriptorEntry);

/// Computation graph entry emitted by `#[computation_graph]` for the
/// reactor-triggered (split) form. The `package!()` shell walks
/// `inventory::iter::<ComputationGraphEntry>` to build the metadata
/// returned by `get_graph_metadata` and to dispatch
/// execution in `execute_graph`. At most one entry is expected per cdylib;
/// the shell's body errors if it finds more than one.
pub struct ComputationGraphEntry {
    pub name: &'static str,
    pub constructor: fn() -> ComputationGraphRegistration,
    /// Serialized node/edge topology JSON for this graph, emitted by the
    /// `#[computation_graph]` macro so `get_graph_metadata` can surface the DAG
    /// to the API/UI. Empty string when not emitted. (CLOACI-T-0673)
    pub graph_data_json: &'static str,
    /// CLOACI-I-0128 (T-0758): JSON array of `InputSlot` — one per cache source
    /// (accumulator name → boundary type), derived from the graph's node fn
    /// signatures. Boundary types that derive `schemars::JsonSchema` get a rich
    /// schema; others degrade to a permissive `{}` (opt-in typing via the
    /// `SchemaProbe` autoref specialization). Returns `"[]"` for graphs with no
    /// cache sources (e.g. trigger-less graphs). The `package!` shell joins this
    /// source→schema map with reactor declarations to answer
    /// `get_input_interface`.
    pub input_interface: fn() -> ::std::string::String,
}
inventory::collect!(ComputationGraphEntry);

/// Trigger entry emitted by `#[trigger]`. The `package!()` shell walks
/// `inventory::iter::<TriggerEntry>` to populate `get_trigger_metadata`,
/// calling each entry's constructor and querying the resulting trigger's
/// `name` / `poll_interval` / `cron_expression` / `allow_concurrent`.
/// (T-0552 — relocated from `cloacina` so packaged cdylibs can reach it.)
pub struct TriggerEntry {
    pub name: &'static str,
    pub constructor: fn() -> Arc<dyn Trigger>,
}
inventory::collect!(TriggerEntry);

/// Trigger-less computation graph entry emitted by `#[computation_graph]`
/// for graphs declared without a `trigger = reactor(...)` clause. These
/// graphs operate on `Context<Value>` rather than `InputCache` and are
/// invoked directly by workflow tasks. (T-0552 — relocated from `cloacina`.)
pub struct TriggerlessGraphEntry {
    pub name: &'static str,
    pub constructor: fn() -> TriggerlessGraphRegistration,
}
inventory::collect!(TriggerlessGraphEntry);
