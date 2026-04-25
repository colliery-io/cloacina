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

// T-0540 compile-fail: a workflow task cannot bind to a reactor-triggered
// computation graph. The macro emits `<H as TriggerlessGraph>::compiled_fn()`,
// and reactor-triggered graphs do NOT implement `TriggerlessGraph` — only the
// trigger-less form does. The expected diagnostic is "trait bound `... :
// TriggerlessGraph` is not satisfied".

use cloacina::{computation_graph, reactor, task, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlphaData {
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedData {
    pub result: f64,
}

#[reactor(
    name = "trybuild_reactor",
    accumulators = [alpha],
    criteria = when_any(alpha),
)]
pub struct TrybuildReactor;

#[computation_graph(
    trigger = reactor(TrybuildReactor),
    graph = {
        entry(alpha) -> output,
    }
)]
pub mod trybuild_split_graph {
    use super::*;

    pub async fn entry(alpha: Option<&AlphaData>) -> ProcessedData {
        ProcessedData {
            result: alpha.map(|a| a.value).unwrap_or(0.0),
        }
    }

    pub async fn output(input: &ProcessedData) -> ProcessedData {
        ProcessedData {
            result: input.result,
        }
    }
}

#[task(
    id = "trybuild_invoker",
    invokes = computation_graph(__CGHandle_trybuild_split_graph),
)]
async fn trybuild_invoker(
    _context: &mut Context<Value>,
) -> Result<(), cloacina_workflow::TaskError> {
    Ok(())
}

fn main() {}
