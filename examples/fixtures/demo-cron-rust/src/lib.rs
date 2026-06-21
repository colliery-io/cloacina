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
//   poll ──┬─▶ process ─▶ record
//          └─▶ alert (SKIPPED)
//
// The bulk of the demo's executions are cron runs, so this workflow is
// branching (not a single dot) and carries a skipped node on every run, to make
// the Executions list's DAGs interesting: `poll` sets `raise_alert = false`, so
// `alert`'s trigger rule never fires → Skipped; `record` fans in on
// `process` + `alert` (a Skipped dep counts as resolved) and still runs.
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
    description = "demo cron workflow — a branching schedule run with a skipped path",
    author = "cloacina-ui-demo"
)]
pub mod demo_cron_wf {
    use super::*;

    // Jittered millis in [base - jitter, base + jitter], seeded from the wall
    // clock (xorshift64, no `rand` dep) so the steady stream of scheduled runs
    // builds a real duration distribution for the UI's runtime view.
    fn jitter_ms(base: u64, jitter: u64) -> u64 {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1)
            | 1;
        let mut x = seed;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let span = jitter.saturating_mul(2).saturating_add(1);
        base.saturating_sub(jitter).saturating_add(x % span)
    }

    #[task(retry_attempts = 0)]
    pub async fn poll(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        tokio::time::sleep(std::time::Duration::from_millis(jitter_ms(800, 400))).await;
        // Gate the alert branch off → it skips every run.
        context.insert("raise_alert", serde_json::json!(false))?;
        Ok(())
    }

    #[task(dependencies = ["poll"], retry_attempts = 0)]
    pub async fn process(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        // The visible-duration step: ~2–10s, lingers in Running for the demo.
        tokio::time::sleep(std::time::Duration::from_millis(jitter_ms(6_000, 4_000))).await;
        context.insert("demo_cron_ran", serde_json::json!(true))?;
        Ok(())
    }

    // Gated-off branch sibling → lands Skipped (the dashed node on the DAG).
    #[task(
        dependencies = ["poll"],
        retry_attempts = 0,
        trigger_rules = context_value("raise_alert", equals, true)
    )]
    pub async fn alert(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("alert_raised", serde_json::json!(true))?;
        Ok(())
    }

    // Fan-in on the run path + the skipped path.
    #[task(dependencies = ["process", "alert"], retry_attempts = 0)]
    pub async fn record(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        tokio::time::sleep(std::time::Duration::from_millis(jitter_ms(700, 300))).await;
        context.insert("demo_cron_recorded", serde_json::json!(true))?;
        Ok(())
    }
}
