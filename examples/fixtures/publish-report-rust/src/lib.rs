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


// Fixture of the publishing behaviour spec (CLOACINA-T-1026): team A's report
// job, version 0.1.0. One task, and a cron trigger that fires the workflow
// every 10 seconds, so the spec can see the schedule start after a publish.

use cloacina_macros::{task, trigger, workflow};
use cloacina_workflow::{Context, TaskError};

cloacina_workflow_plugin::package!();

#[trigger(on = "publish_report", cron = "*/10 * * * * *")]
pub async fn publish_report_schedule() {}

#[workflow(
    name = "publish_report",
    description = "publishing spec fixture: report job on a schedule",
    author = "team-a"
)]
pub mod publish_report {
    use super::*;

    #[task(retry_attempts = 0)]
    pub async fn build_report(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("report_version", serde_json::json!("0.1.0"))?;
        Ok(())
    }
}
