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

//! Demo Kafka-stream computation-graph fixture (CLOACI-T-0676).
//!
//! A reactor-bound computation graph whose entry accumulator `kafka_alpha` is
//! fed from a Kafka topic. The macro declares `kafka_alpha` as a plain
//! accumulator; `package.toml`'s `[[metadata.accumulators]]` block upgrades it
//! to a `stream` accumulator with a Kafka `broker`/`topic`/`group` config (the
//! broker resolves via `{{ KAFKA_BROKER }}` → `CLOACINA_VAR_KAFKA_BROKER`). This
//! is the demo + soak's Kafka-ingest exercise: produce to the topic → the
//! stream accumulator fires the reactor → the graph runs.

use cloacina_macros::reactor;
use cloacina_workflow_plugin as cloacina_plugin;
use serde::{Deserialize, Serialize};

cloacina_plugin::package!();

/// Reactor the graph binds to. `kafka_alpha` becomes a Kafka stream source via
/// the `package.toml` accumulator override.
#[reactor(
    name = "demo_kafka_rx",
    accumulators = [kafka_alpha],
    criteria = when_any(kafka_alpha),
)]
pub struct DemoKafkaRx;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputData {
    pub result: f64,
}

#[cloacina_macros::computation_graph(
    trigger = reactor("demo_kafka_rx"),
    graph = {
        process(kafka_alpha) -> output,
    }
)]
pub mod demo_kafka_graph {
    use super::*;

    pub async fn process(kafka_alpha: Option<&EventData>) -> OutputData {
        OutputData {
            result: kafka_alpha.map(|e| e.value * 2.0).unwrap_or(0.0),
        }
    }

    pub async fn output(input: &OutputData) -> OutputData {
        input.clone()
    }
}
