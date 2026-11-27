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

// Fixture of the behaviour suite's standing workload (CLOACINA-T-1107): a
// short data job in Rust, on a five-minute schedule. It moves no real data.
// Each run writes the interval the scheduler gave it back into its context
// under `standing_rollup_done`, so the suite's checker can compare the
// interval a run saw with the interval of its fire time.

use cloacina_macros::{task, trigger, workflow};
use cloacina_workflow::{Context, TaskError};

cloacina_workflow_plugin::package!();

// Every fifth minute.
#[trigger(on = "standing_rollup", cron = "*/5 * * * *")]
pub async fn standing_rollup_schedule() {}

#[workflow(
    name = "standing_rollup",
    description = "standing workload: a short data job every five minutes",
    author = "behaviour-suite"
)]
pub mod standing_rollup {
    use super::*;

    #[task(retry_attempts = 0)]
    pub async fn read_interval(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        let start = context.get("interval_start").cloned();
        let end = context.get("interval_end").cloned();
        let fired = context.get("scheduled_time").cloned();
        // One marked line for each start of a task body.
        println!(
            "STANDING_MARK {}",
            serde_json::json!({
                "workflow": "standing_rollup",
                "task": "read_interval",
                "scheduled_time": fired,
            })
        );
        context.insert(
            "standing_rollup_interval",
            serde_json::json!({ "start": start, "end": end }),
        )?;
        Ok(())
    }

    #[task(dependencies = ["read_interval"], retry_attempts = 0)]
    pub async fn roll_up(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let interval = context
            .get("standing_rollup_interval")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let fired = context.get("scheduled_time").cloned();
        println!(
            "STANDING_MARK {}",
            serde_json::json!({
                "workflow": "standing_rollup",
                "task": "roll_up",
                "scheduled_time": fired,
            })
        );
        context.insert(
            "standing_rollup_done",
            serde_json::json!({
                "interval_start": interval.get("start"),
                "interval_end": interval.get("end"),
                "scheduled_time": fired,
            }),
        )?;
        Ok(())
    }
}
