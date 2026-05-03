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

//! Mixed-primitive fixture (T-0553 / I-0102 T-D): one cdylib that
//! exercises every primitive the unified shell macro covers — reactor,
//! custom-poll trigger, reactor-bound computation graph, and a workflow
//! that subscribes to the trigger via `#[workflow(triggers = […])]`.
//!
//! Used by the precedence-pipeline integration tests to verify cron-vs-
//! custom routing, reactor-bound CG dispatch, and workflow → trigger
//! binding all coexist in a single cdylib.

use cloacina_macros::{reactor, task, trigger, workflow};
use cloacina_workflow::{Context, TaskError, TriggerResult};
use serde::{Deserialize, Serialize};

cloacina_workflow_plugin::package!();

// --- Reactor (one accumulator) ---

#[reactor(
    name = "mixed_reactor",
    accumulators = [alpha],
    criteria = when_any(alpha),
)]
pub struct MixedReactor;

// --- Custom-poll trigger ---

#[trigger(on = "mixed_workflow", poll_interval = "5s")]
pub async fn mixed_trigger() -> Result<TriggerResult, cloacina_workflow::TriggerError> {
    Ok(TriggerResult::Skip)
}

// --- Boundary type ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlphaIn {
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactorOutput {
    pub doubled: f64,
}

// --- Reactor-bound CG ---

#[cloacina_macros::computation_graph(
    trigger = reactor("mixed_reactor"),
    graph = {
        compute(alpha) -> output,
    }
)]
pub mod mixed_graph {
    use super::*;

    pub async fn compute(alpha: Option<&AlphaIn>) -> ReactorOutput {
        ReactorOutput {
            doubled: alpha.map(|a| a.value * 2.0).unwrap_or(0.0),
        }
    }

    pub async fn output(input: &ReactorOutput) -> ReactorOutput {
        input.clone()
    }
}

// --- Workflow subscribing to the trigger ---

#[workflow(
    name = "mixed_workflow",
    description = "Mixed-primitive fixture workflow",
    triggers = ["mixed_trigger"]
)]
pub mod mixed_wf {
    use super::*;

    #[task(id = "mixed_step", dependencies = [])]
    pub async fn mixed_step(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let _ = context;
        Ok(())
    }
}
