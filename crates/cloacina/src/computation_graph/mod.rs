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

//! # Computation Graph Runtime Types
//!
//! Types used by compiled computation graphs at runtime. These are the interface
//! between the `#[computation_graph]` macro-generated code and the reactor.
//!
//! - [`InputCache`] — holds the last-seen boundary per source, used by the reactor
//! - [`GraphResult`] — returned by the compiled graph function
//! - [`SourceName`] — identifies an accumulator source

pub mod accumulator;
pub mod global_registry;
pub mod packaging_bridge;
pub mod reactor;
pub mod registry;
pub mod scheduler;
pub mod stream_backend;
pub mod triggerless;
pub mod types;

pub use accumulator::{
    accumulator_runtime, accumulator_runtime_with_source, batch_accumulator_runtime, flush_signal,
    polling_accumulator_runtime, shutdown_signal, Accumulator, AccumulatorContext,
    AccumulatorError, AccumulatorRuntimeConfig, BatchAccumulator, BatchAccumulatorConfig,
    BoundarySender, EventSource, PollingAccumulator,
};
pub use global_registry::ComputationGraphRegistration;
pub use triggerless::{TriggerlessGraphFn, TriggerlessGraphRegistration};
pub use types::{GraphError, GraphResult, InputCache, SourceName};
