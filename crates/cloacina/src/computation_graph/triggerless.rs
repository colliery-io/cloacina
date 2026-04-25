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

//! Trigger-less computation graph registration.
//!
//! Trigger-less graphs (declared with `#[computation_graph(graph = { ... })]`
//! and no `trigger = reactor(...)` clause) operate on a `Context<Value>`
//! instead of an `InputCache`. They are invoked directly by workflow tasks
//! (T-02) and Python decorators (T-03), not by reactors.
//!
//! These types live in `cloacina` rather than `cloacina-computation-graph`
//! because the compiled function references `cloacina_workflow::Context`,
//! which the leaf cg crate doesn't depend on.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use cloacina_computation_graph::GraphResult;
use cloacina_workflow::Context;
use serde_json::Value;

/// The compiled function emitted for a trigger-less computation graph.
///
/// Takes the workflow context the graph was invoked with; returns a
/// `GraphResult` carrying the terminal node outputs.
pub type TriggerlessGraphFn =
    Arc<dyn Fn(Context<Value>) -> Pin<Box<dyn Future<Output = GraphResult> + Send>> + Send + Sync>;

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
