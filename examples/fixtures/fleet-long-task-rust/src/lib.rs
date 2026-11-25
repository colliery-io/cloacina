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

// Long-task fixture: one task that appends a line to a marker file when it
// starts and when it ends, and blocks in between. The fleet long-task e2e
// counts the start lines: more than one means the fleet ran the task again
// while the first copy was still running.
//
// Marker path: env FLEET_LONG_TASK_MARKER (the agent process passes it on).
// Run time: env FLEET_LONG_TASK_SECONDS, default 150.

use std::io::Write;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cloacina_workflow::{Context, TaskError, task, workflow};

cloacina_workflow_plugin::package!();

fn mark(event: &str) {
    let Ok(path) = std::env::var("FLEET_LONG_TASK_MARKER") else {
        return;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{} {} {}", event, now, std::process::id());
    }
}

#[workflow(
    name = "fleet_long_task_workflow",
    description = "fleet long-task e2e fixture",
    author = "fleet-e2e"
)]
pub mod fleet_long_task_workflow {
    use super::*;

    #[task(retry_attempts = 0)]
    pub async fn long_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let seconds = std::env::var("FLEET_LONG_TASK_SECONDS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(150);
        mark("start");
        std::thread::sleep(Duration::from_secs(seconds));
        mark("end");
        context.insert("fleet_long_task_ran", serde_json::json!(true))?;
        Ok(())
    }
}
