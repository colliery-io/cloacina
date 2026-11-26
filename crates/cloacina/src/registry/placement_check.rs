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

//! The check of a package's task placements when the package is loaded
//! (CLOACINA-T-1027).
//!
//! A task may name an agent pool and request CPU and memory
//! (`Task::placement()`). What a tenant's pools are, and how much one agent
//! of a pool can hold, is knowledge of the server, not of this crate. So the
//! server registers a [`TaskPlacementCheck`], and the registry reconciler
//! asks it when it loads a package: that is the first moment the tasks of a
//! package are known (for a Rust package, after its compile).
//!
//! A package that the check refuses is not loaded, and its row gets build
//! status `failed` with the reason in `build_error`
//! ([`WorkflowRegistry::refuse_package`](super::traits::WorkflowRegistry::refuse_package)):
//! the place a publisher already reads for a build that failed.
//!
//! A process that registers no check (an embedded runner, the daemon) loads
//! every package: it has no agent pools to check against.

use std::sync::{Arc, OnceLock};

use cloacina_workflow::TaskPlacement;

/// One task of a package, for the check: its id and what it declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedTask {
    /// The task's id inside its workflow (`generate_reports`).
    pub task_id: String,
    /// The task's pool, requests and inline mark.
    pub placement: TaskPlacement,
}

/// Decides whether the tasks of a package can be placed in a tenant.
#[async_trait::async_trait]
pub trait TaskPlacementCheck: Send + Sync {
    /// `Ok(())` when every task can be placed. `Err(reason)` refuses the
    /// package; the reason is one plain sentence for the publisher, and names
    /// the task, the pool and the numbers.
    async fn check(&self, tenant_id: &str, tasks: &[PlacedTask]) -> Result<(), String>;
}

static TASK_PLACEMENT_CHECK: OnceLock<Arc<dyn TaskPlacementCheck>> = OnceLock::new();

/// Register the check of this process. The first registration wins; a later
/// one is ignored. The server calls it once at start.
pub fn register_task_placement_check(check: Arc<dyn TaskPlacementCheck>) {
    let _ = TASK_PLACEMENT_CHECK.set(check);
}

/// The check of this process, when one is registered.
pub fn task_placement_check() -> Option<Arc<dyn TaskPlacementCheck>> {
    TASK_PLACEMENT_CHECK.get().cloned()
}
