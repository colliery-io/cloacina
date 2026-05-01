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
//! Currently only `ReactorEntry` is hosted here (T-A scope). Other entry
//! types (`TaskEntry`, `TriggerEntry`, `ComputationGraphEntry`,
//! `TriggerlessGraphEntry`, `WorkflowEntry`, `StreamBackendEntry`) remain
//! in `cloacina/src/inventory_entries.rs` until their constructor return
//! types are likewise relocated.

use cloacina_computation_graph::ReactorRegistration;

/// Reactor entry emitted by the `#[reactor]` attribute macro. The `package!()`
/// shell walks `inventory::iter::<ReactorEntry>` at FFI call time to produce
/// `Vec<ReactorPackageMetadata>` for `get_reactor_metadata`.
pub struct ReactorEntry {
    pub name: &'static str,
    pub constructor: fn() -> ReactorRegistration,
}
inventory::collect!(ReactorEntry);
