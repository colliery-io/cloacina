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

use cloacina_computation_graph::{ComputationGraphRegistration, ReactorRegistration};
use cloacina_workflow::{Task, TaskNamespace};
use std::sync::Arc;

/// Reactor entry emitted by the `#[reactor]` attribute macro. The `package!()`
/// shell walks `inventory::iter::<ReactorEntry>` at FFI call time to produce
/// `Vec<ReactorPackageMetadata>` for `get_reactor_metadata`.
pub struct ReactorEntry {
    pub name: &'static str,
    pub constructor: fn() -> ReactorRegistration,
}
inventory::collect!(ReactorEntry);

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
}
inventory::collect!(ComputationGraphEntry);
