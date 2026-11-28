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

// Fixture for priority in a package (CLOACINA-T-1123).
//
// The workflow declares priority 2. The task `urgent` declares its own, 0;
// the task `plain` declares none and takes its workflow's. The test in
// `crates/cloacina/tests/integration/packaged_priority.rs` builds this
// package, loads it through the reconciler and runs it on SQLite.

use cloacina_workflow::{Context, TaskError, task, workflow};

cloacina_workflow_plugin::package!();

#[workflow(
    name = "packaged_priority_workflow",
    description = "packaged priority fixture",
    author = "priority",
    priority = 2
)]
pub mod packaged_priority_workflow {
    use super::*;

    /// More important than the other tasks of its workflow.
    #[task(id = "urgent", retry_attempts = 0, priority = 0)]
    pub async fn urgent(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("urgent", serde_json::json!(true))?;
        Ok(())
    }

    /// Takes the priority of its workflow.
    #[task(id = "plain", retry_attempts = 0)]
    pub async fn plain(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("plain", serde_json::json!(true))?;
        Ok(())
    }
}
