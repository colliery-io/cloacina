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

// Worker-process fixture (CLOACINA-T-1028). Four tasks with no dependency
// between them, so one run of the workflow starts them together on one agent:
//
// - `steady` requests 256Mi, blocks for a few seconds and ends well.
// - `greedy` requests 64Mi, then takes and touches 512 MB. The agent stops
//   it: it never writes its end line.
// - `slow` has a time limit of 3 s and blocks for 60 s. The agent stops it:
//   it never writes its end line.
// - `quick` is marked `inline`: it runs in the agent process.
//
// Each task appends `<task> <event> <process id>` to a marker file. The
// process id shows where the task ran: `quick` in the agent, the others in a
// worker process each.
//
// Marker path: env FLEET_WORKERS_MARKER (the agent process passes it on).
// Run time of `steady`: env FLEET_WORKERS_STEADY_SECONDS, default 6.

use std::io::Write;
use std::time::Duration;

use cloacina_workflow::{Context, TaskError, task, workflow};

cloacina_workflow_plugin::package!();

fn mark(task: &str, event: &str) {
    let Ok(path) = std::env::var("FLEET_WORKERS_MARKER") else {
        return;
    };
    // The tasks run in different processes at the same time: each line goes
    // out in one write, so two lines do not mix.
    let line = format!("{} {} {}\n", task, event, std::process::id());
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = f.write_all(line.as_bytes());
    }
}

#[workflow(
    name = "fleet_workers_workflow",
    description = "fleet worker processes e2e fixture",
    author = "fleet-e2e"
)]
pub mod fleet_workers_workflow {
    use super::*;

    #[task(retry_attempts = 0, memory = "256Mi")]
    pub async fn steady(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let seconds = std::env::var("FLEET_WORKERS_STEADY_SECONDS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(6);
        mark("steady", "start");
        std::thread::sleep(Duration::from_secs(seconds));
        mark("steady", "end");
        context.insert("fleet_workers_steady_ran", serde_json::json!(true))?;
        Ok(())
    }

    #[task(retry_attempts = 0, memory = "64Mi")]
    pub async fn greedy(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        mark("greedy", "start");
        let mut block = vec![0u8; 512 * 1000 * 1000];
        for at in (0..block.len()).step_by(4096) {
            block[at] = 1;
        }
        std::thread::sleep(Duration::from_secs(30));
        mark("greedy", "end");
        context.insert("fleet_workers_greedy_sum", serde_json::json!(block[0]))?;
        Ok(())
    }

    #[task(retry_attempts = 0, timeout_seconds = 3)]
    pub async fn slow(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        mark("slow", "start");
        std::thread::sleep(Duration::from_secs(60));
        mark("slow", "end");
        context.insert("fleet_workers_slow_ran", serde_json::json!(true))?;
        Ok(())
    }

    #[task(retry_attempts = 0, inline = true)]
    pub async fn quick(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        mark("quick", "start");
        mark("quick", "end");
        context.insert("fleet_workers_quick_ran", serde_json::json!(true))?;
        Ok(())
    }
}
