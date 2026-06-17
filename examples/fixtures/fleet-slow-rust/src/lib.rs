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

// Fleet in-flight reclaim fixture: one task that sleeps for a context-supplied
// duration. The deliberate sleep holds the task "in flight" on whichever agent
// claims it, so the e2e can kill that agent mid-execution and assert the server
// reclaims the orphaned work onto a survivor (CLOACI-T-0638 / T-0634).

use cloacina_workflow::{task, workflow, Context, TaskError};

// I-0102 / T-C: unified plugin shell.
cloacina_workflow_plugin::package!();

#[workflow(
    name = "fleet_slow_workflow",
    description = "fleet in-flight reclaim fixture — one task that sleeps",
    author = "fleet-e2e"
)]
pub mod fleet_slow_workflow {
    use super::*;

    // `retry_attempts = 0`: reclaim re-targets delivery of the SAME attempt to a
    // live agent (it is not a task-level retry), so leaving retries off keeps the
    // reclaim semantics clean to assert on.
    #[task(
        retry_attempts = 0
    )]
    pub async fn slow(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        // Default small so the fixture is cheap to run standalone; the reclaim
        // e2e passes a long value (e.g. 90) via `workflow run --context`.
        let secs = context
            .get("sleep_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(2);
        // BLOCKING sleep on purpose: this runs inside a dlopened cdylib whose
        // statically-linked tokio is a *different* instance than the agent's, so
        // `tokio::time::sleep` finds no runtime in its thread-local context and
        // panics ("no reactor running"). `std::thread::sleep` needs no runtime —
        // it just parks the worker thread. The agent's multi-threaded runtime
        // keeps heartbeating on its other workers.
        std::thread::sleep(std::time::Duration::from_secs(secs));
        context.insert("fleet_slow_ran", serde_json::json!(true))?;
        Ok(())
    }
}
