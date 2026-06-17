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

// Demo failing fixture (CLOACI-I-0117 / T-0660): a two-task workflow that
// does a little work and then deterministically fails. `prepare` succeeds
// (so the execution shows partial progress), then `boom` returns a
// TaskError — giving the seed/demo harness a guaranteed failed execution to
// drive the UI's failed-state / debug view.

use cloacina_workflow::{task, workflow, Context, TaskError};

cloacina_workflow_plugin::package!();

#[workflow(
    name = "demo_fail_workflow",
    description = "demo failing workflow — a task that deterministically errors",
    author = "cloacina-ui-demo"
)]
pub mod demo_fail_workflow {
    use super::*;

    #[task(retry_attempts = 0)]
    pub async fn prepare(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        // A pause so the failed run isn't instantaneous — the partial-progress
        // (Running) state is visible before the failure lands (WS-10 liveness).
        std::thread::sleep(std::time::Duration::from_secs(3));
        context.insert("prepare_done", serde_json::json!(true))?;
        Ok(())
    }

    #[task(dependencies = ["prepare"], retry_attempts = 0)]
    pub async fn boom(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        Err(TaskError::ExecutionFailed {
            message: "demo failure: this task is designed to fail".to_string(),
            task_id: "boom".to_string(),
            timestamp: chrono::Utc::now(),
        })
    }
}
