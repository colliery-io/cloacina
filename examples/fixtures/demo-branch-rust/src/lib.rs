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

// Demo branch fixture (CLOACI-T-0719): exercises the SKIPPED task state so the
// execution DAG shows a skipped (dashed) node on every run.
//
//   decide ──┬─▶ branch_a   (runs:  take_branch_a == true)
//            └─▶ branch_b   (SKIPPED: its rule wants take_branch_a == false)
//                              │
//   branch_a, branch_b ───────▶ merge
//
// `decide` always sets `take_branch_a = true`, so `branch_b`'s trigger rule is
// never satisfied → the scheduler marks it Skipped (deps satisfied, rule fails).
// `merge` depends on both; a Skipped dependency counts as resolved, so merge
// still runs. Net: decide/branch_a/merge complete (green), branch_b skips.

use cloacina_workflow::{task, workflow, Context, TaskError};

cloacina_workflow_plugin::package!();

#[workflow(
    name = "demo_branch_workflow",
    description = "demo branch workflow — a gated branch that skips one path each run",
    author = "cloacina-ui-demo",
    // CLOACI-T-0768: declared injectors (typed execute-time params).
    params(
        branch_key: String,
        threshold: u32 = 50,
        take_all: bool = false,
    )
)]
pub mod demo_branch_workflow {
    use super::*;

    // Blocking sleep on purpose (cdylib statically-linked tokio — same lesson as
    // the other demo fixtures), kept short so the run is watchable but quick.
    fn pause(secs: u64) {
        std::thread::sleep(std::time::Duration::from_secs(secs));
    }

    #[task(retry_attempts = 0)]
    pub async fn decide(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(1);
        // Always take branch A → branch_b's rule (== false) fails → Skipped.
        context.insert("take_branch_a", serde_json::json!(true))?;
        Ok(())
    }

    #[task(
        dependencies = ["decide"],
        retry_attempts = 0,
        trigger_rules = context_value("take_branch_a", equals, true)
    )]
    pub async fn branch_a(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(2);
        context.insert("branch_a_done", serde_json::json!(true))?;
        Ok(())
    }

    #[task(
        dependencies = ["decide"],
        retry_attempts = 0,
        trigger_rules = context_value("take_branch_a", equals, false)
    )]
    pub async fn branch_b(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        // Never executes — gated off every run. Kept so the DAG has the node.
        context.insert("branch_b_done", serde_json::json!(true))?;
        Ok(())
    }

    #[task(dependencies = ["branch_a", "branch_b"], retry_attempts = 0)]
    pub async fn merge(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        pause(1);
        context.insert("demo_branch_complete", serde_json::json!(true))?;
        Ok(())
    }
}
