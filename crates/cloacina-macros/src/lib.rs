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

#![allow(unexpected_cfgs)]
//! # Cloacina Macros
//!
//! Procedural macros for defining tasks and workflows in the Cloacina framework.
//!
//! ## Key Features
//!
//! - `#[task]` — define tasks with retry policies and trigger rules
//! - `#[workflow]` — define workflows as modules containing `#[task]` functions
//! - Compile-time validation of task dependencies and workflow structure
//! - Automatic task and workflow registration
//! - Code fingerprinting for task versioning
//!
//! ## Example
//!
//! ```rust,ignore
//! use cloacina::{task, workflow, Context, TaskError};
//!
//! #[workflow(name = "my_pipeline", description = "Process data")]
//! pub mod my_pipeline {
//!     use super::*;
//!
//!     #[task(id = "fetch", dependencies = [])]
//!     pub async fn fetch(ctx: &mut Context<Value>) -> Result<(), TaskError> { Ok(()) }
//!
//!     #[task(id = "process", dependencies = ["fetch"])]
//!     pub async fn process(ctx: &mut Context<Value>) -> Result<(), TaskError> { Ok(()) }
//! }
//! ```

pub(crate) mod computation_graph;
pub(crate) mod packaged_workflow;
mod registry;
pub(crate) mod tasks;
mod trigger_attr;
mod workflow_attr;

use proc_macro::TokenStream;

/// Define a task with retry policies and trigger rules.
#[proc_macro_attribute]
pub fn task(args: TokenStream, input: TokenStream) -> TokenStream {
    tasks::task(args, input)
}

/// Define a workflow as a module containing `#[task]` functions.
///
/// Applied to a `pub mod` containing `#[task]` functions. Auto-discovers tasks,
/// validates dependencies, and generates registration code based on delivery mode:
///
/// - **Embedded** (default): `#[ctor]` auto-registration
/// - **Packaged** (`features = ["packaged"]`): FFI exports for `.cloacina` packages
///
/// # Example
///
/// ```rust,ignore
/// #[workflow(name = "my_pipeline", description = "Process data")]
/// pub mod my_pipeline {
///     use super::*;
///
///     #[task(id = "fetch", dependencies = [])]
///     pub async fn fetch(ctx: &mut Context<Value>) -> Result<(), TaskError> { Ok(()) }
///
///     #[task(id = "process", dependencies = ["fetch"])]
///     pub async fn process(ctx: &mut Context<Value>) -> Result<(), TaskError> { Ok(()) }
/// }
/// ```
#[proc_macro_attribute]
pub fn workflow(args: TokenStream, input: TokenStream) -> TokenStream {
    workflow_attr::workflow_attr(args, input)
}

/// Define a trigger that fires a workflow on a schedule or condition.
///
/// # Custom poll trigger
///
/// ```rust,ignore
/// #[trigger(on = "my_workflow", poll_interval = "5s")]
/// pub async fn check_inbox() -> Result<TriggerResult, TriggerError> {
///     // check condition, return Fire(ctx) or Skip
/// }
/// ```
///
/// # Cron trigger (T-0305)
///
/// ```rust,ignore
/// #[trigger(on = "my_workflow", cron = "0 2 * * *", timezone = "UTC")]
/// ```
#[proc_macro_attribute]
pub fn trigger(args: TokenStream, input: TokenStream) -> TokenStream {
    trigger_attr::trigger_attr(args, input)
}

/// Define a computation graph as a module containing async node functions.
///
/// The topology is declared in the macro attribute. Nodes are pure async functions
/// within the module. The macro compiles the topology into a single async function
/// with nested match arms for enum routing.
///
/// # Example
///
/// ```rust,ignore
/// #[computation_graph(
///     react = when_any(alpha, beta),
///     graph = {
///         decision(alpha, beta) => {
///             Signal -> output_handler,
///             NoAction -> audit_logger,
///         },
///     }
/// )]
/// mod my_strategy {
///     async fn decision(alpha: Option<&AlphaData>, beta: Option<&BetaData>) -> DecisionOutcome { ... }
///     async fn output_handler(signal: &Signal) -> OutputConfirmation { ... }
///     async fn audit_logger(reason: &NoActionReason) -> AuditRecord { ... }
/// }
/// ```
#[proc_macro_attribute]
pub fn computation_graph(args: TokenStream, input: TokenStream) -> TokenStream {
    computation_graph::computation_graph_attr(args, input)
}

/// Define a passthrough accumulator (socket-only, no event loop).
///
/// ```rust,ignore
/// #[passthrough_accumulator]
/// fn beta(event: PricingUpdate) -> BetaData {
///     BetaData { estimate: event.mid_price }
/// }
/// ```
#[proc_macro_attribute]
pub fn passthrough_accumulator(args: TokenStream, input: TokenStream) -> TokenStream {
    match computation_graph::accumulator_macros::passthrough_accumulator_impl(
        args.into(),
        input.into(),
    ) {
        Ok(output) => output.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Define a stream-backed accumulator.
///
/// ```rust,ignore
/// #[stream_accumulator(type = "kafka", topic = "market.orderbook")]
/// fn alpha(event: OrderBookUpdate) -> AlphaData {
///     AlphaData { top_high: event.best_ask, top_low: event.best_bid }
/// }
/// ```
#[proc_macro_attribute]
pub fn stream_accumulator(args: TokenStream, input: TokenStream) -> TokenStream {
    match computation_graph::accumulator_macros::stream_accumulator_impl(args.into(), input.into())
    {
        Ok(output) => output.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Define a polling accumulator (timer-based, queries pull-based sources).
///
/// ```rust,ignore
/// #[polling_accumulator(interval = "5s")]
/// async fn config_source() -> Option<ConfigData> {
///     let data = fetch_config().await;
///     if data.changed() { Some(data) } else { None }
/// }
/// ```
#[proc_macro_attribute]
pub fn polling_accumulator(args: TokenStream, input: TokenStream) -> TokenStream {
    match computation_graph::accumulator_macros::polling_accumulator_impl(args.into(), input.into())
    {
        Ok(output) => output.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
