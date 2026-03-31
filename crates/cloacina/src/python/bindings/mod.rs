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

//! Python API wrapper types for the cloaca wheel.
//!
//! These types wrap cloacina's Rust API for Python consumers:
//! - `PyDefaultRunner` / `PyPipelineResult` — workflow execution
//! - `PyDefaultRunnerConfig` — runner configuration
//! - `PyDatabaseAdmin` / `PyTenantConfig` / `PyTenantCredentials` — admin
//! - `PyTriggerResult` — trigger results
//! - `PyRetryPolicy` / `PyBackoffStrategy` / `PyRetryCondition` — retry config

#[cfg(feature = "postgres")]
pub mod admin;
pub mod context;
pub mod runner;
pub mod trigger;
pub mod value_objects;

#[cfg(feature = "postgres")]
pub use admin::{PyDatabaseAdmin, PyTenantConfig, PyTenantCredentials};
pub use context::PyDefaultRunnerConfig;
pub use runner::{PyDefaultRunner, PyPipelineResult};
pub use trigger::PyTriggerResult;
pub use value_objects::{PyBackoffStrategy, PyRetryCondition, PyRetryPolicy, PyRetryPolicyBuilder};
