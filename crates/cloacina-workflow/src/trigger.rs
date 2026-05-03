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
//! T-0552 (I-0102 follow-up) relocated the `Trigger` trait from `cloacina`
//! (engine-only) into this leaf crate so packaged cdylibs can collect
//! `TriggerEntry` inventory entries (which hold `Arc<dyn Trigger>`) at
//! link time, and the unified `cloacina::package!()` shell macro can walk
//! them at FFI call time. Engine paths re-export `cloacina_workflow::Trigger`.

use crate::Context;
use async_trait::async_trait;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::time::Duration;

/// Result of a trigger poll operation.
#[derive(Debug)]
pub enum TriggerResult {
    /// Do not fire the workflow, continue polling on the next interval.
    Skip,
    /// Fire the workflow with an optional context.
    Fire(Option<Context<serde_json::Value>>),
}

impl TriggerResult {
    /// Returns true if this result indicates the workflow should fire.
    pub fn should_fire(&self) -> bool {
        matches!(self, TriggerResult::Fire(_))
    }

    /// Extracts the context if this is a Fire result.
    pub fn into_context(self) -> Option<Context<serde_json::Value>> {
        match self {
            TriggerResult::Fire(ctx) => ctx,
            TriggerResult::Skip => None,
        }
    }

    /// Computes a hash of the context for deduplication purposes.
    ///
    /// If no context is provided, returns a constant hash. This allows
    /// deduplication based on the specific trigger conditions.
    pub fn context_hash(&self) -> String {
        match self {
            TriggerResult::Skip => "skip".to_string(),
            TriggerResult::Fire(None) => "fire_no_context".to_string(),
            TriggerResult::Fire(Some(ctx)) => {
                let mut hasher = DefaultHasher::new();
                if let Ok(serialized) = serde_json::to_string(ctx.data()) {
                    serialized.hash(&mut hasher);
                }
                format!("{:016x}", hasher.finish())
            }
        }
    }
}

/// Errors that can occur during trigger polling.
#[derive(Debug, thiserror::Error)]
pub enum TriggerError {
    /// Error during trigger polling
    #[error("Trigger poll error: {message}")]
    PollError { message: String },
    /// Context creation error
    #[error("Context error: {0}")]
    ContextError(#[from] crate::error::ContextError),
}

/// Core trait for user-defined triggers.
///
/// Triggers are polling functions that determine when a workflow should
/// execute. Each trigger has a name, poll interval, and a `poll()` method
/// that returns whether the workflow should fire.
#[async_trait]
pub trait Trigger: Send + Sync + fmt::Debug {
    /// Returns the unique name of this trigger.
    fn name(&self) -> &str;

    /// Returns how often this trigger should be polled.
    fn poll_interval(&self) -> Duration;

    /// Returns whether concurrent executions with the same context are
    /// allowed. When `false`, if a workflow execution with the same context
    /// hash is already running, the trigger will not fire again until it
    /// completes.
    fn allow_concurrent(&self) -> bool;

    /// Polls the trigger condition and returns whether to fire the workflow.
    ///
    /// Called at the configured `poll_interval`. Returns `TriggerResult::Skip`
    /// to continue polling, `TriggerResult::Fire(ctx)` to fire the workflow.
    /// Errors are logged and polling continues on the next interval.
    async fn poll(&self) -> Result<TriggerResult, TriggerError>;

    /// Returns this trigger's cron expression, if any. Cron-shaped triggers
    /// override this to return `Some(expr)`; their `poll_interval` is ignored
    /// and the reconciler routes them to the cron scheduler instead of the
    /// runtime trigger registry. Default `None` covers all custom-poll
    /// triggers.
    fn cron_expression(&self) -> Option<String> {
        None
    }
}
