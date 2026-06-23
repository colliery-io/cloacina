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

// Acme Corp demo fixture (CLOACI-T-0779). Seeded only into the `acme` tenant to
// demonstrate tenant isolation — distinctly named so it's obvious you're looking
// at a different company's workflows than `public`.

use cloacina_macros::{task, workflow};
use cloacina_workflow::{Context, TaskError};

cloacina_workflow_plugin::package!();

#[workflow(
    name = "acme_billing",
    description = "Acme Corp billing run — generate and dispatch customer invoices.",
    params(
        billing_cycle: String = "monthly",
        dry_run: bool = false,
    )
)]
pub mod acme_billing_wf {
    use super::*;

    #[task]
    pub async fn post_invoices(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let _ = context;
        Ok(())
    }
}
