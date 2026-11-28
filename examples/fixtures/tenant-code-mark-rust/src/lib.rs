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

// Fixture of the behaviour suite chapter "tenant_code" (CLOACINA-T-1079,
// CLOACINA-A-0019). Three pieces of code leave a mark:
//
// - load-time code: a function in the library's init array runs when any
//   process opens the library. It writes one marked line to stderr and keeps
//   the same facts;
// - the task: it writes its own mark and copies the load-time facts of its
//   process into the run's context;
// - a poll trigger: it fires the workflow with its own facts in the context.
//
// The chapter reads the marks from the run's context (where the code ran) and
// looks for the marked lines in the logs of the server and the compiler
// (where it must not run). A mark holds names only: the program, its host
// name, and which credential names its environment holds. Never a value.

use std::io::Write;
use std::sync::OnceLock;

use cloacina_macros::{task, trigger, workflow};
use cloacina_workflow::{Context, TaskError, TriggerResult};

cloacina_workflow_plugin::package!();

pub const LOAD_MARK: &str = "TENANT_CODE_LOAD_MARK";
pub const TASK_MARK: &str = "TENANT_CODE_TASK_MARK";
pub const POLL_MARK: &str = "TENANT_CODE_POLL_MARK";

// Names that give their holder the orchestrator's own reach.
const ORCHESTRATOR_CREDENTIALS: [&str; 4] = [
    "DATABASE_URL",
    "CLOACINA_BOOTSTRAP_KEY",
    "CLOACINA_SECRET_KEK",
    "PGPASSWORD",
];

static LOADED: OnceLock<serde_json::Value> = OnceLock::new();

/// Which process runs this code, and which credential names it holds.
fn facts(moment: &str) -> serde_json::Value {
    let process = std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "unknown".to_string());
    let describe_mode = std::env::args_os()
        .nth(1)
        .map(|arg| arg == "describe")
        .unwrap_or(false);
    let hostname = std::fs::read_to_string("/etc/hostname")
        .map(|text| text.trim().to_string())
        .unwrap_or_default();
    let credentials: Vec<&str> = ORCHESTRATOR_CREDENTIALS
        .iter()
        .copied()
        .filter(|name| std::env::var_os(name).is_some())
        .collect();
    serde_json::json!({
        "moment": moment,
        "process": process,
        "pid": std::process::id(),
        "describe_mode": describe_mode,
        "hostname": hostname,
        "orchestrator_credentials_in_environment": credentials,
    })
}

fn emit(marker: &str, facts: &serde_json::Value) {
    // Never panic here: a write error is dropped.
    let _ = writeln!(std::io::stderr(), "{marker} {facts}");
}

extern "C" fn on_load() {
    let here = facts("load");
    emit(LOAD_MARK, &here);
    let _ = LOADED.set(here);
}

// The load-time code: the loader calls each entry of the init array when it
// opens the library, before any symbol of the library is called.
#[used]
#[cfg_attr(target_os = "linux", unsafe(link_section = ".init_array"))]
#[cfg_attr(target_os = "macos", unsafe(link_section = "__DATA,__mod_init_func"))]
static ON_LOAD: extern "C" fn() = on_load;

#[trigger(on = "tenant_code_mark", poll_interval = "60s")]
pub async fn tenant_code_mark_poll() -> Result<TriggerResult, cloacina_workflow::TriggerError> {
    let here = facts("poll");
    emit(POLL_MARK, &here);
    let mut context = Context::new();
    let _ = context.insert("poll_mark", here);
    Ok(TriggerResult::Fire(Some(context)))
}

#[workflow(
    name = "tenant_code_mark",
    description = "tenant_code chapter fixture: load-time code, a task and a poll trigger leave marks",
    author = "tenant-code"
)]
pub mod tenant_code_mark {
    use super::*;

    #[task(retry_attempts = 0)]
    pub async fn record_where(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let here = facts("task");
        emit(TASK_MARK, &here);
        context.insert("task_mark", here)?;
        context.insert(
            "load_mark_in_this_process",
            LOADED.get().cloned().unwrap_or(serde_json::Value::Null),
        )?;
        Ok(())
    }
}
