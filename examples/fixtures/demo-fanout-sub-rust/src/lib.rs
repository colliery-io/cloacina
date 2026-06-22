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

// Demo trigger fan-out — subscriber half (CLOACI-T-0777).
//
// `settle_audit` is a second workflow subscribed to the `settlement_close`
// trigger declared in `demo-fanout-rust`. It declares no trigger itself — only
// the subscription via `triggers = ["settlement_close"]`. When an operator fires
// settlement_close, the fan-out runs BOTH settle_ledger (publisher package) and
// settle_audit (this package).
//
// Its declared params (`region`, `deep_scan`) differ from settle_ledger's, so the
// trigger's pass-through interface is the UNION of the two — the operator's typed
// fire form carries region + as_of_date + deep_scan.

use cloacina_macros::{task, workflow};
use cloacina_workflow::{Context, TaskError};

cloacina_workflow_plugin::package!();

#[workflow(
    name = "settle_audit",
    description = "Settlement audit leg — reconciles the close. Fans out from settlement_close.",
    triggers = ["settlement_close"],
    params(
        region: String = "us-east",
        deep_scan: bool = false,
    )
)]
pub mod audit_wf {
    use super::*;

    #[task]
    pub async fn reconcile(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let _ = context;
        Ok(())
    }
}
