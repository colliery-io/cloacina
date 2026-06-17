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

// Demo poll-trigger fixture (CLOACI-I-0124 / WS-6). A custom *poll* trigger
// fires `demo_poll_workflow` on a fixed interval, so the UI's Triggers view
// shows a non-cron (`trigger`-type) schedule and executions appear
// automatically — the poll/event-driven counterpart to demo-cron-rust.
//
// A poll trigger binds to its target workflow via the `on` attribute and
// `poll_interval`. The reconciler routes it to the runtime trigger registry
// (not the cron scheduler) and records a `trigger`-type schedule row with the
// poll interval. The workflow does NOT list it in `#[workflow(triggers = …)]`;
// the standalone `on = …` binding is enough.

use cloacina_macros::{task, trigger, workflow};
use cloacina_workflow::{Context, TaskError, TriggerResult};

cloacina_workflow_plugin::package!();

// Polled every 30 seconds; fires the workflow each interval so the demo shows
// a live stream of poll-driven executions (mirrors the cron demo's cadence).
#[trigger(on = "demo_poll_workflow", poll_interval = "30s")]
pub async fn demo_poll_trigger() -> Result<TriggerResult, cloacina_workflow::TriggerError> {
    Ok(TriggerResult::Fire(None))
}

#[workflow(
    name = "demo_poll_workflow",
    description = "demo poll workflow — fires on a poll interval",
    author = "cloacina-ui-demo"
)]
pub mod demo_poll_wf {
    use super::*;

    #[task(id = "demo_poll_step", dependencies = [])]
    pub async fn demo_poll_step(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("demo_poll_ran", serde_json::json!(true))?;
        Ok(())
    }
}
