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

// Demo trigger fan-out — publisher half (CLOACI-T-0777).
//
// `settlement_close` is a MANUAL-ONLY trigger: its poll body always returns
// `Skip`, so the scheduler never fires it on its own. The only way it fires is
// an operator "fire" (POST /tenants/{t}/triggers/settlement_close/fire), which
// fans out to every workflow subscribed to it. This package provides the trigger
// and one subscriber (`settle_ledger`); `demo-fanout-sub-rust` adds a second
// subscriber (`settle_audit`) by listing the same trigger in its `triggers = […]`.
//
// Each subscriber declares its own typed params, so the trigger's pass-through
// schema (GET …/triggers/settlement_close/interface) is the UNION of both — the
// operator fills one typed form and it reaches both DAGs.

use cloacina_macros::{task, trigger, workflow};
use cloacina_workflow::{Context, TaskError, TriggerResult};

cloacina_workflow_plugin::package!();

/// Manual-only trigger: poll always Skips → fires only on an operator fire.
#[trigger(on = "settle_ledger", poll_interval = "3600s")]
pub async fn settlement_close() -> Result<TriggerResult, cloacina_workflow::TriggerError> {
    Ok(TriggerResult::Skip)
}

#[workflow(
    name = "settle_ledger",
    description = "Settlement ledger leg — posts the day's close. Fans out from settlement_close.",
    triggers = ["settlement_close"],
    params(
        region: String = "us-east",
        as_of_date: String = "2026-06-22",
    )
)]
pub mod ledger_wf {
    use super::*;

    #[task]
    pub async fn post_ledger(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let _ = context;
        Ok(())
    }
}
