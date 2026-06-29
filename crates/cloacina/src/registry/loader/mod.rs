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

/// WASM task-constructor loader + executor adapter (CLOACI-I-0132 / T-0823).
/// Default-OFF behind the `constructors-wasm` feature.
#[cfg(feature = "constructors-wasm")]
pub mod constructor_loader;
pub mod ffi_trigger;
pub mod ffi_triggerless_graph;
pub mod package_loader;
pub mod task_registrar;

pub use package_loader::PackageLoader;
pub use task_registrar::TaskRegistrar;

#[cfg(feature = "constructors-wasm")]
pub use constructor_loader::{
    load_constructor, load_task_constructor, load_task_constructor_from_package,
    load_trigger_constructor, unpack_provider_archive, ConstructorBinding, TriggerBinding,
    WasmTaskConstructor, WasmTriggerConstructor,
};
