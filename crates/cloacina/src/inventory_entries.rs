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

//! Inventory entry types for linker-collected registry seeding.
//!
//! The macros (`#[task]`, `#[workflow]`, `#[trigger]`, `#[computation_graph]`,
//! and the stream-backend registration helper) emit
//! `inventory::submit!` statements of these types instead of `#[ctor]`
//! constructors. The runtime reads them post-`main()` via `inventory::iter`,
//! eliminating the initialization-ordering bug that sank I-0095.
//!
//! Function pointers — not `Box<dyn Fn>` — are used because `inventory` stores
//! entries in a linker section with `'static` + `Sized` bounds. Zero-capture
//! closures at the macro call site coerce to `fn` pointers automatically, so
//! the ergonomics stay identical.
//!
//! Nothing in this file reads inventory yet. That wiring lands in T-0506
//! together with the removal of the global static registries.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::computation_graph::stream_backend::{StreamBackend, StreamConfig, StreamError};
use crate::task::{Task, TaskNamespace};
use crate::trigger::Trigger;
use crate::workflow::Workflow;
use cloacina_computation_graph::ComputationGraphRegistration;

/// Task entry emitted by `#[task]`.
pub struct TaskEntry {
    /// Deferred construction of the task namespace (cannot be const because
    /// `TaskNamespace` contains `String`).
    pub namespace: fn() -> TaskNamespace,
    /// Task constructor — instantiates a fresh task object on each call.
    pub constructor: fn() -> Arc<dyn Task>,
}
inventory::collect!(TaskEntry);

/// Workflow entry emitted by `#[workflow]`.
pub struct WorkflowEntry {
    pub name: &'static str,
    pub constructor: fn() -> Workflow,
}
inventory::collect!(WorkflowEntry);

/// Trigger entry emitted by `#[trigger]`.
pub struct TriggerEntry {
    pub name: &'static str,
    pub constructor: fn() -> Arc<dyn Trigger>,
}
inventory::collect!(TriggerEntry);

/// Computation graph entry emitted by `#[computation_graph]`.
pub struct ComputationGraphEntry {
    pub name: &'static str,
    pub constructor: fn() -> ComputationGraphRegistration,
}
inventory::collect!(ComputationGraphEntry);

/// Stream-backend entry emitted by the stream-backend registration helper.
///
/// The factory is a function pointer that takes an owned `StreamConfig` and
/// returns a heap-allocated future; at seed time the runtime wraps the
/// pointer into a `Box<dyn Fn(StreamConfig) -> Pin<Box<Future<..>>> + Send + Sync>`
/// to match the shape of dynamically-registered backends.
pub type StreamBackendFactoryFn =
    fn(
        StreamConfig,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn StreamBackend>, StreamError>> + Send>>;

pub struct StreamBackendEntry {
    pub type_name: &'static str,
    pub factory: StreamBackendFactoryFn,
}
inventory::collect!(StreamBackendEntry);
