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

// Agent pools fixture (CLOACINA-T-1027). Two tasks with no dependency between
// them:
//
// - `small` names no pool and requests nothing: it runs in the pool `default`.
// - `big` names the pool `large` and requests 24Gi of memory. It blocks for a
//   few seconds, so that two runs of the workflow want the same agent at the
//   same time.
//
// Each task appends `<task> <event> <unix millis> <agent label>` to a marker
// file. The label is what the test gave the agent process, so the file shows
// which agent ran which task, and when.
//
// Marker path: env FLEET_POOLS_MARKER (the agent process passes it on).
// Agent label: env FLEET_POOLS_AGENT.
// Run time of `big`: env FLEET_POOLS_BIG_SECONDS, default 8.

use std::io::Write;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cloacina_workflow::{Context, TaskError, task, workflow};

cloacina_workflow_plugin::package!();

fn mark(task: &str, event: &str) {
    let Ok(path) = std::env::var("FLEET_POOLS_MARKER") else {
        return;
    };
    let agent = std::env::var("FLEET_POOLS_AGENT").unwrap_or_else(|_| "unknown".to_string());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{} {} {} {}", task, event, now, agent);
    }
}

#[workflow(
    name = "fleet_pools_workflow",
    description = "fleet agent pools e2e fixture",
    author = "fleet-e2e"
)]
pub mod fleet_pools_workflow {
    use super::*;

    #[task(retry_attempts = 0)]
    pub async fn small(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        mark("small", "start");
        mark("small", "end");
        context.insert("fleet_pools_small_ran", serde_json::json!(true))?;
        Ok(())
    }

    #[task(retry_attempts = 0, pool = "large", cpu = "1", memory = "24Gi")]
    pub async fn big(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let seconds = std::env::var("FLEET_POOLS_BIG_SECONDS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(8);
        mark("big", "start");
        std::thread::sleep(Duration::from_secs(seconds));
        mark("big", "end");
        context.insert("fleet_pools_big_ran", serde_json::json!(true))?;
        Ok(())
    }
}
