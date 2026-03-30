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

//! Trigger types for workflow authoring.
//!
//! These types are used by `#[trigger]` macro-generated code.
//! The full `Trigger` trait lives in `cloacina` (runtime crate).

use crate::Context;

/// Result of a trigger poll operation.
#[derive(Debug)]
pub enum TriggerResult {
    /// Do not fire the workflow, continue polling on the next interval.
    Skip,
    /// Fire the workflow with an optional context.
    Fire(Option<Context<serde_json::Value>>),
}

/// Errors that can occur during trigger polling.
#[derive(Debug, thiserror::Error)]
pub enum TriggerError {
    /// Error during trigger polling
    #[error("Trigger poll error: {0}")]
    PollError(String),
    /// Context creation error
    #[error("Context error: {0}")]
    ContextError(#[from] crate::error::ContextError),
}
