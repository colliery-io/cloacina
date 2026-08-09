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

//! Slot access for packaged tasks that defer (CLOACI-T-0897).
//!
//! An embedded task reaches its concurrency slot through the [`TaskHandle`]
//! the executor installs in a task-local. A PACKAGED task cannot: its
//! `defer_until` runs inside a cdylib and comes back through the
//! `CloacinaHost` callback channel, which lands on a tokio blocking-pool
//! thread (the host invokes packaged tasks via `spawn_blocking`) where that
//! task-local is invisible.
//!
//! So the executor also registers the handle's slot here, keyed by the
//! task-execution UUID the callback carries. It is the SAME `Arc`, not a copy
//! — there is exactly one `SlotToken` per running task, whichever door reaches
//! it.
//!
//! Entries are removed when the task finishes. A packaged task that never
//! defers simply never looks its entry up.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use tokio::sync::Mutex as AsyncMutex;

use super::slot_token::SlotToken;
use crate::dal::DAL;
use crate::database::universal_types::UniversalUuid;

/// What a host callback needs to service a deferral for one running task.
#[derive(Clone)]
pub(crate) struct DeferralEntry {
    /// The running task's one and only slot, shared with its `TaskHandle`.
    pub(crate) slot: Arc<AsyncMutex<SlotToken>>,
    /// For `sub_status` writes, so an operator can see a task is deferred
    /// rather than wedged.
    pub(crate) dal: DAL,
}

/// Process-wide, because the binding fidius installs is per loaded library and
/// the callback carries only a task id — there is no ambient executor to ask.
/// Keyed by task-execution UUID, which is unique across tenants.
fn registry() -> &'static Mutex<HashMap<UniversalUuid, DeferralEntry>> {
    static REGISTRY: OnceLock<Mutex<HashMap<UniversalUuid, DeferralEntry>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Publish a running task's slot so host callbacks can find it.
pub(crate) fn register(
    task_execution_id: UniversalUuid,
    slot: Arc<AsyncMutex<SlotToken>>,
    dal: DAL,
) {
    registry()
        .lock()
        .expect("deferral registry poisoned")
        .insert(task_execution_id, DeferralEntry { slot, dal });
}

/// Remove a finished task's entry. Idempotent — a task that never deferred is
/// removed the same way.
pub(crate) fn deregister(task_execution_id: &UniversalUuid) {
    registry()
        .lock()
        .expect("deferral registry poisoned")
        .remove(task_execution_id);
}

/// Look up a running task's slot. `None` once the task has finished, which is
/// what turns a late callback into a typed error rather than a panic.
pub(crate) fn lookup(task_execution_id: &UniversalUuid) -> Option<DeferralEntry> {
    registry()
        .lock()
        .expect("deferral registry poisoned")
        .get(task_execution_id)
        .cloned()
}

/// How many tasks are currently registered. Test-facing: lets a test prove
/// entries are cleaned up rather than leaked.
#[cfg(test)]
pub(crate) fn len() -> usize {
    registry().lock().expect("poisoned").len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::slot_token::SlotToken;
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    async fn a_slot() -> Arc<AsyncMutex<SlotToken>> {
        let sem = Arc::new(Semaphore::new(1));
        let permit = sem.clone().acquire_owned().await.unwrap();
        Arc::new(AsyncMutex::new(SlotToken::new(permit, sem)))
    }

    /// The registry hands back the SAME slot the caller registered — a copy
    /// would mean two owners of one permit and a double release.
    #[tokio::test]
    async fn lookup_returns_the_same_slot_arc() {
        let id = UniversalUuid::new_v4();
        let slot = a_slot().await;
        register(
            id,
            Arc::clone(&slot),
            DAL::new(crate::database::Database::new(
                &format!(
                    "file:defreg_{}?mode=memory&cache=shared",
                    uuid::Uuid::new_v4()
                ),
                "",
                2,
            )),
        );

        let found = lookup(&id).expect("registered");
        assert!(
            Arc::ptr_eq(&found.slot, &slot),
            "registry must share the slot, not clone the token"
        );

        deregister(&id);
        assert!(lookup(&id).is_none(), "deregister must remove the entry");
    }

    /// A callback that arrives after the task finished must find nothing, so
    /// the host can answer with a typed error instead of touching a dead slot.
    #[tokio::test]
    async fn lookup_after_deregister_is_none() {
        let id = UniversalUuid::new_v4();
        assert!(lookup(&id).is_none());
    }
}
