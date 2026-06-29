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

//! Package loader module for workflow registry.
//!
//! This module provides functionality to load workflow packages (.so files),
//! extract metadata, validate package integrity, and register tasks with the
//! global task registry.

pub mod ffi_trigger;
pub mod ffi_triggerless_graph;
/// WASM task-operator loader + executor adapter (CLOACI-I-0132 / T-0823).
/// Default-OFF behind the `operators-wasm` feature.
#[cfg(feature = "operators-wasm")]
pub mod operator_loader;
pub mod package_loader;
pub mod task_registrar;

pub use package_loader::PackageLoader;
pub use task_registrar::TaskRegistrar;

#[cfg(feature = "operators-wasm")]
pub use operator_loader::{
    load_operator, load_task_operator, load_trigger_operator, OperatorBinding, TriggerBinding,
    WasmTaskOperator, WasmTriggerOperator,
};
