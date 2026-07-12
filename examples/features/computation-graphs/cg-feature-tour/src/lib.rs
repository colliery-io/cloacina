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

/*!
# Computation-Graph Feature Tour

Three surfaces in one package, all through the primary interface:

1. **Kafka stream accumulator** — the `ticks` accumulator is upgraded to a
   Kafka stream source by `package.toml` (`[[metadata.accumulators]]`); each
   message on the topic fires the reactor and runs `tour_stream_graph`.
2. **Typed inject/fire** — `Tick` derives `schemars::JsonSchema`, giving the
   accumulator a typed inject form (`cloacinactl accumulator inject`) and the
   reactor a typed fire form.
3. **Task→graph invocation** — the `tour_pipeline` workflow's `crunch` task
   `invokes` the trigger-less `tour_math_graph`; terminal outputs merge back
   into the task's context, and a `post_invocation` hook sees the merged
   result.
*/

use cloacina_macros::{computation_graph, reactor, task, workflow};
use cloacina_workflow::{Context, TaskError};
use serde::{Deserialize, Serialize};

cloacina_workflow_plugin::package!();

// ---------------------------------------------------------------------------
// Surface 1 + 2: a Kafka-fed reactor-bound graph with a typed boundary
// ---------------------------------------------------------------------------

/// One market tick. Deriving `JsonSchema` opts the accumulator into the TYPED
/// inject/fire form — the server validates injected events against this shape.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Tick {
    pub price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedTick {
    pub price: f64,
    pub spread_bps: f64,
}

/// Fires on every tick. `ticks` is a plain accumulator here; `package.toml`
/// upgrades it to a Kafka `stream` source (broker/topic/group).
#[reactor(
    name = "tour_rx",
    accumulators = [ticks],
    criteria = when_any(ticks),
)]
pub struct TourRx;

#[computation_graph(
    trigger = reactor("tour_rx"),
    graph = {
        enrich(ticks) -> emit,
    }
)]
pub mod tour_stream_graph {
    use super::*;

    pub async fn enrich(ticks: Option<&Tick>) -> EnrichedTick {
        let price = ticks.map(|t| t.price).unwrap_or(0.0);
        EnrichedTick {
            price,
            spread_bps: price * 0.0005,
        }
    }

    pub async fn emit(input: &EnrichedTick) -> EnrichedTick {
        input.clone()
    }
}

// ---------------------------------------------------------------------------
// Surface 3: a trigger-less graph a workflow task INVOKES
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Normalized {
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathResult {
    pub value: f64,
    pub squared: f64,
}

/// No `trigger = …`: this graph is trigger-less, which is exactly what makes
/// it invocable from a task (`invokes = computation_graph(...)` refuses
/// reactor-driven graphs at compile time).
#[computation_graph(graph = {
    normalize -> output,
})]
pub mod tour_math_graph {
    use super::*;

    /// Entry node reads the invoking task's context directly.
    pub async fn normalize(ctx: &Context<serde_json::Value>) -> Normalized {
        let raw = ctx.get("raw_value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        Normalized { value: raw / 100.0 }
    }

    pub async fn output(input: &Normalized) -> MathResult {
        MathResult {
            value: input.value,
            squared: input.value * input.value,
        }
    }
}

// ---------------------------------------------------------------------------
// The workflow that invokes it
// ---------------------------------------------------------------------------

/// `post_invocation` hook: runs after the graph, sees the merged context
/// (terminal outputs land under their node names — here, `output`).
async fn summarize(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let squared = context
        .get("output")
        .and_then(|o| o.get("squared"))
        .and_then(|v| v.as_f64())
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "graph terminal `output` missing from merged context".to_string(),
        })?;
    context.insert("summary", serde_json::json!({ "squared": squared }))?;
    Ok(())
}

#[workflow(
    name = "tour_pipeline",
    description = "Workflow that invokes a trigger-less computation graph",
    author = "Cloacina Demo Team"
)]
pub mod tour_pipeline {
    use super::*;

    /// Seed the value the graph's entry node reads.
    #[task]
    pub async fn prep(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("raw_value", serde_json::json!(250.0))?;
        Ok(())
    }

    /// Pre-work runs first, then the macro-generated invocation of
    /// `tour_math_graph`, then the `summarize` post-hook.
    #[task(
        dependencies = ["prep"],
        invokes = computation_graph("tour_math_graph"),
        post_invocation = summarize,
    )]
    pub async fn crunch(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("crunch_ran", serde_json::json!(true))?;
        Ok(())
    }

    /// Consume the merged result downstream.
    #[task(dependencies = ["crunch"])]
    pub async fn report(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let summary =
            context
                .get("summary")
                .cloned()
                .ok_or_else(|| TaskError::ValidationFailed {
                    message: "missing summary from post_invocation hook".to_string(),
                })?;
        println!("📊 tour_pipeline result: {summary}");
        context.insert("tour_report", summary)?;
        Ok(())
    }
}
