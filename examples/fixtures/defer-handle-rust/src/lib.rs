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

// Fixture for packaged `defer_until` (CLOACI-T-0897).
//
// This is the shape that could NOT be built before: a PACKAGED task taking a
// handle parameter. The macro used to emit `::cloacina::take_task_handle()`
// unconditionally, which does not resolve in a cdylib that links only
// `cloacina-workflow` — the build failed with a bare `cannot find crate
// cloacina`. Its existence compiling at all is half the regression net; the
// other half is the e2e lane observing the slot actually being released.
//
// The condition is time-based rather than filesystem-based so the fixture is
// hermetic: it defers once, then resolves on its own. A file-watch version
// would need the harness to race a file into place.

use std::time::Duration;

use cloacina_workflow::{task, workflow, Context, TaskError};

cloacina_workflow_plugin::package!();

#[workflow(
    name = "defer_handle_workflow",
    description = "packaged defer_until fixture — a task that releases its slot while waiting",
    author = "defer-e2e"
)]
pub mod defer_handle_workflow {
    use super::*;

    /// Defers until a deadline passes, then records that it resumed.
    ///
    /// The handle parameter is the point: obtaining it inside a packaged build
    /// exercises the plugin-side `TaskHandle`, and `defer_until` drives the
    /// full callback round trip — set sub_status, release the slot, poll here
    /// in the plugin, reclaim, restore sub_status.
    #[task(id = "wait_then_work", retry_attempts = 0)]
    pub async fn wait_then_work(
        context: &mut Context<serde_json::Value>,
        handle: &mut cloacina_workflow_plugin::TaskHandle,
    ) -> Result<(), TaskError> {
        // Record the id the host told us we are, so the harness can prove the
        // plugin received a real task-execution id rather than an empty string.
        context.insert(
            "observed_task_execution_id",
            serde_json::json!(handle.task_execution_id()),
        )?;

        let deadline = std::time::Instant::now() + Duration::from_millis(1200);
        handle
            .defer_until(
                || async move { std::time::Instant::now() >= deadline },
                Duration::from_millis(200),
            )
            .await?;

        context.insert("deferred_and_resumed", serde_json::json!(true))?;
        Ok(())
    }
}
