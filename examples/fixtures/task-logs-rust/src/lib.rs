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

// Fixture for the log lines of a packaged Rust task (CLOACINA-T-1099).
//
// One task writes one line each way a task author writes one: `tracing` at
// INFO (with a field and its own target), `tracing` at DEBUG, the `log`
// crate at WARN, and `println!`. The test in
// `crates/cloacina/tests/packaged_task_logs.rs` builds this package, loads it
// through the task registrar and runs the task in the in-process executor.

use cloacina_workflow::{Context, TaskError, task, workflow};

cloacina_workflow_plugin::package!();

#[workflow(
    name = "task_logs_workflow",
    description = "packaged task log fixture",
    author = "task-logs"
)]
pub mod task_logs_workflow {
    use super::*;

    /// Writes one line with each of `tracing`, `log` and `println!`.
    #[task(id = "speak", retry_attempts = 0)]
    pub async fn speak(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        tracing::info!(target: "task_logs::speak", rows = 3, "packaged tracing line");
        tracing::debug!(target: "task_logs::speak", "packaged debug line");
        log::warn!(target: "task_logs::legacy", "packaged log line");
        println!("packaged println line");
        context.insert("spoke", serde_json::json!(true))?;
        Ok(())
    }
}
