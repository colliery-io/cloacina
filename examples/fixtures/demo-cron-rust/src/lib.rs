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

// Demo cron-trigger fixture (CLOACI-I-0117 / T-0664). A cron trigger fires
// `demo_cron_workflow` on a schedule, so the UI's Triggers view shows the
// schedule and executions appear automatically.
//
// A cron trigger binds to its target workflow via the `on` attribute and is
// driven directly by the cron scheduler — the workflow does NOT list it in
// `#[workflow(triggers = [...])]` (that list is for custom/poll-trigger
// *subscriptions*, e.g. mixed-rust). Mirrors trigger-only-rust's standalone
// cron trigger.

use cloacina_macros::{task, trigger, workflow};
use cloacina_workflow::{Context, TaskError};

cloacina_workflow_plugin::package!();

// Every 15 seconds — frequent enough to watch in the demo.
#[trigger(on = "demo_cron_workflow", cron = "*/15 * * * * *")]
pub async fn demo_cron_trigger() {}

#[workflow(
    name = "demo_cron_workflow",
    description = "demo cron workflow — fires on a schedule",
    author = "cloacina-ui-demo"
)]
pub mod demo_cron_wf {
    use super::*;

    #[task(id = "demo_cron_step", dependencies = [])]
    pub async fn demo_cron_step(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        // Sleep so each cron run lingers visibly in the Running state — the
        // task was instantaneous, so the UI never showed a running (blue) run
        // (CLOACI-I-0124 / WS-10 demo liveness).
        tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        context.insert("demo_cron_ran", serde_json::json!(true))?;
        Ok(())
    }
}
