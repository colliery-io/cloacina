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

//! The engine's implementation of the `CloacinaHost` callback interface
//! (CLOACI-T-0897) — what a packaged task's `defer_until` actually drives.
//!
//! ## Why blocking here is safe
//!
//! These methods are synchronous (fidius's boundary is sync by design) while
//! the operations behind them are async, so each bridges with
//! `Handle::block_on`. That is safe *specifically* because of where the call
//! lands: the host invokes packaged tasks through
//! `tokio::task::spawn_blocking` (`registry/loader/task_registrar/dynamic_task.rs`),
//! so a callback runs on the BLOCKING pool, never on a runtime worker. Runtime
//! workers keep draining, other tasks keep completing, and the slot this call
//! is waiting for keeps being freed.
//!
//! An earlier reading of this had it backwards and predicted a self-deadlock;
//! see the ticket for the retraction. The property to preserve is the one that
//! makes it safe: **never call these from a runtime worker.**
//!
//! Residual cost, accepted and documented: a deferred task holds its
//! blocking-pool thread for the whole wait. It has released its concurrency
//! slot, so it is not blocking other work — it is just parked.

use std::str::FromStr;

use cloacina_workflow_plugin::fidius::PluginError;
use cloacina_workflow_plugin::CloacinaHost;
use tokio::runtime::Handle;

use super::deferral_registry;
use crate::database::universal_types::UniversalUuid;

/// Services `defer_until` callbacks from packaged tasks.
///
/// Stateless: everything it needs is looked up per call from the deferral
/// registry, keyed by the task-execution id the plugin sends. That keeps one
/// bound instance correct for every tenant and every concurrently running task.
pub struct EngineHost;

impl EngineHost {
    fn parse_id(raw: &str) -> Result<UniversalUuid, PluginError> {
        uuid::Uuid::from_str(raw)
            .map(UniversalUuid::from)
            .map_err(|e| PluginError::new("BAD_TASK_ID", format!("malformed task id {raw:?}: {e}")))
    }

    fn entry(raw: &str) -> Result<deferral_registry::DeferralEntry, PluginError> {
        let id = Self::parse_id(raw)?;
        deferral_registry::lookup(&id).ok_or_else(|| {
            // Not a panic and not a silent success: a callback for a task that
            // has already finished (or was never registered) is a real error
            // the task should see.
            PluginError::new(
                "TASK_NOT_RUNNING",
                format!("no running task registered for {raw} — it may have already completed"),
            )
        })
    }
}

impl CloacinaHost for EngineHost {
    fn release_slot(&self, task_execution_id: String) -> Result<(), PluginError> {
        let entry = Self::entry(&task_execution_id)?;
        Handle::current().block_on(async move {
            // `release` is idempotent — releasing an already-released slot
            // returns false rather than erroring, so a task that defers twice
            // is not punished for it.
            entry.slot.lock().await.release();
        });
        Ok(())
    }

    fn reclaim_slot(&self, task_execution_id: String) -> Result<(), PluginError> {
        let entry = Self::entry(&task_execution_id)?;
        Handle::current().block_on(async move {
            entry.slot.lock().await.reclaim().await.map_err(|e| {
                // The semaphore is closed only when the executor is shutting
                // down. Surfacing it lets the deferred task fail cleanly
                // instead of waiting on capacity that will never come.
                PluginError::new("SLOT_RECLAIM_FAILED", format!("{e}"))
            })
        })
    }

    fn set_sub_status(
        &self,
        task_execution_id: String,
        sub_status: String,
    ) -> Result<(), PluginError> {
        let id = Self::parse_id(&task_execution_id)?;
        let entry = Self::entry(&task_execution_id)?;
        let value = if sub_status.is_empty() {
            None
        } else {
            Some(sub_status)
        };
        Handle::current().block_on(async move {
            entry
                .dal
                .task_execution()
                .set_sub_status(id, value.as_deref())
                .await
                .map_err(|e| PluginError::new("SUB_STATUS_FAILED", format!("{e}")))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A callback naming a task that is not running must be a typed error, not
    /// a panic across the FFI boundary and not a silent success.
    #[tokio::test]
    async fn unknown_task_is_a_typed_error() {
        let host = EngineHost;
        let err = host
            .release_slot(uuid::Uuid::new_v4().to_string())
            .expect_err("unknown task must error");
        assert_eq!(err.code, "TASK_NOT_RUNNING");
    }

    /// A malformed id is rejected before any lookup, with its own code so the
    /// cause is obvious in a plugin's error message.
    #[tokio::test]
    async fn malformed_task_id_is_rejected() {
        let host = EngineHost;
        let err = host
            .release_slot("not-a-uuid".to_string())
            .expect_err("malformed id must error");
        assert_eq!(err.code, "BAD_TASK_ID");
    }
}
